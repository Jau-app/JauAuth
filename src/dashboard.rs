//! Dashboard API for managing MCP servers and settings

use axum::{
    extract::{Path, State, Json, Extension},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tokio::fs;
use tracing::{info, error};

use crate::{
    AuthContext,
    simple_router::{BackendServer, RouterConfig},
    backend_manager::BackendManager,
    server_store::ServerStore,
    crypto::derive_key,
    session::Session,
};

/// Dashboard state shared across handlers
#[derive(Clone)]
pub struct DashboardState {
    pub auth_context: AuthContext,
    pub router_config: Arc<RwLock<RouterConfig>>,
    pub backend_manager: Arc<BackendManager>,
    pub config_path: Option<PathBuf>,
}

/// Server status information
#[derive(Debug, Serialize)]
pub struct ServerStatus {
    pub id: String,
    pub name: String,
    pub healthy: bool,
    pub tool_count: usize,
    pub sandbox_type: String,
    pub uptime_seconds: Option<u64>,
}

/// Dashboard overview
#[derive(Debug, Serialize)]
pub struct DashboardOverview {
    pub total_servers: usize,
    pub healthy_servers: usize,
    pub total_tools: usize,
    pub router_uptime: u64,
    pub active_sessions: usize,
}

/// Request to add/update a server
#[derive(Debug, Deserialize)]
pub struct ServerRequest {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub requires_auth: bool,
    pub allowed_users: Vec<String>,
    pub sandbox: crate::sandbox::SandboxConfig,
    #[serde(default = "default_persist_to_config")]
    pub persist_to_config: bool,
}

fn default_persist_to_config() -> bool {
    true
}

/// API Error type
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: String,
    #[serde(skip)]
    pub status: StatusCode,
}

