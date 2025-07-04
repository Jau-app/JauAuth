//! Unit tests for sandboxing functionality

use jau_auth::sandbox::{SandboxConfig, SandboxStrategy, Sandbox};
use std::collections::HashMap;

#[test]
fn test_sandbox_strategy_selection() {
    // Test None strategy
    let config = SandboxConfig {
        strategy: SandboxStrategy::None,
        env_passthrough: vec![],
    };
    let sandbox = Sandbox::new(config);
    assert!(matches!(sandbox.strategy(), &SandboxStrategy::None));
    
    // Test Docker strategy
    let config = SandboxConfig {
        strategy: SandboxStrategy::Docker {
            image: Some("alpine:latest".to_string()),
            network: false,
            mounts: vec![],
            memory_limit: None,
            cpu_limit: None,
        },
        env_passthrough: vec![],
    };
    let sandbox = Sandbox::new(config);
    assert!(matches!(sandbox.strategy(), SandboxStrategy::Docker { .. }));
}

#[test]
fn test_sandbox_command_validation() {
    use jau_auth::backend_manager::ALLOWED_COMMANDS;
    
    // Test allowed commands
    for cmd in ALLOWED_COMMANDS.iter() {
        assert!(jau_auth::backend_manager::is_command_allowed(cmd));
    }
    
    // Test disallowed commands
    assert!(!jau_auth::backend_manager::is_command_allowed("rm"));
    assert!(!jau_auth::backend_manager::is_command_allowed("chmod"));
    assert!(!jau_auth::backend_manager::is_command_allowed("/bin/sh"));
}

#[test]
fn test_shell_metacharacter_validation() {
    // Test safe arguments
    assert!(jau_auth::backend_manager::validate_shell_safety("hello"));
    assert!(jau_auth::backend_manager::validate_shell_safety("test-file.js"));
    assert!(jau_auth::backend_manager::validate_shell_safety("/path/to/file"));
    
    // Test unsafe arguments
    assert!(!jau_auth::backend_manager::validate_shell_safety("test; rm -rf /"));
    assert!(!jau_auth::backend_manager::validate_shell_safety("test && echo pwned"));
    assert!(!jau_auth::backend_manager::validate_shell_safety("test | cat /etc/passwd"));
    assert!(!jau_auth::backend_manager::validate_shell_safety("$(whoami)"));
    assert!(!jau_auth::backend_manager::validate_shell_safety("`id`"));
}

#[test]
fn test_environment_variable_filtering() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin".to_string());
    env.insert("SECRET_KEY".to_string(), "secret123".to_string());
    env.insert("HOME".to_string(), "/home/user".to_string());
    env.insert("USER".to_string(), "testuser".to_string());
    
    let config = SandboxConfig {
        strategy: SandboxStrategy::None,
        env_passthrough: vec!["PATH".to_string(), "USER".to_string()],
    };
    
    let filtered = config.filter_env(&env);
    
    // Should only contain allowed vars
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered.get("PATH"), Some(&"/usr/bin".to_string()));
    assert_eq!(filtered.get("USER"), Some(&"testuser".to_string()));
    assert!(!filtered.contains_key("SECRET_KEY"));
    assert!(!filtered.contains_key("HOME"));
}

#[test]
fn test_firejail_profile_generation() {
    let config = SandboxConfig {
        strategy: SandboxStrategy::Firejail {
            profile: Some("default".to_string()),
            net: false,
            whitelist_paths: vec!["/tmp/test".to_string()],
            read_only_paths: vec!["/etc".to_string()],
            no_root: true,
        },
        env_passthrough: vec!["PATH".to_string()],
    };
    
    if let SandboxStrategy::Firejail { net, no_root, .. } = &config.strategy {
        assert!(!net);
        assert!(no_root);
    }
}

#[tokio::test]
async fn test_sandbox_availability_check() {
    use jau_auth::sandbox::check_sandbox_availability;
    
    let available = check_sandbox_availability().await;
    
    // At least one strategy should be available
    assert!(!available.is_empty());
    
    // None should always be available
    assert!(available.iter().any(|s| matches!(s, SandboxStrategy::None)));
}