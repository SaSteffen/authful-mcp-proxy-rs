//! Configuration parsing and validation

use crate::error::{ProxyError, Result};
use clap::Parser;

const DEFAULT_SCOPES: &str = "openid profile email";
const DEFAULT_REDIRECT_URL: &str = "http://localhost:8080/auth/callback";

#[derive(Parser, Debug, Clone)]
#[command(
    name = "authful-mcp-proxy-ng",
    version,
    about = "Authful Remote-HTTP-to-Local-stdio MCP Proxy",
    long_about = "MCP proxy that bridges remote HTTP MCP servers with OIDC authentication to local stdio transport for MCP clients like Claude Desktop"
)]
pub struct Config {
    /// URL of remote backend MCP server to be proxied
    #[arg(value_name = "MCP_BACKEND_URL", env = "MCP_BACKEND_URL")]
    pub backend_url: String,

    /// OIDC issuer URL (e.g., https://auth.example.com)
    #[arg(long, env = "OIDC_ISSUER_URL")]
    pub oidc_issuer_url: String,

    /// OAuth client ID
    #[arg(long, env = "OIDC_CLIENT_ID")]
    pub oidc_client_id: String,

    /// OAuth client secret (optional for public clients)
    #[arg(long, env = "OIDC_CLIENT_SECRET")]
    pub oidc_client_secret: Option<String>,

    /// Space-separated OAuth scopes (default: "openid profile email")
    #[arg(long, env = "OIDC_SCOPES")]
    pub oidc_scopes: Option<String>,

    /// Localhost URL for OAuth redirect (default: http://localhost:8080/auth/callback)
    #[arg(long, env = "OIDC_REDIRECT_URL")]
    pub oidc_redirect_url: Option<String>,

    /// Don't show the proxy server banner
    #[arg(long)]
    pub no_banner: bool,

    /// Show only error messages
    #[arg(long, conflicts_with = "debug")]
    pub silent: bool,

    /// Enable debug logging
    #[arg(long, env = "MCP_PROXY_DEBUG")]
    pub debug: bool,

    /// Dump all messages to a log file for debugging (format: YYYY-MM-DD_HH-MM-SS_messages.log)
    #[arg(long, env = "MCP_PROXY_DUMP_MESSAGES")]
    pub dump_messages: Option<String>,
}

impl Config {
    /// Parse configuration from CLI arguments and environment variables
    pub fn parse_args() -> Self {
        Config::parse()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.backend_url.is_empty() {
            return Err(ProxyError::Config(
                "MCP backend URL is required".to_string(),
            ));
        }

        if self.oidc_issuer_url.is_empty() {
            return Err(ProxyError::Config(
                "OIDC issuer URL is required".to_string(),
            ));
        }

        if self.oidc_client_id.is_empty() {
            return Err(ProxyError::Config("OIDC client ID is required".to_string()));
        }

        // Validate URLs
        url::Url::parse(&self.backend_url)
            .map_err(|e| ProxyError::Config(format!("Invalid backend URL: {}", e)))?;

        url::Url::parse(&self.oidc_issuer_url)
            .map_err(|e| ProxyError::Config(format!("Invalid OIDC issuer URL: {}", e)))?;

        if let Some(ref redirect_url) = self.oidc_redirect_url {
            url::Url::parse(redirect_url)
                .map_err(|e| ProxyError::Config(format!("Invalid redirect URL: {}", e)))?;
        }

        Ok(())
    }

    /// Get OAuth scopes as a list (with defaults)
    pub fn scopes(&self) -> Vec<String> {
        let scopes_str = self.oidc_scopes.as_deref().unwrap_or(DEFAULT_SCOPES);

        let mut scopes: Vec<String> = scopes_str.split_whitespace().map(String::from).collect();

        // Ensure "openid" scope is always included
        if !scopes.iter().any(|s| s == "openid") {
            scopes.insert(0, "openid".to_string());
        }

        scopes
    }

    /// Get redirect URL (with default)
    pub fn redirect_url(&self) -> String {
        self.oidc_redirect_url
            .clone()
            .unwrap_or_else(|| DEFAULT_REDIRECT_URL.to_string())
    }

    /// Get log level based on flags
    pub fn log_level(&self) -> tracing::Level {
        if self.silent {
            tracing::Level::ERROR
        } else if self.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scopes_with_default() {
        let config = Config {
            backend_url: "https://backend.example.com".to_string(),
            oidc_issuer_url: "https://auth.example.com".to_string(),
            oidc_client_id: "client-id".to_string(),
            oidc_client_secret: None,
            oidc_scopes: None,
            oidc_redirect_url: None,
            no_banner: false,
            silent: false,
            debug: false,
            dump_messages: None,
        };

        let scopes = config.scopes();
        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"profile".to_string()));
        assert!(scopes.contains(&"email".to_string()));
    }

    #[test]
    fn test_scopes_ensures_openid() {
        let config = Config {
            backend_url: "https://backend.example.com".to_string(),
            oidc_issuer_url: "https://auth.example.com".to_string(),
            oidc_client_id: "client-id".to_string(),
            oidc_client_secret: None,
            oidc_scopes: Some("profile email".to_string()),
            oidc_redirect_url: None,
            no_banner: false,
            silent: false,
            debug: false,
            dump_messages: None,
        };

        let scopes = config.scopes();
        assert_eq!(scopes[0], "openid");
        assert!(scopes.contains(&"profile".to_string()));
        assert!(scopes.contains(&"email".to_string()));
    }

    #[test]
    fn test_redirect_url_default() {
        let config = Config {
            backend_url: "https://backend.example.com".to_string(),
            oidc_issuer_url: "https://auth.example.com".to_string(),
            oidc_client_id: "client-id".to_string(),
            oidc_client_secret: None,
            oidc_scopes: None,
            oidc_redirect_url: None,
            no_banner: false,
            silent: false,
            debug: false,
            dump_messages: None,
        };

        assert_eq!(config.redirect_url(), DEFAULT_REDIRECT_URL);
    }
}