impl ApiError {
    /// Create a new ApiError with BAD_REQUEST status
    pub fn bad_request(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    /// Create a new ApiError with INTERNAL_SERVER_ERROR status
    pub fn internal(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Create a new ApiError with NOT_FOUND status
    pub fn not_found(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    /// Create a new ApiError with UNAUTHORIZED status
    pub fn unauthorized(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    /// Create a new ApiError with FORBIDDEN status
    pub fn forbidden(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            status: StatusCode::FORBIDDEN,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        (status, Json(self)).into_response()
    }
}

/// Get dashboard overview
pub async fn get_overview(
    State(state): State<DashboardState>,
) -> Result<Json<DashboardOverview>, ApiError> {
    let config = state.router_config.read().await;
    let backend_status = state.backend_manager.get_status().await;
    let all_tools = state.backend_manager.get_all_tools().await;
    
    let healthy_count = backend_status.values().filter(|&&h| h).count();
    
    // TODO: Get actual router uptime and session count
    Ok(Json(DashboardOverview {
        total_servers: config.servers.len(),
        healthy_servers: healthy_count,
        total_tools: all_tools.len(),
        router_uptime: 0, // TODO: Track actual uptime
        active_sessions: 0, // TODO: Get from session manager
    }))
}

/// List all configured servers with status
pub async fn list_servers(
    State(state): State<DashboardState>,
) -> Result<Json<Vec<ServerStatus>>, ApiError> {
    let config = state.router_config.read().await;
    let backend_status = state.backend_manager.get_status().await;
    
    let mut servers = Vec::new();
    
    for server in &config.servers {
        let healthy = backend_status.get(&server.id).copied().unwrap_or(false);
        
        // Get sandbox type string
        let sandbox_type = match &server.sandbox.strategy {
            crate::sandbox::SandboxStrategy::None => "None",
            crate::sandbox::SandboxStrategy::Docker { .. } => "Docker",
            crate::sandbox::SandboxStrategy::Podman { .. } => "Podman",
            crate::sandbox::SandboxStrategy::Firejail { .. } => "Firejail",
            crate::sandbox::SandboxStrategy::Bubblewrap { .. } => "Bubblewrap",
        }.to_string();
        
        servers.push(ServerStatus {
            id: server.id.clone(),
            name: server.name.clone(),
            healthy,
            tool_count: 0, // TODO: Get actual tool count per server
            sandbox_type,
            uptime_seconds: None, // TODO: Track per-server uptime
        });
    }
    
    Ok(Json(servers))
}

/// Get detailed server information
pub async fn get_server(
    Path(server_id): Path<String>,
    State(state): State<DashboardState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let config = state.router_config.read().await;
    
    let server = config.servers.iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| ApiError::not_found(
            format!("Server '{}' not found", server_id),
            "SERVER_NOT_FOUND"
        ))?;
    
    // Create a temporary config with just this server to use the masking function
    let temp_config = crate::simple_router::RouterConfig {
        servers: vec![server.clone()],
        timeout_ms: config.timeout_ms,
        cache_tools: config.cache_tools,
    };
    
    let masked_config = crate::server_loader::create_display_config(&temp_config);
    let servers = masked_config["servers"].as_array()
        .and_then(|arr| arr.first())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    
    Ok(Json(servers))
}

/// Add a new server
pub async fn add_server(
    State(state): State<DashboardState>,
    session: Option<Extension<Session>>,
    Json(request): Json<ServerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Adding new server: {}", request.id);
    
    // Validate server config
    let server = BackendServer {
        id: request.id.clone(),
        name: request.name,
        r#type: crate::simple_router::ServerType::Local, // Default to local
        command: Some(request.command),
        args: request.args,
        url: None,
        transport: crate::simple_router::TransportType::Sse,
        auth: None,
        env: request.env,
        requires_auth: request.requires_auth,
        allowed_users: request.allowed_users,
        sandbox: request.sandbox,
        timeout_ms: 30000,
        retry: None,
        tls: None,
    };
    
    // Validate the server configuration
    if let Err(e) = crate::simple_router::validate_server_config(&server).await {
        return Err(ApiError::bad_request(
            format!("Invalid server configuration: {}", e),
            "INVALID_CONFIG"
        ));
    }
    
    // Check if user is authenticated
    if let Some(Extension(session)) = session {
        // User is authenticated - save to database
        let user_id = session.user_id;
        info!("Saving server to database for user {}", user_id);
        
        // Prefix server ID with user context
        let mut user_server = server.clone();
        user_server.id = format!("user_{}_{}", user_id, server.id);
        
        // Apply stricter sandboxing for user servers
        if matches!(user_server.sandbox.strategy, crate::sandbox::SandboxStrategy::None) {
            user_server.sandbox.strategy = crate::sandbox::SandboxStrategy::Firejail {
                profile: Some("default".to_string()),
                whitelist_paths: vec![
                    format!("/tmp/jauauth-user-{}-{}", user_id, server.id)
                ],
                read_only_paths: vec![],
                net: false,
                no_root: true,
                netfilter: None,
            };
        }
        
        // Create ServerStore with user's encryption key
        let master_key = state.auth_context.config.jwt_secret.as_bytes();
        let encryption_key = derive_key(master_key, &format!("user-{}", user_id))
            .map_err(|e| ApiError::internal(
                format!("Failed to derive encryption key: {}", e),
                "ENCRYPTION_ERROR"
            ))?;
        let server_store = ServerStore::new(state.auth_context.db.clone(), encryption_key);
        
        // Save to database
        server_store.add_server(user_id, &user_server).await
            .map_err(|e| ApiError::internal(
                format!("Failed to save server to database: {}", e),
                "DB_SAVE_FAILED"
            ))?;
        
        // Spawn the backend
        if let Err(e) = state.backend_manager.spawn_backend(user_server.clone()).await {
            error!("Failed to spawn backend: {}", e);
            // TODO: Remove from database on failure
            return Err(ApiError::internal(
                format!("Failed to start server: {}", e),
                "SPAWN_FAILED"
            ));
        }
        
        Ok(Json(serde_json::json!({
            "success": true,
            "server_id": user_server.id,
            "source": "database",
            "message": "Server saved to your personal configuration"
        })))
    } else {
        // No authentication - save to JSON config (admin mode)
        info!("No session found, saving to JSON config");
        
        // Use a block to ensure the write lock is dropped before saving
        {
            let mut config = state.router_config.write().await;
            
            // Check if ID already exists
            if config.servers.iter().any(|s| s.id == server.id) {
                return Err(ApiError {
                    error: format!("Server with ID '{}' already exists", server.id),
                    code: "DUPLICATE_ID".to_string(),
                    status: StatusCode::CONFLICT,
                });
            }
            
            config.servers.push(server.clone());
        } // Write lock dropped here
        
        // Spawn the backend
        if let Err(e) = state.backend_manager.spawn_backend(server.clone()).await {
            error!("Failed to spawn backend: {}", e);
            // Remove from config on failure
            let mut config = state.router_config.write().await;
            config.servers.retain(|s| s.id != request.id);
            return Err(ApiError::internal(
                format!("Failed to start server: {}", e),
                "SPAWN_FAILED"
            ));
        }
        
        // Persist config to file if requested
        if request.persist_to_config {
            match save_config_to_file(&state).await {
                Ok(_) => info!("Successfully saved configuration to file"),
                Err(e) => {
                    error!("Failed to save configuration: {}", e);
                    error!("Config path: {:?}", state.config_path);
                    // Continue anyway - server is running
                }
            }
        } else {
            info!("persist_to_config is false, not saving to file");
        }
        
        Ok(Json(serde_json::json!({
            "success": true,
            "server_id": request.id,
            "source": "json",
            "message": "Server saved to system configuration"
        })))
    }
}

/// Update an existing server
pub async fn update_server(
    Path(server_id): Path<String>,
    State(state): State<DashboardState>,
    Json(request): Json<ServerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Updating server: {}", server_id);
    
    // For now, we'll remove and re-add
    // TODO: Implement proper update with graceful restart
    
    // Build updated server first
    let server = BackendServer {
        id: request.id.clone(),
        name: request.name,
        r#type: crate::simple_router::ServerType::Local, // Default to local
        command: Some(request.command),
        args: request.args,
        url: None,
        transport: crate::simple_router::TransportType::Sse,
        auth: None,
        env: request.env,
        requires_auth: request.requires_auth,
        allowed_users: request.allowed_users,
        sandbox: request.sandbox,
        timeout_ms: 30000,
        retry: None,
        tls: None,
    };
    
    // Update config in a block to release lock before saving
    {
        let mut config = state.router_config.write().await;
        
        // Find and remove old server
        let old_pos = config.servers.iter().position(|s| s.id == server_id)
            .ok_or_else(|| ApiError::not_found(
                format!("Server '{}' not found", server_id),
                "SERVER_NOT_FOUND"
            ))?;
        
        config.servers.remove(old_pos);
        
        // TODO: Shutdown old backend
        
        // Add updated server
        config.servers.push(server.clone());
    } // Write lock dropped here
    
    // Spawn new backend
    if let Err(e) = state.backend_manager.spawn_backend(server).await {
        error!("Failed to spawn updated backend: {}", e);
        return Err(ApiError::internal(
            format!("Failed to restart server: {}", e),
            "SPAWN_FAILED"
        ));
    }
    
    // Persist config to file if requested
    if request.persist_to_config {
        if let Err(e) = save_config_to_file(&state).await {
            error!("Failed to save configuration: {}", e);
            // Continue anyway - server is updated
        }
    }
    
    Ok(Json(serde_json::json!({
        "success": true,
        "server_id": request.id
    })))
}

/// Request to remove a server
#[derive(Debug, Deserialize)]
pub struct RemoveServerRequest {
    #[serde(default = "default_persist_to_config")]
    pub persist_to_config: bool,
}

/// Remove a server
pub async fn remove_server(
    Path(server_id): Path<String>,
    State(state): State<DashboardState>,
    session: Option<Extension<Session>>,
    Json(request): Json<Option<RemoveServerRequest>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Removing server: {}", server_id);
    
    // Check if this is a user server (has user prefix)
    if server_id.starts_with("user_") {
        // This is a user server - need to remove from database
        if let Some(Extension(session)) = session {
            let user_id = session.user_id;
            
            // Extract the original server ID (remove user_{id}_ prefix)
            let prefix = format!("user_{}_", user_id);
            if !server_id.starts_with(&prefix) {
                return Err(ApiError::forbidden(
                    "You can only remove your own servers",
                    "FORBIDDEN"
                ));
            }
            
            // Create ServerStore with user's encryption key
            let master_key = state.auth_context.config.jwt_secret.as_bytes();
            let encryption_key = derive_key(master_key, &format!("user-{}", user_id))
                .map_err(|e| ApiError::internal(
                    format!("Failed to derive encryption key: {}", e),
                    "ENCRYPTION_ERROR"
                ))?;
            let server_store = ServerStore::new(state.auth_context.db.clone(), encryption_key);
            
            // Remove from database
            server_store.delete_server(user_id, &server_id).await
                .map_err(|e| ApiError::internal(
                    format!("Failed to remove server from database: {}", e),
                    "DB_REMOVE_FAILED"
                ))?;
            
            // Also remove from in-memory config
            let mut config = state.router_config.write().await;
            config.servers.retain(|s| s.id != server_id);
            
            // TODO: Shutdown backend process
            
            Ok(Json(serde_json::json!({
                "success": true,
                "server_id": server_id,
                "source": "database",
                "message": "Server removed from your personal configuration"
            })))
        } else {
            return Err(ApiError::unauthorized(
                "Authentication required to remove user servers",
                "UNAUTHORIZED"
            ));
        }
    } else {
        // System server - only remove from JSON config (admin mode)
        {
            let mut config = state.router_config.write().await;
            
            // Find and remove server
            let pos = config.servers.iter().position(|s| s.id == server_id)
                .ok_or_else(|| ApiError::not_found(
                    format!("Server '{}' not found", server_id),
                    "SERVER_NOT_FOUND"
                ))?;
            
            config.servers.remove(pos);
        } // Write lock dropped here
        
        // TODO: Shutdown backend process
        
        // Persist config to file if requested
        let request = request.unwrap_or(RemoveServerRequest { persist_to_config: true });
        if request.persist_to_config {
            if let Err(e) = save_config_to_file(&state).await {
                error!("Failed to save configuration: {}", e);
                // Continue anyway - server is removed from memory
            }
        }
        
        Ok(Json(serde_json::json!({
            "success": true,
            "server_id": server_id,
            "source": "json",
            "message": "Server removed from system configuration"
        })))
    }
}

/// Get server logs (last N lines)
pub async fn get_server_logs(
    Path(server_id): Path<String>,
    State(_state): State<DashboardState>,
) -> Result<Json<Vec<String>>, ApiError> {
    // TODO: Implement log capture and retrieval
    Ok(Json(vec![
        format!("Log functionality not yet implemented for server: {}", server_id)
    ]))
}

/// Get available tools from all servers
pub async fn list_tools(
    State(state): State<DashboardState>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let tools = state.backend_manager.get_all_tools().await;
    Ok(Json(tools))
}

/// Test a tool call
#[derive(Debug, Deserialize)]
pub struct ToolTestRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

pub async fn test_tool(
    State(state): State<DashboardState>,
    Json(request): Json<ToolTestRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match state.backend_manager.route_tool_call(&request.tool_name, request.arguments).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err(ApiError::internal(
            format!("Tool call failed: {}", e),
            "TOOL_CALL_FAILED"
        )),
    }
}

/// Get auth settings
pub async fn get_auth_settings(
    State(state): State<DashboardState>,
) -> Json<serde_json::Value> {
    let config = &state.auth_context.config;
    
    Json(serde_json::json!({
        "app_name": config.app_name,
        "host": config.host,
        "port": config.port,
        "session_duration_minutes": config.session_duration.as_secs() / 60,
        "pin_grace_period_minutes": config.pin_grace_period.as_secs() / 60,
        "max_login_attempts": config.max_login_attempts,
        "rate_limit_window_minutes": config.rate_limit_window.as_secs() / 60,
        "webauthn_enabled": true, // WebAuthn config is always present
        "webauthn_rp_name": config.webauthn.rp_name,
        "first_access_commands": config.first_access_commands,
        "permission_groups": config.permission_groups,
    }))
}

/// Update auth settings
#[derive(Debug, Deserialize)]
pub struct AuthSettingsUpdate {
    pub session_duration_minutes: Option<u64>,
    pub require_pin: Option<bool>,
    pub require_device_trust: Option<bool>,
    pub max_devices_per_user: Option<usize>,
}

pub async fn update_auth_settings(
    State(_state): State<DashboardState>,
    Json(_request): Json<AuthSettingsUpdate>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Implement auth settings update
    // This would require making config mutable or reloadable
    
    Err(ApiError {
        error: "Auth settings update not yet implemented".to_string(),
        code: "NOT_IMPLEMENTED".to_string(),
        status: StatusCode::NOT_IMPLEMENTED,
    })
}

/// Get MCP config settings
pub async fn get_mcp_config(
    State(_state): State<DashboardState>,
) -> Json<serde_json::Value> {
    // Check if USE_CONFIG_FILE environment variable is set
    let use_config_file = std::env::var("USE_CONFIG_FILE")
        .unwrap_or_else(|_| "false".to_string()) == "true";
    
    Json(serde_json::json!({
        "use_config_file": use_config_file,
        "config_file_path": "mcp-server/config.json",
        "backend_url": std::env::var("RUST_BACKEND_URL").unwrap_or_else(|_| "http://localhost:7447".to_string()),
        "api_timeout": std::env::var("API_TIMEOUT").unwrap_or_else(|_| "30000".to_string()),
    }))
}

/// Update MCP config settings
#[derive(Debug, Deserialize)]
pub struct McpConfigUpdate {
    pub use_config_file: bool,
}

pub async fn update_mcp_config(
    State(_state): State<DashboardState>,
    Json(request): Json<McpConfigUpdate>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Set the environment variable for the TypeScript MCP server
    std::env::set_var("USE_CONFIG_FILE", request.use_config_file.to_string());
    
    info!("Updated MCP config: use_config_file = {}", request.use_config_file);
    
    Ok(Json(serde_json::json!({
        "success": true,
        "use_config_file": request.use_config_file,
        "message": "Config updated. Restart the TypeScript MCP server for changes to take effect."
    })))
}

/// Save the current router configuration to file
async fn save_config_to_file(state: &DashboardState) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(config_path) = &state.config_path {
        let config = state.router_config.read().await;
        let json = serde_json::to_string_pretty(&*config)?;
        
        // Log the actual content being saved for debugging
        info!("Saving {} servers to config file", config.servers.len());
        for server in &config.servers {
            info!("  - {} ({})", server.name, server.id);
        }
        
        fs::write(config_path, json).await?;
        info!("Successfully saved configuration to {:?}", config_path);
        
        // Verify the file was written
        if let Ok(contents) = fs::read_to_string(config_path).await {
            if let Ok(verify_config) = serde_json::from_str::<RouterConfig>(&contents) {
                info!("Verified: config file now contains {} servers", verify_config.servers.len());
            }
        }
    } else {
        error!("No config path specified, cannot save configuration!");
    }
    Ok(())
}

/// Install NPM package endpoint
#[derive(Debug, Deserialize)]
pub struct InstallNpmPackageRequest {
    pub package: String,
    pub global: bool,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub message: Option<String>,
}

pub async fn install_npm_package(
    State(_state): State<DashboardState>,
    Json(request): Json<InstallNpmPackageRequest>,
) -> Result<Json<StatusResponse>, ApiError> {
    // Validate package name to prevent command injection
    if !request.package.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '@' || c == '/' || c == '.') {
        return Err(ApiError::bad_request(
            "Invalid package name",
            "INVALID_PACKAGE"
        ));
    }

    info!("Installing NPM package: {} (global: {})", request.package, request.global);

    let args = if request.global {
        vec!["install", "-g", &request.package]
    } else {
        vec!["install", &request.package]
    };

    let output = tokio::process::Command::new("npm")
        .args(&args)
        .output()
        .await
        .map_err(|e| ApiError::internal(
            format!("Failed to run npm: {}", e),
            "NPM_ERROR"
        ))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("npm install failed: {}", stderr);
        return Err(ApiError::internal(
            format!("npm install failed: {}", stderr),
            "INSTALL_FAILED"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("npm install successful: {}", stdout);

    Ok(Json(StatusResponse {
        status: "success".to_string(),
        message: Some(format!("Successfully installed {}", request.package)),
    }))
}