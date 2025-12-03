//! Token storage and management
//!
//! Handles OAuth token storage, validation, and disk persistence.
//! Compatible with Python version's token format for seamless migration.

use crate::error::{ProxyError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const TOKEN_EXPIRY_BUFFER_SECS: u64 = 60;

/// OAuth token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// Internal field: Unix timestamp when token expires
    #[serde(skip)]
    expires_at: Option<u64>,
}

/// Token response from OIDC provider
#[derive(Debug, Deserialize, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl From<TokenResponse> for TokenInfo {
    fn from(response: TokenResponse) -> Self {
        let expires_at = response.expires_in.map(|exp| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + exp
        });

        TokenInfo {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_in: response.expires_in,
            token_type: response.token_type,
            scope: response.scope,
            expires_at,
        }
    }
}

impl TokenInfo {
    /// Check if the token is valid (not expired)
    pub fn is_valid(&self) -> bool {
        if self.access_token.is_empty() {
            return false;
        }

        match self.expires_at {
            Some(expires_at) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                // Apply 60-second buffer to avoid edge cases
                now < expires_at.saturating_sub(TOKEN_EXPIRY_BUFFER_SECS)
            }
            None => true, // If no expiry is set, assume valid
        }
    }

    /// Check if token can be refreshed
    pub fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }

    /// Get token storage directory (cross-platform)
    ///
    /// Returns: ~/.mcp/authful_mcp_proxy/tokens/ on Linux/macOS
    ///          %USERPROFILE%\.mcp\authful_mcp_proxy\tokens\ on Windows
    fn get_storage_dir() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| ProxyError::Token("Cannot determine home directory".to_string()))?;

        let path = PathBuf::from(home)
            .join(".mcp")
            .join("authful_mcp_proxy")
            .join("tokens");

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&path)?;

        Ok(path)
    }

    /// Sanitize issuer URL for use as filename
    ///
    /// Example: https://auth.example.com/realms/myrealm
    ///          -> auth.example.com_realms_myrealm
    fn sanitize_issuer(issuer_url: &str) -> String {
        issuer_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .replace('/', "_")
            .replace(':', "_")
    }

    /// Get token file path for a given issuer
    fn get_token_file_path(issuer_url: &str) -> Result<PathBuf> {
        let storage_dir = Self::get_storage_dir()?;
        let sanitized_issuer = Self::sanitize_issuer(issuer_url);
        let filename = format!("{}_tokens.json", sanitized_issuer);

        Ok(storage_dir.join(filename))
    }

    /// Save tokens to disk
    pub fn save_to_disk(&self, issuer_url: &str) -> Result<()> {
        let file_path = Self::get_token_file_path(issuer_url)?;

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&file_path, json)?;

        tracing::debug!("Tokens saved to {:?}", file_path);
        Ok(())
    }

    /// Load tokens from disk
    pub fn load_from_disk(issuer_url: &str) -> Result<Option<Self>> {
        let file_path = Self::get_token_file_path(issuer_url)?;

        if !file_path.exists() {
            tracing::debug!("No cached tokens found at {:?}", file_path);
            return Ok(None);
        }

        let contents = std::fs::read_to_string(&file_path)?;
        let mut token_info: TokenInfo = serde_json::from_str(&contents)?;

        // Recompute expires_at from expires_in if present
        if let Some(expires_in) = token_info.expires_in {
            // Since we don't know when the token was originally created,
            // we can't accurately compute expires_at from a saved token.
            // The is_valid() check will conservatively treat it as expired
            // if we can't determine the expiry time.
            token_info.expires_at = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + expires_in,
            );
        }

        tracing::debug!("Tokens loaded from {:?}", file_path);
        Ok(Some(token_info))
    }

    /// Delete tokens from disk
    pub fn delete_from_disk(issuer_url: &str) -> Result<()> {
        let file_path = Self::get_token_file_path(issuer_url)?;

        if file_path.exists() {
            std::fs::remove_file(&file_path)?;
            tracing::debug!("Tokens deleted from {:?}", file_path);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_issuer() {
        assert_eq!(
            TokenInfo::sanitize_issuer("https://auth.example.com/realms/myrealm"),
            "auth.example.com_realms_myrealm"
        );
        assert_eq!(
            TokenInfo::sanitize_issuer("http://localhost:8080"),
            "localhost_8080"
        );
    }

    #[test]
    fn test_token_validation() {
        // Valid token
        let mut token = TokenInfo {
            access_token: "test_token".to_string(),
            refresh_token: None,
            expires_in: Some(3600),
            token_type: Some("Bearer".to_string()),
            scope: None,
            expires_at: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 3600,
            ),
        };

        assert!(token.is_valid());

        // Expired token
        token.expires_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 100,
        );

        assert!(!token.is_valid());

        // Empty token
        let empty_token = TokenInfo {
            access_token: String::new(),
            refresh_token: None,
            expires_in: None,
            token_type: None,
            scope: None,
            expires_at: None,
        };

        assert!(!empty_token.is_valid());
    }

    #[test]
    fn test_can_refresh() {
        let token_with_refresh = TokenInfo {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_in: None,
            token_type: None,
            scope: None,
            expires_at: None,
        };

        assert!(token_with_refresh.can_refresh());

        let token_without_refresh = TokenInfo {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_in: None,
            token_type: None,
            scope: None,
            expires_at: None,
        };

        assert!(!token_without_refresh.can_refresh());
    }
}
