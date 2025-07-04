//! Security middleware for the web dashboard

use axum::{
    middleware::Next,
    response::Response,
    extract::Request,
    http::{StatusCode, HeaderMap, HeaderValue},
};
use tower_http::set_header::SetResponseHeaderLayer;
use uuid::Uuid;

/// Security headers to prevent common attacks
pub fn security_headers() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(
        axum::http::header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline'; \
             style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
             img-src 'self' data:; \
             font-src 'self' https://fonts.gstatic.com; \
             connect-src 'self'; \
             frame-ancestors 'none'; \
             base-uri 'self'; \
             form-action 'self';"
        ),
    )
}

/// Additional security headers middleware
pub async fn security_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    
    // Prevent clickjacking
    headers.insert(
        "X-Frame-Options",
        HeaderValue::from_static("DENY")
    );
    
    // Prevent MIME type sniffing
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff")
    );
    
    // Enable XSS protection
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block")
    );
    
    // HSTS for HTTPS
    headers.insert(
        "Strict-Transport-Security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains")
    );
    
    // Referrer policy
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin")
    );
    
    // Permissions policy
    headers.insert(
        "Permissions-Policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()")
    );
    
    Ok(response)
}

/// Generate a CSRF token
pub fn generate_csrf_token() -> String {
    Uuid::new_v4().to_string()
}

/// Validate CSRF token from request
pub fn validate_csrf_token(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("X-CSRF-Token")
        .and_then(|v| v.to_str().ok())
        .map(|token| token == expected)
        .unwrap_or(false)
}