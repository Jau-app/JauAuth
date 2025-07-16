use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use reqwest::{Client, ClientBuilder, header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use tokio::sync::Mutex;
use std::sync::Arc;

use super::{Transport, AuthConfig, RetryConfig, TlsConfig};
use crate::mcp_types::{InitializeResult, ListToolsResult, JsonRpcRequest, JsonRpcResponse};
use crate::Error;

/// SSE transport for remote MCP servers
pub struct SseTransport {
    client: Client,
    url: String,
    auth: AuthConfig,
    timeout: Duration,
    retry: RetryConfig,
    request_id: Arc<Mutex<u64>>,
    // Note: eventsource_client::Client is a trait, we'll handle SSE differently
    _event_source: Arc<Mutex<Option<String>>>, // Placeholder for now
}

impl SseTransport {
    /// Create a new SSE transport for remote server
    pub async fn new(
        url: String,
        auth: AuthConfig,
        timeout_ms: u64,
        retry: RetryConfig,
        tls: TlsConfig,
    ) -> Result<Self, Error> {
        // Build HTTP client with TLS config
        let mut client_builder = ClientBuilder::new()
            .timeout(Duration::from_millis(timeout_ms))
            .danger_accept_invalid_certs(!tls.verify_cert);

        // Add custom CA if provided
        if let Some(ca_path) = &tls.ca_cert {
            let ca_cert = std::fs::read(ca_path)
                .map_err(|e| Error::ConfigError(format!("Failed to read CA cert: {}", e)))?;
            let ca = reqwest::Certificate::from_pem(&ca_cert)
                .map_err(|e| Error::ConfigError(format!("Invalid CA cert: {}", e)))?;
            client_builder = client_builder.add_root_certificate(ca);
        }

        // Add client cert for mTLS
        // Note: Current version of reqwest doesn't support client certificates in this way
        // TODO: Implement mTLS support with a different approach or update reqwest
        if tls.client_cert.is_some() || tls.client_key.is_some() {
            tracing::warn!("Client certificate authentication (mTLS) not yet implemented");
        }

        let client = client_builder.build()
            .map_err(|e| Error::ConfigError(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            client,
            url,
            auth,
            timeout: Duration::from_millis(timeout_ms),
            retry,
            request_id: Arc::new(Mutex::new(1)),
            _event_source: Arc::new(Mutex::new(None)),
        })
    }

    /// Build authorization headers based on auth config
    fn build_auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        match &self.auth {
            AuthConfig::None => {},
            AuthConfig::Bearer { token } => {
                if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                    headers.insert(AUTHORIZATION, value);
                }
            },
            AuthConfig::Basic { username, password } => {
                use base64::Engine;
                let credentials = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                if let Ok(value) = HeaderValue::from_str(&format!("Basic {}", credentials)) {
                    headers.insert(AUTHORIZATION, value);
                }
            },
            AuthConfig::OAuth { .. } => {
                // OAuth flow would be handled separately, resulting in a bearer token
                tracing::warn!("OAuth not yet implemented, treating as no auth");
            },
            AuthConfig::Custom { headers: custom } => {
                for (key, value) in custom {
                    if let (Ok(name), Ok(val)) = (
                        reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                        HeaderValue::from_str(value)
                    ) {
                        headers.insert(name, val);
                    }
                }
            },
        }

        headers
    }

    /// Send request with retry logic
    async fn send_request_with_retry(
        &self, 
        method: &str, 
        params: Value
    ) -> Result<JsonRpcResponse, Error> {
        let mut attempt = 0;
        let mut backoff = self.retry.initial_backoff_ms;

        loop {
            match self.send_request_once(method, params.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) if attempt < self.retry.max_attempts => {
                    attempt += 1;
                    tracing::warn!(
                        "Request failed (attempt {}/{}): {}", 
                        attempt, 
                        self.retry.max_attempts, 
                        e
                    );
                    
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                    
                    // Exponential backoff with cap
                    backoff = (backoff * 2).min(self.retry.max_backoff_ms);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Send a single request
    async fn send_request_once(&self, method: &str, params: Value) -> Result<JsonRpcResponse, Error> {
        let request_id = {
            let mut id = self.request_id.lock().await;
            let current = *id;
            *id += 1;
            current
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(request_id)),
            method: method.to_string(),
            params: Some(params),
        };

        let response = self.client
            .post(&self.url)
            .headers(self.build_auth_headers())
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::NetworkError(format!(
                "HTTP {} from {}", 
                response.status(), 
                self.url
            )));
        }

        let json_response: JsonRpcResponse = response.json().await
            .map_err(|e| Error::NetworkError(format!("Invalid JSON response: {}", e)))?;

        if let Some(error) = json_response.error {
            return Err(Error::BackendError(format!(
                "Remote error: {} ({})", 
                error.message, 
                error.code
            )));
        }

        Ok(json_response)
    }

    /// Connect to SSE endpoint for streaming
    async fn connect_sse(&self) -> Result<(), Error> {
        // TODO: Implement proper SSE support
        // For now, we'll use regular HTTP requests
        tracing::info!("SSE connection would be established to: {}", self.url);
        Ok(())
    }
}

#[async_trait]
impl Transport for SseTransport {
    async fn initialize(&mut self, client_info: Value) -> Result<InitializeResult, Error> {
        let params = serde_json::json!({
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": client_info
        });

        let response = self.send_request_with_retry("initialize", params).await?;
        
        let result = response.result
            .ok_or_else(|| Error::BackendError("No result in initialize response".to_string()))?;
        
        // After successful init, connect SSE for streaming
        self.connect_sse().await?;
        
        serde_json::from_value(result)
            .map_err(|e| Error::BackendError(format!("Invalid initialize result: {}", e)))
    }

    async fn list_tools(&mut self) -> Result<ListToolsResult, Error> {
        let response = self.send_request_with_retry("tools/list", serde_json::json!({})).await?;
        
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

        let response = self.send_request_with_retry("tools/call", params).await?;
        
        response.result
            .ok_or_else(|| Error::BackendError("No result in tool call response".to_string()))
    }

    async fn shutdown(&mut self) -> Result<(), Error> {
        // Send shutdown notification
        let _ = self.send_request_once("shutdown", serde_json::json!({})).await;
        
        // Close SSE connection
        // TODO: Close actual SSE connection when implemented
        
        Ok(())
    }

    async fn health_check(&mut self) -> Result<bool, Error> {
        // Simple ping to check if server is alive
        match self.send_request_once("ping", serde_json::json!({})).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn transport_type(&self) -> &'static str {
        "sse"
    }
}