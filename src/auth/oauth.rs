//! OAuth 2.0 Device Authorization Grant (RFC 8628) implementation.
//!
//! This module implements the device code flow used by providers like GitHub Copilot.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration as StdDuration;
use tokio::time::sleep;

/// Device code response from the authorization server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCode {
    /// The device verification code.
    pub device_code: String,
    /// The end-user verification code to display to the user.
    pub user_code: String,
    /// The verification URI the user should visit.
    pub verification_uri: String,
    /// Optional direct URI with the code embedded (not all providers support this).
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    /// Lifetime in seconds of the device_code and user_code.
    pub expires_in: u64,
    /// Minimum polling interval in seconds.
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

impl DeviceCode {
    /// Calculate when this device code expires.
    pub fn expires_at(&self) -> DateTime<Utc> {
        Utc::now() + Duration::seconds(self.expires_in as i64)
    }

    /// Check if this device code has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at()
    }
}

/// OAuth token response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    /// The access token.
    pub access_token: String,
    /// Token type (usually "bearer").
    pub token_type: String,
    /// OAuth scope granted.
    #[serde(default)]
    pub scope: Option<String>,
    /// Token lifetime in seconds (if provided).
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// Refresh token (if provided).
    #[serde(default)]
    pub refresh_token: Option<String>,
}

impl OAuthToken {
    /// Calculate when this token expires, if expiry is known.
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_in
            .map(|secs| Utc::now() + Duration::seconds(secs as i64))
    }
}

/// Error response from the authorization server during polling.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

/// Result of polling for the token.
#[derive(Debug)]
pub enum PollResult {
    /// Token received successfully.
    Success(OAuthToken),
    /// Authorization is still pending (user hasn't completed auth yet).
    Pending,
    /// Server requested slower polling.
    SlowDown,
    /// The device code has expired.
    Expired,
    /// User denied the authorization.
    AccessDenied,
    /// Other error.
    Error(String),
}

/// Configuration for a device code flow.
#[derive(Debug, Clone)]
pub struct DeviceCodeConfig {
    /// URL to request device code from.
    pub device_code_url: String,
    /// URL to poll for token.
    pub token_url: String,
    /// OAuth client ID.
    pub client_id: String,
    /// OAuth scope to request.
    pub scope: Option<String>,
}

/// GitHub Copilot OAuth configuration.
impl DeviceCodeConfig {
    /// Create configuration for GitHub Copilot authentication.
    pub fn github_copilot() -> Self {
        Self {
            // VS Code Copilot client ID - this is a well-known public client ID
            client_id: "Iv1.b507a08c87ecfe98".to_string(),
            device_code_url: "https://github.com/login/device/code".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            scope: Some("read:user".to_string()),
        }
    }
}

/// Handler for OAuth device code flow.
pub struct DeviceCodeFlow {
    client: Client,
    config: DeviceCodeConfig,
}

