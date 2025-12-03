//! Authful MCP Proxy - Main entry point
//!
//! A Rust implementation of the authful MCP proxy that bridges remote HTTP MCP servers
//! with OIDC authentication to local stdio transport for MCP clients like Claude Desktop.

mod config;
mod error;
mod middleware;
mod oidc;
mod proxy;

use config::Config;
use error::Result;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

const BANNER: &str = r#"
╔══════════════════════════════════════════════════════════════╗
║                   Authful MCP Proxy (Rust)                   ║
║          OIDC-authenticated MCP HTTP-to-stdio bridge         ║
╚══════════════════════════════════════════════════════════════╝
"#;

fn setup_logging(config: &Config) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("{}", config.log_level())));

    // Ensure logs go to stderr, not stdout (stdout is for JSON-RPC only)
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();
}

#[tokio::main]
async fn main() {
    let config = Config::parse_args();

    // Set up logging
    setup_logging(&config);

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration error: {}", e);
        std::process::exit(1);
    }

    // Show banner unless suppressed (write to stderr, not stdout!)
    if !config.no_banner && !config.silent {
        eprintln!("{}", BANNER);
        info!("Backend URL: {}", config.backend_url);
        info!("OIDC Issuer: {}", config.oidc_issuer_url);
        info!("Client ID: {}", config.oidc_client_id);
        info!("Scopes: {}", config.scopes().join(" "));
        info!("Redirect URL: {}", config.redirect_url());
        eprintln!();
    }

    // Run the proxy
    if let Err(e) = run_proxy(config).await {
        error!("Proxy error: {}", e);
        std::process::exit(1);
    }
}

async fn run_proxy(config: Config) -> Result<()> {
    info!("Initializing OIDC client...");

    // Initialize OIDC client
    let oidc_client = oidc::OidcClient::new(
        config.oidc_issuer_url.clone(),
        config.oidc_client_id.clone(),
        config.oidc_client_secret.clone(),
        config.scopes(),
        config.redirect_url(),
    )
    .await?;

    info!("OIDC client initialized");

    // Start MCP proxy server
    let proxy_handle = tokio::spawn({
        let config = config.clone();
        async move { proxy::run_proxy_server(config, oidc_client).await }
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down gracefully...");
        }
        result = proxy_handle => {
            match result {
                Ok(Ok(())) => info!("Proxy server stopped"),
                Ok(Err(e)) => {
                    error!("Proxy server error: {}", e);
                    return Err(e);
                }
                Err(e) => {
                    error!("Proxy server task panicked: {}", e);
                    return Err(error::ProxyError::Mcp(format!("Server task panicked: {}", e)));
                }
            }
        }
    }

    Ok(())
}
