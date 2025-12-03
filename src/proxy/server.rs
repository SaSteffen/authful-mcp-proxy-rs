//! MCP proxy server
//!
//! Bridges stdio transport (for MCP clients like Claude Desktop) to HTTP transport
//! (for remote MCP servers with OIDC authentication).

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use reqwest_middleware::ClientBuilder;
use crate::config::Config;
use crate::error::{ProxyError, Result};
use crate::middleware::AuthMiddleware;
use crate::oidc::OidcClient;

/// Run the MCP proxy server
///
/// This function sets up a bidirectional bridge between:
/// - Frontend: stdio (for MCP clients like Claude Desktop)
/// - Backend: HTTP with OIDC authentication (for remote MCP servers)
///
/// MCP messages are JSON-RPC formatted and forwarded transparently between
/// both transports. The OIDC middleware automatically injects bearer tokens
/// and handles 401 responses with token refresh.
pub async fn run_proxy_server(config: Config, oidc_client: OidcClient) -> Result<()> {
    tracing::info!("MCP proxy server starting...");
    tracing::info!("Backend URL: {}", config.backend_url);

    // Create authenticated HTTP client with middleware
    let auth_middleware = AuthMiddleware::new(Arc::new(oidc_client));
    let http_client = ClientBuilder::new(reqwest::Client::new())
        .with(auth_middleware)
        .build();

    tracing::info!("Authenticated HTTP client created");

    // Set up stdio transport (read from stdin, write to stdout)
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);

    tracing::info!("MCP proxy server running on stdio transport");
    tracing::info!("Ready to forward messages between stdio and {}", config.backend_url);

    // Message forwarding loop
    let mut line = String::new();
    loop {
        line.clear();

        // Read JSON-RPC message from stdin
        let bytes_read = reader.read_line(&mut line).await
            .map_err(|e| ProxyError::Mcp(format!("Failed to read from stdin: {}", e)))?;

        // EOF or client disconnect
        if bytes_read == 0 {
            tracing::info!("Client disconnected (EOF on stdin)");
            break;
        }

        let request_line = line.trim();
        if request_line.is_empty() {
            continue;
        }

        tracing::debug!("Received from client: {}", request_line);

        // Validate JSON-RPC format
        if let Err(e) = serde_json::from_str::<serde_json::Value>(request_line) {
            tracing::warn!("Invalid JSON received: {}", e);
            continue;
        }

        // Forward to backend HTTP server
        match http_client
            .post(&config.backend_url)
            .header("Content-Type", "application/json")
            .body(request_line.to_string())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                tracing::debug!("Backend response status: {}", status);

                match response.text().await {
                    Ok(response_body) => {
                        tracing::debug!("Received from backend: {}", response_body);

                        // Write response back to stdout (with newline for JSON-RPC)
                        stdout.write_all(response_body.as_bytes()).await
                            .map_err(|e| ProxyError::Mcp(format!("Failed to write to stdout: {}", e)))?;
                        stdout.write_all(b"\n").await
                            .map_err(|e| ProxyError::Mcp(format!("Failed to write newline to stdout: {}", e)))?;
                        stdout.flush().await
                            .map_err(|e| ProxyError::Mcp(format!("Failed to flush stdout: {}", e)))?;
                    }
                    Err(e) => {
                        tracing::error!("Failed to read backend response body: {}", e);
                        // Send JSON-RPC error response
                        let error_response = serde_json::json!({
                            "jsonrpc": "2.0",
                            "error": {
                                "code": -32603,
                                "message": format!("Backend error: {}", e)
                            },
                            "id": null
                        });
                        stdout.write_all(error_response.to_string().as_bytes()).await
                            .map_err(|e| ProxyError::Mcp(format!("Failed to write error to stdout: {}", e)))?;
                        stdout.write_all(b"\n").await
                            .map_err(|e| ProxyError::Mcp(format!("Failed to write newline to stdout: {}", e)))?;
                        stdout.flush().await
                            .map_err(|e| ProxyError::Mcp(format!("Failed to flush stdout: {}", e)))?;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to forward request to backend: {}", e);
                // Send JSON-RPC error response
                let error_response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32603,
                        "message": format!("Proxy error: {}", e)
                    },
                    "id": null
                });
                stdout.write_all(error_response.to_string().as_bytes()).await
                    .map_err(|e| ProxyError::Mcp(format!("Failed to write error to stdout: {}", e)))?;
                stdout.write_all(b"\n").await
                    .map_err(|e| ProxyError::Mcp(format!("Failed to write newline to stdout: {}", e)))?;
                stdout.flush().await
                    .map_err(|e| ProxyError::Mcp(format!("Failed to flush stdout: {}", e)))?;
            }
        }
    }

    tracing::info!("MCP proxy server stopped");
    Ok(())
}
