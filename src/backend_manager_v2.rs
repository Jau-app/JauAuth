//! Backend MCP server management with support for both local and remote servers

use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use std::time::Duration;

use crate::simple_router::{BackendServer, ServerType};
use crate::sandbox::build_sandbox_command;
use crate::transport::{Transport, create_transport, TransportConfig, AuthConfig as TransportAuth, RetryConfig, TlsConfig};
use crate::mcp_types::{Tool};

// Re-export validation functions
pub use crate::simple_router::{ALLOWED_COMMANDS, is_command_allowed, validate_shell_safety};

/// Represents a backend MCP server (either local or remote)
pub enum BackendHandle {
    Local {
        server: BackendServer,
        transport: Box<dyn Transport>,
        tools: Vec<Value>,
        healthy: bool,
    },
    Remote {
        server: BackendServer,
        transport: Box<dyn Transport>,
        tools: Vec<Value>,
        healthy: bool,
    }
}

impl BackendHandle {
    /// Get server configuration
    pub fn server(&self) -> &BackendServer {
        match self {
            BackendHandle::Local { server, .. } => server,
            BackendHandle::Remote { server, .. } => server,
        }
    }
    
    /// Get tools list
    pub fn tools(&self) -> &[Value] {
        match self {
            BackendHandle::Local { tools, .. } => tools,
            BackendHandle::Remote { tools, .. } => tools,
        }
    }
    
    /// Check if healthy
    pub fn is_healthy(&self) -> bool {
        match self {
            BackendHandle::Local { healthy, .. } => *healthy,
            BackendHandle::Remote { healthy, .. } => *healthy,
        }
    }
    
    /// Set health status
    pub fn set_healthy(&mut self, status: bool) {
        match self {
            BackendHandle::Local { healthy, .. } => *healthy = status,
            BackendHandle::Remote { healthy, .. } => *healthy = status,
        }
    }
    
    /// Set tools list
    pub fn set_tools(&mut self, new_tools: Vec<Value>) {
        match self {
            BackendHandle::Local { tools, .. } => *tools = new_tools,
            BackendHandle::Remote { tools, .. } => *tools = new_tools,
        }
    }
    
    /// Initialize the MCP connection
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing backend: {}", self.server().id);
        
        let client_info = serde_json::json!({
            "name": "JauAuth Router",
            "version": "1.0.0"
        });
        
        match self {
            BackendHandle::Local { transport, server, .. } |
            BackendHandle::Remote { transport, server, .. } => {
                let result = transport.initialize(client_info).await
                    .map_err(|e| anyhow!("Failed to initialize {}: {}", server.id, e))?;
                
                info!("Backend {} initialized with protocol version: {}", 
                      server.id, result.protocol_version);
                
                self.set_healthy(true);
                Ok(())
            }
        }
    }
    
    /// Get the list of tools from this backend
    pub async fn list_tools(&mut self) -> Result<Vec<Value>> {
        let server_id = self.server().id.clone();
        
        match self {
            BackendHandle::Local { transport, .. } |
            BackendHandle::Remote { transport, .. } => {
                let result = transport.list_tools().await
                    .map_err(|e| anyhow!("Failed to list tools from {}: {}", server_id, e))?;
                
                // Prefix tool names with server ID
                let prefixed_tools: Vec<Value> = result.tools.into_iter()
                    .map(|tool| {
                        let prefixed_tool = Tool {
                            name: format!("{}:{}", server_id, tool.name),
                            description: tool.description,
                            input_schema: tool.input_schema,
                        };
                        serde_json::to_value(prefixed_tool).unwrap()
                    })
                    .collect();
                
                self.set_tools(prefixed_tools.clone());
                Ok(prefixed_tools)
            }
        }
    }
    
    /// Call a tool on this backend
    pub async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<Value> {
        match self {
            BackendHandle::Local { transport, server, .. } |
            BackendHandle::Remote { transport, server, .. } => {
                transport.call_tool(tool_name, arguments).await
                    .map_err(|e| anyhow!("Tool call failed on {}: {}", server.id, e))
            }
        }
    }
    
    /// Check if the backend is still healthy
    pub async fn health_check(&mut self) -> bool {
        let server_id = self.server().id.clone();
        debug!("Performing health check for server {}", server_id);
        
        match self {
            BackendHandle::Local { transport, .. } |
            BackendHandle::Remote { transport, .. } => {
                match transport.health_check().await {
                    Ok(healthy) => {
                        debug!("Health check for server {} returned: {}", server_id, healthy);
                        self.set_healthy(healthy);
                        healthy
                    }
                    Err(e) => {
                        warn!("Health check failed for server {}: {}", server_id, e);
                        self.set_healthy(false);
                        false
                    }
                }
            }
        }
    }
    
    /// Gracefully shutdown the backend
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down backend: {}", self.server().id);
        
        match self {
            BackendHandle::Local { mut transport, server, .. } => {
                // Shutdown transport (this will kill the process)
                transport.shutdown().await
                    .map_err(|e| anyhow!("Local shutdown failed for {}: {}", server.id, e))
            }
            BackendHandle::Remote { mut transport, server, .. } => {
                transport.shutdown().await
                    .map_err(|e| anyhow!("Remote shutdown failed for {}: {}", server.id, e))
            }
        }
    }
}

