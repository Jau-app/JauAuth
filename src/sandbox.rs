//! Sandbox execution for MCP servers - multiple strategies for different platforms

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command;
use tracing::info;

/// Sandbox strategy to use for isolating MCP servers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxStrategy {
    /// No sandboxing (not recommended)
    None,
    
    /// Use Docker containers (most secure, requires Docker)
    Docker {
        /// Base image to use (e.g., "node:18-alpine", "python:3.11-slim")
        image: Option<String>,
        /// Memory limit (e.g., "512m")
        memory_limit: Option<String>,
        /// CPU limit (e.g., "0.5")
        cpu_limit: Option<String>,
        /// Additional docker run flags
        extra_flags: Vec<String>,
        /// Whether to allow network access
        network: bool,
        /// Volume mounts
        mounts: Vec<String>,
    },
    
    /// Use Podman containers (rootless alternative to Docker)
    Podman {
        image: Option<String>,
        memory_limit: Option<String>,
        cpu_limit: Option<String>,
    },
    
    /// Use Firejail (Linux only, lightweight)
    Firejail {
        /// Profile to use (e.g., "default", "nodejs", "python3")
        profile: Option<String>,
        /// Whitelist paths that can be accessed
        whitelist_paths: Vec<String>,
        /// Read-only paths
        read_only_paths: Vec<String>,
        /// Network access (default: false)
        net: bool,
        /// Disable root user
        no_root: bool,
        /// Network filter file path
        netfilter: Option<String>,
    },
    
    /// Use bubblewrap (Linux only, used by Flatpak)
    Bubblewrap {
        /// Read-only bind mounts
        ro_binds: Vec<(String, String)>,
        /// Read-write bind mounts
        rw_binds: Vec<(String, String)>,
        /// Share network namespace
        share_net: bool,
    },
    
    /// Windows Sandbox (Windows 10/11 Pro only)
    #[cfg(target_os = "windows")]
    WindowsSandbox {
        /// Memory limit in MB
        memory_mb: Option<u32>,
        /// Allowed folders
        mapped_folders: Vec<String>,
    },
    
    /// macOS App Sandbox (requires code signing)
    #[cfg(target_os = "macos")]
    MacOSSandbox {
        /// Entitlements to grant
        entitlements: Vec<String>,
    },
}

impl Default for SandboxStrategy {
    fn default() -> Self {
        // Default to Firejail on Linux, Docker elsewhere if available
        #[cfg(target_os = "linux")]
        {
            SandboxStrategy::Firejail {
                profile: Some("default".to_string()),
                whitelist_paths: vec![],
                read_only_paths: vec![],
                net: true, // MCP servers often need network
                no_root: true,
                netfilter: None,
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // On non-Linux, try Docker as default since it's cross-platform
            // User can still explicitly choose None if needed
            SandboxStrategy::Docker {
                image: Some("node:18-alpine".to_string()),
                memory_limit: Some("512m".to_string()),
                cpu_limit: Some("0.5".to_string()),
                extra_flags: vec![],
                network: true,
                mounts: vec![],
            }
        }
    }
}

/// Sandbox configuration for a backend server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Sandbox strategy to use
    pub strategy: SandboxStrategy,
    
    /// Working directory inside sandbox
    pub work_dir: Option<String>,
    
    /// Environment variables to pass through
    pub env_passthrough: Vec<String>,
    
    /// Temporary directory for this sandbox
    pub temp_dir: Option<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            strategy: SandboxStrategy::default(),
            work_dir: Some("/tmp/mcp-work".to_string()),
            env_passthrough: vec!["PATH".to_string(), "HOME".to_string()],
            temp_dir: None,
        }
    }
}

impl SandboxConfig {
    /// Filter environment variables to only include allowed ones
    pub fn filter_env(&self, env: &HashMap<String, String>) -> HashMap<String, String> {
        let mut filtered = HashMap::new();
        for key in &self.env_passthrough {
            if let Some(value) = env.get(key) {
                filtered.insert(key.clone(), value.clone());
            }
        }
        filtered
    }
}

