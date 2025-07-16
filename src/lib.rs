//! JauAuth - Universal MCP Authentication System
//! 
//! A secure, plug-and-play authentication system for MCP servers.

pub mod auth;
pub mod auth_middleware;
pub mod config;
pub mod dashboard;
pub mod database;
pub mod device;
pub mod middleware;
// pub mod router; // Temporarily disabled
// Use the new backend_manager_v2 that supports remote servers
pub use backend_manager_v2 as backend_manager;
pub mod backend_manager_v2;
pub mod sandbox;
pub mod security;
pub mod simple_router;
pub mod session;
pub mod web;
pub mod webauthn;
pub mod mcp_api;
pub mod rate_limit;
pub mod transport;
pub mod mcp_types;

pub use auth::{AuthService, AuthError};
pub use config::{AuthConfig, PermissionGroup};
pub use middleware::AuthMiddleware;
pub use session::{Session, SessionManager};

/// General error type for JauAuth
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Backend error: {0}")]
    BackendError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Spawn error: {0}")]
    SpawnError(String),
    
    #[error("Config error: {0}")]
    ConfigError(String),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}

use std::sync::Arc;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

/// Quick protection helper for MCP servers
pub async fn quick_protect<S>(
    server: S,
    app_name: &str,
) -> Result<AuthMiddleware<S>, AuthError> {
    let config = AuthConfig::builder()
        .app_name(app_name)
        .build();
    
    AuthMiddleware::new(server, config).await
}

/// Main authentication context shared across the application
#[derive(Clone)]
pub struct AuthContext {
    pub db: SqlitePool,
    pub config: Arc<AuthConfig>,
    pub session_manager: Arc<RwLock<SessionManager>>,
}

impl AuthContext {
    pub async fn new(config: AuthConfig) -> Result<Self, AuthError> {
        // Initialize database
        let db = database::init_db(&config.database_url).await?;
        
        // Create session manager
        let session_manager = Arc::new(RwLock::new(
            SessionManager::new(config.session_duration)
        ));
        
        Ok(Self {
            db,
            config: Arc::new(config),
            session_manager,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_context_creation() {
        let config = AuthConfig::builder()
            .app_name("test-app")
            .database_url(":memory:")
            .build();
        
        let result = AuthContext::new(config).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_config_builder() {
        let config = AuthConfig::builder()
            .app_name("test")
            .port(8080)
            .build();
        
        assert_eq!(config.app_name, "test");
        assert_eq!(config.port, 8080);
    }
}