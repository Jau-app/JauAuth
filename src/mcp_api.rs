
//! HTTP API for MCP server communication

use axum::{
    extract::{State, Json},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, error, debug};

use crate::{
    simple_router::RouterConfig,
    backend_manager::BackendManager,
};

/// MCP API state
#[derive(Clone)]
pub struct McpApiState {
    pub router_config: Arc<RwLock<RouterConfig>>,
    pub backend_manager: Arc<BackendManager>,
}

/// Tool information
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Tool call request
#[derive(Debug, Deserialize)]
pub struct ToolCallRequest {
    pub tool: String,
    pub arguments: Value,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Tool call response
#[derive(Debug, Serialize)]
pub struct ToolCallResponse {
    pub result: Value,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

/// Get all available tools from backend servers
pub async fn list_tools(
    State(state): State<McpApiState>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let all_tools = state.backend_manager.get_all_tools().await;
    let backend_status = state.backend_manager.get_status().await;

    let mut tools: Vec<ToolInfo> = Vec::new();

    // Process tools from all backends
    for tool_value in all_tools {
        if let Some(tool_obj) = tool_value.as_object() {
            if let Some(name) = tool_obj.get("name").and_then(|n| n.as_str()) {
                // Get server_id from tool name (format: server_id:tool_name)
                let server_id = name.split(':').next().unwrap_or("");

                // Only include tools from healthy backends
                if backend_status.get(server_id).copied().unwrap_or(false) {
                    tools.push(ToolInfo {
                        name: name.to_string(),
                        description: tool_obj.get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("No description")
                            .to_string(),
                        input_schema: tool_obj.get("inputSchema")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!({
                                "type": "object",
                                "properties": {},
                                "required": []
                            })),
                    });
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "tools": tools
    })))
}

/// Call a tool on a backend server
pub async fn call_tool(
    State(state): State<McpApiState>,
    Json(request): Json<ToolCallRequest>,
) -> Result<Json<ToolCallResponse>, ErrorResponse> {
    info!("MCP API: Calling tool {} (timeout: {:?}ms)", request.tool, request.timeout_ms);
    
    // Convert timeout_ms to Duration if provided
    let timeout = request.timeout_ms.map(Duration::from_millis);
    
    // Use async version if timeout is specified, otherwise use sync for backward compatibility
    let result = if timeout.is_some() {
        debug!("Using async tool call with timeout: {:?}", timeout);
        state.backend_manager.route_tool_call_async(&request.tool, request.arguments, timeout).await
    } else {
        debug!("Using synchronous tool call (no timeout specified)");
        state.backend_manager.route_tool_call(&request.tool, request.arguments).await
    };
    
    match result {
        Ok(result) => Ok(Json(ToolCallResponse { result })),
        Err(e) => {
            error!("Tool call failed: {}", e);
            Err(ErrorResponse {
                error: format!("Tool call failed: {}", e),
            })
        }
    }
}

/// Get router status
pub async fn get_status(
    State(state): State<McpApiState>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let config = state.router_config.read().await;
    let backend_status = state.backend_manager.get_status().await;

    let servers_status: Vec<serde_json::Value> = config.servers.iter().map(|server| {
        let healthy = backend_status.get(&server.id).copied().unwrap_or(false);
        serde_json::json!({
            "id": server.id,
            "name": server.name,
            "healthy": healthy,
            "command": server.command,
        })
    }).collect();

    Ok(Json(serde_json::json!({
        "router": "healthy",
        "total_servers": config.servers.len(),
        "healthy_servers": backend_status.values().filter(|&&h| h).count(),
        "servers": servers_status,
    })))
}

/// List configured servers
pub async fn list_servers(
    State(state): State<McpApiState>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let config = state.router_config.read().await;
    let backend_status = state.backend_manager.get_status().await;

    let servers: Vec<serde_json::Value> = config.servers.iter().map(|server| {
        let healthy = backend_status.get(&server.id).copied().unwrap_or(false);
        serde_json::json!({
            "id": server.id,
            "name": server.name,
            "command": server.command,
            "args": server.args,
            "healthy": healthy,
            "requires_auth": server.requires_auth,
        })
    }).collect();

    Ok(Json(serde_json::json!({
        "servers": servers
    })))
}

/// Router for MCP API endpoints
pub fn mcp_api_routes() -> axum::Router<McpApiState> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/tools", get(list_tools))
        .route("/tool/call", post(call_tool))
        .route("/status", get(get_status))
        .route("/servers", get(list_servers))
}