/// Sandbox wrapper for easy usage
pub struct Sandbox {
    config: SandboxConfig,
}

impl Sandbox {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }
    
    pub fn strategy(&self) -> &SandboxStrategy {
        &self.config.strategy
    }
}

/// Check which sandbox strategies are available on this system
pub async fn check_sandbox_availability() -> Vec<SandboxStrategy> {
    let mut available = vec![SandboxStrategy::None];
    
    // Check for Docker
    if Command::new("docker").arg("--version").output().await.is_ok() {
        available.push(SandboxStrategy::Docker {
            image: Some("alpine:latest".to_string()),
            memory_limit: None,
            cpu_limit: None,
            extra_flags: vec![],
            network: true,
            mounts: vec![],
        });
    }
    
    // Check for Podman
    if Command::new("podman").arg("--version").output().await.is_ok() {
        available.push(SandboxStrategy::Podman {
            image: Some("alpine:latest".to_string()),
            memory_limit: None,
            cpu_limit: None,
        });
    }
    
    // Check for Firejail (Linux only)
    #[cfg(target_os = "linux")]
    if Command::new("firejail").arg("--version").output().await.is_ok() {
        available.push(SandboxStrategy::Firejail {
            profile: None,
            whitelist_paths: vec![],
            read_only_paths: vec![],
            net: true,
            no_root: true,
            netfilter: None,
        });
    }
    
    // Check for Bubblewrap (Linux only)
    #[cfg(target_os = "linux")]
    if Command::new("bwrap").arg("--version").output().await.is_ok() {
        available.push(SandboxStrategy::Bubblewrap {
            ro_binds: vec![],
            rw_binds: vec![],
            share_net: true,
        });
    }
    
    info!("Available sandbox strategies: {:?}", available);
    available
}

/// Expand environment variables in a string value
fn expand_env_var(value: &str) -> String {
    if value.starts_with('$') {
        std::env::var(&value[1..]).unwrap_or_else(|_| value.to_string())
    } else {
        value.to_string()
    }
}

/// Expand environment variables in command arguments
/// Supports $VAR_NAME syntax and looks up in the provided env map first
fn expand_arg_with_env(arg: &str, env_map: &HashMap<String, String>) -> String {
    // Check if the arg contains $VARIABLE syntax
    if arg.contains('$') {
        let mut result = arg.to_string();
        
        // Simple pattern matching for $VARIABLE (not using regex to avoid dependency)
        let mut pos = 0;
        while let Some(dollar_pos) = result[pos..].find('$') {
            let start = pos + dollar_pos;
            let var_start = start + 1;
            
            // Find the end of the variable name
            let var_end = result[var_start..]
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .map(|i| var_start + i)
                .unwrap_or(result.len());
            
            if var_end > var_start {
                let var_name = &result[var_start..var_end];
                
                // First check the provided env map
                if let Some(value) = env_map.get(var_name) {
                    result.replace_range(start..var_end, value);
                    pos = start + value.len();
                } else if let Ok(value) = std::env::var(var_name) {
                    // Fall back to system environment
                    result.replace_range(start..var_end, &value);
                    pos = start + value.len();
                } else {
                    // Variable not found, skip past it
                    pos = var_end;
                }
            } else {
                pos = start + 1;
            }
        }
        result
    } else {
        arg.to_string()
    }
}