/// Manages all backend MCP server processes
pub struct BackendManager {
    backends: Arc<RwLock<HashMap<String, BackendHandle>>>,
}

impl BackendManager {
    pub fn new() -> Self {
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Spawn a new backend server (local or remote)
    pub async fn spawn_backend(&self, server: BackendServer) -> Result<()> {
        info!("Spawning backend server: {} ({}) - Type: {:?}", 
              server.name, server.id, server.r#type);
        
        // Validate auth requirements if enabled
        if server.requires_auth {
            self.validate_auth_config(&server)?;
        }
        
        let backend = match server.r#type {
            ServerType::Local => {
                // Validate local server configuration
                let command = server.command.as_ref()
                    .ok_or_else(|| anyhow!("Local server {} missing command", server.id))?;
                
                // Build sandbox command
                debug!("Building sandbox command for server {} with strategy: {:?}", 
                       server.id, server.sandbox.strategy);
                let mut cmd = build_sandbox_command(
                    &server.sandbox,
                    command,
                    &server.args,
                    &server.env
                )?;
                
                // Configure for MCP stdio communication
                cmd.stdin(std::process::Stdio::piped());
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());
                
                // Special handling for npx commands - check if package exists
                if command == "npx" {
                    if let Some(package_name) = server.args.first() {
                        // Check if package is installed globally
                        let check_cmd = std::process::Command::new("npm")
                            .arg("list")
                            .arg("-g")
                            .arg(package_name)
                            .arg("--depth=0")
                            .output();
                        
                        if check_cmd.is_err() || !check_cmd.unwrap().status.success() {
                            return Err(anyhow!(
                                "NPX package '{}' not found. Install with: npm install -g {}",
                                package_name, package_name
                            ));
                        }
                    }
                }
                
                // Spawn the process
                debug!("Spawning process for server {}: {:?}", server.id, cmd);
                let child = cmd.spawn()
                    .map_err(|e| {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            if command == "npx" {
                                anyhow!("Command 'npx' not found. Please install Node.js from https://nodejs.org/")
                            } else {
                                anyhow!("Command '{}' not found. Please ensure it's installed and in your PATH", command)
                            }
                        } else {
                            anyhow!("Failed to spawn backend {}: {}", server.id, e)
                        }
                    })?;
                debug!("Successfully spawned process for server {}", server.id);
                
                // Create stdio transport from the spawned child
                let transport = Box::new(crate::transport::stdio::StdioTransport::from_child(
                    child,
                    server.id.clone(),
                )?);
                
                BackendHandle::Local {
                    server,
                    transport,
                    tools: vec![],
                    healthy: false,
                }
            }
            ServerType::Remote => {
                // Validate remote server configuration
                let url = server.url.as_ref()
                    .ok_or_else(|| anyhow!("Remote server {} missing URL", server.id))?;
                
                // Convert auth config
                let auth = match &server.auth {
                    Some(auth) => match auth {
                        crate::simple_router::RemoteAuthConfig::None => TransportAuth::None,
                        crate::simple_router::RemoteAuthConfig::Bearer { token } => 
                            TransportAuth::Bearer { token: token.clone() },
                        crate::simple_router::RemoteAuthConfig::Basic { username, password } => 
                            TransportAuth::Basic { username: username.clone(), password: password.clone() },
                        crate::simple_router::RemoteAuthConfig::OAuth { provider, client_id, client_secret, scopes } => 
                            TransportAuth::OAuth { 
                                provider: provider.clone(),
                                client_id: client_id.clone(),
                                client_secret: client_secret.clone(),
                                scopes: scopes.clone()
                            },
                        crate::simple_router::RemoteAuthConfig::Custom { headers } => 
                            TransportAuth::Custom { headers: headers.clone() },
                    },
                    None => TransportAuth::None,
                };
                
                // Create transport configuration
                let transport_config = TransportConfig::Remote {
                    url: url.clone(),
                    auth,
                    timeout_ms: server.timeout_ms,
                    retry: server.retry.clone().unwrap_or_default().into(),
                    tls: server.tls.clone().unwrap_or_default().into(),
                };
                
                // Create transport based on type
                debug!("Creating transport for remote server {}", server.id);
                let transport = create_transport(transport_config).await?;
                
                BackendHandle::Remote {
                    server,
                    transport,
                    tools: vec![],
                    healthy: false,
                }
            }
        };
        
