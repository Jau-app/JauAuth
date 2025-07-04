//! Unit tests for session management

use jau_auth::SessionManager;
use std::time::Duration;

#[tokio::test]
async fn test_session_creation() {
    // Set JWT secret for testing
    std::env::set_var("JWT_SECRET", "test_secret_key_for_testing_only_32_chars_long");
    std::env::set_var("JAUAUTH_JWT_SECRET", "test_secret_key_for_testing_only_32_chars_long");
    
    let mut session_manager = SessionManager::new(Duration::from_secs(3600));
    
    let session = session_manager.create_session(1, 1).unwrap();
    
    assert_eq!(session.user_id, 1);
    assert_eq!(session.device_id, 1);
    assert!(!session.token.is_empty());
    assert!(session.is_valid());
}

#[tokio::test]
async fn test_session_validation() {
    std::env::set_var("JWT_SECRET", "test_secret_key_for_testing_only_32_chars_long");
    
    let mut session_manager = SessionManager::new(Duration::from_secs(3600));
    
    // Create session
    let session = session_manager.create_session(1, 1).unwrap();
    let token = session.token.clone();
    
    // Validate token
    let validated = session_manager.validate_token(&token).unwrap();
    
    assert_eq!(validated.user_id, 1);
    assert_eq!(validated.device_id, 1);
    assert!(validated.is_valid());
}

#[tokio::test]
async fn test_session_expiration() {
    std::env::set_var("JWT_SECRET", "test_secret_key_for_testing_only_32_chars_long");
    
    // Create session with very short duration
    let mut session_manager = SessionManager::new(Duration::from_millis(100));
    
    let session = session_manager.create_session(1, 1).unwrap();
    
    // Session should be valid initially
    assert!(session.is_valid());
    
    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Token validation should fail due to expiration
    let result = session_manager.validate_token(&session.token);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_invalid_token_validation() {
    std::env::set_var("JWT_SECRET", "test_secret_key_for_testing_only_32_chars_long");
    
    let mut session_manager = SessionManager::new(Duration::from_secs(3600));
    
    // Try to validate invalid token
    let result = session_manager.validate_token("invalid_token");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_session_invalidation() {
    std::env::set_var("JWT_SECRET", "test_secret_key_for_testing_only_32_chars_long");
    
    let mut session_manager = SessionManager::new(Duration::from_secs(3600));
    
    // Create session
    let session = session_manager.create_session(1, 1).unwrap();
    let session_id = session.id.clone();
    
    // Invalidate session
    session_manager.invalidate_session(&session_id);
    
    // Try to get invalidated session
    let result = session_manager.get_session(&session_id);
    assert!(result.is_none());
}

#[tokio::test]
async fn test_user_sessions_cleanup() {
    std::env::set_var("JWT_SECRET", "test_secret_key_for_testing_only_32_chars_long");
    
    let mut session_manager = SessionManager::new(Duration::from_secs(3600));
    
    // Create multiple sessions for same user
    session_manager.create_session(1, 1).unwrap();
    session_manager.create_session(1, 2).unwrap();
    session_manager.create_session(1, 3).unwrap();
    
    // Create session for different user
    session_manager.create_session(2, 1).unwrap();
    
    // Invalidate all sessions for user 1
    session_manager.invalidate_user_sessions(1);
    
    // User 1 sessions should be gone
    let user1_sessions = session_manager.get_user_sessions(1);
    assert!(user1_sessions.is_empty());
    
    // User 2 sessions should remain
    let user2_sessions = session_manager.get_user_sessions(2);
    assert_eq!(user2_sessions.len(), 1);
}