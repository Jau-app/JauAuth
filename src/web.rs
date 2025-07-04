//! Web portal and API routes

use axum::{
    Router, 
    routing::{get, post, put, delete},
    middleware,
};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use crate::dashboard::DashboardState;
use crate::security::{security_headers, security_middleware};
use crate::rate_limit::{rate_limit_middleware, presets};

pub fn create_router(dashboard_state: DashboardState) -> Router {
    // Configure CORS - restrict in production
    let cors = if cfg!(debug_assertions) {
        CorsLayer::permissive()
    } else {
        CorsLayer::new()
            .allow_origin(["http://localhost:7447".parse().unwrap()])
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                "X-CSRF-Token".parse().unwrap(),
            ])
            .allow_credentials(true)
    };
    
    Router::new()
        // Static files with security headers
        .nest_service("/assets", ServeDir::new("web/dist/assets")
            .precompressed_gzip()
            .precompressed_br())
        .route_service("/", ServeFile::new("web/dist/index.html"))
        
        // Auth API routes (public)
        .route("/api/register", post(register_handler))
        .route("/api/login", post(login_handler))
        .route("/api/verify-pin", post(verify_pin_handler))
        
        // Protected routes
        .route("/api/logout", post(logout_handler))
        .route("/api/devices", get(list_devices_handler))
        .route("/api/webauthn/register", post(webauthn_register_handler))
        .route("/api/webauthn/verify", post(webauthn_verify_handler))
        
        // Dashboard API routes (all protected)
        .route("/api/dashboard/overview", get(crate::dashboard::get_overview))
        .route("/api/dashboard/servers", get(crate::dashboard::list_servers))
        .route("/api/dashboard/servers", post(crate::dashboard::add_server))
        .route("/api/dashboard/servers/{id}", get(crate::dashboard::get_server))
        .route("/api/dashboard/servers/{id}", put(crate::dashboard::update_server))
        .route("/api/dashboard/servers/{id}", delete(crate::dashboard::remove_server))
        .route("/api/dashboard/servers/{id}/logs", get(crate::dashboard::get_server_logs))
        .route("/api/dashboard/tools", get(crate::dashboard::list_tools))
        .route("/api/dashboard/tools/test", post(crate::dashboard::test_tool))
        .route("/api/dashboard/auth/settings", get(crate::dashboard::get_auth_settings))
        .route("/api/dashboard/auth/settings", put(crate::dashboard::update_auth_settings))
        .route("/api/dashboard/mcp/config", get(crate::dashboard::get_mcp_config))
        .route("/api/dashboard/mcp/config", put(crate::dashboard::update_mcp_config))
        // Temporary stub routes for missing endpoints
        .route("/api/dashboard/user/profile", get(user_profile_stub))
        .route("/api/dashboard/sessions", get(sessions_stub))
        
        // Fallback for SPA client-side routing
        .fallback_service(ServeFile::new("web/dist/index.html"))
        
        // Security layers
        .layer(security_headers())
        .layer(middleware::from_fn(security_middleware))
        .layer(RequestBodyLimitLayer::new(5 * 1024 * 1024)) // 5MB limit
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            presets::api_endpoints(),
            rate_limit_middleware
        ))
        .with_state(dashboard_state)
}

async fn register_handler() -> &'static str {
    "TODO: Implement registration"
}

async fn login_handler() -> &'static str {
    "TODO: Implement login"
}

async fn verify_pin_handler() -> &'static str {
    "TODO: Implement PIN verification"
}

async fn logout_handler() -> &'static str {
    "TODO: Implement logout"
}

async fn list_devices_handler() -> &'static str {
    "TODO: List devices"
}

async fn webauthn_register_handler() -> &'static str {
    "TODO: WebAuthn registration"
}

async fn webauthn_verify_handler() -> &'static str {
    "TODO: WebAuthn verification"
}

// Temporary stub handlers
async fn user_profile_stub() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "username": "admin",
        "email": "admin@jauauth.local",
        "created_at": "2025-01-01T00:00:00Z"
    }))
}

async fn sessions_stub() -> axum::Json<Vec<serde_json::Value>> {
    axum::Json(vec![])
}