# 🔐 JauAuth - Secure MCP Router

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-blue)](https://modelcontextprotocol.io)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0+-blue)](https://www.typescriptlang.org)
[![GitHub Sponsors](https://img.shields.io/badge/Sponsor-❤-ea4aaa)](https://github.com/sponsors/Jau-app)

> ⚠️ **SECURITY DISCLAIMER**: While JauAuth implements multiple security layers including sandboxing, authentication, and validation, no security solution is 100% foolproof. This software is provided "as is" without warranty of any kind. Use at your own risk. The authors and contributors are not responsible for any security breaches, data loss, or damages arising from the use of this software. Always follow security best practices and regularly update your dependencies.

## The Problem: MCP Server Sprawl and Security Nightmares

When we started building AI tools with Claude Desktop, we quickly ran into a frustrating problem. Every new capability meant connecting another MCP server - one for file access, another for memory, one more for notifications. Soon, our Claude configuration was a tangled mess of server connections, each running with full system access.

It felt wrong. Here we were, giving AI assistants powerful tools, but with no way to manage or secure them properly. Each server was a potential security risk, running unsandboxed with whatever permissions it wanted.

Then came the wake-up call. The recent [critical vulnerability in Anthropic's MCP Inspector (CVE-2025-49596)](https://thehackernews.com/2025/07/critical-vulnerability-in-anthropics.html?m=1) exposed how dangerous this approach really is. With a CVSS score of 9.4/10, attackers could gain complete access to developer machines simply by tricking them into visiting a malicious website. The vulnerability chained browser flaws with MCP's lack of authentication, turning development tools into backdoors. As security researchers noted, this was "one of the first critical RCEs in Anthropic's MCP ecosystem, exposing a new class of browser-based attacks against AI developer tools."

Even worse, researchers found hundreds of MCP servers vulnerable to "NeighborJack" attacks - servers accidentally exposed on 0.0.0.0, allowing anyone on the same network (think coffee shops, coworking spaces) to hijack your AI tools. SQL injection vulnerabilities in reference implementations meant attackers could poison AI context and steal data through trusted internal channels.

The security community's message was clear: the MCP ecosystem desperately needed a security-first approach. And every time we switched projects, we still had to reconfigure everything from scratch. There had to be a better way.

## Our Solution: One Router to Rule Them All

JauAuth transforms the chaos of multiple MCP connections into elegant simplicity. Instead of connecting Claude to a dozen different servers, you connect to JauAuth once. It acts as a secure gateway, managing all your MCP servers behind the scenes.

We built it with security as the foundation, not an afterthought. Every MCP server runs in its own sandbox - whether that's Docker, Firejail, or another isolation technology. No more servers listening on 0.0.0.0. No more unauthenticated endpoints. No more hoping developers configured everything correctly. JauAuth enforces security by default:

- **Authentication required** - Every connection is authenticated with JWT tokens
- **Origin validation** - Prevents CSRF and DNS rebinding attacks
- **Sandboxed execution** - Each server runs isolated, with only the permissions it needs
- **Command allowlisting** - Only approved binaries can be executed
- **Network isolation** - Servers can't accidentally expose themselves to the network

Through the beautiful web dashboard, you can monitor everything in real-time, adding or removing servers without ever restarting Claude. You get fine-grained control over what each server can access, who can use it, and how it behaves.

The magic happens through intelligent tool routing. When you ask Claude to remember something, JauAuth automatically routes that to your memory server. When you need to send a notification, it goes to your Telegram server. Each tool is namespaced (like `jaumemory_remember` or `jau-tg_send_message`), keeping everything organized and conflict-free.

A secure MCP (Model Context Protocol) router that provides a single connection point for multiple MCP servers, with built-in sandboxing, authentication capabilities, and a beautiful web dashboard.

## 🌟 Why JauAuth?

Managing multiple MCP servers can be complex and insecure. JauAuth solves this by providing:
- **One connection** instead of many in Claude Desktop
- **Centralized security** with sandboxing for all servers
- **Easy management** through a web dashboard
- **Tool organization** with automatic namespacing

## ✨ Features

### MCP Router
- 🔀 **Single Connection Point**: Connect to multiple MCP servers through one interface
- 🏷️ **Automatic Tool Prefixing**: Tools are namespaced by server ID (e.g., `jaumemory_remember`)
- 🚀 **TypeScript/Rust Hybrid**: TypeScript for MCP protocol, Rust for security and performance
- 🛡️ **Multiple Sandboxing Strategies**: Docker, Podman, Firejail, Bubblewrap
- 📊 **Web Dashboard**: Monitor and manage all backend servers at http://localhost:7447
- 🔄 **Hot-swappable Servers**: Add/remove servers without restarting
- ⚡ **Process Management**: Automatic spawning, health checks, and graceful shutdown

### Security Features
- 🔒 **Sandboxed Execution**: Isolate MCP servers with configurable security strategies
- 🛡️ **Command Allowlisting**: Control which commands servers can execute
- 🔑 **Secure Token Storage**: Store API keys in environment variables with automatic substitution
- 🎭 **Token Masking**: Sensitive values displayed as `hf_...` in dashboard
- 💾 **Persistent Configuration**: Servers save to JSON config and reload on startup
- 📝 **Audit Logging**: Track all tool calls and server activities
- 🚦 **Rate Limiting**: Protect against abuse
- 🔐 **CSRF Protection**: Secure web endpoints

### Coming Soon
- 🔐 **Universal Authentication**: One auth system for all MCP tools
- 📱 **WebAuthn Support**: Biometric authentication
- 🎯 **Fine-grained Permissions**: Control access per tool/server

## 🚀 Quick Start

### Prerequisites
- Rust 1.75 or higher
- Node.js 18 or higher
- (Optional) Docker, Podman, or Firejail for sandboxing

### Installation

1. **Clone the repository:**
```bash
git clone https://github.com/Jau-app/JauAuth.git
cd Jau-Auth
```

2. **Build the TypeScript MCP server:**
```bash
cd mcp-server
npm install
npm run build
cd ..
```

3. **Build the Rust backend:**
```bash
cargo build --release
```

4. **Configure your MCP servers** in `claude-router-config.json`:
```json
{
  "servers": [
    {
      "id": "jaumemory",
      "name": "JauMemory Persistent Memory",
      "command": "node",
      "args": ["/path/to/jaumemory/mcp-server/dist/index.js"],
      "env": {
        "GRPC_SERVER_ADDRESS": "localhost:50051"
      },
      "requires_auth": false,
      "allowed_users": [],
      "sandbox": {
        "strategy": "none",
        "env_passthrough": ["HOME", "USER", "PATH", "NODE_PATH"]
      }
    }
  ],
  "timeout_ms": 30000,
  "cache_tools": true
}
```

### 🔐 Secure Token Handling

JauAuth provides secure storage for API keys and tokens using environment variable substitution:

```json
{
  "id": "hf-mcp-server",
  "name": "HuggingFace MCP Server",
  "command": "npx",
  "args": [
    "mcp-remote",
    "https://huggingface.co/mcp",
    "--header",
    "Authorization: Bearer $HF_TOKEN"  // Reference env variable
  ],
  "env": {
    "HF_TOKEN": "hf_yourActualTokenHere"  // Secure storage
  },
  "sandbox": {
    "strategy": "none",
    "env_passthrough": ["HOME", "USER", "PATH", "HF_TOKEN"]
  }
}
```

**Security Features:**
- Tokens stored in `env` section are automatically masked in the dashboard
- Use `$VARIABLE_NAME` syntax in args to reference environment variables
- All environment values display as `hf_...` (first 4 and last 4 characters)
- Actual tokens never appear in logs or UI displays

5. **Start the combined server:**
```bash
./run-combined-server.sh
```

6. **Add to Claude Desktop:**
```bash
claude mcp add jauauth /path/to/JauAuth/mcp-launcher.sh
```

## 🏗️ Architecture

JauAuth uses a TypeScript/Rust hybrid architecture for optimal security and compatibility:

```
┌─────────────┐     JSON-RPC      ┌──────────────────┐     HTTP API      ┌─────────────────┐
│   Claude    │ ◄─────────────► │ TypeScript MCP   │ ◄────────────► │  Rust Backend   │
│   Desktop   │     (stdio)      │     Server       │   (port 7447)   │     Router      │
└─────────────┘                  └──────────────────┘                └────────┬────────┘
                                                                               │
                                                                               │ stdio
                                                                               ▼
                                                    ┌──────────────┬──────────────┬──────────────┐
                                                    │ MCP Server 1 │ MCP Server 2 │ MCP Server N │
                                                    └──────────────┴──────────────┴──────────────┘
```

- **TypeScript MCP Server**: Handles MCP protocol using the official SDK
- **Rust Backend**: Manages server processes, routing, security, and web dashboard
- **Web Dashboard**: Beautiful UI for server management and monitoring

## 📦 Sandboxing Options

JauAuth supports multiple sandboxing strategies for different security needs:

### 1. None (Development Only)
```json
"sandbox": {
  "strategy": "none",
  "env_passthrough": ["HOME", "USER", "PATH"]
}
```

### 2. Firejail (Recommended for Linux)
```json
"sandbox": {
  "strategy": {
    "firejail": {
      "profile": "default",
      "whitelist_paths": [],
      "read_only_paths": [],
      "net": true,
      "no_root": true
    }
  }
}
```

### 3. Docker (Maximum Security)
```json
"sandbox": {
  "strategy": {
    "docker": {
      "image": "node:18-alpine",
      "memory_limit": "512m",
      "cpu_limit": "0.5",
      "network": false
    }
  }
}
```

## 🖥️ Web Dashboard

Access the dashboard at http://localhost:7447 to:

- View all connected MCP servers and their status
- Add, edit, or remove servers with config persistence
- Monitor server health and resource usage
- View available tools from each server
- Test tools directly from the UI
- Configure authentication and security settings

## 🔧 Available Tools

When connected, tools are prefixed with their server ID:

**Router Management**:
- `router_status` - Get status of all backend servers
- `router_list_servers` - List configured servers

**Example with JauMemory** (`jaumemory_*`):
- `jaumemory_remember` - Store memories
- `jaumemory_recall` - Search memories
- `jaumemory_forget` - Delete memories
- `jaumemory_analyze` - Analyze patterns

**Example with Telegram** (`jau-tg_*`):
- `jau-tg_send_message` - Send Telegram message
- `jau-tg_send_alert` - Send alert to default chat
- `jau-tg_read_messages` - Read message buffer

## 🛠️ Development

### Running in Development Mode
```bash
# Terminal 1: Run the combined server
./run-combined-server.sh

# Terminal 2: Watch dashboard logs
tail -f /tmp/jauauth-backend.log

# Terminal 3: Watch TypeScript logs
tail -f /tmp/jauauth-mcp.log
```

### Running Tests
```bash
# Rust tests
cargo test

# Integration tests
node scripts/test-full.js

# Test individual tools with timeout support
node scripts/test-tool.js router_status
node scripts/test-tool.js jau-tg_send_alert '{"message":"Test"}'
node scripts/test-tool.js jau-tg_wait_for_approval '{"content":"Approve?"}' --timeout=300000
node scripts/test-tool.js any_tool '{"args":"here","__timeout":"*"}'

# Test timeout functionality
node scripts/test-timeout.js

# Check available sandboxing
cargo run --bin sandbox-check
```

### Building for Production
```bash
# Optimized build
make build

# Run with systemd (example service file in docs/)
sudo systemctl start jauauth
```

## 🔒 Security Best Practices

1. **Always use sandboxing** in production (never `"strategy": "none"`)
2. **Set strong JWT secret**: `export JAUAUTH_JWT_SECRET=$(openssl rand -base64 32)`
3. **Enable authentication**: Set `"requires_auth": true` for sensitive servers
4. **Use HTTPS** for the web dashboard in production
5. **Regular updates**: Keep dependencies updated for security patches

## 📚 Documentation

- [Changelog](CHANGELOG.md) - Latest updates and version history
- [Architecture Overview](docs/ARCHITECTURE.md)
- [Security & Sandboxing](docs/SANDBOX-SECURITY.md)
- [Dashboard Guide](docs/DASHBOARD-README.md)
- [Timeout Configuration](docs/TIMEOUT-CONFIGURATION.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup
1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Repository Structure
- **src/** - Rust source code
- **mcp-server/** - TypeScript MCP server
- **web/** - Dashboard frontend
- **docs/** - Public documentation
- **scripts/** - Essential scripts (install tools, setup)
- **tests/** - Test suite
- **.dev/** - Development files (gitignored)

## 🐛 Troubleshooting

### Common Issues

**TypeScript server not connecting:**
```bash
# Check if backend is running
curl http://localhost:7447/api/mcp/status

# Check logs
tail -f /tmp/jauauth-mcp.log
```

**Tools not showing in Claude:**
- Ensure tool names only contain letters, numbers, underscores, and hyphens
- Tools refresh every 30 seconds
- Check server health in dashboard

See [Troubleshooting Guide](docs/TROUBLESHOOTING.md) for more solutions.

## 🔐 Security

JauAuth takes security seriously. If you discover a security vulnerability, please email security@jau.app instead of using the issue tracker.

## 📄 License

This project is licensed under the Business Source License 1.1 with Additional Use Grants.

**Free to use for:**
- 🏠 Personal, non-commercial use
- 🏢 Companies with fewer than 5 employees
- 🎓 Educational and research purposes

**Commercial License Required for:**
- Companies with 5 or more employees
- Commercial production use beyond the grants above

For commercial licensing, please contact: license@jau.app

See the [LICENSE](LICENSE) file for full details.

## 🙏 Acknowledgments

- Built with the [MCP SDK](https://github.com/modelcontextprotocol/sdk)
- Inspired by the need for secure, manageable MCP deployments
- Special thanks to the Claude and Anthropic teams for MCP
- Thanks to all contributors who help make JauAuth better!

## 🌟 Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Jau-app/JauAuth&type=Date)](https://star-history.com/#Jau-app/JauAuth&Date)

## 📞 Support

- 📧 Email: support@jau.app
- 💬 Discord: [Join our community](https://discord.gg/jau-app)
- 🐛 Issues: [GitHub Issues](https://github.com/Jau-app/JauAuth/issues)
- 📖 Docs: [Documentation](https://docs.jau.app/jauauth)

---

<p align="center">
  Made with ❤️ by the <a href="https://jau.app">Jau</a> team
</p>

<p align="center">
  <a href="https://github.com/Jau-app/Jau-Auth/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=Jau-app/Jau-Auth" />
  </a>
</p>
