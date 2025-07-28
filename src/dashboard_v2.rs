//! Enhanced dashboard API with database persistence

use axum::{
    extract::{State, Path, Json, Extension},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};
use std::path::PathBuf;
use std::collections::HashMap;

use crate::{
    simple_router::{RouterConfig, BackendServer},
    backend_manager::BackendManager,
    server_store::ServerStore,
    AuthContext,
    session::Session,
};

/// Enhanced dashboard state with database support
#[derive(Clone)]
pub struct DashboardStateV2 {
    pub auth_context: AuthContext,
    pub router_config: Arc<RwLock<RouterConfig>>,
    pub backend_manager: Arc<BackendManager>,
    pub config_path: Option<PathBuf>,
    pub server_store: Arc<ServerStore>,
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
    pub source: String, // "config" or "database"
}

/// Request to add/update a server
#[derive(Debug, Deserialize)]
pub struct ServerRequest {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub requires_auth: bool,
    pub allowed_users: Vec<String>,
    pub sandbox: crate::sandbox::SandboxConfig,
    #[serde(default = "default_persist_to_config")]
    pub persist_to_config: bool,
    #[serde(default = "default_save_to_database")]
    pub save_to_database: bool,
    #[serde(default)]
    pub save_without_starting: bool,
}

fn default_persist_to_config() -> bool {
    false // Don't save to config by default
}

