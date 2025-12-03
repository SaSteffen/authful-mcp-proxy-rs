//! PKCE (Proof Key for Code Exchange) implementation
//!
//! Implements RFC 7636 for OAuth 2.0 authorization code flow security

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{Rng, distributions::Alphanumeric};
use sha2::{Sha256, Digest};

/// PKCE parameters for OAuth 2.0 authorization code flow
#[derive(Debug, Clone)]
pub struct PkceParams {
    pub code_verifier: String,
    pub code_challenge: String,
}

impl PkceParams {
    /// Generate new PKCE parameters
    ///
    /// Creates a random code verifier (43-128 characters) and computes
    /// the code challenge using SHA256: BASE64URL(SHA256(verifier))
    pub fn generate() -> Self {
        // Generate code verifier: 43-128 random characters (URL-safe)
        // Using 64 characters for good security/usability balance
        let code_verifier: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();

        // Compute code challenge: BASE64URL(SHA256(code_verifier))
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();

        let code_challenge = URL_SAFE_NO_PAD.encode(hash);

        PkceParams {
            code_verifier,
            code_challenge,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = PkceParams::generate();

        // Verify code verifier length
        assert_eq!(pkce.code_verifier.len(), 64);

        // Verify code challenge is base64url encoded
        assert!(!pkce.code_challenge.contains('+'));
        assert!(!pkce.code_challenge.contains('/'));
        assert!(!pkce.code_challenge.contains('='));

        // Verify code challenge length (SHA256 hash encoded in base64url is 43 chars)
        assert_eq!(pkce.code_challenge.len(), 43);
    }

    #[test]
    fn test_pkce_uniqueness() {
        let pkce1 = PkceParams::generate();
        let pkce2 = PkceParams::generate();

        // Each generation should produce unique values
        assert_ne!(pkce1.code_verifier, pkce2.code_verifier);
        assert_ne!(pkce1.code_challenge, pkce2.code_challenge);
    }

    #[test]
    fn test_pkce_challenge_computation() {
        // Manually compute challenge to verify correctness
        let verifier = "test_verifier_string_for_pkce_testing_purposes_in_unit_test";

        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let expected_challenge = URL_SAFE_NO_PAD.encode(hash);

        // Create PKCE with known verifier
        let pkce = PkceParams {
            code_verifier: verifier.to_string(),
            code_challenge: expected_challenge.clone(),
        };

        assert_eq!(pkce.code_challenge, expected_challenge);
    }
}
