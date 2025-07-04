//! End-to-end test for complete user workflow

use reqwest::Client;
use serde_json::json;

#[tokio::test]
#[ignore] // Run with: cargo test --test e2e_full_workflow -- --ignored
async fn test_complete_user_workflow() {
    // This test requires the server to be running
    // Start with: cargo run -- combined
    
    let client = Client::new();
    let base_url = "http://localhost:7447";
    
    // Step 1: Check health
    let response = client
        .get(format!("{}/api/health", base_url))
        .send()
        .await
        .expect("Failed to connect to server");
    
    assert_eq!(response.status(), 200);
    
    // Step 2: Get dashboard overview
    let response = client
        .get(format!("{}/api/dashboard/overview", base_url))
        .send()
        .await
        .unwrap();
    
    if response.status() == 401 {
        println!("Authentication required - skipping protected endpoints");
        return;
    }
    
    let overview: serde_json::Value = response.json().await.unwrap();
    assert!(overview.get("total_servers").is_some());
    
    // Step 3: List servers
    let response = client
        .get(format!("{}/api/dashboard/servers", base_url))
        .send()
        .await
        .unwrap();
    
    let servers: Vec<serde_json::Value> = response.json().await.unwrap();
    println!("Found {} configured servers", servers.len());
    
    // Step 4: Test MCP API
    let response = client
        .get(format!("{}/api/mcp/status", base_url))
        .send()
        .await
        .unwrap();
    
    let status: serde_json::Value = response.json().await.unwrap();
    assert_eq!(status["router"], "healthy");
    
    // Step 5: List available tools
    let response = client
        .get(format!("{}/api/mcp/tools", base_url))
        .send()
        .await
        .unwrap();
    
    let tools_response: serde_json::Value = response.json().await.unwrap();
    let tools = tools_response["tools"].as_array().unwrap();
    assert!(!tools.is_empty());
    
    // Step 6: Call a tool
    let response = client
        .post(format!("{}/api/mcp/tool/call", base_url))
        .json(&json!({
            "tool": "router:status",
            "arguments": {}
        }))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let result: serde_json::Value = response.json().await.unwrap();
    assert!(result.get("result").is_some());
}

#[tokio::test]
#[ignore]
async fn test_authentication_flow() {
    let client = Client::new();
    let base_url = "http://localhost:7447";
    
    // Step 1: Register new user
    let response = client
        .post(format!("{}/api/register", base_url))
        .json(&json!({
            "telegram_id": "test_123",
            "username": "testuser",
            "email": "test@example.com",
            "pin": "1234"
        }))
        .send()
        .await;
    
    if let Ok(resp) = response {
        if resp.status() == 200 {
            println!("User registered successfully");
        }
    }
    
    // Step 2: Login
    let response = client
        .post(format!("{}/api/login", base_url))
        .json(&json!({
            "username": "testuser",
            "email": "test@example.com",
            "pin": "1234",
            "device_hash": "test_device_123"
        }))
        .send()
        .await;
    
    if let Ok(resp) = response {
        if resp.status() == 200 {
            let login_response: serde_json::Value = resp.json().await.unwrap();
            let token = login_response["token"].as_str().unwrap();
            println!("Login successful, token: {}...", &token[..20]);
            
            // Step 3: Access protected endpoint with token
            let response = client
                .get(format!("{}/api/dashboard/overview", base_url))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .unwrap();
            
            assert_eq!(response.status(), 200);
        }
    }
}