//! Check available sandbox strategies on the system

use jau_auth::sandbox::{detect_available_strategies, SandboxStrategy};
use colored::*;

#[tokio::main]
async fn main() {
    println!("{}", "JauAuth Sandbox Checker".bold().blue());
    println!("{}", "======================".blue());
    println!();
    
    println!("Detecting available sandbox strategies...\n");
    
    let strategies = detect_available_strategies().await;
    
    if strategies.is_empty() {
        println!("{}", "❌ No sandbox strategies detected!".red());
        return;
    }
    
    println!("{}", format!("✅ Found {} available strategies:", strategies.len()).green());
    println!();
    
    for strategy in strategies {
        match strategy {
            SandboxStrategy::None => {
                println!("  {} None (No sandboxing)", "⚠️ ".yellow());
                println!("     Security: {}", "LOW - Processes run with full access".red());
                println!("     Use when: Testing only or trusted internal tools");
            }
            
            SandboxStrategy::Docker { .. } => {
                println!("  {} Docker", "🐳".blue());
                println!("     Security: {}", "HIGH - Full container isolation".green());
                println!("     Use when: Maximum security needed, untrusted code");
                println!("     Setup: Requires Docker daemon running");
            }
            
            SandboxStrategy::Podman { .. } => {
                println!("  {} Podman", "🦭".cyan());
                println!("     Security: {}", "HIGH - Rootless container isolation".green());
                println!("     Use when: Like Docker but without root daemon");
                println!("     Setup: More secure than Docker, slightly slower");
            }
            
            SandboxStrategy::Firejail { .. } => {
                println!("  {} Firejail", "🔥".red());
                println!("     Security: {}", "MEDIUM - Process isolation".yellow());
                println!("     Use when: Quick isolation, Linux only");
                println!("     Setup: sudo apt install firejail");
            }
            
            SandboxStrategy::Bubblewrap { .. } => {
                println!("  {} Bubblewrap", "🫧".magenta());
                println!("     Security: {}", "MEDIUM-HIGH - Namespace isolation".green());
                println!("     Use when: Flatpak-style isolation needed");
                println!("     Setup: sudo apt install bubblewrap");
            }
            
            _ => {
                println!("  {} Platform-specific sandbox", "🔒");
                println!("     Security: Varies by platform");
                println!("     Use when: Platform-specific security is needed");
            }
        }
        println!();
    }
    
    println!("{}", "Recommendations:".bold());
    println!("• For development: {} or {}", "Firejail".yellow(), "None".red());
    println!("• For production: {} or {}", "Docker".green(), "Podman".green());
    println!("• For untrusted code: {} with network disabled", "Docker".green());
    println!();
    
    println!("{}", "Example configurations available in:".dimmed());
    println!("  router-config-sandbox.example.json");
}