        // Initialize and get tools
        let mut backend = backend;
        backend.initialize().await?;
        
        // Store the backend first so we can track progress
        let server_id = backend.server().id.clone();
        let mut backends = self.backends.write().await;
        backends.insert(server_id.clone(), backend);
        drop(backends); // Release the lock
        
        // Now try to get tools with progress tracking
        info!("Getting tools from {} (this may take time on first run)...", server_id);
        
        // Spawn a progress tracking task
        let progress_handle = {
            let server_id = server_id.clone();
            tokio::spawn(async move {
                let mut elapsed = 0;
                loop {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    elapsed += 5;
                    info!("Still waiting for {} to list tools... ({}s elapsed)", server_id, elapsed);
                    
                    // For NPX servers, show additional info about potential npm install
                    if elapsed == 15 {
                        info!("{}: If this is the first run, npm packages are being downloaded...", server_id);
                    }
                    if elapsed == 30 {
                        info!("{}: Large dependencies may take time to install. Please wait...", server_id);
                    }
                }
            })
        };
        
        // Try to get tools
        let tools_result = {
            let mut backends = self.backends.write().await;
            if let Some(backend) = backends.get_mut(&server_id) {
                backend.list_tools().await
            } else {
                Err(anyhow!("Backend {} disappeared during initialization", server_id))
            }
        };
        
        // Stop the progress tracking
        progress_handle.abort();
        
