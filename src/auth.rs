//! Core authentication logic

use crate::{AuthContext, database::User};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("User not found")]
    UserNotFound,
    
    #[error("Device not recognized")]
    UnknownDevice,
    
    #[error("Session expired")]
    SessionExpired,
    
    #[error("Too many login attempts")]
    TooManyAttempts,
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Hashing error: {0}")]
    Hashing(String),
    
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

pub struct AuthService {
    context: AuthContext,
    argon2: Argon2<'static>,
}

impl AuthService {
    pub fn new(context: AuthContext) -> Self {
        Self {
            context,
            argon2: Argon2::default(),
        }
    }
    
    /// Register a new user
    pub async fn register(
        &self,
        telegram_id: &str,
        username: &str,
        email: &str,
        pin: &str,
    ) -> Result<User, AuthError> {
        // Create combined auth string
        let auth_string = format!("{}{}{}", username, email, pin);
        
        // Generate salt and hash
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = self.argon2
            .hash_password(auth_string.as_bytes(), &salt)
            .map_err(|e| AuthError::Hashing(e.to_string()))?;
        
        // Store in database
        let user = User {
            id: 0, // Will be set by database
            telegram_id: telegram_id.to_string(),
            username: username.to_string(),
            email: email.to_string(),
            auth_hash: password_hash.to_string(),
            created_at: chrono::Utc::now().naive_utc(),
            last_login: None,
        };
        
        let user = crate::database::create_user(&self.context.db, user).await?;
        
        Ok(user)
    }
    
    /// Authenticate a user
    pub async fn authenticate(
        &self,
        username: &str,
        email: &str,
        pin: &str,
        device_hash: &str,
    ) -> Result<(User, String), AuthError> {
        // Find user by username
        let user = crate::database::get_user_by_username(&self.context.db, username)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        
        // Verify email matches
        if user.email != email {
            return Err(AuthError::InvalidCredentials);
        }
        
        // Create auth string and verify
        let auth_string = format!("{}{}{}", username, email, pin);
        let parsed_hash = PasswordHash::new(&user.auth_hash)
            .map_err(|e| AuthError::Hashing(e.to_string()))?;
        
        self.argon2
            .verify_password(auth_string.as_bytes(), &parsed_hash)
            .map_err(|_| AuthError::InvalidCredentials)?;
        
        // Check device
        let device = crate::database::get_or_create_device(
            &self.context.db,
            user.id,
            device_hash,
        ).await?;
        
        // Create session
        let session = self.context.session_manager
            .write()
            .await
            .create_session(user.id, device.id)?;
        
        // Update last login
        crate::database::update_last_login(&self.context.db, user.id).await?;
        
        Ok((user, session.token))
    }
    
    /// Verify PIN for quick re-authentication
    pub async fn verify_pin(
        &self,
        user_id: i64,
        _pin: &str,
    ) -> Result<bool, AuthError> {
        let _user = crate::database::get_user_by_id(&self.context.db, user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        
        // For PIN verification, we need to store the PIN hash separately
        // This is a simplified version - in production, store PIN hash separately
        // For now, we'll require full re-auth
        
        Err(AuthError::InvalidCredentials)
    }
    
    /// Revoke all sessions for a user
    pub async fn logout(&self, user_id: i64) -> Result<(), AuthError> {
        self.context.session_manager
            .write()
            .await
            .revoke_user_sessions(user_id);
        
        Ok(())
    }
}