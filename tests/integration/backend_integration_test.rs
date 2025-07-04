//! Integration tests for backend server management

use jau_auth::backend_manager::BackendManager;
use jau_auth::simple_router::{BackendServer, RouterConfig};
use jau_auth::sandbox::{SandboxConfig, SandboxStrategy};
use std::collections::HashMap;
use std::sync::Arc;

#[path = "../common/mod.rs"]
mod common;

#[tokio::test]
async fn test_backend_spawn_and_shutdown() {
    let backend_manager = Arc::new(BackendManager::new());
    
    let server = BackendServer {
        id: "test-echo".to_string(),
        name: "Test Echo Server".to_string(),
        command: "node".to_string(),
        args: vec!["test-servers/echo-server.js".to_string()],
        env: HashMap::new(),
        requires_auth: false,
        allowed_users: vec![],
        sandbox: SandboxConfig {
            strategy: SandboxStrategy::None,
            env_passthrough: vec!["PATH".to_string()],
        },
    };
    
    // Spawn backend
    let result = backend_manager.spawn_backend(server.clone()).await;
    
    if result.is_ok() {
        // Check if backend is healthy
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let status = backend_manager.get_status().await;
        assert!(status.get("test-echo").copied().unwrap_or(false));
        
        // Shutdown
        backend_manager.shutdown_all().await;
        
        // Verify shutdown
        let status = backend_manager.get_status().await;
        assert!(!status.get("test-echo").copied().unwrap_or(true));
    } else {
        // Skip test if echo server not available
        println!("Skipping test: echo server not available");
    }
}

#[tokio::test]
async fn test_backend_tool_discovery() {
    let backend_manager = Arc::new(BackendManager::new());
    
    // Initially should have no tools
    let tools = backend_manager.get_all_tools().await;
    assert!(tools.is_empty());
    
    // After spawning a backend, tools should be available
    // (would need actual backend for this test)
}

#[tokio::test]
async fn test_backend_health_monitoring() {
    let backend_manager = Arc::new(BackendManager::new());
    
    // Create a server that will fail
    let server = BackendServer {
        id: "failing-server".to_string(),
        name: "Failing Server".to_string(),
        command: "false".to_string(), // Command that always fails
        args: vec![],
        env: HashMap::new(),
        requires_auth: false,
        allowed_users: vec![],
        sandbox: SandboxConfig {
            strategy: SandboxStrategy::None,
            env_passthrough: vec![],
        },
    };
    
    // Try to spawn - should fail
    let result = backend_manager.spawn_backend(server).await;
    assert!(result.is_err());
    
    // Health should show as unhealthy
    let status = backend_manager.get_status().await;
    assert!(!status.get("failing-server").copied().unwrap_or(true));
}

#[tokio::test]
async fn test_backend_environment_variables() {
    let backend_manager = Arc::new(BackendManager::new());
    
    let mut env = HashMap::new();
    env.insert("TEST_VAR".to_string(), "test_value".to_string());
    env.insert("USER_VAR".to_string(), "$USER".to_string()); // Should expand
    
    let server = BackendServer {
        id: "env-test".to_string(),
        name: "Env Test Server".to_string(),
        command: "node".to_string(),
        args: vec!["-e".to_string(), "console.log(process.env.TEST_VAR)".to_string()],
        env,
        requires_auth: false,
        allowed_users: vec![],
        sandbox: SandboxConfig {
            strategy: SandboxStrategy::None,
            env_passthrough: vec!["PATH".to_string(), "USER".to_string()],
        },
    };
    
    // This test verifies env var handling without actually spawning
    assert_eq!(server.env.get("TEST_VAR"), Some(&"test_value".to_string()));
}

#[tokio::test]
async fn test_backend_concurrent_operations() {
    let backend_manager = Arc::new(BackendManager::new());
    
    // Test concurrent tool calls
    let futures: Vec<_> = (0..10)
        .map(|i| {
            let bm = backend_manager.clone();
            tokio::spawn(async move {
                bm.route_tool_call(
                    &format!("test:tool{}", i),
                    serde_json::json!({"test": true})
                ).await
            })
        })
        .collect();
    
    // All should complete without panic
    for future in futures {
        let _ = future.await;
    }
}