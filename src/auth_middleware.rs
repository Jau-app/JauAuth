//! Authentication middleware for dashboard routes

use axum::{
    middleware::Next,
    response::Response,
    extract::{Request, State},
    http::StatusCode,
};
use axum_extra::headers::{Authorization, authorization::Bearer};
use axum_extra::TypedHeader;
use crate::dashboard::DashboardState;

/// Require valid JWT token for dashboard access
pub async fn require_auth(
    State(state): State<DashboardState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract token from Authorization header
    let token = auth_header
        .and_then(|auth| Some(auth.token().to_string()))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Verify token with session manager
    let mut session_manager = state.auth_context.session_manager.write().await;
    let session = session_manager
        .validate_token(&token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    // Check if session is valid
    if !session.is_valid() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Add session to request extensions for use in handlers
    drop(session_manager); // Release the lock before modifying request
    request.extensions_mut().insert(session);
    
    Ok(next.run(request).await)
}

/// Optional auth - adds session to request if valid token provided
pub async fn optional_auth(
    State(state): State<DashboardState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract and verify token
    if let Some(auth) = auth_header {
        if let Ok(session) = state.auth_context.session_manager
            .write()
            .await
            .validate_token(auth.token()) 
        {
            if session.is_valid() {
                // Add session to request extensions
                request.extensions_mut().insert(session);
            }
        }
    }
    
    next.run(request).await
}