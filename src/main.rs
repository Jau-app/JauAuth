//! JauAuth - MCP Router and Authentication System

use jau_auth::{AuthConfig, AuthContext, simple_router::{SimpleRouter, load_config}};
use tracing::{info, error};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as MCP router (default)
    Router {
        /// Path to router configuration file
        #[arg(short, long, default_value = "router-config.json")]
        config: PathBuf,
        
        /// Enable authentication
        #[arg(short, long)]
        auth: bool,
    },
    
    /// Run web portal for user management
    Web {
        /// Port to listen on
        #[arg(short, long, default_value = "7447")]
        port: u16,
        
        /// Path to router configuration file (optional)
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    
    /// Run both router and web dashboard together
    Combined {
        /// Path to router configuration file
        #[arg(short, long, default_value = "router-config.json")]
        config: PathBuf,
        
        /// Port for web dashboard
        #[arg(short, long, default_value = "7447")]
        port: u16,
        
        /// Enable authentication
        #[arg(short, long)]
        auth: bool,
    },
    
    /// Generate example configuration
    Init {
        /// Output path for configuration
        #[arg(short, long, default_value = "router-config.json")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("jau_auth=debug,rust_mcp_sdk=info")
        .init();
    
    let cli = Cli::parse();
    
    match cli.command.unwrap_or(Commands::Router { 
        config: PathBuf::from("router-config.json"),
        auth: false 
    }) {
        Commands::Router { config, auth } => {
            info!("Starting JauAuth MCP Router...");
            run_router(config, auth).await?;
        }
        
        Commands::Web { port, config } => {
            info!("Starting JauAuth Web Portal on port {}...", port);
            run_web_portal(port, config).await?;
        }
        
        Commands::Combined { config, port, auth } => {
            info!("Starting JauAuth Combined Mode - Router + Web Dashboard");
            run_combined(config, port, auth).await?;
        }
        
        Commands::Init { output } => {
            info!("Generating example configuration...");
            generate_config(output).await?;
        }
    }
    
    Ok(())
}

async fn run_router(config_path: PathBuf, enable_auth: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Load router configuration
    let router_config = load_config(config_path.to_str().unwrap()).await
        .unwrap_or_else(|e| {
            error!("Failed to load router config: {}. Using default.", e);
            error!("Run 'jau-auth init' to generate an example configuration.");
            Default::default()
        });
    
    info!("Loaded {} backend servers", router_config.servers.len());
    
    // Create the simple router
    let router = SimpleRouter::new(router_config);
    
    // Note: Authentication will be added in a future version
    if enable_auth {
        info!("Note: Authentication support coming soon!");
    }
    
    // Run the router
    info!("Starting MCP router - connect your MCP client to this process");
    router.run().await?;
    
    Ok(())
}

async fn run_web_portal(port: u16, router_config_path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use jau_auth::dashboard::DashboardState;
    use jau_auth::backend_manager::BackendManager;
    
    // Load auth configuration
    let mut config = AuthConfig::from_env()
        .unwrap_or_else(|e| {
            error!("Failed to load config: {}", e);
            info!("Using default configuration");
            AuthConfig::default()
        });
    
    config.port = port;
    
    info!("Configuration loaded for: {}", config.app_name);
    info!("Server will run on {}:{}", config.host, config.port);
    
    // Initialize auth context
    let auth_context = AuthContext::new(config.clone()).await?;
    info!("Database initialized");
    
    // Load router configuration if provided
    let router_config = if let Some(ref path) = router_config_path {
        load_config(path.to_str().unwrap()).await
            .unwrap_or_else(|e| {
                error!("Failed to load router config: {}. Using default.", e);
                Default::default()
            })
    } else {
        info!("No router config specified. Dashboard will start with empty server list.");
        Default::default()
    };
    
    // Create backend manager
    let backend_manager = Arc::new(BackendManager::new());
    
    // Create dashboard state
    let dashboard_state = DashboardState {
        auth_context,
        router_config: Arc::new(RwLock::new(router_config)),
        backend_manager,
        config_path: router_config_path,
    };
    
    // Create web app with dashboard state
    let app = jau_auth::web::create_router(dashboard_state);
    
    // Start web server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    info!("Web portal with dashboard running at http://{}", addr);
    info!("Open your browser to access the MCP server management dashboard");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn run_combined(config_path: PathBuf, port: u16, enable_auth: bool) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use jau_auth::dashboard::DashboardState;
    use jau_auth::backend_manager::BackendManager;
    use jau_auth::mcp_api::{McpApiState, mcp_api_routes};
    
    info!("Starting combined mode with router and web dashboard...");
    
    // Load router configuration
    let router_config = load_config(config_path.to_str().unwrap()).await
        .unwrap_or_else(|e| {
            error!("Failed to load router config: {}. Using default.", e);
            error!("Run 'jau-auth init' to generate an example configuration.");
            Default::default()
        });
    
    info!("Loaded {} backend servers", router_config.servers.len());
    
    // Create shared backend manager
    let backend_manager = Arc::new(BackendManager::new());
    
    // Create shared router config
    let shared_router_config = Arc::new(RwLock::new(router_config.clone()));
    
    // Load auth configuration
    let mut auth_config = AuthConfig::from_env()
        .unwrap_or_else(|e| {
            error!("Failed to load config: {}", e);
            info!("Using default configuration");
            AuthConfig::default()
        });
    
    auth_config.port = port;
    
    // Initialize auth context
    let auth_context = AuthContext::new(auth_config.clone()).await?;
    info!("Database initialized");
    
    // Create dashboard state
    let dashboard_state = DashboardState {
        auth_context,
        router_config: shared_router_config.clone(),
        backend_manager: backend_manager.clone(),
        config_path: Some(config_path.clone()),
    };
    
    // Create MCP API state
    let mcp_api_state = McpApiState {
        router_config: shared_router_config.clone(),
        backend_manager: backend_manager.clone(),
    };
    
    // Create the simple router with shared backend manager
    let router = SimpleRouter::new_with_manager(router_config, backend_manager.clone());
    
    if enable_auth {
        info!("Note: Authentication support coming soon!");
    }
    
    // Create web app with MCP API routes
    let mut app = jau_auth::web::create_router(dashboard_state);
    
    // Add MCP API routes under /api/mcp
    app = app.nest("/api/mcp", mcp_api_routes().with_state(mcp_api_state));
    
    // Start web server in a separate task
    let addr = format!("{}:{}", auth_config.host, auth_config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    info!("Web dashboard running at http://{}", addr);
    info!("MCP API available at http://{}/api/mcp", addr);
    
    let web_handle = tokio::spawn(async move {
        axum::serve(listener, app).await
    });
    
    // Run the MCP router on the main thread (stdio)
    info!("MCP router ready - connect your MCP client to this process");
    
    // Run router in a select! so we can handle both services
    tokio::select! {
        router_result = router.run() => {
            if let Err(e) = router_result {
                error!("Router error: {}", e);
            }
        }
        web_result = web_handle => {
            if let Err(e) = web_result {
                error!("Web server error: {}", e);
            }
        }
    }
    
    Ok(())
}

async fn generate_config(output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Check if file already exists
    if output.exists() {
        error!("Configuration file already exists: {:?}", output);
        return Err("File already exists".into());
    }
    
    // Read the example configuration
    let example_config = include_str!("../router-config.example.json");
    
    // Write to output file
    tokio::fs::write(&output, example_config).await?;
    
    info!("Generated configuration file: {:?}", output);
    info!("Edit this file to add your MCP servers, then run:");
    info!("  jau-auth router --config {:?}", output);
    
    Ok(())
}