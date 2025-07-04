//! Rate limiting middleware for API endpoints

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
    http::StatusCode,
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use serde_json::json;

/// Rate limiter configuration
#[derive(Clone, Debug)]
pub struct RateLimiterConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window duration
    pub window_duration: Duration,
    /// Whether to use IP-based limiting
    pub use_ip_limiting: bool,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_duration: Duration::from_secs(60), // 100 requests per minute
            use_ip_limiting: true,
        }
    }
}

/// Rate limiter state
#[derive(Clone)]
pub struct RateLimiter {
    config: RateLimiterConfig,
    requests: Arc<RwLock<HashMap<String, RequestInfo>>>,
}

#[derive(Debug, Clone)]
struct RequestInfo {
    count: u32,
    window_start: Instant,
}

impl RateLimiter {
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            config,
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Clean up expired entries
    async fn cleanup(&self) {
        let mut requests = self.requests.write().await;
        let now = Instant::now();
        
        requests.retain(|_, info| {
            now.duration_since(info.window_start) < self.config.window_duration
        });
    }

    /// Check if request should be rate limited
    async fn check_rate_limit(&self, key: String) -> Result<(), RateLimitError> {
        // Periodically clean up old entries
        if rand::random::<f32>() < 0.01 { // 1% chance
            self.cleanup().await;
        }

        let mut requests = self.requests.write().await;
        let now = Instant::now();

        let info = requests.entry(key.clone()).or_insert(RequestInfo {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(info.window_start) >= self.config.window_duration {
            info.count = 0;
            info.window_start = now;
        }

        // Check rate limit
        if info.count >= self.config.max_requests {
            let retry_after = self.config.window_duration
                .saturating_sub(now.duration_since(info.window_start))
                .as_secs();
            
            return Err(RateLimitError {
                retry_after,
                limit: self.config.max_requests,
            });
        }

        // Increment counter
        info.count += 1;
        Ok(())
    }

    /// Extract client identifier from request
    fn get_client_key(request: &Request) -> Option<String> {
        // Try to get IP from X-Forwarded-For or X-Real-IP headers first
        if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
            if let Ok(value) = forwarded_for.to_str() {
                // Take the first IP in the chain
                if let Some(ip) = value.split(',').next() {
                    return Some(ip.trim().to_string());
                }
            }
        }

        if let Some(real_ip) = request.headers().get("x-real-ip") {
            if let Ok(value) = real_ip.to_str() {
                return Some(value.to_string());
            }
        }

        // Fall back to connection info
        request
            .extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|conn_info| conn_info.0.ip().to_string())
    }
}

#[derive(Debug)]
struct RateLimitError {
    retry_after: u64,
    limit: u32,
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    // Skip rate limiting for certain paths
    let path = request.uri().path();
    if path.starts_with("/assets/") || path == "/" || path.starts_with("/api/health") {
        return next.run(request).await;
    }

    // Get client identifier
    let client_key = if limiter.config.use_ip_limiting {
        RateLimiter::get_client_key(&request)
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        // Could use user ID from JWT token here if authenticated
        "global".to_string()
    };

    // Check rate limit
    if let Err(err) = limiter.check_rate_limit(client_key).await {
        let mut response = (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(json!({
                "error": "Too many requests",
                "retry_after": err.retry_after,
                "limit": err.limit,
            }))
        ).into_response();

        // Add standard rate limit headers
        let headers = response.headers_mut();
        headers.insert("X-RateLimit-Limit", err.limit.to_string().parse().unwrap());
        headers.insert("X-RateLimit-Remaining", "0".parse().unwrap());
        headers.insert("Retry-After", err.retry_after.to_string().parse().unwrap());

        return response;
    }

    next.run(request).await
}

/// Create a pre-configured rate limiter for different endpoint types
pub mod presets {
    use super::*;

    /// Strict rate limiting for auth endpoints (10 requests per minute)
    pub fn auth_endpoints() -> RateLimiter {
        RateLimiter::new(RateLimiterConfig {
            max_requests: 10,
            window_duration: Duration::from_secs(60),
            use_ip_limiting: true,
        })
    }

    /// Normal rate limiting for API endpoints (100 requests per minute)
    pub fn api_endpoints() -> RateLimiter {
        RateLimiter::new(RateLimiterConfig {
            max_requests: 100,
            window_duration: Duration::from_secs(60),
            use_ip_limiting: true,
        })
    }

    /// Relaxed rate limiting for dashboard endpoints (300 requests per minute)
    pub fn dashboard_endpoints() -> RateLimiter {
        RateLimiter::new(RateLimiterConfig {
            max_requests: 300,
            window_duration: Duration::from_secs(60),
            use_ip_limiting: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_requests: 3,
            window_duration: Duration::from_secs(1),
            use_ip_limiting: true,
        });

        let key = "test_client".to_string();

        // First 3 requests should succeed
        for _ in 0..3 {
            assert!(limiter.check_rate_limit(key.clone()).await.is_ok());
        }

        // 4th request should fail
        assert!(limiter.check_rate_limit(key.clone()).await.is_err());

        // Wait for window to expire
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Should succeed again
        assert!(limiter.check_rate_limit(key.clone()).await.is_ok());
    }
}