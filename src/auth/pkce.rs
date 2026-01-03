//! PKCE (Proof Key for Code Exchange) implementation for OAuth 2.0.
//!
//! PKCE is a security extension to OAuth 2.0 that prevents authorization code
//! interception attacks. It's required by Anthropic's OAuth flow.

use anyhow::Result;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};

/// PKCE verifier and challenge pair.
#[derive(Debug, Clone)]
pub struct Pkce {
    /// Code verifier - a cryptographically random string (43-128 characters).
    pub verifier: String,
    /// Code challenge - base64url(sha256(verifier)).
    pub challenge: String,
}

impl Pkce {
    /// Generate a new PKCE verifier and challenge pair.
    ///
    /// The verifier is a cryptographically random string of 43-128 characters
    /// using unreserved characters [A-Z, a-z, 0-9, -, ., _, ~].
    ///
    /// The challenge is computed as: base64url(sha256(verifier))
    pub fn new() -> Result<Self> {
        // Generate a 43-character verifier (minimum length per RFC 7636)
        let verifier = Self::generate_verifier(43);
        let challenge = Self::compute_challenge(&verifier);

        Ok(Self {
            verifier,
            challenge,
        })
    }

    /// Generate a cryptographically random verifier string.
    fn generate_verifier(length: usize) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
        let mut rng = rand::thread_rng();

        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Compute S256 challenge from verifier: base64url(sha256(verifier))
    fn compute_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();

        URL_SAFE_NO_PAD.encode(hash)
    }
}

impl Default for Pkce {
    fn default() -> Self {
        Self::new().expect("PKCE generation should not fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = Pkce::new().unwrap();

        // Verifier should be 43 characters
        assert_eq!(pkce.verifier.len(), 43);

        // Verifier should only contain valid characters
        assert!(pkce
            .verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._~".contains(c)));

        // Challenge should be base64url encoded (43 chars for SHA-256)
        assert_eq!(pkce.challenge.len(), 43);
        assert!(pkce
            .challenge
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_challenge_is_deterministic() {
        let verifier = "test_verifier_string_for_testing_purposes_abc";
        let challenge1 = Pkce::compute_challenge(verifier);
        let challenge2 = Pkce::compute_challenge(verifier);

        assert_eq!(challenge1, challenge2);
    }

    #[test]
    fn test_different_verifiers_produce_different_challenges() {
        let pkce1 = Pkce::new().unwrap();
        let pkce2 = Pkce::new().unwrap();

        assert_ne!(pkce1.verifier, pkce2.verifier);
        assert_ne!(pkce1.challenge, pkce2.challenge);
    }

    #[test]
    fn test_challenge_format() {
        let verifier = "test";
        let challenge = Pkce::compute_challenge(verifier);

        // SHA-256 hash base64url encoded should be 43 characters
        assert_eq!(challenge.len(), 43);

        // Should not contain padding
        assert!(!challenge.contains('='));

        // Should use URL-safe alphabet
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
    }
}
