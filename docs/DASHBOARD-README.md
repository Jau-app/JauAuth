# JauAuth Dashboard

A secure web-based dashboard for managing MCP servers and authentication settings.

## Features

### 🎯 Server Management
- Add, edit, and remove MCP servers
- Real-time health monitoring
- View server logs
- Configure sandbox strategies per server

### 🔧 Tool Explorer
- Browse all available tools from connected servers
- Test tools directly from the dashboard
- View tool documentation and schemas

### 🔐 Security Features
- Content Security Policy (CSP) headers
- CSRF protection
- XSS prevention
- Request size limits
- Secure session management
- HTTPS-only cookies in production

### ⚙️ Auth Settings
- Configure session duration
- Manage permission groups
- Set rate limits
- WebAuthn configuration

## Running the Dashboard

```bash
# Start with default settings
cargo run --bin jau-auth -- web

# Start on custom port
cargo run --bin jau-auth -- web --port 7448

# Start with router configuration
cargo run --bin jau-auth -- web --config router-config.json
```

## Accessing the Dashboard

Open your browser to: http://localhost:7447

The dashboard provides:
1. **Overview** - System status and health monitoring
2. **Servers** - Manage MCP server configurations
3. **Tools** - Browse and test available tools
4. **Settings** - Configure authentication settings

## Adding a Server

1. Navigate to the "Servers" tab
2. Click "Add Server"
3. Fill in the configuration:
   - **Server ID**: Unique identifier (alphanumeric + dashes)
   - **Display Name**: Human-readable name
   - **Command**: Executable command (npx, node, python, etc.)
   - **Arguments**: Command arguments
   - **Sandbox Strategy**: Security isolation method

Example configuration:
```
ID: filesystem-server
Name: Filesystem MCP Server
Command: npx
Arguments: @modelcontextprotocol/server-filesystem /home/user/docs
Sandbox: Firejail
```

## Security Configuration

The dashboard implements multiple security layers:

### Frontend Security
- Strict CSP headers prevent XSS attacks
- All API calls include CSRF tokens
- Sensitive data never stored in localStorage
- Input validation and sanitization

### Backend Security
- Authentication required for all dashboard routes
- Rate limiting on API endpoints
- Request body size limits (5MB)
- Sandbox validation for server configurations

### Production Deployment
- Enable HTTPS with proper certificates
- Set secure session cookies
- Configure CORS for your domain only
- Use strong JWT secrets
- Enable HSTS headers

## API Endpoints

All dashboard APIs are prefixed with `/api/dashboard/`:

- `GET /overview` - System overview stats
- `GET /servers` - List all servers
- `POST /servers` - Add new server
- `GET /servers/:id` - Get server details
- `PUT /servers/:id` - Update server
- `DELETE /servers/:id` - Remove server
- `GET /servers/:id/logs` - View server logs
- `GET /tools` - List all available tools
- `POST /tools/test` - Test a tool
- `GET /auth/settings` - Get auth configuration
- `PUT /auth/settings` - Update auth settings

## Development

### Frontend
- Pure vanilla JavaScript (no framework dependencies)
- Modern CSS with CSS variables
- Responsive design
- Accessible UI components

### Backend
- Axum web framework
- Tower middleware for security
- SQLite database
- JWT authentication

## Troubleshooting

### "No servers configured"
- Add servers through the dashboard
- Or provide a router-config.json file

### "Unauthorized" errors
- Check if JWT_SECRET is set
- Ensure session hasn't expired
- Clear browser cookies and re-login

### Server won't start
- Verify the command is in the allowlist
- Check sandbox strategy is available
- Review server logs for errors