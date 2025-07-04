//! Performance benchmarks for authentication operations

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jau_auth::{AuthService, AuthContext, AuthConfig};

fn setup_auth_context() -> AuthContext {
    let config = AuthConfig::builder()
        .app_name("bench-app")
        .database_url(":memory:")
        .build();
    
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(AuthContext::new(config))
        .unwrap()
}

fn bench_password_hashing(c: &mut Criterion) {
    use argon2::{Argon2, PasswordHasher};
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    
    c.bench_function("argon2_hash", |b| {
        let argon2 = Argon2::default();
        let password = b"testpassword123";
        
        b.iter(|| {
            let salt = SaltString::generate(&mut OsRng);
            argon2.hash_password(black_box(password), &salt).unwrap()
        });
    });
}

fn bench_jwt_generation(c: &mut Criterion) {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use jau_auth::session::Claims;
    use chrono::Utc;
    
    let encoding_key = EncodingKey::from_secret(b"test_secret_key_32_chars_long_ok");
    
    c.bench_function("jwt_encode", |b| {
        b.iter(|| {
            let claims = Claims {
                sub: "1".to_string(),
                device: "1".to_string(),
                session: "test-session".to_string(),
                exp: Utc::now().timestamp() + 3600,
                iat: Utc::now().timestamp(),
            };
            
            encode(&Header::default(), &claims, &encoding_key).unwrap()
        });
    });
}

fn bench_jwt_validation(c: &mut Criterion) {
    use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
    use jau_auth::session::Claims;
    use chrono::Utc;
    
    let encoding_key = EncodingKey::from_secret(b"test_secret_key_32_chars_long_ok");
    let decoding_key = DecodingKey::from_secret(b"test_secret_key_32_chars_long_ok");
    
    let claims = Claims {
        sub: "1".to_string(),
        device: "1".to_string(),
        session: "test-session".to_string(),
        exp: Utc::now().timestamp() + 3600,
        iat: Utc::now().timestamp(),
    };
    
    let token = encode(&Header::default(), &claims, &encoding_key).unwrap();
    
    c.bench_function("jwt_decode", |b| {
        b.iter(|| {
            decode::<Claims>(
                black_box(&token),
                &decoding_key,
                &Validation::default()
            ).unwrap()
        });
    });
}

fn bench_session_creation(c: &mut Criterion) {
    use jau_auth::SessionManager;
    use std::time::Duration;
    
    std::env::set_var("JWT_SECRET", "test_secret_key_for_benchmarking_32c");
    
    c.bench_function("session_create", |b| {
        let mut session_manager = SessionManager::new(Duration::from_secs(3600));
        
        b.iter(|| {
            session_manager.create_session(
                black_box(1),
                black_box(1)
            ).unwrap()
        });
    });
}

fn bench_rate_limiter(c: &mut Criterion) {
    use jau_auth::rate_limit::{RateLimiter, RateLimiterConfig};
    use std::time::Duration;
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("rate_limit_check", |b| {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_requests: 1000,
            window_duration: Duration::from_secs(60),
            use_ip_limiting: true,
        });
        
        b.iter(|| {
            rt.block_on(async {
                let _ = limiter.check_rate_limit(
                    black_box("127.0.0.1".to_string())
                ).await;
            });
        });
    });
}

criterion_group!(
    benches,
    bench_password_hashing,
    bench_jwt_generation,
    bench_jwt_validation,
    bench_session_creation,
    bench_rate_limiter
);
criterion_main!(benches);