# JauAuth Makefile - Speed up development workflow

# Default target
.PHONY: help
help:
	@echo "JauAuth Development Commands:"
	@echo "  make build       - Build release version (fast runtime)"
	@echo "  make run         - Run combined mode (uses release build)"
	@echo "  make dev         - Development mode with auto-reload"
	@echo "  make install     - Install as system command"
	@echo "  make clean       - Clean build artifacts"
	@echo "  make check       - Fast compilation check"
	@echo "  make test        - Run tests"
	@echo "  make ts-build    - Build TypeScript MCP server"
	@echo "  make all         - Build everything (Rust + TypeScript)"
	@echo "  make quick       - Quick start (build + run)"

# Build release version (much faster runtime)
.PHONY: build
build:
	@echo "Building release version..."
	cargo build --release
	@echo "✅ Release build complete: target/release/jau-auth"

# Run using release build
.PHONY: run
run:
	@if [ ! -f target/release/jau-auth ]; then \
		echo "Release build not found, building first..."; \
		cargo build --release; \
	fi
	@echo "Starting JauAuth (combined mode)..."
	./target/release/jau-auth combined

# Development mode with auto-reload
.PHONY: dev
dev:
	@command -v cargo-watch >/dev/null 2>&1 || { \
		echo "Installing cargo-watch..."; \
		cargo install cargo-watch; \
	}
	@echo "Starting development mode with auto-reload..."
	cargo watch -x "run -- combined"

# Install as system command
.PHONY: install
install: build
	@echo "Installing jau-auth to ~/.cargo/bin/"
	cargo install --path .
	@echo "✅ Installed! You can now run 'jau-auth combined' from anywhere"

# Clean build artifacts
.PHONY: clean
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf mcp-server/dist
	rm -rf mcp-server/node_modules
	@echo "✅ Clean complete"

# Fast compilation check
.PHONY: check
check:
	@echo "Running quick compilation check..."
	cargo check

# Run tests
.PHONY: test
test:
	@echo "Running tests..."
	cargo test

# Build TypeScript MCP server
.PHONY: ts-build
ts-build:
	@echo "Building TypeScript MCP server..."
	cd mcp-server && npm install && npm run build
	@echo "✅ TypeScript build complete"

# Build everything
.PHONY: all
all: build ts-build
	@echo "✅ All builds complete!"

# Quick start - build and run
.PHONY: quick
quick: build
	@echo "Starting JauAuth..."
	./target/release/jau-auth combined

# Development setup
.PHONY: setup
setup:
	@echo "Setting up development environment..."
	@command -v cargo-watch >/dev/null 2>&1 || cargo install cargo-watch
	@command -v sccache >/dev/null 2>&1 || cargo install sccache
	cd mcp-server && npm install
	@echo "✅ Development setup complete"

# Profile-guided optimization build (even faster)
.PHONY: pgo
pgo:
	@echo "Building with Profile-Guided Optimization..."
	@echo "Step 1: Building with profiling..."
	RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release
	@echo "Step 2: Running profiling workload..."
	./target/release/jau-auth combined &
	@sleep 5
	@pkill -f "jau-auth combined" || true
	@echo "Step 3: Building with profile data..."
	RUSTFLAGS="-Cprofile-use=/tmp/pgo-data" cargo build --release
	@rm -rf /tmp/pgo-data
	@echo "✅ PGO build complete (fastest possible)"

# Incremental build optimization
.PHONY: fast-build
fast-build:
	@echo "Building with incremental compilation..."
	CARGO_INCREMENTAL=1 cargo build --release

# LTO (Link Time Optimization) build - slower build, faster runtime
.PHONY: lto
lto:
	@echo "Building with LTO (this will take longer but run faster)..."
	CARGO_PROFILE_RELEASE_LTO=true cargo build --release