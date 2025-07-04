# JauAuth Architecture

## Overview

JauAuth is a universal authentication system for MCP (Model Context Protocol) servers, built in Rust for security and performance.

## Core Components

### 1. **Authentication Service** (`auth.rs`)
- User registration with combined username+email+PIN hashing
- Argon2id password hashing (winner of Password Hashing Competition)
- Login authentication with device tracking
- Session creation and management

### 2. **Database Layer** (`database.rs`)
- SQLite with SQLx for type-safe queries
- Tables: users, devices, sessions, credentials, auth_log, permissions
- Automatic migrations on startup
- Prepared statements to prevent SQL injection

### 3. **Session Management** (`session.rs`)
- JWT tokens with RS256 signing
- 30-minute default expiration with 5-minute grace period
- Token validation and extension
- Session cleanup and revocation

### 4. **Device Fingerprinting** (`device.rs`)
- Comprehensive device identification
- Anomaly detection (Minor/Major/Critical levels)
- IP subnet tracking (/24 for IPv4, /48 for IPv6)
- Browser and OS detection

### 5. **Web Portal** (`web.rs`)
- Axum-based web server
- RESTful API endpoints
- CORS support
- Static file serving for web UI

### 6. **WebAuthn Integration** (`webauthn.rs`)
- Biometric authentication support
- FIDO2 security key support
- Platform authenticators (Touch ID, Face ID, Windows Hello)

### 7. **MCP Middleware** (`middleware.rs`)
- Transparent authentication layer for MCP servers
- Command filtering based on authentication status
- Permission-based access control

## Security Features

1. **Defense in Depth**
   - Multiple authentication factors
   - Device trust levels
   - Anomaly detection
   - Rate limiting

2. **Cryptography**
   - Argon2id for password hashing
   - SHA-256 for device fingerprints
   - JWT with RS256 for tokens
   - WebAuthn for biometrics

3. **Network Security**
   - HTTPS only (with self-signed cert for localhost)
   - CORS protection
   - IP-based rate limiting
   - Subnet tracking

4. **Data Protection**
   - SQL injection prevention via prepared statements
   - XSS protection via CSP headers
   - Secure session cookies
   - Audit logging

## Integration Flow

### For MCP Server Developers

```rust
// 1. Add dependency
[dependencies]
jau-auth = { path = "../JauAuth" }

// 2. Wrap your server
let protected = jau_auth::quick_protect(my_server, "My App").await?;

// 3. Run protected server
protected.run().await
```

### For End Users

1. **First Time**:
   - Send any command to MCP server
   - Receive registration link
   - Complete web registration
   - Set up biometrics (optional)

2. **Daily Use**:
   - Commands work transparently
   - PIN re-entry every 30 minutes
   - Full re-auth on device changes

## Data Flow

```
User Command → MCP Client → JauAuth Middleware → Check Auth
                                ↓
                         [Not Authenticated]
                                ↓
                         Return Auth URL
                                ↓
                         User Opens Portal
                                ↓
                         Login/Register
                                ↓
                         Create Session
                                ↓
                         [Authenticated]
                                ↓
                         Forward to MCP Server
```

## Future Enhancements

1. **OAuth2/OIDC Support**: Integration with external identity providers
2. **Multi-factor Authentication**: TOTP/SMS codes
3. **Admin Dashboard**: User management interface
4. **Clustering**: Redis-based session storage for scaling
5. **SDK**: Client libraries for various languages
6. **Audit Compliance**: SOC2/HIPAA compliant logging