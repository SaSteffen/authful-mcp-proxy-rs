//! OIDC client implementation
//!
//! Main OIDC client that orchestrates the OAuth 2.0 authorization code flow with PKCE.
//! Manages token lifecycle (cache, refresh, re-authentication).

use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;
use crate::error::{ProxyError, Result};
use super::{OidcConfig, PkceParams, TokenInfo, TokenResponse, callback};

/// OIDC client for managing OAuth 2.0 authentication
pub struct OidcClient {
    issuer_url: String,
    client_id: String,
    client_secret: Option<String>,
    scopes: Vec<String>,
    redirect_url: String,
    oidc_config: OidcConfig,
    token_info: Arc<RwLock<Option<TokenInfo>>>,
}

impl OidcClient {
    /// Create a new OIDC client
    pub async fn new(
        issuer_url: String,
        client_id: String,
        client_secret: Option<String>,
        scopes: Vec<String>,
        redirect_url: String,
    ) -> Result<Self> {
        // Discover OIDC configuration
        let oidc_config = OidcConfig::discover(&issuer_url).await?;

        // Try to load cached tokens
        let token_info = TokenInfo::load_from_disk(&issuer_url)?;

        Ok(Self {
            issuer_url,
            client_id,
            client_secret,
            scopes,
            redirect_url,
            oidc_config,
            token_info: Arc::new(RwLock::new(token_info)),
        })
    }

    /// Get a valid access token (cached, refreshed, or newly authenticated)
    pub async fn get_token(&self) -> Result<String> {
        // Check if we have a valid cached token
        {
            let token_guard = self.token_info.read().await;
            if let Some(ref token) = *token_guard {
                if token.is_valid() {
                    return Ok(token.access_token.clone());
                }
            }
        }

        // Token expired or missing - try to renew
        self.renew_token().await
    }

    /// Renew token (refresh or full auth flow)
    pub async fn renew_token(&self) -> Result<String> {
        // Check if we can refresh
        let can_refresh = {
            let token_guard = self.token_info.read().await;
            token_guard.as_ref().map(|t| t.can_refresh()).unwrap_or(false)
        };

        if can_refresh {
            match self.refresh_access_token().await {
                Ok(token) => return Ok(token),
                Err(e) => {
                    tracing::warn!("Token refresh failed: {}, performing full auth flow", e);
                }
            }
        }

        // Fall back to full auth flow
        self.perform_auth_flow().await
    }

    /// Perform full OAuth 2.0 authorization code flow with PKCE
    async fn perform_auth_flow(&self) -> Result<String> {
        tracing::info!("Starting OAuth 2.0 authorization code flow with PKCE");

        // Generate PKCE parameters and state
        let pkce = PkceParams::generate();
        let state = generate_state();

        // Build authorization URL
        let auth_url = self.build_authorization_url(&state, &pkce)?;

        // Open browser
        tracing::info!("Opening browser for authorization: {}", auth_url);
        if let Err(e) = webbrowser::open(&auth_url) {
            tracing::warn!("Failed to open browser: {}", e);
            eprintln!("\n⚠️  Could not open browser automatically.");
            eprintln!("Please open this URL in your browser:\n\n{}\n", auth_url);
        }

        // Extract port and path from redirect URL
        let redirect_uri = Url::parse(&self.redirect_url)?;
        let port = redirect_uri.port().unwrap_or(8080);
        let path = redirect_uri.path();

        // Run callback server and wait for authorization code
        let callback_result = callback::run_callback_server(port, path).await?;

        // Validate state to prevent CSRF attacks
        if callback_result.state != state {
            return Err(ProxyError::Auth(
                "State mismatch - possible CSRF attack".to_string(),
            ));
        }

        // Exchange authorization code for tokens
        let tokens = self.exchange_code_for_tokens(&callback_result.code, &pkce).await?;

        // Save and cache tokens
        tokens.save_to_disk(&self.issuer_url)?;
        let access_token = tokens.access_token.clone();

        {
            let mut token_guard = self.token_info.write().await;
            *token_guard = Some(tokens);
        }

        tracing::info!("OAuth flow completed successfully");
        Ok(access_token)
    }

    /// Refresh access token using refresh token
    async fn refresh_access_token(&self) -> Result<String> {
        let refresh_token = {
            let token_guard = self.token_info.read().await;
            token_guard
                .as_ref()
                .and_then(|t| t.refresh_token.clone())
                .ok_or_else(|| ProxyError::Token("No refresh token available".to_string()))?
        };

        tracing::debug!("Refreshing access token");

        let client = reqwest::Client::new();
        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
            ("client_id", &self.client_id),
        ];

        // Add client secret if present
        let client_secret_ref;
        if let Some(ref secret) = self.client_secret {
            client_secret_ref = secret.clone();
            params.push(("client_secret", &client_secret_ref));
        }

        let response = client
            .post(&self.oidc_config.token_endpoint)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ProxyError::Token(format!(
                "Token refresh failed with status: {}",
                response.status()
            )));
        }

        let token_response: TokenResponse = response.json().await?;
        let tokens = TokenInfo::from(token_response);

        // Save and cache tokens
        tokens.save_to_disk(&self.issuer_url)?;
        let access_token = tokens.access_token.clone();

        {
            let mut token_guard = self.token_info.write().await;
            *token_guard = Some(tokens);
        }

        tracing::debug!("Access token refreshed successfully");
        Ok(access_token)
    }

    /// Exchange authorization code for tokens
    async fn exchange_code_for_tokens(&self, code: &str, pkce: &PkceParams) -> Result<TokenInfo> {
        let client = reqwest::Client::new();
        let mut params = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.redirect_url),
            ("client_id", &self.client_id),
            ("code_verifier", &pkce.code_verifier),
        ];

        // Add client secret if present
        let client_secret_ref;
        if let Some(ref secret) = self.client_secret {
            client_secret_ref = secret.clone();
            params.push(("client_secret", &client_secret_ref));
        }

        let response = client
            .post(&self.oidc_config.token_endpoint)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProxyError::Token(format!(
                "Token exchange failed with status {}: {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(TokenInfo::from(token_response))
    }

    /// Build authorization URL with PKCE parameters
    fn build_authorization_url(&self, state: &str, pkce: &PkceParams) -> Result<String> {
        let mut url = Url::parse(&self.oidc_config.authorization_endpoint)?;

        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_url)
            .append_pair("scope", &self.scopes.join(" "))
            .append_pair("state", state)
            .append_pair("code_challenge", &pkce.code_challenge)
            .append_pair("code_challenge_method", "S256");

        Ok(url.to_string())
    }
}

/// Generate a random state parameter for CSRF protection
fn generate_state() -> String {
    use rand::Rng;
    use rand::distributions::Alphanumeric;

    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}
