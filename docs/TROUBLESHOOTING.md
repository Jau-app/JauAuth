# Troubleshooting Guide

This guide helps you resolve common issues with JauAuth.

## Common Issues

### TypeScript MCP Server Not Connecting

**Symptoms:**
- Claude shows "Failed to connect to MCP server"
- No tools appear in Claude
- TypeScript server crashes on startup

**Solutions:**

1. **Check if Rust backend is running:**
```bash
curl http://localhost:7447/api/mcp/status
```

2. **Verify backend URL:**
```bash
# Default is http://localhost:7447
export RUST_BACKEND_URL=http://localhost:7447
```

3. **Check TypeScript logs:**
```bash
tail -f /tmp/jauauth-mcp.log
```

4. **Ensure correct startup order:**
```bash
# 1. Start Rust backend FIRST
cargo run -- combined

# 2. Then connect Claude to TypeScript MCP server
claude mcp add jau-auth -- node /path/to/jau-auth/mcp-server/dist/index.js
```

### Backend Server Not Starting

**Symptoms:**
- Server shows as "unhealthy" in dashboard
- "Command not found" errors
- Sandbox strategy errors

**Solutions:**

1. **Verify command exists:**
```bash
which node  # or python, npx, etc.
```

2. **Check server path:**
   - Ensure paths in config are absolute
   - Verify file exists: `ls -la /path/to/server.js`

3. **Test without sandbox:**
```json
"sandbox": {
  "strategy": "none"
}
```

4. **Check available sandbox strategies:**
```bash
cargo run --bin sandbox-check
```

### Tools Not Appearing in Claude

**Symptoms:**
- Connected but no tools show up
- Only router tools appear
- Tools disappear after working

**Solutions:**

1. **Check backend health:**
```bash
# Via dashboard
http://localhost:7447

# Via API
curl http://localhost:7447/api/mcp/tools
```

2. **Wait for tool refresh:**
   - Tools refresh every 30 seconds
   - Check dashboard for server status

3. **Verify tool names:**
   - Must contain only: letters, numbers, underscores, hyphens
   - Format: `server_id:tool_name`

4. **Restart Claude Desktop:**
   - Sometimes Claude caches MCP connections
   - Fully quit and restart Claude

### Server Configuration Not Persisting

**Symptoms:**
- Servers disappear after restart
- Configuration changes not saved
- "Failed to save configuration" errors

**Solutions:**

1. **Check file permissions:**
```bash
ls -la claude-router-config.json
# Should be writable by your user
```

2. **Verify JSON syntax:**
```bash
# Use jq to validate
jq . claude-router-config.json
```

3. **Check for trailing commas:**
   - JSON doesn't allow trailing commas
   - Remove comma after last item in arrays/objects

### Authentication Token Issues

**Symptoms:**
- "Authentication required" errors
- Token not being substituted
- Masked tokens showing as "***"

**Solutions:**

1. **Proper token configuration:**
```json
{
  "env": {
    "HF_TOKEN": "hf_actualTokenHere"
  },
  "args": [
    "--header",
    "Authorization: Bearer $HF_TOKEN"
  ]
}
```

2. **Include in env_passthrough:**
```json
"sandbox": {
  "env_passthrough": ["HF_TOKEN"]
}
```

3. **Check token masking:**
   - Dashboard shows as `hf_a...Here`
   - Actual token is in config file

### NPX Package Not Found

**Symptoms:**
- "NPX package 'xyz' not found" error
- "Command 'npx' not found"

**Solutions:**

1. **Install Node.js:**
```bash
# Check if installed
node --version
npm --version

# Install from https://nodejs.org/
```

2. **Install package globally:**
```bash
npm install -g @modelcontextprotocol/server-xyz
```

3. **Use full path:**
```json
{
  "command": "/usr/local/bin/npx",
  "args": ["-y", "package-name"]
}
```

### Long Operations Timing Out

**Symptoms:**
- "Request timeout" after 30 seconds
- Operations incomplete
- "Consider using __timeout parameter"

**Solutions:**

1. **Use __timeout parameter:**
```javascript
// 5 minute timeout
wait_for_approval({
  content: "Please approve",
  __timeout: 300000
})

// No timeout
process_large_file({
  path: "/big/file.csv",
  __timeout: "*"
})
```

2. **Increase default timeout:**
```json
{
  "timeout_ms": 60000,  // 1 minute default
  "servers": [...]
}
```

## Debug Mode

Enable detailed logging for troubleshooting:

```bash
# TypeScript MCP Server
export LOG_LEVEL=debug

# Rust Backend
export RUST_LOG=jau_auth=debug,rust_mcp_sdk=info

# Run with verbose output
cargo run -- combined
```

## Getting Help

If these solutions don't work:

1. **Check logs:**
   - `/tmp/jauauth-mcp.log` - TypeScript server
   - `/tmp/jauauth-backend.log` - Rust backend
   - Dashboard logs in browser console

2. **GitHub Issues:**
   - Search existing issues: https://github.com/jau-app/jau-auth/issues
   - Create new issue with:
     - Error messages
     - Configuration (remove sensitive data)
     - Steps to reproduce

3. **Discord Community:**
   - Join for real-time help
   - Share logs and config (sanitized)

## Common Error Messages

| Error | Cause | Solution |
|-------|-------|----------|
| "Failed to parse configuration file" | Invalid JSON | Check for trailing commas, quotes |
| "Backend 'xyz' not found" | Tool routing error | Verify server ID matches config |
| "Sandbox strategy not available" | Missing sandbox tool | Run `install-sandbox-tools.sh` |
| "Command not allowed" | Security restriction | Only allowed: node, python, npx, etc. |
| "Failed to spawn backend" | Binary not found | Check PATH, install missing tools |
| "No tools found" | Backend unhealthy | Check server logs, test manually |