/// Build the sandbox command based on the strategy
pub fn build_sandbox_command(
    sandbox: &SandboxConfig,
    server_cmd: &str,
    server_args: &[String],
    server_env: &HashMap<String, String>,
) -> Result<Command> {
    match &sandbox.strategy {
        SandboxStrategy::None => {
            // No sandboxing - direct execution
            let mut cmd = Command::new(server_cmd);
            
            // Substitute environment variables in args
            let expanded_args: Vec<String> = server_args.iter()
                .map(|arg| expand_arg_with_env(arg, server_env))
                .collect();
            cmd.args(&expanded_args);
            
            // Set working directory if specified
            if let Some(work_dir) = &sandbox.work_dir {
                cmd.current_dir(work_dir);
            }
            
            for (key, value) in server_env {
                cmd.env(key, expand_env_var(value));
            }
            Ok(cmd)
        }
        
        SandboxStrategy::Docker { image, memory_limit, cpu_limit, extra_flags, network, mounts } => {
            let mut cmd = Command::new("docker");
            cmd.args(&["run", "--rm", "-i"]); // Remove container after exit, interactive
            
            // Security flags
            cmd.args(&[
                "--security-opt", "no-new-privileges",
                "--cap-drop", "ALL",
                "--read-only",
            ]);
            
            // Resource limits
            if let Some(mem) = memory_limit {
                cmd.args(&["--memory", mem]);
            }
            if let Some(cpu) = cpu_limit {
                cmd.args(&["--cpus", cpu]);
            }
            
            // Network configuration
            if !network {
                cmd.arg("--network=none");
            }
            
            // Volume mounts
            for mount in mounts {
                cmd.args(&["-v", mount]);
            }
            
            // Work directory
            if let Some(work_dir) = &sandbox.work_dir {
                cmd.args(&["--workdir", work_dir]);
            }
            
            // Environment variables
            for (key, value) in server_env {
                cmd.args(&["-e", &format!("{}={}", key, expand_env_var(value))]);
            }
            
            // Additional flags
            for flag in extra_flags {
                cmd.arg(flag);
            }
            
            // Image and command
            let image_name = image.as_ref().map(|s| s.as_str()).unwrap_or("node:18-alpine");
            cmd.arg(image_name);
            cmd.arg(server_cmd);
            
            // Substitute environment variables in args
            let expanded_args: Vec<String> = server_args.iter()
                .map(|arg| expand_arg_with_env(arg, server_env))
                .collect();
            cmd.args(&expanded_args);
            
            Ok(cmd)
        }
        
        SandboxStrategy::Firejail { profile, whitelist_paths, read_only_paths, net, no_root, netfilter } => {
            let mut cmd = Command::new("firejail");
            
            // Basic security flags
            cmd.args(&[
                "--noprofile", // Don't load default profile
                "--caps.drop=all", // Drop all capabilities
                "--nonewprivs", // No new privileges
                "--nosound", // No sound devices
                "--no3d", // No 3D acceleration
            ]);
            
            // No root user
            if *no_root {
                cmd.arg("--noroot");
            }
            
            // Network
            if !*net {
                cmd.arg("--net=none");
            }
            
            // Profile
            if let Some(prof) = profile {
                cmd.args(&["--profile", prof]);
            }
            
            // Whitelisted paths
            for path in whitelist_paths {
                cmd.args(&["--whitelist", path]);
            }
            
            // Read-only paths
            for path in read_only_paths {
                cmd.args(&["--read-only", path]);
            }
            
            // Private directories and additional security
            cmd.args(&["--private-tmp", "--private-dev", "--nodbus", "--machine-id", "--nogroups", "--disable-mnt"]);
            
            // Network filter if specified
            if let Some(nf) = netfilter {
                cmd.args(&["--netfilter", nf]);
            }
            
            // The actual command
            cmd.arg("--");
            cmd.arg(server_cmd);
            
            // Substitute environment variables in args
            let expanded_args: Vec<String> = server_args.iter()
                .map(|arg| expand_arg_with_env(arg, server_env))
                .collect();
            cmd.args(&expanded_args);
            
            // Environment
            for (key, value) in server_env {
                cmd.env(key, expand_env_var(value));
            }
            
            Ok(cmd)
        }
        
        SandboxStrategy::Bubblewrap { ro_binds, rw_binds, share_net } => {
            let mut cmd = Command::new("bwrap");
            
            // Basic isolation
            cmd.args(&[
                "--unshare-all", // Unshare all namespaces
                "--die-with-parent", // Exit if parent dies
                "--new-session", // New session
            ]);
            
            // Network
            if *share_net {
                cmd.arg("--share-net");
            }
            
            // Basic filesystem
            cmd.args(&[
                "--proc", "/proc",
                "--dev", "/dev",
                "--tmpfs", "/tmp",
            ]);
            
            // Bind mounts
            for (src, dst) in ro_binds {
                cmd.args(&["--ro-bind", src, dst]);
            }
            for (src, dst) in rw_binds {
                cmd.args(&["--bind", src, dst]);
            }
            
            // Command
            cmd.arg("--");
            cmd.arg(server_cmd);
            
            // Substitute environment variables in args
            let expanded_args: Vec<String> = server_args.iter()
                .map(|arg| expand_arg_with_env(arg, server_env))
                .collect();
            cmd.args(&expanded_args);
            
            // Environment variables
            for (key, value) in server_env {
                cmd.env(key, expand_env_var(value));
            }
            
            Ok(cmd)
        }
        
        _ => Err(anyhow!("Sandbox strategy not implemented for this platform")),
    }
}

