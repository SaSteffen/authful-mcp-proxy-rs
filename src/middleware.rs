//! HTTP middleware for OIDC token injection and 401 retry logic
//!
//! Implements `reqwest-middleware::Middleware` to automatically inject bearer tokens
//! and handle 401 responses by renewing tokens and retrying.

use crate::oidc::OidcClient;
use async_trait::async_trait;
use http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next, Result as MiddlewareResult};
use std::sync::Arc;
use tracing::{debug, warn};

/// Middleware that injects OIDC bearer tokens and handles 401 responses
pub struct AuthMiddleware {
    oidc_client: Arc<OidcClient>,
}

impl AuthMiddleware {
    /// Create a new auth middleware with the given OIDC client
    pub fn new(oidc_client: Arc<OidcClient>) -> Self {
        Self { oidc_client }
    }
}

#[async_trait]
impl Middleware for AuthMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> MiddlewareResult<Response> {
        // 1. Get access token and inject into Authorization header
        let token = self
            .oidc_client
            .get_token()
            .await
            .map_err(|e| reqwest_middleware::Error::Middleware(e.into()))?;

        req.headers_mut().insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().map_err(|e| {
                reqwest_middleware::Error::Middleware(anyhow::anyhow!("Invalid token: {}", e))
            })?,
        );

        debug!("Injected Authorization header with bearer token");

        // 2. Send the request
        let response = next
            .clone()
            .run(req.try_clone().unwrap(), extensions)
            .await?;

        // 3. Handle 401 Unauthorized - renew token and retry once
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            warn!("Received 401 Unauthorized, renewing token and retrying");

            // Renew token (will refresh or perform full auth flow)
            let new_token = self
                .oidc_client
                .renew_token()
                .await
                .map_err(|e| reqwest_middleware::Error::Middleware(e.into()))?;

            // Update Authorization header with new token
            req.headers_mut().insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", new_token).parse().map_err(|e| {
                    reqwest_middleware::Error::Middleware(anyhow::anyhow!("Invalid token: {}", e))
                })?,
            );

            debug!("Retrying request with renewed token");

            // Retry the request (only once to prevent infinite loops)
            return next.run(req, extensions).await;
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oidc::OidcClient;
    use mockito;
    use reqwest_middleware::ClientBuilder;

    #[tokio::test]
    async fn test_middleware_injects_token() {
        // This is a placeholder test - will be implemented with proper mocking
        // in integration tests
    }

    #[tokio::test]
    async fn test_middleware_retries_on_401() {
        // This is a placeholder test - will be implemented with proper mocking
        // in integration tests
    }
}
