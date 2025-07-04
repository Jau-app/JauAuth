# JauAuth Test Suite

This directory contains all tests for JauAuth, organized by test type.

## Test Structure

```
tests/
├── unit/              # Unit tests for individual components
│   ├── auth/         # Authentication logic tests
│   ├── router/       # Router functionality tests
│   ├── sandbox/      # Sandboxing tests
│   └── session/      # Session management tests
│
├── integration/       # Integration tests
│   ├── mcp/          # MCP protocol integration
│   ├── backend/      # Backend server integration
│   └── dashboard/    # Web dashboard API tests
│
├── e2e/              # End-to-end tests
│   ├── scenarios/    # User scenario tests
│   └── fixtures/     # Test data and mock servers
│
├── performance/      # Performance and load tests
│   ├── benchmarks/   # Micro-benchmarks
│   └── load/         # Load testing scenarios
│
└── security/         # Security tests
    ├── auth/         # Authentication security
    ├── injection/    # Injection attack tests
    └── sandbox/      # Sandbox escape tests
```

## Running Tests

### Run all tests
```bash
cargo test
```

### Run specific test category
```bash
# Unit tests only
cargo test --test unit_*

# Integration tests only
cargo test --test integration_*

# E2E tests only
cargo test --test e2e_*

# Performance benchmarks
cargo bench

# Security tests (requires special setup)
cargo test --test security_* -- --test-threads=1
```

### Run with coverage
```bash
cargo tarpaulin --out Html
```

## Writing Tests

### Unit Tests
- Test individual functions and modules in isolation
- Use mocks for external dependencies
- Keep tests fast and focused

### Integration Tests
- Test interaction between components
- Use real implementations where possible
- Test error conditions and edge cases

### E2E Tests
- Test complete user workflows
- Include setup and teardown
- Test from the user's perspective

### Performance Tests
- Use criterion for benchmarks
- Test both throughput and latency
- Compare against baselines

### Security Tests
- Test authentication bypass attempts
- Test injection vulnerabilities
- Test sandbox escape attempts
- Run in isolated environment

## Test Utilities

Common test utilities are available in `tests/common/mod.rs`:
- Mock MCP servers
- Test authentication helpers
- Database fixtures
- Configuration builders