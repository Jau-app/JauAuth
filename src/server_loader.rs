//! Server configuration loading from both JSON file and database

use anyhow::{Result, Context};
use std::collections::HashMap;
use sqlx::SqlitePool;
use tracing::{info, warn};

use crate::{
    simple_router::{RouterConfig, BackendServer, validate_server_config},
    server_store::ServerStore,
    crypto::derive_key,
};

/// Load server configurations from both JSON file and database
pub async fn load_all_servers(
    config_path: &str,
    db_pool: SqlitePool,
    master_key: &[u8],
    user_id: Option<i64>,
) -> Result<RouterConfig> {
    // First, load the base configuration from JSON file
    let mut config = load_json_config(config_path).await?;
    
    // Then, load user-specific servers from database if user_id provided
    if let Some(user_id) = user_id {
        let encryption_key = derive_key(master_key, &format!("user-{}", user_id))?;
        let server_store = ServerStore::new(db_pool, encryption_key);
        
        // Get user's servers from database
        let db_servers = server_store.get_user_servers(user_id).await
            .context("Failed to load servers from database")?;
        
        info!("Loaded {} servers from database for user {}", db_servers.len(), user_id);
        
        // Convert and merge with config
        for db_server in db_servers {
            // Skip if server ID already exists in config (config takes precedence)
            if config.servers.iter().any(|s| s.id == db_server.server_id) {
                warn!("Server '{}' exists in both config and database, using config version", db_server.server_id);
                continue;
            }
            
            // Convert to BackendServer
            let backend_server = BackendServer {
                id: db_server.server_id,
                name: db_server.name,
                r#type: match db_server.server_type.as_str() {
                    "remote" => crate::simple_router::ServerType::Remote,
                    _ => crate::simple_router::ServerType::Local,
                },
                command: db_server.command,
                args: db_server.args,
                url: db_server.url,
                transport: match db_server.transport_type.as_str() {
                    "sse" => crate::simple_router::TransportType::Sse,
                    "websocket" => crate::simple_router::TransportType::WebSocket,
                    _ => crate::simple_router::TransportType::Sse, // Default for local servers
                },
                auth: None, // TODO: Support auth config in database
                env: db_server.env,
                requires_auth: db_server.requires_auth,
                allowed_users: db_server.allowed_users,
                sandbox: db_server.sandbox_config,
                timeout_ms: db_server.timeout_ms as u64,
                retry: None, // TODO: Support retry config in database
                tls: None, // TODO: Support TLS config in database
            };
            
            // Validate before adding
            if let Err(e) = validate_server_config(&backend_server).await {
                warn!("Invalid server '{}' from database: {}", backend_server.id, e);
                continue;
            }
            
            config.servers.push(backend_server);
        }
    } else {
        // No user context - load all enabled servers (for system-wide usage)
        let encryption_key = derive_key(master_key, "system")?;
        let server_store = ServerStore::new(db_pool, encryption_key);
        
        let all_servers = server_store.get_all_servers().await
            .context("Failed to load servers from database")?;
        
        info!("Loaded {} system-wide servers from database", all_servers.len());
        
        for server in all_servers {
            if config.servers.iter().any(|s| s.id == server.id) {
                continue;
            }
            
            if let Err(e) = validate_server_config(&server).await {
                warn!("Invalid server '{}' from database: {}", server.id, e);
                continue;
            }
            
            config.servers.push(server);
        }
    }
    
    info!("Total servers loaded: {} (JSON: {}, Database: {})",
        config.servers.len(),
        config_path,
        config.servers.len() - load_json_config(config_path).await?.servers.len()
    );
    
    Ok(config)
}

/// Load configuration from JSON file only
async fn load_json_config(path: &str) -> Result<RouterConfig> {
    let content = tokio::fs::read_to_string(path).await
        .context("Failed to read configuration file")?;
    
    let config: RouterConfig = serde_json::from_str(&content)
        .context("Failed to parse configuration file")?;
    
    // Validate all server configurations
    for server in &config.servers {
        validate_server_config(server).await
            .with_context(|| format!("Invalid configuration for server '{}'", server.id))?;
    }
    
    Ok(config)
}

/// Mask sensitive values showing only first 4 and last 4 characters
fn mask_sensitive_value(value: &str) -> String {
    if value.len() <= 8 {
        // Too short to mask properly
        "***".to_string()
    } else {
        format!("{}...{}", &value[..4], &value[value.len()-4..])
    }
}

/// Create a merged configuration for display (masks sensitive data)
pub fn create_display_config(config: &RouterConfig) -> serde_json::Value {
    let servers: Vec<serde_json::Value> = config.servers.iter().map(|server| {
        let mut display = serde_json::json!({
            "id": server.id,
            "name": server.name,
            "type": format!("{:?}", server.r#type),
            "requires_auth": server.requires_auth,
            "allowed_users": server.allowed_users,
            "timeout_ms": server.timeout_ms,
        });
        
        // Add command or URL based on type
        match server.r#type {
            crate::simple_router::ServerType::Local => {
                if let Some(cmd) = &server.command {
                    display["command"] = serde_json::Value::String(cmd.clone());
                    display["args"] = serde_json::json!(server.args);
                }
            }
            crate::simple_router::ServerType::Remote => {
                if let Some(url) = &server.url {
                    display["url"] = serde_json::Value::String(url.clone());
                }
            }
        }
        
        // Mask ALL environment variables
        if !server.env.is_empty() {
            let masked_env: HashMap<String, String> = server.env.iter()
                .map(|(k, v)| (k.clone(), mask_sensitive_value(v)))
                .collect();
            display["env"] = serde_json::json!(masked_env);
        }
        
        display
    }).collect();
    
    serde_json::json!({
        "timeout_ms": config.timeout_ms,
        "cache_tools": config.cache_tools,
        "servers": servers,
        "server_count": config.servers.len(),
    })
}