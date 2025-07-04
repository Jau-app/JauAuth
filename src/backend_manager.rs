//! Backend MCP server process management

use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{info, warn, error, debug};

use crate::simple_router::{BackendServer};
use crate::sandbox::{build_sandbox_command};

// Re-export validation functions
pub use crate::simple_router::{ALLOWED_COMMANDS, is_command_allowed, validate_shell_safety};

/// Represents a running backend MCP server
pub struct BackendProcess {
    /// Server configuration
    pub server: BackendServer,
    /// Child process handle
    child: Child,
    /// Stdin for sending requests
    stdin: ChildStdin,
    /// Buffered reader for stdout
    stdout_reader: BufReader<ChildStdout>,
    /// Next request ID
    next_id: Arc<RwLock<u64>>,
    /// Tools exposed by this backend
    pub tools: Vec<Value>,
    /// Whether the backend is healthy
    pub healthy: bool,
}

impl BackendProcess {
    /// Get the next request ID
    async fn next_request_id(&self) -> u64 {
        let mut id = self.next_id.write().await;
        let current = *id;
        *id += 1;
        current
    }
    
    /// Send a JSON-RPC request and wait for response
    pub async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_request_id().await;
        
        // Build JSON-RPC request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        
        debug!("Sending request to {}: {}", self.server.id, request);
        
        // Send request
        let request_str = format!("{}\n", request);
        self.stdin.write_all(request_str.as_bytes()).await
            .map_err(|e| anyhow!("Failed to write to backend {}: {}", self.server.id, e))?;
        self.stdin.flush().await?;
        
        // Read response
        let mut line = String::new();
        self.stdout_reader.read_line(&mut line).await
            .map_err(|e| anyhow!("Failed to read from backend {}: {}", self.server.id, e))?;
        
        if line.is_empty() {
            return Err(anyhow!("Backend {} closed connection", self.server.id));
        }
        
        debug!("Received response from {}: {}", self.server.id, line.trim());
        
        // Parse response
        let response: Value = serde_json::from_str(&line)
            .map_err(|e| anyhow!("Invalid JSON from backend {}: {}", self.server.id, e))?;
        
        // Check for error
        if let Some(error) = response.get("error") {
            return Err(anyhow!("Backend {} error: {}", self.server.id, error));
        }
        
        // Extract result
        response.get("result")
            .cloned()
            .ok_or_else(|| anyhow!("No result in response from backend {}", self.server.id))
    }
    
    /// Initialize the MCP connection
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing backend: {}", self.server.id);
        
        // Send initialize request
        let init_params = serde_json::json!({
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "JauAuth Router",
                "version": "1.0.0"
            }
        });
        
        let result = self.send_request("initialize", init_params).await?;
        
        // Validate response has required fields
        if !result.is_object() || !result.get("protocolVersion").is_some() {
            return Err(anyhow!("Invalid initialize response from backend {}", self.server.id));
        }
        
        info!("Backend {} initialized successfully", self.server.id);
        
        // Send initialized notification
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        
        let notification_str = format!("{}\n", notification);
        self.stdin.write_all(notification_str.as_bytes()).await?;
        self.stdin.flush().await?;
        
        Ok(())
    }
    
    /// Get the list of tools from this backend
    pub async fn list_tools(&mut self) -> Result<Vec<Value>> {
        let result = self.send_request("tools/list", serde_json::json!({})).await?;
        
        let tools = result.get("tools")
            .and_then(|t| t.as_array())
            .ok_or_else(|| anyhow!("Invalid tools response from backend {}", self.server.id))?;
        
        // Prefix tool names with server ID
        let prefixed_tools: Vec<Value> = tools.iter()
            .map(|tool| {
                let mut tool_copy = tool.clone();
                if let Some(name) = tool_copy.get("name").and_then(|n| n.as_str()) {
                    tool_copy["name"] = serde_json::json!(format!("{}:{}", self.server.id, name));
                }
                tool_copy
            })
            .collect();
        
        self.tools = prefixed_tools.clone();
        Ok(prefixed_tools)
    }
    
    /// Call a tool on this backend
    pub async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<Value> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        });
        
        self.send_request("tools/call", params).await
    }
    
    /// Check if the backend is still healthy
    pub async fn health_check(&mut self) -> bool {
        // Try a simple ping or tools/list request
        match self.send_request("tools/list", serde_json::json!({})).await {
            Ok(_) => {
                self.healthy = true;
                true
            }
            Err(e) => {
                warn!("Health check failed for backend {}: {}", self.server.id, e);
                self.healthy = false;
                false
            }
        }
    }
    
    /// Gracefully shutdown the backend
    pub async fn shutdown(mut self) -> Result<()> {
        info!("Shutting down backend: {}", self.server.id);
        
        // Try to send a nice shutdown signal first
        if let Err(e) = self.stdin.write_all(b"\n").await {
            debug!("Failed to send EOF to backend {}: {}", self.server.id, e);
        }
        
        // Give it a moment to exit cleanly
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Then kill if needed
        if let Err(e) = self.child.kill().await {
            debug!("Failed to kill backend {}: {}", self.server.id, e);
        }
        
        Ok(())
    }
}

