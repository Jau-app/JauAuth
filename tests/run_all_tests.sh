#!/bin/bash
# Comprehensive test runner for JauAuth

set -e

echo "🧪 JauAuth Test Suite"
echo "===================="

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test categories
UNIT_TESTS=true
INTEGRATION_TESTS=true
E2E_TESTS=false  # Disabled by default as they need server running
PERFORMANCE_TESTS=false  # Disabled by default as they take time
SECURITY_TESTS=true

# Parse arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --all) E2E_TESTS=true; PERFORMANCE_TESTS=true ;;
        --e2e) E2E_TESTS=true ;;
        --perf) PERFORMANCE_TESTS=true ;;
        --quick) INTEGRATION_TESTS=false; SECURITY_TESTS=false ;;
        *) echo "Unknown parameter: $1"; exit 1 ;;
    esac
    shift
done

# Function to run test category
run_tests() {
    local category=$1
    local pattern=$2
    echo -e "\n${YELLOW}Running $category tests...${NC}"
    
    if cargo test $pattern -- --nocapture; then
        echo -e "${GREEN}✅ $category tests passed${NC}"
        return 0
    else
        echo -e "${RED}❌ $category tests failed${NC}"
        return 1
    fi
}

# Set required env vars for tests
export JWT_SECRET="test_secret_key_for_testing_only_32_chars_long"
export JAUAUTH_DATABASE_URL=":memory:"

# Track failures
FAILED=0

# Run unit tests
if [ "$UNIT_TESTS" = true ]; then
    run_tests "Unit" "unit_" || FAILED=$((FAILED + 1))
fi

# Run integration tests
if [ "$INTEGRATION_TESTS" = true ]; then
    run_tests "Integration" "integration_" || FAILED=$((FAILED + 1))
fi

# Run security tests
if [ "$SECURITY_TESTS" = true ]; then
    run_tests "Security" "security_" || FAILED=$((FAILED + 1))
fi

# Run E2E tests (requires server)
if [ "$E2E_TESTS" = true ]; then
    echo -e "\n${YELLOW}Starting server for E2E tests...${NC}"
    cargo run -- combined &
    SERVER_PID=$!
    sleep 5
    
    run_tests "E2E" "e2e_" -- --ignored || FAILED=$((FAILED + 1))
    
    kill $SERVER_PID 2>/dev/null || true
fi

# Run performance benchmarks
if [ "$PERFORMANCE_TESTS" = true ]; then
    echo -e "\n${YELLOW}Running performance benchmarks...${NC}"
    if cargo bench; then
        echo -e "${GREEN}✅ Benchmarks completed${NC}"
    else
        echo -e "${RED}❌ Benchmarks failed${NC}"
        FAILED=$((FAILED + 1))
    fi
fi

# Coverage report (optional)
if command -v cargo-tarpaulin &> /dev/null; then
    echo -e "\n${YELLOW}Generating coverage report...${NC}"
    cargo tarpaulin --out Html --output-dir target/coverage
    echo -e "${GREEN}Coverage report generated at target/coverage/tarpaulin-report.html${NC}"
fi

# Summary
echo -e "\n===================="
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ $FAILED test suites failed${NC}"
    exit 1
fi