# Changelog

All notable changes to JauAuth will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Timeout Configuration Support** - Added `__timeout` parameter for long-running operations
  - Supports millisecond values, string numbers, and "*" for no timeout
  - Automatically enhances all tool descriptions with timeout information
  - Helpful error messages when operations timeout
  - Test utilities: `test-tool.js` and `test-timeout.js` for timeout testing
  - Comprehensive documentation in dashboard and docs/TIMEOUT-CONFIGURATION.md
- **Test Tool Utility** (`scripts/test-tool.js`) - Command-line tool for testing individual MCP tools
  - Supports custom timeout via `--timeout` flag or `__timeout` parameter
  - Shows elapsed time and helpful error messages
  - Pre-flight checks ensure backend is running
- **Timeout Test Suite** (`scripts/test-timeout.js`) - Automated tests for timeout functionality
  - Tests various timeout scenarios
  - Validates proper timeout behavior
  - Provides pass/fail summary

### Changed
- Enhanced TypeScript MCP server to handle `__timeout` parameter
- Updated tool aggregation to automatically document `__timeout` in all tools
- Improved error messages to suggest using `__timeout` for timeout errors

### Fixed
- Fixed blocking issue with long-running operations like `wait_for_approval`
- Resolved two-way communication issues with Telegram bot through JauAuth

### Documentation
- Added Timeout Configuration section to web dashboard docs
- Created docs/TIMEOUT-CONFIGURATION.md with detailed timeout usage guide
- Updated README.md with timeout testing examples
- Updated CLAUDE.md with timeout parameter documentation
- Enhanced docs/ROUTER-STATUS.md with timeout feature status

## [0.1.0] - 2025-07-04

### Added
- **TypeScript/Rust Hybrid Architecture** - TypeScript MCP server + Rust backend
- **MCP Router Core Functionality**
  - Single connection point for multiple MCP servers
  - Automatic tool routing with server ID prefixes
  - Backend process management with health monitoring
  - Tool aggregation from all backend servers
- **Web Dashboard** (Port 7447)
  - Server management UI
  - Real-time status monitoring
  - Add/edit/remove servers
  - Dark mode support
  - Comprehensive documentation tab
- **Security Features**
  - Multiple sandboxing strategies (Docker, Podman, Firejail, Bubblewrap)
  - Command allowlisting for security
  - Environment variable sanitization
  - CSRF protection and security headers
- **Configuration System**
  - JSON-based server configuration
  - Environment variable support with expansion
  - Per-server sandbox configuration
  - Authentication settings (not yet integrated)
- **Router Management Tools**
  - `router:status` - Check health of all backends
  - `router:list_servers` - List configured servers
- **Pre-configured Servers**
  - echo - Test server for development
  - jaumemory - Persistent memory system
  - jau-tg - Telegram integration

### Known Issues
- Authentication system exists but not integrated with router
- No automatic recovery for failed backends
- Tools refresh every 30 seconds without circuit breaker
- Some servers configured with `"strategy": "none"` (should use sandboxing)
- Echo server path incorrect in default config

### Infrastructure
- GitHub Actions CI/CD pipeline
- Cargo workspace structure
- TypeScript build system for MCP server
- SQLite database for auth (not yet used)
- Comprehensive test suite

## Future Releases

### Planned Features
- Authentication integration for protected servers
- Automatic backend recovery with health checks
- Circuit breaker for tool refresh
- Hot reload configuration
- Metrics and monitoring (Prometheus/OpenTelemetry)
- WebSocket support for real-time updates
- Unix socket communication option
- Rate limiting implementation

### Security Improvements
- Enable sandboxing by default
- Implement rate limiting
- Add audit logging
- WebAuthn/biometric authentication
- Session management improvements

---

For more details on each release, see the [GitHub Releases](https://github.com/yourusername/JauAuth/releases) page.