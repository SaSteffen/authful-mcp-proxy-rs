# Authful MCP Proxy (Rust)

A high-performance [Model Context Protocol](https://modelcontextprotocol.com) (MCP) proxy server written in Rust that performs OIDC authentication to obtain access tokens for remote MCP servers protected by token validation, and bridges HTTP transport to local stdio for MCP clients like Claude Desktop.

**Rust rewrite** of the original [Python authful-mcp-proxy](https://github.com/stephaneberle9/authful-mcp-proxy) with:
- 🚀 **Fast startup**: <1 second (vs 2-3 seconds for Python)
- 📦 **Single binary**: 3.3 MB standalone executable (no runtime dependencies)
- 💾 **Low memory**: ~10-20 MB footprint (vs 50-80 MB for Python)
- 🔒 **Type-safe**: Compile-time guarantees with Rust's ownership system
- 🌍 **Cross-platform**: Native binaries for Linux, macOS (Intel & Apple Silicon), and Windows

## Table of Contents

- [Authful MCP Proxy (Rust)](#authful-mcp-proxy-rust)
  - [Table of Contents](#table-of-contents)
  - [What Is This For?](#what-is-this-for)
    - [Technical Background](#technical-background)
  - [Installation](#installation)
    - [Download Prebuilt Binary](#download-prebuilt-binary)
    - [Build from Source](#build-from-source)
  - [Usage](#usage)
    - [Quick Start](#quick-start)
    - [Configuration Options](#configuration-options)
    - [Usage Examples](#usage-examples)
      - [Example 1: Claude Desktop (Recommended)](#example-1-claude-desktop-recommended)
      - [Example 2: With Client Secret (Confidential Client)](#example-2-with-client-secret-confidential-client)
      - [Example 3: Custom Redirect Port](#example-3-custom-redirect-port)
      - [Example 4: Debug Mode](#example-4-debug-mode)
    - [Using with Other MCP Clients](#using-with-other-mcp-clients)
      - [MCP Inspector](#mcp-inspector)
      - [Cursor / Windsurf](#cursor--windsurf)
      - [Command Line / Direct Usage](#command-line--direct-usage)
  - [Credential Management](#credential-management)
    - [Where Are Credentials Stored?](#where-are-credentials-stored)
    - [Clear Cached Credentials](#clear-cached-credentials)
  - [Troubleshooting](#troubleshooting)
    - [Browser Doesn't Open for Authentication](#browser-doesnt-open-for-authentication)
    - [401 Unauthorized Errors](#401-unauthorized-errors)
    - [Redirect URI Mismatch](#redirect-uri-mismatch)
    - [Token Refresh Failures](#token-refresh-failures)
    - [Connection to Backend Fails](#connection-to-backend-fails)
    - [MCP Client Doesn't Recognize the Proxy](#mcp-client-doesnt-recognize-the-proxy)
    - [Debug Logging](#debug-logging)
    - [Still Having Issues?](#still-having-issues)
  - [Migration from Python Version](#migration-from-python-version)
  - [Contributing](#contributing)
  - [License](#license)

## What Is This For?

Use `authful-mcp-proxy-rs` when you need to connect your MCP client (like Claude Desktop, Cursor, or Windsurf) to a remote MCP server that:
- Is protected by OAuth/OIDC token validation
- Doesn't handle authentication itself (no built-in OAuth flows)
- Returns `401 Unauthorized` without proper access tokens

The proxy handles the full OIDC authentication flow, securely stores your credentials in `~/.mcp/authful_mcp_proxy/tokens/`, and automatically refreshes tokens as needed.

### Technical Background

Typically, securing MCP connections with OAuth or OpenID Connect (OIDC) requires "authful" MCP servers that coordinate with external identity providers. MCP clients handle authentication through the MCP server, which in turn interacts with the OAuth or OIDC authorization server. However, this doesn't work with MCP servers only protected by token validation, i.e., MCP servers that trust tokens from a known issuer but don't coordinate with the OAuth/OIDC authorization server themselves. In such scenarios, MCP clients detect the MCP server isn't authful and skip the OAuth/OIDC authentication entirely, resulting in `401 Unauthorized` errors for all tool, resource, and prompt requests.

This MCP proxy fills that gap by handling authentication independently through direct OIDC authorization server interaction. It performs the OAuth authorization code flow by opening the user's browser to the OIDC authorization endpoint for login and scope approval. A temporary local HTTP server receives the OAuth redirect and exchanges the authorization code for access and refresh tokens using PKCE. The access token is used as a Bearer token for all backend MCP server requests and cached locally to avoid repeated browser interactions. When tokens expire, the proxy automatically obtains new ones using the refresh token.

## Installation

### Download Prebuilt Binary

Download the latest release for your platform:

**Linux (x86_64)**:
```bash
# Download and install to /usr/local/bin
curl -L https://github.com/yourusername/authful-mcp-proxy-rs/releases/latest/download/authful-mcp-proxy-rs-linux-x64.tar.gz | tar xz
sudo mv authful-mcp-proxy-rs /usr/local/bin/
chmod +x /usr/local/bin/authful-mcp-proxy-rs
```

**macOS (Intel)**:
```bash
# Download and install
curl -L https://github.com/yourusername/authful-mcp-proxy-rs/releases/latest/download/authful-mcp-proxy-rs-macos-intel.tar.gz | tar xz
sudo mv authful-mcp-proxy-rs /usr/local/bin/
chmod +x /usr/local/bin/authful-mcp-proxy-rs
```

**macOS (Apple Silicon - M1/M2/M3/M4)**:
```bash
# Download and install
curl -L https://github.com/yourusername/authful-mcp-proxy-rs/releases/latest/download/authful-mcp-proxy-rs-macos-arm.tar.gz | tar xz
sudo mv authful-mcp-proxy-rs /usr/local/bin/
chmod +x /usr/local/bin/authful-mcp-proxy-rs
```

**Windows (x64)**:
```powershell
# Download and extract to Program Files
Invoke-WebRequest -Uri "https://github.com/yourusername/authful-mcp-proxy-rs/releases/latest/download/authful-mcp-proxy-rs-windows-x64.zip" -OutFile "mcp-proxy.zip"
Expand-Archive mcp-proxy.zip -DestinationPath "$env:ProgramFiles\authful-mcp-proxy-rs"

# Add to PATH (optional - or use full path in Claude Desktop config)
```

### Build from Source

Requires [Rust](https://rustup.rs/) 1.70 or later:

```bash
# Clone the repository
git clone https://github.com/yourusername/authful-mcp-proxy-rs.git
cd authful-mcp-proxy-rs

# Build release binary
cargo build --release

# Install to /usr/local/bin (Linux/macOS)
sudo cp target/release/authful-mcp-proxy-rs /usr/local/bin/

# Or on Windows, copy to a directory in your PATH
# copy target\release\authful-mcp-proxy-rs.exe C:\Windows\System32\
```

## Usage

### Quick Start

The simplest way to use with MCP clients like Claude Desktop:

**Linux/macOS**:
```jsonc
{
  "mcpServers": {
    "my-protected-server": {
      "command": "/usr/local/bin/authful-mcp-proxy-rs",
      "args": ["https://mcp-backend.company.com/mcp"],
      "env": {
        "OIDC_ISSUER_URL": "https://auth.company.com",
        "OIDC_CLIENT_ID": "your-client-id"
      }
    }
  }
}
```

**Windows**:
```jsonc
{
  "mcpServers": {
    "my-protected-server": {
      "command": "C:\\Program Files\\authful-mcp-proxy-rs\\authful-mcp-proxy-rs.exe",
      "args": ["https://mcp-backend.company.com/mcp"],
      "env": {
        "OIDC_ISSUER_URL": "https://auth.company.com",
        "OIDC_CLIENT_ID": "your-client-id"
      }
    }
  }
}
```

> ℹ️ **Note:** Only two essential OIDC parameters (issuer URL and client ID) must be specified. Other OIDC parameters use sensible defaults (see [Configuration Options](#configuration-options)).

> ⚠️ **Important:** Make sure your OIDC client is configured with `http://localhost:8080/auth/callback` as an allowed redirect URI!

**First Run**: The proxy will open your browser for authentication. After you log in and approve the required scopes, your credentials are cached locally and you won't need to authenticate again until tokens expire.

### Configuration Options

All options can be set via environment variables in the `env` block or passed as CLI arguments.

**Required Configuration:**

| Environment Variable | CLI Flag            | Description                                 | Example                       |
| -------------------- | ------------------- | ------------------------------------------- | ----------------------------- |
| `MCP_BACKEND_URL`    | `<MCP_BACKEND_URL>` | Remote MCP server URL (positional argument) | `https://mcp.example.com/mcp` |
| `OIDC_ISSUER_URL`    | `--oidc-issuer-url` | Your OIDC provider's issuer URL             | `https://auth.example.com`    |
| `OIDC_CLIENT_ID`     | `--oidc-client-id`  | OAuth client ID from your OIDC provider     | `my-app-client-id`            |

**Optional Configuration:**

| Environment Variable | CLI Flag               | Default                               | Description                                   |
| -------------------- | ---------------------- | ------------------------------------- | --------------------------------------------- |
| `OIDC_CLIENT_SECRET` | `--oidc-client-secret` | _(none)_                              | Client secret (not needed for public clients) |
| `OIDC_SCOPES`        | `--oidc-scopes`        | `openid profile email`                | Space-separated OAuth scopes                  |
| `OIDC_REDIRECT_URL`  | `--oidc-redirect-url`  | `http://localhost:8080/auth/callback` | OAuth callback URL                            |

**Advanced Options:**

| CLI Flag      | Description                   |
| ------------- | ----------------------------- |
| `--no-banner` | Suppress the startup banner   |
| `--silent`    | Show only error messages      |
| `--debug`     | Enable detailed debug logging |

Run `authful-mcp-proxy-rs --help` for complete CLI documentation.

### Usage Examples

#### Example 1: Claude Desktop (Recommended)

Add to your Claude Desktop config (accessible via Settings → Developer → Edit Config):

```jsonc
{
  "mcpServers": {
    "company-tools": {
      "command": "/usr/local/bin/authful-mcp-proxy-rs",
      "args": ["https://mcp-backend.company.com/mcp"],
      "env": {
        "OIDC_ISSUER_URL": "https://auth.company.com",
        "OIDC_CLIENT_ID": "claude-desktop-client",
        "OIDC_SCOPES": "openid profile mcp:read mcp:write"
      }
    }
  }
}
```

> ⚠️ **Important:** Make sure your OIDC client is configured with `http://localhost:8080/auth/callback` as an allowed redirect URI!

Restart Claude Desktop to apply changes.

#### Example 2: With Client Secret (Confidential Client)

For OIDC confidential clients requiring a secret:

```jsonc
{
  "mcpServers": {
    "secure-server": {
      "command": "/usr/local/bin/authful-mcp-proxy-rs",
      "args": ["https://api.example.com/mcp"],
      "env": {
        "OIDC_ISSUER_URL": "https://login.example.com",
        "OIDC_CLIENT_ID": "your-confidential-client-id",
        "OIDC_CLIENT_SECRET": "your-client-secret",
        "OIDC_SCOPES": "openid profile email api:access"
      }
    }
  }
}
```

#### Example 3: Custom Redirect Port

If port 8080 is already in use, specify a different port:

```jsonc
{
  "mcpServers": {
    "my-server": {
      "command": "/usr/local/bin/authful-mcp-proxy-rs",
      "args": ["https://mcp.example.com"],
      "env": {
        "OIDC_ISSUER_URL": "https://auth.example.com",
        "OIDC_CLIENT_ID": "my-client-id",
        "OIDC_REDIRECT_URL": "http://localhost:9090/auth/callback"
      }
    }
  }
}
```

> ⚠️ **Important:** Update your OIDC client configuration to allow the custom redirect URL!

#### Example 4: Debug Mode

Enable detailed logging for troubleshooting:

```jsonc
{
  "mcpServers": {
    "debug-server": {
      "command": "/usr/local/bin/authful-mcp-proxy-rs",
      "args": [
        "--debug",
        "https://mcp.example.com"
      ],
      "env": {
        "OIDC_ISSUER_URL": "https://auth.example.com",
        "OIDC_CLIENT_ID": "my-client-id"
      }
    }
  }
}
```

### Using with Other MCP Clients

#### MCP Inspector

Create an `mcp.json` file:

```jsonc
{
  "mcpServers": {
    "authful-mcp-proxy": {
      "command": "/usr/local/bin/authful-mcp-proxy-rs",
      "args": ["https://mcp.example.com/mcp"],
      "env": {
        "OIDC_ISSUER_URL": "https://auth.example.com",
        "OIDC_CLIENT_ID": "inspector-client"
      }
    }
  }
}
```

Start the inspector:
```bash
npx @modelcontextprotocol/inspector --config mcp.json --server authful-mcp-proxy
```

#### Cursor / Windsurf

These editors use the same configuration format as Claude Desktop. Add the server config to your MCP settings file with the appropriate binary path.

#### Command Line / Direct Usage

```bash
# Run directly
authful-mcp-proxy-rs \
  --oidc-issuer-url https://auth.example.com \
  --oidc-client-id my-client \
  https://mcp.example.com/mcp

# Or use environment variables
export OIDC_ISSUER_URL=https://auth.example.com
export OIDC_CLIENT_ID=my-client
authful-mcp-proxy-rs https://mcp.example.com/mcp
```

## Credential Management

### Where Are Credentials Stored?

Credentials are cached in `~/.mcp/authful_mcp_proxy/tokens/` (same location as Python version for compatibility) with filenames based on the OIDC issuer URL:

```
~/.mcp/authful_mcp_proxy/tokens/
  └── auth.example.com_realms_myrealm_tokens.json
```

**Windows**: `%USERPROFILE%\.mcp\authful_mcp_proxy\tokens\`

### Clear Cached Credentials

To force re-authentication (e.g., to switch accounts or clear expired tokens):

**Linux/macOS**:
```bash
rm -rf ~/.mcp/authful_mcp_proxy/tokens/
```

**Windows**:
```powershell
rmdir /s %USERPROFILE%\.mcp\authful_mcp_proxy\tokens
```

The next time you connect, you'll be prompted to authenticate again.

## Troubleshooting

### Browser Doesn't Open for Authentication

**Problem:** The proxy starts but no browser window opens.

**Solutions:**
1. Check that port 8080 (or your custom redirect port) isn't blocked
2. Manually open the URL shown in the proxy logs
3. Verify your firewall isn't blocking localhost connections

### 401 Unauthorized Errors

**Problem:** Backend MCP server returns 401 errors.

**Solutions:**
1. Verify `OIDC_ISSUER_URL` matches your provider exactly
2. Check that `OIDC_CLIENT_ID` is correct
3. Ensure requested scopes are granted by the authorization server
4. Clear cached credentials and re-authenticate: `rm -rf ~/.mcp/authful_mcp_proxy/tokens/`
5. Enable debug mode to see token details: `--debug`

### Redirect URI Mismatch

**Problem:** OIDC provider shows "redirect_uri mismatch" error.

**Solutions:**
1. Add `http://localhost:8080/auth/callback` to your OIDC client's allowed redirect URIs
2. If using a custom port, update both the proxy config (`OIDC_REDIRECT_URL`) and OIDC client settings
3. Ensure the redirect URI matches exactly (including trailing slashes)

### Token Refresh Failures

**Problem:** Proxy works initially but fails after some time.

**Solutions:**
1. Check if your OIDC provider issued a refresh token (some providers don't for certain grant types)
2. Verify the `offline_access` scope is requested if required by your provider
3. Clear cached credentials to get new tokens: `rm -rf ~/.mcp/authful_mcp_proxy/tokens/`

### Connection to Backend Fails

**Problem:** Can't connect to remote MCP server.

**Solutions:**
1. Verify the backend URL is correct and accessible
2. Check network connectivity to the backend server
3. Ensure the backend server is running and accepting connections
4. Try accessing the backend URL directly in a browser to verify it's reachable
5. Check for proxy/VPN issues that might block the connection

### MCP Client Doesn't Recognize the Proxy

**Problem:** Claude Desktop or other client shows error about the server.

**Solutions:**
1. Verify JSON syntax is correct (no trailing commas, proper quotes)
2. Check that the binary path is correct and the file is executable
3. Restart your MCP client completely (not just refresh)
4. Review client logs for specific error messages

### Debug Logging

Enable debug mode to see detailed information about the authentication flow:

```bash
authful-mcp-proxy-rs --debug https://mcp.example.com/mcp
```

Or via environment variable:
```jsonc
{
  "env": {
    "MCP_PROXY_DEBUG": "1",
    // ... other config
  }
}
```

### Still Having Issues?

1. Run with `--debug` to get detailed logs
2. Verify your OIDC provider configuration
3. Open an issue on GitHub with debug logs (redact sensitive information)

## Migration from Python Version

The Rust version is fully compatible with the Python version:

✅ **Token storage**: Uses the same file format and location (`~/.mcp/authful_mcp_proxy/tokens/`)
✅ **CLI arguments**: Identical argument names and behavior
✅ **Environment variables**: Same variable names (OIDC_*, MCP_*)
✅ **OIDC flow**: Same OAuth 2.0 authorization code + PKCE flow
✅ **Client configuration**: Drop-in replacement in Claude Desktop config

**Migration steps:**
1. Install the Rust binary (see [Installation](#installation))
2. Update your Claude Desktop config to use the binary path instead of `uvx authful-mcp-proxy`
3. Restart Claude Desktop - your existing cached tokens will work automatically!

**Performance improvements:**
- 🚀 **Faster startup**: <1 second vs 2-3 seconds
- 💾 **Lower memory**: ~10-20 MB vs 50-80 MB
- 📦 **Smaller footprint**: 3.3 MB binary vs 50+ MB with Python + dependencies

## Contributing

Contributions are welcome! This is a Rust rewrite with feature parity to the original Python version.

**Development:**
```bash
# Clone and build
git clone https://github.com/yourusername/authful-mcp-proxy-rs.git
cd authful-mcp-proxy-rs
cargo build

# Run tests
cargo test

# Run with example backend
cargo run -- \
  --oidc-issuer-url https://auth.example.com \
  --oidc-client-id test-client \
  https://mcp.example.com/mcp
```

**Project Structure:**
- `src/config.rs` - CLI and configuration parsing
- `src/oidc/` - OIDC client implementation (discovery, PKCE, tokens, callback)
- `src/middleware.rs` - HTTP middleware for token injection and 401 retry
- `src/proxy/` - MCP proxy server (stdio ↔ HTTP bridge)
- `tests/` - Integration and unit tests

**Cross-platform testing:**
This project aims for first-class support on Linux, macOS, and Windows. Please test changes on all platforms before submitting PRs.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

This is a Rust rewrite of the original [Python authful-mcp-proxy](https://github.com/yourusername/authful-mcp-proxy).
