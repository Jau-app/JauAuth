//! Database models and operations

use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub telegram_id: String,
    pub username: String,
    pub email: String,
    pub auth_hash: String,
    pub created_at: NaiveDateTime,
    pub last_login: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Device {
    pub id: i64,
    pub user_id: i64,
    pub device_hash: String,
    pub device_name: Option<String>,
    pub trusted: bool,
    pub last_seen: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: i64,
    pub user_id: i64,
    pub device_id: i64,
    pub token: String,
    pub expires_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebAuthnCredential {
    pub id: i64,
    pub user_id: i64,
    pub credential_id: String,
    pub public_key: String,
    pub counter: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthEvent {
    pub user_id: Option<i64>,
    pub event_type: String,
    pub device_hash: Option<String>,
    pub ip_address: Option<String>,
    pub metadata: serde_json::Value,
}

/// Initialize the database and run migrations
pub async fn init_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    // Create the database file if it doesn't exist
    if database_url.starts_with("sqlite://") {
        let path = database_url.trim_start_matches("sqlite://");
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).ok();
        }
    }
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    Ok(pool)
}

/// Create a new user
pub async fn create_user(pool: &SqlitePool, user: User) -> Result<User, sqlx::Error> {
    let id = sqlx::query!(
        r#"
        INSERT INTO users (telegram_id, username, email, auth_hash, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        user.telegram_id,
        user.username,
        user.email,
        user.auth_hash,
        user.created_at
    )
    .execute(pool)
    .await?
    .last_insert_rowid();
    
    let mut created_user = user;
    created_user.id = id;
    
    Ok(created_user)
}

/// Get user by username
pub async fn get_user_by_username(
    pool: &SqlitePool,
    username: &str
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id as "id!", telegram_id, username, email, auth_hash,
               created_at, last_login
        FROM users
        WHERE username = ?1
        "#,
        username
    )
    .fetch_optional(pool)
    .await
}

/// Get user by ID
pub async fn get_user_by_id(
    pool: &SqlitePool,
    id: i64
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id as "id!", telegram_id, username, email, auth_hash,
               created_at, last_login
        FROM users
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
}

/// Update last login time
pub async fn update_last_login(pool: &SqlitePool, user_id: i64) -> Result<(), sqlx::Error> {
    let now = Utc::now().naive_utc();
    sqlx::query!(
        r#"
        UPDATE users
        SET last_login = ?1
        WHERE id = ?2
        "#,
        now,
        user_id
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Get or create device
pub async fn get_or_create_device(
    pool: &SqlitePool,
    user_id: i64,
    device_hash: &str,
) -> Result<Device, sqlx::Error> {
    // Try to find existing device
    if let Some(device) = sqlx::query_as!(
        Device,
        r#"
        SELECT id as "id!", user_id as "user_id!", device_hash, device_name, trusted as "trusted!",
               last_seen, created_at
        FROM devices
        WHERE user_id = ?1 AND device_hash = ?2
        "#,
        user_id,
        device_hash
    )
    .fetch_optional(pool)
    .await?
    {
        // Update last seen
        let now = Utc::now().naive_utc();
        sqlx::query!(
            r#"UPDATE devices SET last_seen = ?1 WHERE id = ?2"#,
            now,
            device.id
        )
        .execute(pool)
        .await?;
        
        return Ok(device);
    }
    
    // Create new device
    let now = Utc::now().naive_utc();
    let id = sqlx::query!(
        r#"
        INSERT INTO devices (user_id, device_hash, trusted, last_seen, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        user_id,
        device_hash,
        false, // New devices are not trusted by default
        now,
        now
    )
    .execute(pool)
    .await?
    .last_insert_rowid();
    
    Ok(Device {
        id,
        user_id,
        device_hash: device_hash.to_string(),
        device_name: None,
        trusted: false,
        last_seen: now,
        created_at: now,
    })
}

/// Log an authentication event
pub async fn log_auth_event(
    pool: &SqlitePool,
    event: AuthEvent,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().naive_utc();
    sqlx::query!(
        r#"
        INSERT INTO auth_log (user_id, event_type, device_hash, ip_address, metadata, timestamp)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        event.user_id,
        event.event_type,
        event.device_hash,
        event.ip_address,
        event.metadata,
        now
    )
    .execute(pool)
    .await?;
    
    Ok(())
}