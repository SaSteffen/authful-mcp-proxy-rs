//! OIDC provider discovery
//!
//! Fetches OIDC configuration from /.well-known/openid-configuration

use serde::{Deserialize, Serialize};
use crate::error::{ProxyError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    #[serde(default)]
    pub userinfo_endpoint: Option<String>,
    #[serde(default)]
    pub jwks_uri: Option<String>,
}

impl OidcConfig {
    /// Discover OIDC configuration from issuer URL
    pub async fn discover(issuer_url: &str) -> Result<Self> {
        let discovery_url = format!("{}/.well-known/openid-configuration", issuer_url.trim_end_matches('/'));

        let client = reqwest::Client::new();
        let response = client
            .get(&discovery_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| ProxyError::Discovery(format!("Failed to fetch OIDC configuration: {}", e)))?;

        if !response.status().is_success() {
            return Err(ProxyError::Discovery(format!(
                "OIDC discovery request failed with status: {}",
                response.status()
            )));
        }

        let config: OidcConfig = response
            .json()
            .await
            .map_err(|e| ProxyError::Discovery(format!("Failed to parse OIDC configuration: {}", e)))?;

        // Validate required endpoints
        if config.authorization_endpoint.is_empty() {
            return Err(ProxyError::Discovery(
                "OIDC configuration missing authorization_endpoint".to_string(),
            ));
        }

        if config.token_endpoint.is_empty() {
            return Err(ProxyError::Discovery(
                "OIDC configuration missing token_endpoint".to_string(),
            ));
        }

        Ok(config)
    }
}
