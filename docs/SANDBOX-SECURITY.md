# JauAuth Sandbox Security

JauAuth runs each MCP server in an isolated sandbox environment to prevent malicious or buggy servers from compromising your system.

## Why Sandboxing?

MCP servers can execute arbitrary code and access system resources. Without sandboxing:
- A malicious server could read sensitive files
- A buggy server could consume all system resources
- A compromised server could attack other services

## Available Sandbox Strategies

### 1. Docker (Recommended for Production)
**Security Level: HIGH** 🟢

```json
"sandbox": {
  "strategy": {
    "docker": {
      "image": "node:18-alpine",
      "memory_limit": "256m",
      "cpu_limit": "0.5",
      "extra_flags": ["--network", "none"]
    }
  }
}
```

**Pros:**
- Complete process isolation
- Resource limits enforced by kernel
- Network isolation available
- Works on all platforms

**Cons:**
- Requires Docker daemon
- Slight performance overhead
- Image download on first run

### 2. Podman (Rootless Alternative)
**Security Level: HIGH** 🟢

```json
"sandbox": {
  "strategy": {
    "podman": {
      "image": "node:18-alpine",
      "memory_limit": "256m",
      "cpu_limit": "0.5"
    }
  }
}
```

**Pros:**
- Rootless containers (more secure)
- Docker-compatible
- No daemon required

**Cons:**
- Less common than Docker
- Some features may differ

### 3. Firejail (Linux Lightweight)
**Security Level: MEDIUM** 🟡

```json
"sandbox": {
  "strategy": {
    "firejail": {
      "profile": "default",
      "whitelist_paths": ["/home/user/data"],
      "net": false
    }
  }
}
```

**Pros:**
- Very lightweight
- Fast startup
- Good for trusted code

**Cons:**
- Linux only
- Less isolation than containers
- Requires SUID binary

### 4. Bubblewrap (Flatpak-style)
**Security Level: MEDIUM-HIGH** 🟡

```json
"sandbox": {
  "strategy": {
    "bubblewrap": {
      "ro_binds": [["/usr", "/usr"]],
      "rw_binds": [["/tmp/work", "/tmp"]],
      "share_net": false
    }
  }
}
```

**Pros:**
- User namespaces
- Fine-grained control
- No SUID required

**Cons:**
- Linux only
- Complex configuration
- Requires kernel support

### 5. None (Development Only)
**Security Level: NONE** 🔴

```json
"sandbox": {
  "strategy": "none"
}
```

**WARNING:** Only use for trusted internal tools during development!

## Security Hardening Per Strategy

### Docker Hardening
```json
"extra_flags": [
  "--security-opt", "no-new-privileges",
  "--cap-drop", "ALL",
  "--read-only",
  "--network", "none",
  "--pids-limit", "50",
  "--cpuset-cpus", "0",
  "--memory-swap", "256m"
]
```

### Firejail Hardening
```json
"firejail": {
  "profile": "default",
  "whitelist_paths": [],
  "net": false,
  "extra_args": [
    "--nosound",
    "--no3d",
    "--nodvd",
    "--notv",
    "--nou2f",
    "--novideo",
    "--machine-id",
    "--disable-mnt"
  ]
}
```

## Best Practices

### 1. Principle of Least Privilege
- Only grant necessary permissions
- Disable network if not needed
- Whitelist specific paths only

### 2. Resource Limits
- Always set memory limits
- Set CPU limits for untrusted code
- Limit process/thread counts

### 3. Defense in Depth
- Use sandboxing + authentication
- Monitor resource usage
- Log all operations

## Per-Server Recommendations

### Filesystem Servers
```json
{
  "id": "filesystem",
  "sandbox": {
    "strategy": {
      "firejail": {
        "whitelist_paths": ["/home/user/documents"],
        "net": false
      }
    }
  }
}
```

### Network Services
```json
{
  "id": "web-scraper",
  "sandbox": {
    "strategy": {
      "docker": {
        "image": "node:18-alpine",
        "extra_flags": ["--dns", "8.8.8.8"]
      }
    }
  }
}
```

### Untrusted Code
```json
{
  "id": "community-plugin",
  "sandbox": {
    "strategy": {
      "docker": {
        "image": "alpine:latest",
        "memory_limit": "128m",
        "cpu_limit": "0.25",
        "extra_flags": [
          "--network", "none",
          "--security-opt", "no-new-privileges",
          "--cap-drop", "ALL"
        ]
      }
    }
  }
}
```

## Installation

### Ubuntu/Debian
```bash
# Docker
curl -fsSL https://get.docker.com | sh

# Firejail
sudo apt install firejail

# Bubblewrap
sudo apt install bubblewrap
```

### macOS
```bash
# Docker Desktop
brew install --cask docker

# Note: Firejail/Bubblewrap not available
# Use Docker or Podman on macOS
```

### Check Available Strategies
```bash
cargo run --bin sandbox-check
```

## Troubleshooting

### "Docker not found"
- Install Docker: https://docs.docker.com/get-docker/
- Ensure user is in docker group: `sudo usermod -aG docker $USER`

### "Permission denied" with Firejail
- Check AppArmor: `sudo aa-status`
- Verify firejail SUID: `ls -l /usr/bin/firejail`

### Performance Issues
- Use Firejail for lightweight isolation
- Pre-pull Docker images
- Consider native execution for trusted tools

## Security Audit Checklist

- [ ] All servers have sandbox configured
- [ ] Network disabled for filesystem servers
- [ ] Resource limits set appropriately
- [ ] Untrusted code in Docker/Podman
- [ ] No use of "none" strategy in production
- [ ] Regular security updates for sandbox tools

Remember: **No sandbox is perfect**, but any sandbox is better than none!