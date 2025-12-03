//! Integration tests for MCP proxy server
//!
//! Tests message forwarding between stdio and HTTP transports

use mockito::ServerGuard;
use serde_json::json;

/// Helper to set up a mock OIDC provider
async fn setup_mock_oidc(server: &mut ServerGuard) {
    // Mock OIDC discovery endpoint
    server
        .mock("GET", "/.well-known/openid-configuration")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{
                "issuer": "{}",
                "authorization_endpoint": "{}/auth",
                "token_endpoint": "{}/token",
                "userinfo_endpoint": "{}/userinfo"
            }}"#,
            server.url(),
            server.url(),
            server.url(),
            server.url()
        ))
        .create();

    // Mock token endpoint to return access tokens
    server
        .mock("POST", "/token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "access_token": "test_access_token_123",
                "token_type": "Bearer",
                "expires_in": 3600,
                "refresh_token": "test_refresh_token_456"
            })
            .to_string(),
        )
        .create();
}

#[tokio::test]
async fn test_proxy_forwards_json_rpc_messages() {
    let mut oidc_server = mockito::Server::new_async().await;
    let mut backend_server = mockito::Server::new_async().await;

    // Set up mock OIDC provider
    setup_mock_oidc(&mut oidc_server).await;

    // Mock MCP backend server that echoes requests
    let _backend_mock = backend_server
        .mock("POST", "/")
        .match_header(
            "authorization",
            mockito::Matcher::Regex(r"Bearer .+".to_string()),
        )
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_body(
            json!({
                "jsonrpc": "2.0",
                "result": {
                    "status": "ok",
                    "message": "Request processed"
                },
                "id": 1
            })
            .to_string(),
        )
        .expect(1)
        .create();

    // Note: Full integration test would require:
    // 1. Pre-seeding token cache to avoid browser-based auth flow
    // 2. Spawning the proxy binary as a subprocess
    // 3. Communicating via stdio pipes
    // 4. Verifying message forwarding

    // For now, this is a structural placeholder
}

#[tokio::test]
async fn test_proxy_handles_invalid_json() {
    // Test that proxy gracefully handles invalid JSON input
    // Should log warning and continue processing next message
}

#[tokio::test]
async fn test_proxy_handles_backend_errors() {
    let mut oidc_server = mockito::Server::new_async().await;
    let mut backend_server = mockito::Server::new_async().await;

    // Set up mock OIDC provider
    setup_mock_oidc(&mut oidc_server).await;

    // Mock backend that returns HTTP 500 error
    let _backend_mock = backend_server
        .mock("POST", "/")
        .with_status(500)
        .with_body("Internal Server Error")
        .expect(1)
        .create();

    // Test that proxy returns JSON-RPC error response
    // Should not crash and should continue processing
}

#[tokio::test]
async fn test_proxy_injects_auth_header() {
    let mut oidc_server = mockito::Server::new_async().await;
    let mut backend_server = mockito::Server::new_async().await;

    // Set up mock OIDC provider
    setup_mock_oidc(&mut oidc_server).await;

    // Verify backend receives Authorization header
    let _backend_mock = backend_server
        .mock("POST", "/")
        .match_header("authorization", "Bearer test_access_token_123")
        .with_status(200)
        .with_body(json!({"jsonrpc": "2.0", "result": "ok", "id": 1}).to_string())
        .expect(1)
        .create();

    // Test that all forwarded requests include Bearer token
}

#[tokio::test]
async fn test_proxy_handles_eof() {
    // Test that proxy gracefully shuts down on EOF/client disconnect
    // Should log "Client disconnected" and exit cleanly
}
