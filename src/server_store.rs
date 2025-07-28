//! Secure database storage for server configurations

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, FromRow};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::{
    crypto::{encrypt_string, decrypt_string},
    simple_router::BackendServer,
    sandbox::SandboxConfig,
};

/// Database representation of a server configuration
#[derive(Debug, FromRow)]
pub struct DbServer {
    pub id: Option<i64>,  // Can be NULL for autoincrement
    pub server_id: String,
    pub user_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub server_type: String,
    pub transport_type: String,
    
    // Encrypted fields
    pub command_encrypted: Option<String>,
    pub args_encrypted: Option<String>,
    pub url_encrypted: Option<String>,
    pub env_encrypted: Option<String>,
    
    pub requires_auth: bool,
    pub allowed_users_encrypted: Option<String>,
    pub sandbox_config_encrypted: Option<String>,
    
    pub timeout_ms: Option<i64>,  // Can be NULL with default
    pub auto_start: bool,
    pub enabled: bool,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

/// Server configuration for API responses (decrypted)
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub id: i64,
    pub server_id: String,
    pub name: String,
    pub description: Option<String>,
    pub server_type: String,
    pub transport_type: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env: HashMap<String, String>,
    pub requires_auth: bool,
    pub allowed_users: Vec<String>,
    pub sandbox_config: SandboxConfig,
    pub timeout_ms: i64,
    pub auto_start: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Server store for database operations
pub struct ServerStore {
    pool: SqlitePool,
    encryption_key: Vec<u8>,
}

impl ServerStore {
    /// Create a new server store
    pub fn new(pool: SqlitePool, encryption_key: Vec<u8>) -> Self {
        Self { pool, encryption_key }
    }
    
