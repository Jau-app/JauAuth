//! Session management

use crate::auth::AuthError;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: i64,
    pub device_id: i64,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl Session {
    /// Check if session is still valid
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // User ID
    pub device: String,     // Device ID
    pub session: String,    // Session ID
    pub exp: i64,          // Expiration time
    pub iat: i64,          // Issued at
}

pub struct SessionManager {
    sessions: HashMap<String, Session>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    session_duration: Duration,
}

impl SessionManager {
    pub fn new(session_duration: std::time::Duration) -> Self {
        // JWT_SECRET is required for security
        let secret = std::env::var("JWT_SECRET")
            .expect("JWT_SECRET environment variable is required. Set it to a secure random string (at least 32 characters).");
        
        Self {
            sessions: HashMap::new(),
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            session_duration: Duration::from_std(session_duration).unwrap(),
        }
    }
    
    /// Create a new session
    pub fn create_session(
        &mut self,
        user_id: i64,
        device_id: i64,
    ) -> Result<Session, AuthError> {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + self.session_duration;
        
        // Create JWT claims
        let claims = Claims {
            sub: user_id.to_string(),
            device: device_id.to_string(),
            session: session_id.clone(),
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
        };
        
        // Generate token
        let token = encode(&Header::default(), &claims, &self.encoding_key)?;
        
        let session = Session {
            id: session_id.clone(),
            user_id,
            device_id,
            token: token.clone(),
            created_at: now,
            expires_at,
            last_activity: now,
        };
        
        self.sessions.insert(session_id, session.clone());
        
        Ok(session)
    }
    
    /// Validate a token and return the session
    pub fn validate_token(&mut self, token: &str) -> Result<Session, AuthError> {
        // Decode and validate JWT
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::default()
        )?;
        
        let claims = token_data.claims;
        
        // Check if session exists
        let session = self.sessions
            .get_mut(&claims.session)
            .ok_or(AuthError::SessionExpired)?;
        
        // Check expiration
        if Utc::now() > session.expires_at {
            self.sessions.remove(&claims.session);
            return Err(AuthError::SessionExpired);
        }
        
        // Update last activity
        session.last_activity = Utc::now();
        
        Ok(session.clone())
    }
    
    /// Extend a session if within grace period
    pub fn extend_session(&mut self, session_id: &str) -> Result<(), AuthError> {
        let session = self.sessions
            .get_mut(session_id)
            .ok_or(AuthError::SessionExpired)?;
        
        let now = Utc::now();
        
        // Only extend if within grace period (5 minutes before expiry)
        let grace_period = Duration::minutes(5);
        if now > session.expires_at - grace_period {
            session.expires_at = now + self.session_duration;
        }
        
        session.last_activity = now;
        
        Ok(())
    }
    
    /// Revoke a session
    pub fn revoke_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }
    
    /// Revoke all sessions for a user
    pub fn revoke_user_sessions(&mut self, user_id: i64) {
        self.sessions.retain(|_, session| session.user_id != user_id);
    }
    
    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }
    
    /// Invalidate a session (alias for revoke_session)
    pub fn invalidate_session(&mut self, session_id: &str) {
        self.revoke_session(session_id);
    }
    
    /// Invalidate all sessions for a user (alias for revoke_user_sessions)
    pub fn invalidate_user_sessions(&mut self, user_id: i64) {
        self.revoke_user_sessions(user_id);
    }
    
    /// Get all sessions for a user
    pub fn get_user_sessions(&self, user_id: i64) -> Vec<&Session> {
        self.sessions
            .values()
            .filter(|session| session.user_id == user_id)
            .collect()
    }
    
    /// Clean up expired sessions
    pub fn cleanup_expired(&mut self) {
        let now = Utc::now();
        self.sessions.retain(|_, session| session.expires_at > now);
    }
    
    /// Get active session count for a user
    pub fn user_session_count(&self, user_id: i64) -> usize {
        self.sessions
            .values()
            .filter(|s| s.user_id == user_id && s.expires_at > Utc::now())
            .count()
    }
}