impl DeviceCodeFlow {
    /// Create a new device code flow handler.
    pub fn new(config: DeviceCodeConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Create a handler for GitHub Copilot authentication.
    pub fn github_copilot() -> Self {
        Self::new(DeviceCodeConfig::github_copilot())
    }

    /// Request a device code from the authorization server.
    pub async fn request_device_code(&self) -> Result<DeviceCode> {
        let mut form = vec![("client_id", self.config.client_id.as_str())];
        if let Some(ref scope) = self.config.scope {
            form.push(("scope", scope.as_str()));
        }

        let response = self
            .client
            .post(&self.config.device_code_url)
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await
            .context("Failed to request device code")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Device code request failed ({}): {}",
                status,
                body
            ));
        }

        response
            .json::<DeviceCode>()
            .await
            .context("Failed to parse device code response")
    }

    /// Poll once for the access token.
    pub async fn poll_once(&self, device_code: &str) -> Result<PollResult> {
        let form = [
            ("client_id", self.config.client_id.as_str()),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];

        let response = self
            .client
            .post(&self.config.token_url)
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await
            .context("Failed to poll for token")?;

        let body = response.text().await?;

        // Try to parse as success first
        if let Ok(token) = serde_json::from_str::<OAuthToken>(&body) {
            return Ok(PollResult::Success(token));
        }

        // Try to parse as error
        if let Ok(error) = serde_json::from_str::<OAuthError>(&body) {
            return Ok(match error.error.as_str() {
                "authorization_pending" => PollResult::Pending,
                "slow_down" => PollResult::SlowDown,
                "expired_token" => PollResult::Expired,
                "access_denied" => PollResult::AccessDenied,
                _ => PollResult::Error(
                    error
                        .error_description
                        .unwrap_or_else(|| error.error.clone()),
                ),
            });
        }

        Err(anyhow!("Unexpected response: {}", body))
    }

    /// Poll for the access token until success, error, or timeout.
    ///
    /// Returns the token on success, or an error on failure/timeout.
    pub async fn poll_for_token(
        &self,
        device_code: &DeviceCode,
        mut on_pending: impl FnMut(),
    ) -> Result<OAuthToken> {
        let mut interval = device_code.interval;
        let deadline = device_code.expires_at();

        loop {
            // Check for expiration
            if Utc::now() > deadline {
                return Err(anyhow!("Device code expired"));
            }

            // Wait for the interval
            sleep(StdDuration::from_secs(interval)).await;

            match self.poll_once(&device_code.device_code).await? {
                PollResult::Success(token) => return Ok(token),
                PollResult::Pending => {
                    on_pending();
                    // Continue polling
                }
                PollResult::SlowDown => {
                    // Increase interval by 5 seconds as per spec
                    interval += 5;
                }
                PollResult::Expired => {
                    return Err(anyhow!("Device code expired"));
                }
                PollResult::AccessDenied => {
                    return Err(anyhow!("Authorization denied by user"));
                }
                PollResult::Error(msg) => {
                    return Err(anyhow!("Authorization error: {}", msg));
                }
            }
        }
    }

    /// Open the verification URL in the user's browser.
    pub fn open_browser(url: &str) -> Result<()> {
        open::that(url).context("Failed to open browser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_code_config_github() {
        let config = DeviceCodeConfig::github_copilot();
        assert_eq!(config.client_id, "Iv1.b507a08c87ecfe98");
        assert!(config.device_code_url.contains("github.com"));
        assert!(config.token_url.contains("github.com"));
    }

    #[test]
    fn test_device_code_expiry() {
        let code = DeviceCode {
            device_code: "test".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://example.com".to_string(),
            verification_uri_complete: None,
            expires_in: 900,
            interval: 5,
        };

        assert!(!code.is_expired());
        assert!(code.expires_at() > Utc::now());
    }

    #[test]
    fn test_oauth_token_expiry() {
        let token = OAuthToken {
            access_token: "test".to_string(),
            token_type: "bearer".to_string(),
            scope: None,
            expires_in: Some(3600),
            refresh_token: None,
        };

        let expires = token.expires_at().unwrap();
        assert!(expires > Utc::now());
        assert!(expires < Utc::now() + Duration::hours(2));
    }

    #[test]
    fn test_device_code_deserialization() {
        let json = r#"{
            "device_code": "device123",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        }"#;

        let code: DeviceCode = serde_json::from_str(json).unwrap();
        assert_eq!(code.device_code, "device123");
        assert_eq!(code.user_code, "ABCD-1234");
        assert_eq!(code.expires_in, 900);
    }

    #[test]
    fn test_oauth_token_deserialization() {
        let json = r#"{
            "access_token": "gho_xxxx",
            "token_type": "bearer",
            "scope": "read:user"
        }"#;

        let token: OAuthToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.access_token, "gho_xxxx");
        assert_eq!(token.token_type, "bearer");
        assert_eq!(token.scope, Some("read:user".to_string()));
    }

    #[test]
    fn test_default_interval() {
        let json = r#"{
            "device_code": "test",
            "user_code": "TEST",
            "verification_uri": "https://example.com",
            "expires_in": 100
        }"#;

        let code: DeviceCode = serde_json::from_str(json).unwrap();
        assert_eq!(code.interval, 5);
    }
}
