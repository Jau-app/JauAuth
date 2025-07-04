//! Common test utilities and helpers

use jau_auth::{AuthConfig, AuthContext};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Create a test database
pub async fn test_db() -> SqlitePool {
    let db_url = format!("sqlite::memory:");
    let pool = SqlitePool::connect(&db_url).await.unwrap();
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap();
    
    pool
}

/// Create a test auth context
pub async fn test_auth_context() -> AuthContext {
    let config = AuthConfig::builder()
        .app_name("test-app")
        .database_url(":memory:")
        .build();
    
    AuthContext::new(config).await.unwrap()
}

/// Create a test user
pub async fn create_test_user(db: &SqlitePool, username: &str) -> i64 {
    let user_id = sqlx::query!(
        r#"
        INSERT INTO users (telegram_id, username, email, auth_hash, created_at)
        VALUES (?1, ?2, ?3, ?4, datetime('now'))
        RETURNING id
        "#,
        Uuid::new_v4().to_string(),
        username,
        format!("{}@test.com", username),
        "test_hash"
    )
    .fetch_one(db)
    .await
    .unwrap()
    .id;
    
    user_id
}

/// Mock MCP server for testing
pub mod mock_mcp {
    use std::process::Stdio;
    use tokio::process::{Child, Command};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    
    pub struct MockMcpServer {
        process: Child,
    }
    
    impl MockMcpServer {
        pub async fn start(name: &str) -> Self {
            let mut process = Command::new("node")
                .arg("test-servers/mock-server.js")
                .arg(name)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("Failed to start mock MCP server");
            
            // Wait for server to be ready
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            Self { process }
        }
        
        pub async fn stop(mut self) {
            let _ = self.process.kill().await;
        }
    }
}

/// Test configuration builder
pub struct TestConfigBuilder {
    servers: Vec<jau_auth::simple_router::BackendServer>,
}

impl TestConfigBuilder {
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }
    
    pub fn with_server(mut self, id: &str, command: &str) -> Self {
        use jau_auth::simple_router::BackendServer;
        use jau_auth::sandbox::{SandboxConfig, SandboxStrategy};
        
        self.servers.push(BackendServer {
            id: id.to_string(),
            name: format!("Test {}", id),
            command: command.to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
            requires_auth: false,
            allowed_users: vec![],
            sandbox: SandboxConfig {
                strategy: SandboxStrategy::None,
                env_passthrough: vec!["PATH".to_string()],
            },
        });
        self
    }
    
    pub fn build(self) -> jau_auth::simple_router::RouterConfig {
        jau_auth::simple_router::RouterConfig {
            servers: self.servers,
            timeout_ms: 5000,
            cache_tools: true,
        }
    }
}

/// Assert that a future completes within a timeout
#[macro_export]
macro_rules! assert_timeout {
    ($duration:expr, $future:expr) => {
        tokio::time::timeout($duration, $future)
            .await
            .expect("Operation timed out")
    };
}

/// Assert that a result is an error matching a pattern
#[macro_export]
macro_rules! assert_error_matches {
    ($result:expr, $pattern:pat) => {
        match $result {
            Err($pattern) => (),
            Err(e) => panic!("Expected error matching {}, got {:?}", stringify!($pattern), e),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    };
}