/// Detect available sandbox strategies on the current system
pub async fn detect_available_strategies() -> Vec<SandboxStrategy> {
    let mut available = vec![SandboxStrategy::None];
    
    // Check for Docker
    if Command::new("docker").arg("--version").output().await.is_ok() {
        available.push(SandboxStrategy::Docker {
            image: Some("alpine:latest".to_string()),
            memory_limit: Some("512m".to_string()),
            cpu_limit: Some("0.5".to_string()),
            extra_flags: vec![],
            network: false,
            mounts: vec![],
        });
    }
    
    // Check for Podman
    if Command::new("podman").arg("--version").output().await.is_ok() {
        available.push(SandboxStrategy::Podman {
            image: Some("alpine:latest".to_string()),
            memory_limit: Some("512m".to_string()),
            cpu_limit: Some("0.5".to_string()),
        });
    }
    
    #[cfg(target_os = "linux")]
    {
        // Check for Firejail
        if Command::new("firejail").arg("--version").output().await.is_ok() {
            available.push(SandboxStrategy::Firejail {
                profile: None,
                whitelist_paths: vec![],
                read_only_paths: vec![],
                net: true,
                no_root: true,
                netfilter: None,
            });
        }
        
        // Check for bubblewrap
        if Command::new("bwrap").arg("--version").output().await.is_ok() {
            available.push(SandboxStrategy::Bubblewrap {
                ro_binds: vec![],
                rw_binds: vec![],
                share_net: true,
            });
        }
    }
    
    available
}

/// Validate that a sandbox strategy is available
pub async fn validate_sandbox_strategy(strategy: &SandboxStrategy) -> Result<()> {
    match strategy {
        SandboxStrategy::None => Ok(()),
        
        SandboxStrategy::Docker { .. } => {
            Command::new("docker")
                .arg("--version")
                .output()
                .await
                .map_err(|_| anyhow!("Docker not found. Please install Docker to use container sandboxing."))?;
            Ok(())
        }
        
        SandboxStrategy::Firejail { .. } => {
            Command::new("firejail")
                .arg("--version")
                .output()
                .await
                .map_err(|_| anyhow!("Firejail not found. Install with: sudo apt install firejail"))?;
            Ok(())
        }
        
        SandboxStrategy::Bubblewrap { .. } => {
            Command::new("bwrap")
                .arg("--version")
                .output()
                .await
                .map_err(|_| anyhow!("Bubblewrap not found. Install with: sudo apt install bubblewrap"))?;
            Ok(())
        }
        
        _ => Err(anyhow!("Sandbox strategy not supported on this platform")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_detect_strategies() {
        let strategies = detect_available_strategies().await;
        assert!(!strategies.is_empty());
        assert!(strategies.contains(&SandboxStrategy::None));
    }
    
    #[test]
    fn test_default_strategy() {
        let default = SandboxStrategy::default();
        match default {
            #[cfg(target_os = "linux")]
            SandboxStrategy::Firejail { .. } => {},
            #[cfg(not(target_os = "linux"))]
            SandboxStrategy::None => {},
            _ => panic!("Unexpected default strategy"),
        }
    }
}