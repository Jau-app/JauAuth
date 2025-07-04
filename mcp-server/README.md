# JauAuth TypeScript MCP Server

This is the TypeScript MCP (Model Context Protocol) server for JauAuth. It provides a clean MCP interface while delegating the actual backend management and tool routing to the Rust backend via HTTP API.

## Architecture

```
MCP Client (Claude) <--> TypeScript MCP Server <--> Rust Backend <--> MCP Backends
     stdio/JSON-RPC          HTTP API                 stdio
```

## Why TypeScript + Rust?

- **TypeScript**: Better MCP SDK support, cleaner protocol implementation
- **Rust**: Superior process management, security sandboxing, and performance
- **Best of Both**: Protocol handling in TypeScript, heavy lifting in Rust

## Setup

1. Install dependencies:
   ```bash
   npm install
   ```

2. Build the TypeScript server:
   ```bash
   npm run build
   ```

3. Start the Rust backend:
   ```bash
   # From parent directory
   cargo run -- combined
   ```

4. Run the TypeScript MCP server:
   ```bash
   node dist/index.js
   ```

## Configuration

Edit `config.json` to configure the backend URL:

```json
{
  "backend": {
    "url": "http://localhost:7447/api/mcp",
    "timeout": 30000
  }
}
```

Or use environment variables:
- `RUST_BACKEND_URL`: Backend API URL (default: http://localhost:7447)
- `API_TIMEOUT`: API timeout in milliseconds (default: 30000)
- `LOG_LEVEL`: Logging level (default: info)

## Claude Integration

Add JauAuth to Claude:
```bash
claude mcp add jau-auth-ts -- node "/path/to/jau-auth/mcp-server/dist/index.js"
```

## Development

Run in development mode with auto-reload:
```bash
npm run dev
```

## Testing

Run the integration test:
```bash
node test-integration.js
```

## Available Tools

The TypeScript MCP server provides:
- `router:status` - Get status of all backend servers
- `router:list_servers` - List configured backend servers
- All tools from configured backend servers (prefixed with server ID)

## Logging

Logs are written to `/tmp/jauauth-mcp.log` in JSON format.