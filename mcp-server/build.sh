#!/bin/bash

# Build script for JauAuth TypeScript MCP Server

set -e

echo "Building JauAuth TypeScript MCP Server..."

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install
fi

# Build TypeScript
echo "Compiling TypeScript..."
npm run build

echo "Build complete! Output in dist/"
echo ""
echo "To run the MCP server standalone:"
echo "  node dist/index.js"
echo ""
echo "To use with Claude:"
echo "  claude mcp add jau-auth-ts -- node \"$(pwd)/dist/index.js\""