/// Manages all backend MCP server processes
pub struct BackendManager {
    backends: Arc<RwLock<HashMap<String, BackendProcess>>>,
}

impl BackendManager {
    pub fn new() -> Self {
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Spawn a new backend server
    pub async fn spawn_backend(&self, server: BackendServer) -> Result<()> {
        info!("Spawning backend server: {} ({})", server.name, server.id);
        
        // Build sandbox command
        let mut cmd = build_sandbox_command(
            &server.sandbox,
            &server.command,
            &server.args,
            &server.env
        )?;
        
        // Configure for MCP stdio communication
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        
        // Spawn the process
        let mut child = cmd.spawn()
            .map_err(|e| anyhow!("Failed to spawn backend {}: {}", server.id, e))?;
        
        // Get stdin/stdout handles
        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow!("Failed to get stdin for backend {}", server.id))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow!("Failed to get stdout for backend {}", server.id))?;
        
        let stdout_reader = BufReader::new(stdout);
        
        // Create backend process
        let mut backend = BackendProcess {
            server: server.clone(),
            child,
            stdin,
            stdout_reader,
            next_id: Arc::new(RwLock::new(1)),
            tools: vec![],
            healthy: false,
        };
        
        // Initialize the connection
        backend.initialize().await?;
        
        // Get initial tool list
        backend.list_tools().await?;
        
        backend.healthy = true;
        
        info!("Backend {} spawned successfully with {} tools", 
              server.id, backend.tools.len());
        
        // Store the backend
        let mut backends = self.backends.write().await;
        backends.insert(server.id.clone(), backend);
        
        Ok(())
    }
    
    /// Get all available tools from all backends
    pub async fn get_all_tools(&self) -> Vec<Value> {
        let backends = self.backends.read().await;
        let mut all_tools = Vec::new();
        
        for backend in backends.values() {
            all_tools.extend(backend.tools.clone());
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
        
        // Route the call
        backend.call_tool(tool_name, arguments).await
    }
    
    /// Shutdown all backends
    pub async fn shutdown_all(&self) -> Result<()> {
        info!("Shutting down all backends");
        
        let mut backends = self.backends.write().await;
        let backend_list: Vec<_> = backends.drain().map(|(_, v)| v).collect();
        
        for backend in backend_list {
            if let Err(e) = backend.shutdown().await {
                error!("Error shutting down backend: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Get status of all backends
    pub async fn get_status(&self) -> HashMap<String, bool> {
        let backends = self.backends.read().await;
        backends.iter()
            .map(|(id, backend)| (id.clone(), backend.healthy))
            .collect()
    }
}