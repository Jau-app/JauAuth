//! Dashboard API for managing MCP servers and settings

use axum::{
    extract::{Path, State, Json},
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
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
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
            _ => "Platform-specific",
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
) -> Result<Json<BackendServer>, ApiError> {
    let config = state.router_config.read().await;
    
    config.servers.iter()
        .find(|s| s.id == server_id)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError {
            error: format!("Server '{}' not found", server_id),
            code: "SERVER_NOT_FOUND".to_string(),
        })
}

/// Add a new server
pub async fn add_server(
    State(state): State<DashboardState>,
    Json(request): Json<ServerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Adding new server: {}", request.id);
    
    // Validate server config
    let server = BackendServer {
        id: request.id.clone(),
        name: request.name,
        command: request.command,
        args: request.args,
        env: request.env,
        requires_auth: request.requires_auth,
        allowed_users: request.allowed_users,
        sandbox: request.sandbox,
    };
    
    // Validate the server configuration
    if let Err(e) = crate::simple_router::validate_server_config(&server).await {
        return Err(ApiError {
            error: format!("Invalid server configuration: {}", e),
            code: "INVALID_CONFIG".to_string(),
        });
    }
    
    // Add to config
    let mut config = state.router_config.write().await;
    
    // Check if ID already exists
    if config.servers.iter().any(|s| s.id == server.id) {
        return Err(ApiError {
            error: format!("Server with ID '{}' already exists", server.id),
            code: "DUPLICATE_ID".to_string(),
        });
    }
    
    config.servers.push(server.clone());
    
    // Spawn the backend
    if let Err(e) = state.backend_manager.spawn_backend(server).await {
        error!("Failed to spawn backend: {}", e);
        // Remove from config on failure
        config.servers.retain(|s| s.id != request.id);
        return Err(ApiError {
            error: format!("Failed to start server: {}", e),
            code: "SPAWN_FAILED".to_string(),
        });
    }
    
    // Persist config to file if requested
    if request.persist_to_config {
        if let Err(e) = save_config_to_file(&state).await {
            error!("Failed to save configuration: {}", e);
            // Continue anyway - server is running
        }
    }
    
    Ok(Json(serde_json::json!({
        "success": true,
        "server_id": request.id
    })))
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
    
    let mut config = state.router_config.write().await;
    
    // Find and remove old server
    let old_pos = config.servers.iter().position(|s| s.id == server_id)
        .ok_or_else(|| ApiError {
            error: format!("Server '{}' not found", server_id),
            code: "SERVER_NOT_FOUND".to_string(),
        })?;
    
    config.servers.remove(old_pos);
    
    // TODO: Shutdown old backend
    
    // Add updated server
    let server = BackendServer {
        id: request.id.clone(),
        name: request.name,
        command: request.command,
        args: request.args,
        env: request.env,
        requires_auth: request.requires_auth,
        allowed_users: request.allowed_users,
        sandbox: request.sandbox,
    };
    
    config.servers.push(server.clone());
    
    // Spawn new backend
    if let Err(e) = state.backend_manager.spawn_backend(server).await {
        error!("Failed to spawn updated backend: {}", e);
        return Err(ApiError {
            error: format!("Failed to restart server: {}", e),
            code: "SPAWN_FAILED".to_string(),
        });
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
    Json(request): Json<Option<RemoveServerRequest>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Removing server: {}", server_id);
    
    let mut config = state.router_config.write().await;
    
    // Find and remove server
    let pos = config.servers.iter().position(|s| s.id == server_id)
        .ok_or_else(|| ApiError {
            error: format!("Server '{}' not found", server_id),
            code: "SERVER_NOT_FOUND".to_string(),
        })?;
    
    config.servers.remove(pos);
    
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
        "server_id": server_id
    })))
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
        Err(e) => Err(ApiError {
            error: format!("Tool call failed: {}", e),
            code: "TOOL_CALL_FAILED".to_string(),
        }),
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
        fs::write(config_path, json).await?;
        info!("Saved configuration to {:?}", config_path);
    } else {
        info!("No config path specified, skipping save");
    }
    Ok(())
}