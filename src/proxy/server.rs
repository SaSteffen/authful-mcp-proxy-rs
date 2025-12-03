//! MCP proxy server
//!
//! Bridges stdio transport (for MCP clients like Claude Desktop) to HTTP transport
//! (for remote MCP servers with OIDC authentication).

use crate::config::Config;
use crate::error::Result;
use crate::oidc::OidcClient;

/// Run the MCP proxy server
pub async fn run_proxy_server(config: Config, _oidc_client: OidcClient) -> Result<()> {
    tracing::info!("MCP proxy server starting...");
    tracing::info!("Backend URL: {}", config.backend_url);
    tracing::info!("Redirect URL: {}", config.redirect_url());

    // TODO: Implement MCP protocol handling
    // TODO: Set up stdio transport for frontend (Claude Desktop)
    // TODO: Set up HTTP transport for backend (with authenticated client)
    // TODO: Forward MCP messages bidirectionally

    tracing::info!("MCP proxy server running on stdio transport");

    // Keep the server running
    tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;

    Ok(())
}
