#!/bin/bash

echo "Starting JauAuth Combined Server"
echo "================================"
echo ""
echo "Web Dashboard: http://localhost:7447"
echo "MCP Router: Running on stdio (connect your MCP client to this process)"
echo ""
echo "To test the MCP router, run in another terminal:"
echo "  node test-router.js"
echo ""
echo "Press Ctrl+C to stop the server"
echo ""

# Set DATABASE_URL for SQLx
export DATABASE_URL="sqlite:jauauth.db"

# Check if release binary exists
if [ ! -f ./target/release/jau-auth ]; then
    echo "Release binary not found. Building..."
    cargo build --release
fi

# Use the release binary
exec ./target/release/jau-auth combined --config claude-router-config.json --port 7447