        match tools_result {
            Ok(_) => {
                let backends = self.backends.read().await;
                if let Some(backend) = backends.get(&server_id) {
                    info!("Backend {} ready with {} tools", server_id, backend.tools().len());
                }
            }
            Err(e) => {
                // Tools listing failed - but server is initialized
                warn!("Backend {} initialized but failed to list tools: {}. Will retry on next refresh.", 
                      server_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Get all available tools from all backends
    pub async fn get_all_tools(&self) -> Vec<Value> {
        let backends = self.backends.read().await;
        let mut all_tools = Vec::new();
        
        for backend in backends.values() {
            if backend.is_healthy() {
                all_tools.extend(backend.tools().to_vec());
            }
        }
        
        all_tools
    }
    
    /// Route a tool call to the appropriate backend
    pub async fn route_tool_call(&self, full_tool_name: &str, arguments: Value) -> Result<Value> {
        // Parse server_id:tool_name format
        let parts: Vec<&str> = full_tool_name.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid tool name format. Expected 'server_id:tool_name'"));
        }
        
        let server_id = parts[0];
        let tool_name = parts[1];
        
        // Get the backend
        let mut backends = self.backends.write().await;
        let backend = backends.get_mut(server_id)
            .ok_or_else(|| anyhow!("Backend '{}' not found", server_id))?;
        
        if !backend.is_healthy() {
            return Err(anyhow!("Backend '{}' is not healthy", server_id));
        }
        
        // Route the call
        backend.call_tool(tool_name, arguments).await
    }
    
    /// Route a tool call to the appropriate backend with optional timeout
    pub async fn route_tool_call_async(&self, full_tool_name: &str, arguments: Value, timeout: Option<Duration>) -> Result<Value> {
        // For now, ignore timeout and delegate to regular route_tool_call
        // TODO: Implement proper timeout handling with tokio::time::timeout
        if let Some(timeout_duration) = timeout {
            tracing::debug!("Tool call with timeout: {:?}", timeout_duration);
        }
        self.route_tool_call(full_tool_name, arguments).await
    }
    
    /// Shutdown all backends
    pub async fn shutdown_all(&self) -> Result<()> {
        info!("Shutting down all backends");
        
        let mut backends = self.backends.write().await;
        let backend_list: Vec<_> = backends.drain().map(|(_, v)| v).collect();
        
        for backend in backend_list {
            let server_id = backend.server().id.clone();
            if let Err(e) = backend.shutdown().await {
                error!("Error shutting down backend {}: {}", server_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Get status of all backends
    pub async fn get_status(&self) -> HashMap<String, bool> {
        let backends = self.backends.read().await;
        backends.iter()
            .map(|(id, backend)| (id.clone(), backend.is_healthy()))
            .collect()
    }
    
    /// Validate authentication configuration for servers that require auth
    fn validate_auth_config(&self, server: &BackendServer) -> Result<()> {
        let mut warnings = Vec::new();
        
        // Common auth token patterns
        let auth_env_patterns = ["TOKEN", "KEY", "SECRET", "PASSWORD", "API_KEY", "AUTH"];
        let auth_arg_patterns = ["--token", "--key", "--secret", "--auth", "--api-key", "Bearer"];
        
        // Check environment variables
        let mut has_auth_env = false;
        for (key, value) in &server.env {
            let key_upper = key.to_uppercase();
            let is_auth_field = auth_env_patterns.iter().any(|pattern| key_upper.contains(pattern));
            
            if is_auth_field {
                has_auth_env = true;
                if value.is_empty() {
                    warnings.push(format!("Environment variable '{}' is empty", key));
                }
                // Check for common placeholders
                if value.contains('<') || value.contains("YOUR_") || value.contains("_HERE") {
                    warnings.push(format!("Environment variable '{}' appears to contain a placeholder value", key));
                }
            }
        }
        
        // Check arguments for auth patterns
        let mut has_auth_args = false;
        for (i, arg) in server.args.iter().enumerate() {
            let is_auth_arg = auth_arg_patterns.iter().any(|pattern| 
                arg.to_lowercase().contains(&pattern.to_lowercase())
            );
            
            if is_auth_arg {
                has_auth_args = true;
                // Check if this is a flag at the end without a value
                if arg.starts_with("--") && i == server.args.len() - 1 {
                    warnings.push(format!("Argument '{}' may need a value", arg));
                }
                // Check next arg for placeholder
                else if i + 1 < server.args.len() {
                    let next_arg = &server.args[i + 1];
                    if next_arg.contains('<') || next_arg.contains("YOUR_") || 
                       next_arg.is_empty() || next_arg.to_lowercase().contains("token_here") {
                        warnings.push(format!("Argument '{}' appears to have a placeholder value", arg));
                    }
                }
            }
        }
        
        // Special validation for known servers
        match server.id.as_str() {
            "hf-mcp-server" => {
                if !server.env.contains_key("HF_TOKEN") || server.env.get("HF_TOKEN").map_or(true, |v| v.is_empty()) {
                    warnings.push("HuggingFace server requires HF_TOKEN environment variable".to_string());
                }
            }
            "github" | "github-mcp" => {
                if !server.env.contains_key("GITHUB_TOKEN") || server.env.get("GITHUB_TOKEN").map_or(true, |v| v.is_empty()) {
                    warnings.push("GitHub server requires GITHUB_TOKEN environment variable".to_string());
                }
            }
            _ => {}
        }
        
        // Log warnings but don't fail - let user decide
        if !warnings.is_empty() {
            warn!("Server '{}' has authentication configuration warnings:", server.id);
            for warning in &warnings {
                warn!("  - {}", warning);
            }
            info!("Server will attempt to start despite warnings. It may fail if authentication is actually required.");
        }
        
        // Log authentication configuration status
        debug!("Server '{}' authentication config: has_auth_env={}, has_auth_args={}", 
               server.id, has_auth_env, has_auth_args);
        
        if server.requires_auth && !has_auth_env && !has_auth_args {
            warn!("Server '{}' requires authentication but no auth configuration found", server.id);
        }
        
        Ok(())
    }
    
    /// Start health monitoring for all backends
    pub async fn start_health_monitor(&self) {
        let backends = self.backends.clone();
        
        tokio::spawn(async move {
            info!("Starting backend health monitor");
            
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                
                let mut backends = backends.write().await;
                for (id, backend) in backends.iter_mut() {
                    let was_healthy = backend.is_healthy();
                    let is_healthy = backend.health_check().await;
                    
                    if was_healthy && !is_healthy {
                        warn!("Backend {} became unhealthy", id);
                    } else if !was_healthy && is_healthy {
                        info!("Backend {} recovered", id);
                        // Refresh tools list on recovery
                        if let Err(e) = backend.list_tools().await {
                            error!("Failed to refresh tools for {}: {}", id, e);
                        }
                    }
                }
            }
        });
    }
}

// Helper trait implementations for config conversion
impl From<crate::simple_router::RetryConfig> for RetryConfig {
    fn from(config: crate::simple_router::RetryConfig) -> Self {
        RetryConfig {
            max_attempts: config.max_attempts,
            initial_backoff_ms: config.initial_backoff_ms,
            max_backoff_ms: config.max_backoff_ms,
        }
    }
}

impl From<crate::simple_router::TlsConfig> for TlsConfig {
    fn from(config: crate::simple_router::TlsConfig) -> Self {
        TlsConfig {
            verify_cert: config.verify_cert,
            ca_cert: config.ca_cert.map(std::path::PathBuf::from),
            client_cert: config.client_cert.map(std::path::PathBuf::from),
            client_key: config.client_key.map(std::path::PathBuf::from),
        }
    }
}