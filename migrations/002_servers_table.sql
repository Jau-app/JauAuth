-- Migration: Add servers table for secure server configuration storage
-- This table stores user-added MCP server configurations with encrypted sensitive data

-- Servers table
CREATE TABLE IF NOT EXISTS servers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Server identification
    server_id TEXT NOT NULL,  -- Unique identifier for the server (e.g., "hf-mcp-server")
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Basic server info (not encrypted)
    name TEXT NOT NULL,
    description TEXT,
    server_type TEXT NOT NULL DEFAULT 'local', -- local, remote
    transport_type TEXT NOT NULL DEFAULT 'stdio', -- stdio, sse, websocket
    
    -- Command and execution (encrypted for security)
    command_encrypted TEXT,  -- Encrypted command (e.g., "npx")
    args_encrypted TEXT,     -- Encrypted JSON array of arguments
    url_encrypted TEXT,      -- Encrypted URL for remote servers
    
    -- Environment variables (encrypted)
    env_encrypted TEXT,      -- Encrypted JSON object of environment variables
    
    -- Authentication and access control
    requires_auth BOOLEAN NOT NULL DEFAULT 1,
    allowed_users_encrypted TEXT,  -- Encrypted JSON array of allowed usernames
    
    -- Sandbox configuration (encrypted)
    sandbox_config_encrypted TEXT,  -- Encrypted JSON sandbox configuration
    
    -- Server options
    timeout_ms INTEGER DEFAULT 30000,
    auto_start BOOLEAN NOT NULL DEFAULT 1,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    
    -- Metadata
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used TIMESTAMP,
    
    -- Ensure unique server_id per user
    UNIQUE(user_id, server_id)
);

-- Indexes for performance
CREATE INDEX idx_servers_user_id ON servers(user_id);
CREATE INDEX idx_servers_server_id ON servers(server_id);
CREATE INDEX idx_servers_enabled ON servers(enabled);

-- Server health/status tracking table
CREATE TABLE IF NOT EXISTS server_health (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id INTEGER NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    status TEXT NOT NULL, -- healthy, unhealthy, unknown
    last_check TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    error_message TEXT,
    tool_count INTEGER DEFAULT 0,
    uptime_seconds INTEGER DEFAULT 0
);

CREATE INDEX idx_server_health_server_id ON server_health(server_id);
CREATE INDEX idx_server_health_last_check ON server_health(last_check);

-- Server usage statistics
CREATE TABLE IF NOT EXISTS server_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id INTEGER NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    invocation_count INTEGER DEFAULT 0,
    last_used TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(server_id, user_id, tool_name)
);

CREATE INDEX idx_server_usage_server_id ON server_usage(server_id);
CREATE INDEX idx_server_usage_last_used ON server_usage(last_used);

-- Trigger to update the updated_at timestamp
CREATE TRIGGER update_servers_timestamp 
AFTER UPDATE ON servers
BEGIN
    UPDATE servers SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;