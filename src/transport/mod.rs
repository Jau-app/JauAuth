use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mcp_types::{InitializeResult, ListToolsResult};
use crate::Error;

/// Transport trait for MCP server communication
/// Abstracts over stdio (local) and HTTP/SSE (remote) transports
#[async_trait]
pub trait Transport: Send + Sync {
    /// Initialize the MCP connection
    async fn initialize(&mut self, client_info: Value) -> Result<InitializeResult, Error>;
    
    /// List available tools from the server
    async fn list_tools(&mut self) -> Result<ListToolsResult, Error>;
    
    /// Call a tool on the server
    async fn call_tool(&mut self, name: &str, args: Value) -> Result<Value, Error>;
    
    /// Gracefully shutdown the connection
    async fn shutdown(&mut self) -> Result<(), Error>;
    
    /// Check if the transport is healthy
    async fn health_check(&mut self) -> Result<bool, Error> {
        Ok(true) // Default implementation
    }
    
    /// Get transport type for debugging/metrics
    fn transport_type(&self) -> &'static str;
}

/// Configuration for different transport types
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TransportConfig {
    /// Local process communication via stdio
    Local {
        command: String,
        args: Vec<String>,
        #[serde(default)]
        env: std::collections::HashMap<String, String>,
    },
    /// Remote HTTP/SSE communication
    Remote {
        url: String,
        #[serde(default)]
        auth: AuthConfig,
        #[serde(default = "default_timeout")]
        timeout_ms: u64,
        #[serde(default)]
        retry: RetryConfig,
        #[serde(default)]
        tls: TlsConfig,
    },
}

/// Authentication configuration for remote servers
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthConfig {
    /// No authentication
    None,
    /// Bearer token authentication
    Bearer { token: String },
    /// Basic authentication
    Basic { username: String, password: String },
    /// OAuth 2.0 authentication
    OAuth {
        provider: String,
        client_id: String,
        client_secret: String,
        #[serde(default)]
        scopes: Vec<String>,
    },
    /// Custom headers
    Custom {
        headers: std::collections::HashMap<String, String>,
    },
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig::None
    }
}

/// Retry configuration for remote connections
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RetryConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_backoff_ms")]
    pub initial_backoff_ms: u64,
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            initial_backoff_ms: default_backoff_ms(),
            max_backoff_ms: default_max_backoff_ms(),
        }
    }
}

/// TLS configuration for remote connections
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TlsConfig {
    #[serde(default = "default_true")]
    pub verify_cert: bool,
    pub ca_cert: Option<std::path::PathBuf>,
    pub client_cert: Option<std::path::PathBuf>,
    pub client_key: Option<std::path::PathBuf>,
}

// Default value functions for serde
fn default_timeout() -> u64 { 30000 }
fn default_max_attempts() -> u32 { 3 }
fn default_backoff_ms() -> u64 { 1000 }
fn default_max_backoff_ms() -> u64 { 30000 }
fn default_true() -> bool { true }

pub mod stdio;
pub mod sse;

pub use stdio::StdioTransport;
pub use sse::SseTransport;

/// Create a transport based on the configuration
pub async fn create_transport(config: TransportConfig) -> Result<Box<dyn Transport>, Error> {
    match config {
        TransportConfig::Local { command, args, env } => {
            // TODO: server_id should be passed in, using "unknown" for now
            Ok(Box::new(StdioTransport::new(command, args, env, "unknown".to_string())?))
        }
        TransportConfig::Remote { url, auth, timeout_ms, retry, tls } => {
            Ok(Box::new(SseTransport::new(url, auth, timeout_ms, retry, tls).await?))
        }
    }
}