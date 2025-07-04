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

# Keep the process running by reading from stdin
exec ./target/release/jau-auth combined --config claude-router-config.json --port 7447