    /// Add a new server configuration
    pub async fn add_server(&self, user_id: i64, server: &BackendServer) -> Result<i64> {
        // Encrypt sensitive fields
        let command_encrypted = server.command.as_ref()
            .map(|cmd| encrypt_string(cmd, &self.encryption_key))
            .transpose()?;
            
        let args_encrypted = encrypt_string(
            &serde_json::to_string(&server.args)?, 
            &self.encryption_key
        )?;
        
        let url_encrypted = server.url.as_ref()
            .map(|url| encrypt_string(url, &self.encryption_key))
            .transpose()?;
            
        let env_encrypted = encrypt_string(
            &serde_json::to_string(&server.env)?, 
            &self.encryption_key
        )?;
        
        let allowed_users_encrypted = encrypt_string(
            &serde_json::to_string(&server.allowed_users)?, 
            &self.encryption_key
        )?;
        
        let sandbox_config_encrypted = encrypt_string(
            &serde_json::to_string(&server.sandbox)?, 
            &self.encryption_key
        )?;
        
        let timeout_ms = server.timeout_ms as i64;
        
        let result = sqlx::query!(
            r#"
            INSERT INTO servers (
                server_id, user_id, name, description, server_type, transport_type,
                command_encrypted, args_encrypted, url_encrypted, env_encrypted,
                requires_auth, allowed_users_encrypted, sandbox_config_encrypted,
                timeout_ms, auto_start, enabled
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            "#,
            server.id,
            user_id,
            server.name,
            None::<String>, // description
            "local", // server_type
            "stdio", // transport_type
            command_encrypted,
            args_encrypted,
            url_encrypted,
            env_encrypted,
            server.requires_auth,
            allowed_users_encrypted,
            sandbox_config_encrypted,
            timeout_ms,
            true, // auto_start
            true  // enabled
        )
        .execute(&self.pool)
        .await
        .context("Failed to insert server")?;
        
        Ok(result.last_insert_rowid())
    }
    
    /// Get all servers for a user
    pub async fn get_user_servers(&self, user_id: i64) -> Result<Vec<ServerConfig>> {
        let db_servers = sqlx::query_as!(
            DbServer,
            r#"
            SELECT 
                id,
                server_id,
                user_id,
                name,
                description,
                server_type,
                transport_type,
                command_encrypted,
                args_encrypted,
                url_encrypted,
                env_encrypted,
                requires_auth,
                allowed_users_encrypted,
                sandbox_config_encrypted,
                timeout_ms,
                auto_start,
                enabled,
                created_at as "created_at: DateTime<Utc>",
                updated_at as "updated_at: DateTime<Utc>",
                last_used as "last_used: _"
            FROM servers 
            WHERE user_id = ?1 AND enabled = true
            ORDER BY name
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        // Decrypt and convert to ServerConfig
        let mut servers = Vec::new();
        for db_server in db_servers {
            servers.push(self.decrypt_server(db_server).await?);
        }
        
        Ok(servers)
    }
    
    /// Get a specific server
    pub async fn get_server(&self, user_id: i64, server_id: &str) -> Result<Option<ServerConfig>> {
        let db_server = sqlx::query_as!(
            DbServer,
            r#"
            SELECT 
                id,
                server_id,
                user_id,
                name,
                description,
                server_type,
                transport_type,
                command_encrypted,
                args_encrypted,
                url_encrypted,
                env_encrypted,
                requires_auth,
                allowed_users_encrypted,
                sandbox_config_encrypted,
                timeout_ms,
                auto_start,
                enabled,
                created_at as "created_at: DateTime<Utc>",
                updated_at as "updated_at: DateTime<Utc>",
                last_used as "last_used: _"
            FROM servers 
            WHERE user_id = ?1 AND server_id = ?2 AND enabled = true
            "#,
            user_id,
            server_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        match db_server {
            Some(server) => Ok(Some(self.decrypt_server(server).await?)),
            None => Ok(None),
        }
    }
    
    /// Update a server configuration
    pub async fn update_server(&self, user_id: i64, server_id: &str, server: &BackendServer) -> Result<()> {
        // Encrypt sensitive fields
        let command_encrypted = server.command.as_ref()
            .map(|cmd| encrypt_string(cmd, &self.encryption_key))
            .transpose()?;
            
        let args_encrypted = encrypt_string(
            &serde_json::to_string(&server.args)?, 
            &self.encryption_key
        )?;
        
        let env_encrypted = encrypt_string(
            &serde_json::to_string(&server.env)?, 
            &self.encryption_key
        )?;
        
        let allowed_users_encrypted = encrypt_string(
            &serde_json::to_string(&server.allowed_users)?, 
            &self.encryption_key
        )?;
        
        let sandbox_config_encrypted = encrypt_string(
            &serde_json::to_string(&server.sandbox)?, 
            &self.encryption_key
        )?;
        
        let timeout_ms = server.timeout_ms as i64;
        
        sqlx::query!(
            r#"
            UPDATE servers SET
                name = ?3,
                command_encrypted = ?4,
                args_encrypted = ?5,
                env_encrypted = ?6,
                requires_auth = ?7,
                allowed_users_encrypted = ?8,
                sandbox_config_encrypted = ?9,
                timeout_ms = ?10
            WHERE user_id = ?1 AND server_id = ?2
            "#,
            user_id,
            server_id,
            server.name,
            command_encrypted,
            args_encrypted,
            env_encrypted,
            server.requires_auth,
            allowed_users_encrypted,
            sandbox_config_encrypted,
            timeout_ms
        )
        .execute(&self.pool)
        .await
        .context("Failed to update server")?;
        
        Ok(())
    }
    
    /// Delete a server
    pub async fn delete_server(&self, user_id: i64, server_id: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE servers SET enabled = false WHERE user_id = ?1 AND server_id = ?2",
            user_id,
            server_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete server")?;
        
        Ok(())
    }
    
    /// Convert database server to BackendServer
    pub async fn to_backend_server(&self, db_server: DbServer) -> Result<BackendServer> {
        let decrypted = self.decrypt_server(db_server).await?;
        
        Ok(BackendServer {
            id: decrypted.server_id,
            name: decrypted.name,
            r#type: crate::simple_router::ServerType::Local,
            command: decrypted.command,
            args: decrypted.args,
            url: decrypted.url,
            transport: crate::simple_router::TransportType::Sse, // Default for local servers (not used)
            auth: None,
            env: decrypted.env,
            requires_auth: decrypted.requires_auth,
            allowed_users: decrypted.allowed_users,
            sandbox: decrypted.sandbox_config,
            timeout_ms: decrypted.timeout_ms as u64,
            retry: None,
            tls: None,
        })
    }
    
    /// Decrypt a database server to ServerConfig
    async fn decrypt_server(&self, db_server: DbServer) -> Result<ServerConfig> {
        // Decrypt command
        let command = match db_server.command_encrypted {
            Some(encrypted) => Some(decrypt_string(&encrypted, &self.encryption_key)?),
            None => None,
        };
        
        // Decrypt args
        let args: Vec<String> = match db_server.args_encrypted {
            Some(encrypted) => {
                let decrypted = decrypt_string(&encrypted, &self.encryption_key)?;
                serde_json::from_str(&decrypted)?
            },
            None => Vec::new(),
        };
        
        // Decrypt URL
        let url = match db_server.url_encrypted {
            Some(encrypted) => Some(decrypt_string(&encrypted, &self.encryption_key)?),
            None => None,
        };
        
        // Decrypt environment
        let env: HashMap<String, String> = match db_server.env_encrypted {
            Some(encrypted) => {
                let decrypted = decrypt_string(&encrypted, &self.encryption_key)?;
                serde_json::from_str(&decrypted)?
            },
            None => HashMap::new(),
        };
        
        // Decrypt allowed users
        let allowed_users: Vec<String> = match db_server.allowed_users_encrypted {
            Some(encrypted) => {
                let decrypted = decrypt_string(&encrypted, &self.encryption_key)?;
                serde_json::from_str(&decrypted)?
            },
            None => Vec::new(),
        };
        
        // Decrypt sandbox config
        let sandbox_config: SandboxConfig = match db_server.sandbox_config_encrypted {
            Some(encrypted) => {
                let decrypted = decrypt_string(&encrypted, &self.encryption_key)?;
                serde_json::from_str(&decrypted)?
            },
            None => SandboxConfig::default(),
        };
        
        Ok(ServerConfig {
            id: db_server.id.unwrap_or(0),  // Handle nullable id
            server_id: db_server.server_id,
            name: db_server.name,
            description: db_server.description,
            server_type: db_server.server_type,
            transport_type: db_server.transport_type,
            command,
            args,
            url,
            env,
            requires_auth: db_server.requires_auth,
            allowed_users,
            sandbox_config,
            timeout_ms: db_server.timeout_ms.unwrap_or(30000),  // Handle nullable timeout
            auto_start: db_server.auto_start,
            enabled: db_server.enabled,
            created_at: db_server.created_at,
            updated_at: db_server.updated_at,
        })
    }
    
    /// Get all enabled servers across all users (for admin)
    pub async fn get_all_servers(&self) -> Result<Vec<BackendServer>> {
        let db_servers = sqlx::query_as!(
            DbServer,
            r#"
            SELECT 
                id,
                server_id,
                user_id,
                name,
                description,
                server_type,
                transport_type,
                command_encrypted,
                args_encrypted,
                url_encrypted,
                env_encrypted,
                requires_auth,
                allowed_users_encrypted,
                sandbox_config_encrypted,
                timeout_ms,
                auto_start,
                enabled,
                created_at as "created_at: DateTime<Utc>",
                updated_at as "updated_at: DateTime<Utc>",
                last_used as "last_used: _"
            FROM servers 
            WHERE enabled = true 
            ORDER BY user_id, name
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut servers = Vec::new();
        for db_server in db_servers {
            servers.push(self.to_backend_server(db_server).await?);
        }
        
        Ok(servers)
    }
}