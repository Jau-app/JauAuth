//! Configuration for JauAuth

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Application name
    pub app_name: String,
    
    /// Commands allowed without authentication
    pub first_access_commands: Vec<String>,
    
    /// Required permissions for this MCP server
    pub required_permissions: Vec<String>,
    
    /// Permission groups
    pub permission_groups: Vec<PermissionGroup>,
    
    /// Database URL
    pub database_url: String,
    
    /// Web server host
    pub host: String,
    
    /// Web server port
    pub port: u16,
    
    /// JWT secret
    pub jwt_secret: String,
    
    /// Session duration
    pub session_duration: Duration,
    
    /// PIN retry grace period
    pub pin_grace_period: Duration,
    
    /// Max login attempts
    pub max_login_attempts: u32,
    
    /// Rate limit window
    pub rate_limit_window: Duration,
    
    /// WebAuthn configuration
    pub webauthn: WebAuthnConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGroup {
    pub name: String,
    pub description: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnConfig {
    pub rp_id: String,
    pub rp_name: String,
    pub rp_origin: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            app_name: "MCP Server".to_string(),
            first_access_commands: vec![
                "help".to_string(),
                "status".to_string(),
                "register".to_string(),
            ],
            required_permissions: vec!["basic".to_string()],
            permission_groups: vec![
                PermissionGroup {
                    name: "basic".to_string(),
                    description: "Basic access".to_string(),
                    commands: vec!["*".to_string()],
                }
            ],
            database_url: "sqlite://jauauth.db".to_string(),
            host: "127.0.0.1".to_string(),
            port: 7448,
            jwt_secret: generate_secret(),
            session_duration: Duration::from_secs(30 * 60), // 30 minutes
            pin_grace_period: Duration::from_secs(5 * 60),  // 5 minutes
            max_login_attempts: 5,
            rate_limit_window: Duration::from_secs(15 * 60), // 15 minutes
            webauthn: WebAuthnConfig {
                rp_id: "localhost".to_string(),
                rp_name: "JauAuth".to_string(),
                rp_origin: "https://localhost:7448".to_string(),
            },
        }
    }
}

fn generate_secret() -> String {
    use rand::Rng;
    use base64::{Engine as _, engine::general_purpose};
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    general_purpose::STANDARD.encode(bytes)
}

pub struct AuthConfigBuilder {
    config: AuthConfig,
}

impl AuthConfig {
    pub fn builder() -> AuthConfigBuilder {
        AuthConfigBuilder {
            config: AuthConfig::default(),
        }
    }
}

impl AuthConfigBuilder {
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.config.app_name = name.into();
        self
    }
    
    pub fn first_access_commands(mut self, commands: Vec<impl Into<String>>) -> Self {
        self.config.first_access_commands = commands.into_iter()
            .map(|s| s.into())
            .collect();
        self
    }
    
    pub fn required_permissions(mut self, perms: Vec<impl Into<String>>) -> Self {
        self.config.required_permissions = perms.into_iter()
            .map(|s| s.into())
            .collect();
        self
    }
    
    pub fn permission_groups(mut self, groups: Vec<PermissionGroup>) -> Self {
        self.config.permission_groups = groups;
        self
    }
    
    pub fn database_url(mut self, url: impl Into<String>) -> Self {
        self.config.database_url = url.into();
        self
    }
    
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.config.host = host.into();
        self
    }
    
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }
    
    pub fn jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.config.jwt_secret = secret.into();
        self
    }
    
    pub fn session_duration(mut self, duration: Duration) -> Self {
        self.config.session_duration = duration;
        self
    }
    
    pub fn build(self) -> AuthConfig {
        self.config
    }
}

impl AuthConfig {
    /// Load configuration from environment and files
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut config = config::Config::builder();
        
        // Start with default
        config = config.add_source(
            config::Config::try_from(&AuthConfig::default())?
        );
        
        // Layer on .env file
        if let Ok(_) = dotenvy::dotenv() {
            config = config.add_source(config::Environment::with_prefix("JAUAUTH"));
        }
        
        // Layer on config file if exists
        if std::path::Path::new("jauauth.toml").exists() {
            config = config.add_source(config::File::with_name("jauauth"));
        }
        
        config.build()?.try_deserialize()
    }
}