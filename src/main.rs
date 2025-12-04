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

fn setup_logging(config: &Config) -> Option<String> {
    if config.logs_disabled() {
        // Initialize a subscriber that discards all logs (prevents library output)
        tracing_subscriber::registry()
            .with(EnvFilter::new("off"))
            .with(fmt::layer().with_writer(std::io::sink))
            .init();
        return None;
    }

    // Build filter that explicitly controls all crates
    // This ensures library logs go through our tracing system, never directly to stdout
    // Can be overridden with RUST_LOG environment variable
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let level = config.log_level();
        EnvFilter::new(format!(
            "authful_mcp_proxy_ng={},\
             reqwest={},\
             reqwest_middleware={},\
             hyper={},\
             tower={},\
             axum={},\
             h2=warn,\
             rustls=warn,\
             tokio=warn",
            level, level, level, level, level, level
        ))
    });

    if config.log_to_file {
        // Log to file with auto-generated filename
        let log_file_path = Config::generate_log_file_path();
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .expect("Failed to create log file");

        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .with_writer(std::sync::Arc::new(file))
                    .with_ansi(false), // Disable colors in file
            )
            .init();

        Some(log_file_path)
    } else {
        // Log to stderr (stdout is for JSON-RPC only)
        // IMPORTANT: All logs MUST go to stderr to avoid polluting stdout
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(std::io::stderr))
            .init();

        None
    }
}

#[tokio::main]
async fn main() {
    let config = Config::parse_args();

    // Set up logging
    let log_file_path = setup_logging(&config);

    // Validate configuration
    if let Err(e) = config.validate() {
        // Write error to stderr explicitly, even if logging is disabled
        use std::io::Write;
        let mut stderr = std::io::stderr();
        let _ = writeln!(stderr, "Configuration error: {}", e);
        let _ = stderr.flush();
        std::process::exit(1);
    }

    // Show banner and info unless silent
    // CRITICAL: Banner and empty line MUST go to stderr, never stdout
    if !config.silent {
        use std::io::Write;
        let mut stderr = std::io::stderr();
        let _ = writeln!(stderr, "{}", BANNER);
        let _ = stderr.flush();

        if let Some(ref log_path) = log_file_path {
            info!("Logging to file: {}", log_path);
        }
        info!("Backend URL: {}", config.backend_url);
        info!("OIDC Issuer: {}", config.oidc_issuer_url);
        info!("Client ID: {}", config.oidc_client_id);
        info!("Scopes: {}", config.scopes().join(" "));
        info!("Redirect URL: {}", config.redirect_url());

        let _ = writeln!(stderr);
        let _ = stderr.flush();
    }

    // Run the proxy
    if let Err(e) = run_proxy(config).await {
        // Write error to stderr explicitly
        use std::io::Write;
        let mut stderr = std::io::stderr();
        let _ = writeln!(stderr, "Proxy error: {}", e);
        let _ = stderr.flush();
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
