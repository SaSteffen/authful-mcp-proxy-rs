//! OAuth callback server
//!
//! Temporary HTTP server that handles the OAuth authorization callback.
//! Runs on localhost and receives the authorization code from the OIDC provider.

use crate::error::{ProxyError, Result};
use axum::{
    extract::Query,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::sync::oneshot;

const CALLBACK_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

pub struct CallbackResult {
    pub code: String,
    pub state: String,
}

/// Run OAuth callback server and wait for authorization code
pub async fn run_callback_server(port: u16, path: &str) -> Result<CallbackResult> {
    let (tx, rx) = oneshot::channel::<Result<CallbackResult>>();

    // Wrap sender in Arc<Mutex> so it can be shared with the handler
    let tx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx)));

    // Create the callback handler
    let callback_path = path.to_string();
    let app =
        Router::new().route(
            &callback_path,
            get({
                let tx = tx.clone();
                move |Query(params): Query<CallbackQuery>| async move {
                    handle_callback(params, tx).await
                }
            }),
        );

    // Bind to localhost on the specified port
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    tracing::info!(
        "OAuth callback server listening on http://{}{}",
        addr,
        callback_path
    );

    // Start server with graceful shutdown
    let server = axum::serve(tokio::net::TcpListener::bind(addr).await?, app);

    // Run server in background and wait for callback with timeout
    tokio::select! {
        result = rx => {
            result.map_err(|_| ProxyError::Callback("Callback channel closed".to_string()))?
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(CALLBACK_TIMEOUT_SECS)) => {
            Err(ProxyError::Timeout(format!(
                "OAuth callback timed out after {} seconds",
                CALLBACK_TIMEOUT_SECS
            )))
        }
        server_result = server => {
            server_result?;
            Err(ProxyError::Callback("Server stopped unexpectedly".to_string()))
        }
    }
}

async fn handle_callback(
    params: CallbackQuery,
    tx: std::sync::Arc<tokio::sync::Mutex<Option<oneshot::Sender<Result<CallbackResult>>>>>,
) -> impl IntoResponse {
    // Check for errors first
    if let Some(error) = params.error {
        let description = params
            .error_description
            .unwrap_or_else(|| "No description provided".to_string());

        let error_msg = format!("OAuth error: {} - {}", error, description);

        // Send error through channel
        if let Some(sender) = tx.lock().await.take() {
            let _ = sender.send(Err(ProxyError::Callback(error_msg.clone())));
        }

        return Html(format!(
            r#"
            <html>
                <head><title>Authentication Failed</title></head>
                <body>
                    <h1>Authentication Failed</h1>
                    <p>{}</p>
                    <p>You can close this window.</p>
                </body>
            </html>
            "#,
            error_msg
        ));
    }

    // Extract code and state
    match (params.code, params.state) {
        (Some(code), Some(state)) => {
            let result = CallbackResult { code, state };

            // Send result through channel
            if let Some(sender) = tx.lock().await.take() {
                let _ = sender.send(Ok(result));
            }

            Html(
                r#"
                <html>
                    <head><title>Authentication Successful</title></head>
                    <body>
                        <h1>Authentication Successful!</h1>
                        <p>You have been successfully authenticated.</p>
                        <p>You can close this window and return to your application.</p>
                    </body>
                </html>
                "#
                .to_string(),
            )
        }
        _ => {
            let error_msg = "Missing code or state parameter in callback";

            // Send error through channel
            if let Some(sender) = tx.lock().await.take() {
                let _ = sender.send(Err(ProxyError::Callback(error_msg.to_string())));
            }

            Html(format!(
                r#"
                <html>
                    <head><title>Authentication Failed</title></head>
                    <body>
                        <h1>Authentication Failed</h1>
                        <p>{}</p>
                        <p>You can close this window.</p>
                    </body>
                </html>
                "#,
                error_msg
            ))
        }
    }
}
