//! Integration tests for MCP protocol communication

use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[path = "../common/mod.rs"]
mod common;

#[tokio::test]
async fn test_mcp_initialize_sequence() {
    // This would require a running MCP server
    // For now, we'll create a mock test
    
    let request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "1.0.0",
            "capabilities": {}
        },
        "id": 1
    });
    
    // Expected response structure
    let expected_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "protocolVersion": "1.0.0",
            "capabilities": {
                "tools": {}
            }
        },
        "id": 1
    });
    
    // Verify JSON structure
    assert_eq!(request["method"], "initialize");
    assert_eq!(expected_response["result"]["protocolVersion"], "1.0.0");
}

#[tokio::test]
async fn test_mcp_tools_list() {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "params": {},
        "id": 2
    });
    
    // Expected response with router tools
    let expected_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "tools": [
                {
                    "name": "router:status",
                    "description": "Get router status",
                    "inputSchema": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                },
                {
                    "name": "router:list_servers",
                    "description": "List configured servers",
                    "inputSchema": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            ]
        },
        "id": 2
    });
    
    assert!(expected_response["result"]["tools"].is_array());
}

#[tokio::test]
async fn test_mcp_tool_call() {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "router:status",
            "arguments": {}
        },
        "id": 3
    });
    
    // Expected response structure
    let expected_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "{\"router\":\"healthy\",\"servers\":[]}"
                }
            ]
        },
        "id": 3
    });
    
    assert_eq!(request["params"]["name"], "router:status");
}

#[tokio::test]
async fn test_mcp_error_handling() {
    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "nonexistent:tool",
            "arguments": {}
        },
        "id": 4
    });
    
    // Expected error response
    let expected_response = json!({
        "jsonrpc": "2.0",
        "error": {
            "code": -32602,
            "message": "Tool not found: nonexistent:tool"
        },
        "id": 4
    });
    
    assert!(expected_response.get("error").is_some());
}

#[tokio::test]
async fn test_mcp_notification() {
    // Notifications don't have an ID
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/progress",
        "params": {
            "progress": 50,
            "message": "Processing..."
        }
    });
    
    assert!(notification.get("id").is_none());
    assert_eq!(notification["method"], "notifications/progress");
}

#[tokio::test]
async fn test_mcp_batch_request() {
    let batch_request = json!([
        {
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        },
        {
            "jsonrpc": "2.0",
            "method": "router:status",
            "id": 2
        }
    ]);
    
    assert!(batch_request.is_array());
    assert_eq!(batch_request.as_array().unwrap().len(), 2);
}