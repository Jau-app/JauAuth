use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::process::Stdio;

use super::Transport;
use crate::mcp_types::{InitializeResult, ListToolsResult, JsonRpcRequest, JsonRpcResponse};
use crate::Error;

/// Stdio transport for local MCP server processes
pub struct StdioTransport {
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    stdout: Arc<Mutex<BufReader<tokio::process::ChildStdout>>>,
    request_id: Arc<Mutex<u64>>,
    server_id: String,
}

impl StdioTransport {
    /// Create a new stdio transport from an already spawned process
    pub fn from_child(
        mut child: Child,
        server_id: String,
    ) -> Result<Self, Error> {
        let stdin = child.stdin.take()
            .ok_or_else(|| Error::SpawnError("Failed to get stdin".to_string()))?;
        
        let stdout = child.stdout.take()
            .ok_or_else(|| Error::SpawnError("Failed to get stdout".to_string()))?;

        Ok(Self {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
            request_id: Arc::new(Mutex::new(1)),
            server_id,
        })
    }
    
    /// Create a new stdio transport by spawning a local process
    pub fn new(
        command: String, 
        args: Vec<String>, 
        env: HashMap<String, String>,
        server_id: String,
    ) -> Result<Self, Error> {
        let mut cmd = Command::new(&command);
        cmd.args(&args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = cmd.spawn()
            .map_err(|e| Error::SpawnError(format!("Failed to spawn {}: {}", command, e)))?;

        Self::from_child(child, server_id)
    }

    /// Send a request and wait for response
    async fn send_request(&self, method: &str, params: Value) -> Result<JsonRpcResponse, Error> {
        // Get next request ID
        let request_id = {
            let mut id = self.request_id.lock().await;
            let current = *id;
            *id += 1;
            current
        };

        // Create request
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(request_id)),
            method: method.to_string(),
            params: Some(params),
        };

        // Send request
        let request_str = serde_json::to_string(&request)?;
        tracing::debug!("Sending request to {}: {}", self.server_id, request_str);
        
        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(request_str.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }

        // Read response
        let response_str = {
            let mut stdout = self.stdout.lock().await;
            let mut line = String::new();
            stdout.read_line(&mut line).await?;
            line
        };

        if response_str.is_empty() {
            return Err(Error::BackendError(format!("{} closed connection", self.server_id)));
        }

        tracing::debug!("Received response from {}: {}", self.server_id, response_str);
        
        // Parse response
        let response: JsonRpcResponse = serde_json::from_str(&response_str)?;
        
        // Check for errors
        if let Some(error) = response.error {
            return Err(Error::BackendError(format!(
                "{} error: {} ({})", 
                self.server_id, 
                error.message, 
                error.code
            )));
        }

        Ok(response)
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn initialize(&mut self, client_info: Value) -> Result<InitializeResult, Error> {
        let params = serde_json::json!({
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": client_info
        });

        let response = self.send_request("initialize", params).await?;
        
        let result = response.result
            .ok_or_else(|| Error::BackendError("No result in initialize response".to_string()))?;
        
        serde_json::from_value(result)
            .map_err(|e| Error::BackendError(format!("Invalid initialize result: {}", e)))
    }

    async fn list_tools(&mut self) -> Result<ListToolsResult, Error> {
        let response = self.send_request("tools/list", serde_json::json!({})).await?;
        
        let result = response.result
            .ok_or_else(|| Error::BackendError("No result in tools/list response".to_string()))?;
        
        serde_json::from_value(result)
            .map_err(|e| Error::BackendError(format!("Invalid tools/list result: {}", e)))
    }

    async fn call_tool(&mut self, name: &str, args: Value) -> Result<Value, Error> {
        let params = serde_json::json!({
            "name": name,
            "arguments": args
        });

        let response = self.send_request("tools/call", params).await?;
        
        response.result
            .ok_or_else(|| Error::BackendError("No result in tool call response".to_string()))
    }

    async fn shutdown(&mut self) -> Result<(), Error> {
        // Try to send shutdown notification
        let _ = self.send_request("shutdown", serde_json::json!({})).await;
        
        // Kill the process
        let mut child = self.child.lock().await;
        let _ = child.kill().await;
        
        Ok(())
    }

    async fn health_check(&mut self) -> Result<bool, Error> {
        // For stdio, check if process is still alive
        let mut child = self.child.lock().await;
        match child.try_wait() {
            Ok(None) => Ok(true), // Still running
            Ok(Some(status)) => {
                tracing::warn!("{} exited with status: {:?}", self.server_id, status);
                Ok(false)
            }
            Err(e) => {
                tracing::error!("Failed to check {} status: {}", self.server_id, e);
                Ok(false)
            }
        }
    }

    fn transport_type(&self) -> &'static str {
        "stdio"
    }
}