//! Error types for the authful MCP proxy

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("OIDC discovery failed: {0}")]
    Discovery(String),

    #[error("Token error: {0}")]
    Token(String),

    #[error("OAuth callback error: {0}")]
    Callback(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("HTTP middleware error: {0}")]
    Middleware(#[from] reqwest_middleware::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Authentication failed: {0}")]
    Auth(String),
}

pub type Result<T> = std::result::Result<T, ProxyError>;
