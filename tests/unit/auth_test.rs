//! Unit tests for authentication module

use jau_auth::{AuthService, AuthError};

#[path = "../common/mod.rs"]
mod common;

#[tokio::test]
async fn test_user_registration() {
    let ctx = common::test_auth_context().await;
    let auth_service = AuthService::new(ctx);
    
    let result = auth_service.register(
        "telegram_123",
        "testuser",
        "test@example.com",
        "1234"
    ).await;
    
    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.username, "testuser");
    assert_eq!(user.email, "test@example.com");
}

#[tokio::test]
async fn test_duplicate_username_registration() {
    let ctx = common::test_auth_context().await;
    let auth_service = AuthService::new(ctx);
    
    // Register first user
    auth_service.register(
        "telegram_123",
        "testuser",
        "test1@example.com",
        "1234"
    ).await.unwrap();
    
    // Try to register with same username
    let result = auth_service.register(
        "telegram_456",
        "testuser",
        "test2@example.com",
        "5678"
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_authentication_success() {
    let ctx = common::test_auth_context().await;
    let auth_service = AuthService::new(ctx.clone());
    
    // Register user
    auth_service.register(
        "telegram_123",
        "testuser",
        "test@example.com",
        "1234"
    ).await.unwrap();
    
    // Authenticate
    let result = auth_service.authenticate(
        "testuser",
        "test@example.com",
        "1234",
        "device_hash_123"
    ).await;
    
    assert!(result.is_ok());
    let (user, token) = result.unwrap();
    assert_eq!(user.username, "testuser");
    assert!(!token.is_empty());
}

#[tokio::test]
async fn test_authentication_wrong_pin() {
    let ctx = common::test_auth_context().await;
    let auth_service = AuthService::new(ctx);
    
    // Register user
    auth_service.register(
        "telegram_123",
        "testuser",
        "test@example.com",
        "1234"
    ).await.unwrap();
    
    // Try to authenticate with wrong PIN
    let result = auth_service.authenticate(
        "testuser",
        "test@example.com",
        "5678", // Wrong PIN
        "device_hash_123"
    ).await;
    
    assert_error_matches!(result, AuthError::InvalidCredentials);
}

#[tokio::test]
async fn test_authentication_wrong_email() {
    let ctx = common::test_auth_context().await;
    let auth_service = AuthService::new(ctx);
    
    // Register user
    auth_service.register(
        "telegram_123",
        "testuser",
        "test@example.com",
        "1234"
    ).await.unwrap();
    
    // Try to authenticate with wrong email
    let result = auth_service.authenticate(
        "testuser",
        "wrong@example.com", // Wrong email
        "1234",
        "device_hash_123"
    ).await;
    
    assert_error_matches!(result, AuthError::InvalidCredentials);
}

#[tokio::test]
async fn test_authentication_nonexistent_user() {
    let ctx = common::test_auth_context().await;
    let auth_service = AuthService::new(ctx);
    
    let result = auth_service.authenticate(
        "nonexistent",
        "test@example.com",
        "1234",
        "device_hash_123"
    ).await;
    
    assert_error_matches!(result, AuthError::UserNotFound);
}