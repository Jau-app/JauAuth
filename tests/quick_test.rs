//! Quick test to verify basic functionality

#[test]
fn test_basic() {
    assert_eq!(1 + 1, 2);
}

#[test] 
fn test_config() {
    use jau_auth::config::AuthConfig;
    
    let config = AuthConfig::builder()
        .app_name("test")
        .build();
    
    assert_eq!(config.app_name, "test");
}

#[test]
fn test_command_validation() {
    use jau_auth::backend_manager::{is_command_allowed, validate_shell_safety};
    
    assert!(is_command_allowed("node"));
    assert!(!is_command_allowed("rm"));
    
    assert!(validate_shell_safety("hello"));
    assert!(!validate_shell_safety("test; rm -rf /"));
}