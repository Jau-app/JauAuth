//! MCP Router - Routes commands to multiple backend MCP servers

use rust_mcp_sdk::{
    mcp_server::{ServerHandler, ServerRuntime, server_runtime},
};
use rust_mcp_transport::{StdioTransport, TransportOptions};
use rust_mcp_schema::{
    schema::{
        Tool, ToolInfo, CallToolRequest, CallToolResult,
        ListToolsRequest, ListToolsResult,
        Implementation, InitializeResult, ServerCapabilities,
        ServerCapabilitiesTools, Empty,
    },
    error::ErrorResponse,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::process::Stdio;
use tokio::process::Command;

/// Configuration for a backend MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendServer {
    /// Unique identifier for this server
    pub id: String,
    /// Display name for the server
    pub name: String,
    /// Command to launch the server (e.g., "npx", "python", "cargo")
    pub command: String,
    /// Arguments for the command
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Whether authentication is required for this server
    pub requires_auth: bool,
    /// Allowed users/roles (if requires_auth is true)
    pub allowed_users: Vec<String>,
}

/// Configuration for the router
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// List of backend servers to route to
    pub servers: Vec<BackendServer>,
    /// Default timeout for backend operations (ms)
    pub timeout_ms: u64,
    /// Whether to cache tool listings
    pub cache_tools: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            servers: vec![],
            timeout_ms: 30000,
            cache_tools: true,
        }
    }
}

/// The main MCP router that acts as a proxy to multiple MCP servers
pub struct McpRouter {
    config: RouterConfig,
    auth_context: Option<Arc<crate::AuthContext>>,
}

impl McpRouter {
    /// Create a new MCP router with the given configuration
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            auth_context: None,
        }
    }

    /// Set the authentication context for protected servers
    pub fn with_auth(mut self, auth_context: Arc<crate::AuthContext>) -> Self {
        self.auth_context = Some(auth_context);
        self
    }

    /// Initialize all backend connections
    pub async fn initialize(&self) -> Result<()> {
        // For now, we'll just log the servers we would connect to
        for server in &self.config.servers {
            tracing::info!("Would connect to backend server: {} ({})", server.name, server.id);
        }
        Ok(())
    }

    /// Start the MCP server
    pub async fn run(self) -> Result<()> {
        // Initialize backends
        self.initialize().await?;
        
        // Create server details
        let server_details = InitializeResult {
            server_info: Implementation {
                name: "JauAuth Router".to_string(),
                version: "1.0.0".to_string(),
            },
            capabilities: ServerCapabilities {
                tools: Some(ServerCapabilitiesTools { list_changed: None }),
                ..Default::default()
            },
            _meta: None,
        };
        
        // Create stdio transport
        let transport = StdioTransport::new(TransportOptions::default())?;
        
        // Create handler
        let handler = RouterHandler {
            config: self.config,
            auth_context: self.auth_context,
        };
        
        // Create and start server
        let server: ServerRuntime = server_runtime::create_server(server_details, transport, handler);
        server.start().await
            .map_err(|e| anyhow!("Server error: {:?}", e))
    }
}

/// MCP Server Handler for the router
struct RouterHandler {
    config: RouterConfig,
    auth_context: Option<Arc<crate::AuthContext>>,
}

#[async_trait::async_trait]
impl ServerHandler for RouterHandler {
    async fn handle_list_tools_request(&self, _request: ListToolsRequest) -> Result<ListToolsResult, ErrorResponse> {
        // For now, return a simple tool list
        let tools = vec![
            ToolInfo {
                name: "router:status".to_string(),
                description: Some("Check router status and connected servers".to_string()),
                input_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {}
                })),
            },
            ToolInfo {
                name: "router:list_servers".to_string(),
                description: Some("List all configured backend servers".to_string()),
                input_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {}
                })),
            },
        ];
        
        Ok(ListToolsResult { tools, _meta: None })
    }
    
    async fn handle_call_tool_request(&self, request: CallToolRequest) -> Result<CallToolResult, ErrorResponse> {
        match request.name.as_str() {
            "router:status" => {
                let content = vec![serde_json::json!({
                    "type": "text",
                    "text": format!("JauAuth Router is running with {} configured servers", self.config.servers.len())
                })];
                
                Ok(CallToolResult {
                    content,
                    is_error: Some(false),
                    _meta: None,
                })
            }
            
            "router:list_servers" => {
                let servers: Vec<_> = self.config.servers.iter()
                    .map(|s| serde_json::json!({
                        "id": s.id,
                        "name": s.name,
                        "requires_auth": s.requires_auth,
                    }))
                    .collect();
                
                let content = vec![serde_json::json!({
                    "type": "text",
                    "text": serde_json::to_string_pretty(&servers).unwrap()
                })];
                
                Ok(CallToolResult {
                    content,
                    is_error: Some(false),
                    _meta: None,
                })
            }
            
            _ => {
                Err(ErrorResponse {
                    code: -32601,
                    message: format!("Unknown tool: {}", request.name),
                    data: None,
                })
            }
        }
    }
}

/// Load router configuration from a file
pub async fn load_config(path: &str) -> Result<RouterConfig> {
    let content = tokio::fs::read_to_string(path).await?;
    let config: RouterConfig = serde_json::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RouterConfig::default();
        assert_eq!(config.timeout_ms, 30000);
        assert!(config.cache_tools);
        assert!(config.servers.is_empty());
    }
}