fn default_save_to_database() -> bool {
    true // Save to database by default
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

/// List all configured servers with status (enhanced)
pub async fn list_servers_v2(
    State(state): State<DashboardStateV2>,
    Extension(session): Extension<Session>,
) -> Result<Json<Vec<ServerStatus>>, ApiError> {
    let config = state.router_config.read().await;
    let backend_status = state.backend_manager.get_status().await;
    
    let mut servers = Vec::new();
    
    // Add servers from config
    for server in &config.servers {
        let healthy = backend_status.get(&server.id).copied().unwrap_or(false);
        
        let sandbox_type = match &server.sandbox.strategy {
            crate::sandbox::SandboxStrategy::None => "None",
            crate::sandbox::SandboxStrategy::Docker { .. } => "Docker",
            crate::sandbox::SandboxStrategy::Podman { .. } => "Podman",
            crate::sandbox::SandboxStrategy::Firejail { .. } => "Firejail",
            crate::sandbox::SandboxStrategy::Bubblewrap { .. } => "Bubblewrap",
        }.to_string();
        
        // Check if this server is also in the database (user override)
        let source = if let Ok(Some(_)) = state.server_store.get_server(session.user_id, &server.id).await {
            "database"
        } else {
            "config"
        };
        
        servers.push(ServerStatus {
            id: server.id.clone(),
            name: server.name.clone(),
            healthy,
            tool_count: 0, // TODO: Get actual tool count per server
            sandbox_type,
            uptime_seconds: None, // TODO: Track per-server uptime
            source: source.to_string(),
        });
    }
    
    Ok(Json(servers))
}

/// Add a new server (enhanced with database support)
pub async fn add_server_v2(
    State(state): State<DashboardStateV2>,
    Extension(session): Extension<Session>,
    Json(request): Json<ServerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Adding new server: {} for user {}", request.id, session.user_id);
    
    // Create BackendServer
    let server = BackendServer {
        id: request.id.clone(),
        name: request.name,
        r#type: crate::simple_router::ServerType::Local,
        command: Some(request.command),
        args: request.args,
        url: None,
        transport: crate::simple_router::TransportType::Sse, // Default for local servers (not used)
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
        return Err(ApiError {
            error: format!("Invalid server configuration: {}", e),
            code: "INVALID_CONFIG".to_string(),
        });
    }
    
    // Save to database if requested
    if request.save_to_database {
        if let Err(e) = state.server_store.add_server(session.user_id, &server).await {
            error!("Failed to save server to database: {}", e);
            return Err(ApiError {
                error: format!("Failed to save server to database: {}", e),
                code: "DB_ERROR".to_string(),
            });
        }
        info!("Server '{}' saved to database for user {}", server.id, session.user_id);
    }
    
    // Save to config file if requested (for system-wide defaults)
    if request.persist_to_config {
        let mut config = state.router_config.write().await;
        
        // Check if ID already exists
        if config.servers.iter().any(|s| s.id == server.id) {
            return Err(ApiError {
                error: format!("Server with ID '{}' already exists in config", server.id),
                code: "DUPLICATE_ID".to_string(),
            });
        }
        
        config.servers.push(server.clone());
        
        // Save to file
        if let Some(config_path) = &state.config_path {
            let json = serde_json::to_string_pretty(&*config)
                .map_err(|e| ApiError {
                    error: format!("Failed to serialize config: {}", e),
                    code: "SERIALIZE_ERROR".to_string(),
                })?;
                
            tokio::fs::write(config_path, json).await
                .map_err(|e| ApiError {
                    error: format!("Failed to write config file: {}", e),
                    code: "FILE_ERROR".to_string(),
                })?;
                
            info!("Server '{}' saved to config file", server.id);
        }
    }
    
    // Spawn the backend (unless save_without_starting is true)
    if !request.save_without_starting {
        if let Err(e) = state.backend_manager.spawn_backend(server).await {
            error!("Failed to spawn backend: {}", e);
            return Err(ApiError {
                error: format!("Failed to start server: {}", e),
                code: "SPAWN_FAILED".to_string(),
            });
        }
    } else {
        info!("Server '{}' saved without starting (save_without_starting=true)", request.id);
    }
    
    Ok(Json(serde_json::json!({
        "success": true,
        "server_id": request.id,
        "saved_to_database": request.save_to_database,
        "saved_to_config": request.persist_to_config,
        "started": !request.save_without_starting,
    })))
}

/// Update an existing server (enhanced)
pub async fn update_server_v2(
    Path(server_id): Path<String>,
    State(state): State<DashboardStateV2>,
    Extension(session): Extension<Session>,
    Json(request): Json<ServerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Updating server: {} for user {}", server_id, session.user_id);
    
    // Create updated server
    let server = BackendServer {
        id: request.id.clone(),
        name: request.name,
        r#type: crate::simple_router::ServerType::Local,
        command: Some(request.command),
        args: request.args,
        url: None,
        transport: crate::simple_router::TransportType::Sse, // Default for local servers (not used)
        auth: None,
        env: request.env,
        requires_auth: request.requires_auth,
        allowed_users: request.allowed_users,
        sandbox: request.sandbox,
        timeout_ms: 30000,
        retry: None,
        tls: None,
    };
    
    // Update in database if it exists there
    if let Ok(Some(_)) = state.server_store.get_server(session.user_id, &server_id).await {
        if let Err(e) = state.server_store.update_server(session.user_id, &server_id, &server).await {
            error!("Failed to update server in database: {}", e);
            return Err(ApiError {
                error: format!("Failed to update server in database: {}", e),
                code: "DB_ERROR".to_string(),
            });
        }
        info!("Server '{}' updated in database", server_id);
    }
    
    // Update in config if requested
    if request.persist_to_config {
        let mut config = state.router_config.write().await;
        
        // Find and update
        if let Some(pos) = config.servers.iter().position(|s| s.id == server_id) {
            config.servers[pos] = server.clone();
            
            // Save to file
            if let Some(config_path) = &state.config_path {
                let json = serde_json::to_string_pretty(&*config)
                    .map_err(|e| ApiError {
                        error: format!("Failed to serialize config: {}", e),
                        code: "SERIALIZE_ERROR".to_string(),
                    })?;
                    
                tokio::fs::write(config_path, json).await
                    .map_err(|e| ApiError {
                        error: format!("Failed to write config file: {}", e),
                        code: "FILE_ERROR".to_string(),
                    })?;
            }
        }
    }
    
    // TODO: Implement graceful restart of backend
    
    Ok(Json(serde_json::json!({
        "success": true,
        "server_id": server_id,
    })))
}

/// Remove a server (enhanced)
pub async fn remove_server_v2(
    Path(server_id): Path<String>,
    State(state): State<DashboardStateV2>,
    Extension(session): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Removing server: {} for user {}", server_id, session.user_id);
    
    // Remove from database (soft delete)
    if let Ok(Some(_)) = state.server_store.get_server(session.user_id, &server_id).await {
        if let Err(e) = state.server_store.delete_server(session.user_id, &server_id).await {
            error!("Failed to delete server from database: {}", e);
            return Err(ApiError {
                error: format!("Failed to delete server from database: {}", e),
                code: "DB_ERROR".to_string(),
            });
        }
        info!("Server '{}' removed from database", server_id);
    }
    
    // Also remove from in-memory config
    let mut config = state.router_config.write().await;
    config.servers.retain(|s| s.id != server_id);
    
    // TODO: Shutdown backend process
    
    Ok(Json(serde_json::json!({
        "success": true,
        "server_id": server_id,
    })))
}

/// Get server configuration template
pub async fn get_server_template(
    Path(template_type): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let template = match template_type.as_str() {
        "huggingface" => serde_json::json!({
            "id": "hf-mcp-server",
            "name": "HuggingFace MCP Server",
            "command": "npx",
            "args": ["-y", "@huggingface/mcp-server"],
            "env": {
                "HF_TOKEN": ""  // User needs to fill this
            },
            "requires_auth": true,
            "allowed_users": [],
            "sandbox": {
                "strategy": "none",
                "env_passthrough": ["HOME", "USER", "PATH", "NODE_PATH", "HF_TOKEN"]
            }
        }),
        
        "playwright" => serde_json::json!({
            "id": "playwright",
            "name": "Playwright Browser Automation",
            "command": "npx",
            "args": ["-y", "@browserbasehq/mcp-server-playwright"],
            "env": {},
            "requires_auth": true,
            "allowed_users": [],
            "sandbox": {
                "strategy": "none",
                "env_passthrough": ["HOME", "USER", "PATH", "NODE_PATH", "DISPLAY"]
            }
        }),
        
        "github" => serde_json::json!({
            "id": "github-mcp",
            "name": "GitHub MCP Server",
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-github"],
            "env": {
                "GITHUB_TOKEN": ""  // User needs to fill this
            },
            "requires_auth": true,
            "allowed_users": [],
            "sandbox": {
                "strategy": "none",
                "env_passthrough": ["HOME", "USER", "PATH", "NODE_PATH", "GITHUB_TOKEN"]
            }
        }),
        
        _ => return Err(ApiError {
            error: format!("Unknown template type: {}", template_type),
            code: "UNKNOWN_TEMPLATE".to_string(),
        })
    };
    
    Ok(Json(template))
}

/// Enhanced router for dashboard API
pub fn dashboard_api_routes_v2() -> axum::Router<DashboardStateV2> {
    use axum::routing::{get, post, put, delete};
    
    axum::Router::new()
        .route("/servers", get(list_servers_v2))
        .route("/servers", post(add_server_v2))
        .route("/servers/:id", put(update_server_v2))
        .route("/servers/:id", delete(remove_server_v2))
        .route("/templates/:type", get(get_server_template))
}