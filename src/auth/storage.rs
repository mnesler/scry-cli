//! Credential storage for API keys and OAuth tokens.
//!
//! Stores credentials in `~/.local/share/scry-cli/auth.json` following
//! the XDG Base Directory Specification.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Storage for authentication credentials.
///
/// Credentials are stored per-provider (e.g., "anthropic", "github_copilot").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthStorage {
    /// Map of provider name to credential.
    pub credentials: HashMap<String, Credential>,
}

/// A stored credential, either an API key or OAuth tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Credential {
    /// A simple API key.
    #[serde(rename = "api_key")]
    ApiKey {
        /// The API key value.
        key: String,
    },

    /// OAuth tokens with optional refresh capability.
    #[serde(rename = "oauth")]
    OAuth {
        /// The access token for API calls.
        access_token: String,
        /// Optional refresh token for obtaining new access tokens.
        refresh_token: Option<String>,
        /// When the access token expires.
        expires_at: Option<DateTime<Utc>>,
    },
}

impl Credential {
    /// Create a new API key credential.
    pub fn api_key(key: impl Into<String>) -> Self {
        Self::ApiKey { key: key.into() }
    }

    /// Create a new OAuth credential.
    pub fn oauth(
        access_token: impl Into<String>,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self::OAuth {
            access_token: access_token.into(),
            refresh_token,
            expires_at,
        }
    }

    /// Get the token/key value for API requests.
    pub fn token(&self) -> &str {
        match self {
            Self::ApiKey { key } => key,
            Self::OAuth { access_token, .. } => access_token,
        }
    }

    /// Check if this credential is expired.
    ///
    /// Returns `false` for API keys (never expire) or OAuth tokens without expiry.
    pub fn is_expired(&self) -> bool {
        match self {
            Self::ApiKey { .. } => false,
            Self::OAuth { expires_at, .. } => {
                expires_at.map(|exp| exp < Utc::now()).unwrap_or(false)
            }
        }
    }

    /// Check if this credential needs refresh (expired or expiring soon).
    ///
    /// Returns true if the token expires within 5 minutes.
    pub fn needs_refresh(&self) -> bool {
        match self {
            Self::ApiKey { .. } => false,
            Self::OAuth { expires_at, refresh_token, .. } => {
                // Can't refresh without a refresh token
                if refresh_token.is_none() {
                    return false;
                }
                expires_at
                    .map(|exp| exp < Utc::now() + chrono::Duration::minutes(5))
                    .unwrap_or(false)
            }
        }
    }
}

impl AuthStorage {
    /// Get the default storage path.
    ///
    /// Returns `~/.local/share/scry-cli/auth.json` on Linux/macOS.
    pub fn default_path() -> Result<PathBuf> {
        let data_dir = dirs::data_local_dir()
            .context("Could not determine local data directory")?;
        Ok(data_dir.join("scry-cli").join("auth.json"))
    }

    /// Load credentials from the default storage path.
    ///
    /// Returns an empty storage if the file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::default_path()?;
        Self::load_from(&path)
    }

    /// Load credentials from a specific path.
    ///
    /// Returns an empty storage if the file doesn't exist.
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let file = File::open(path)
            .with_context(|| format!("Failed to open auth file: {}", path.display()))?;
        let reader = BufReader::new(file);

        serde_json::from_reader(reader)
            .with_context(|| format!("Failed to parse auth file: {}", path.display()))
    }

    /// Save credentials to the default storage path.
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path()?;
        self.save_to(&path)
    }

    /// Save credentials to a specific path.
    ///
    /// Creates parent directories if needed and sets file permissions to 0600.
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Write to file
        let file = File::create(path)
            .with_context(|| format!("Failed to create auth file: {}", path.display()))?;

        // Set permissions to 0600 (owner read/write only) on Unix
        #[cfg(unix)]
        {
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(0o600);
            file.set_permissions(perms)?;
        }

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)
            .with_context(|| format!("Failed to write auth file: {}", path.display()))?;

        Ok(())
    }

    /// Get a credential for a provider.
    pub fn get(&self, provider: &str) -> Option<&Credential> {
        self.credentials.get(provider)
    }

    /// Set a credential for a provider.
    pub fn set(&mut self, provider: impl Into<String>, credential: Credential) {
        self.credentials.insert(provider.into(), credential);
    }

    /// Remove a credential for a provider.
    pub fn remove(&mut self, provider: &str) -> Option<Credential> {
        self.credentials.remove(provider)
    }

    /// Check if a provider has stored credentials.
    pub fn has(&self, provider: &str) -> bool {
        self.credentials.contains_key(provider)
    }

    /// Get a valid (non-expired) token for a provider.
    ///
    /// Returns `None` if no credential exists or if it's expired.
    pub fn get_valid_token(&self, provider: &str) -> Option<&str> {
        self.get(provider).and_then(|cred| {
            if cred.is_expired() {
                None
            } else {
                Some(cred.token())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_credential_api_key() {
        let cred = Credential::api_key("sk-test-123");
        assert_eq!(cred.token(), "sk-test-123");
        assert!(!cred.is_expired());
        assert!(!cred.needs_refresh());
    }

    #[test]
    fn test_credential_oauth() {
        let cred = Credential::oauth("access-token", Some("refresh-token".to_string()), None);
        assert_eq!(cred.token(), "access-token");
        assert!(!cred.is_expired());
    }

    #[test]
    fn test_credential_oauth_expired() {
        let expired = Utc::now() - chrono::Duration::hours(1);
        let cred = Credential::oauth("access-token", Some("refresh".to_string()), Some(expired));
        assert!(cred.is_expired());
        assert!(cred.needs_refresh());
    }

    #[test]
    fn test_credential_oauth_not_expired() {
        let future = Utc::now() + chrono::Duration::hours(1);
        let cred = Credential::oauth("access-token", None, Some(future));
        assert!(!cred.is_expired());
    }

    #[test]
    fn test_auth_storage_default() {
        let storage = AuthStorage::default();
        assert!(storage.credentials.is_empty());
    }

    #[test]
    fn test_auth_storage_set_get() {
        let mut storage = AuthStorage::default();
        storage.set("anthropic", Credential::api_key("sk-ant-123"));

        let cred = storage.get("anthropic").unwrap();
        assert_eq!(cred.token(), "sk-ant-123");
    }

    #[test]
    fn test_auth_storage_remove() {
        let mut storage = AuthStorage::default();
        storage.set("anthropic", Credential::api_key("sk-ant-123"));
        assert!(storage.has("anthropic"));

        storage.remove("anthropic");
        assert!(!storage.has("anthropic"));
    }

    #[test]
    fn test_auth_storage_get_valid_token() {
        let mut storage = AuthStorage::default();
        storage.set("anthropic", Credential::api_key("sk-ant-123"));

        assert_eq!(storage.get_valid_token("anthropic"), Some("sk-ant-123"));
        assert_eq!(storage.get_valid_token("unknown"), None);
    }

    #[test]
    fn test_auth_storage_get_valid_token_expired() {
        let mut storage = AuthStorage::default();
        let expired = Utc::now() - chrono::Duration::hours(1);
        storage.set(
            "copilot",
            Credential::oauth("token", None, Some(expired)),
        );

        assert_eq!(storage.get_valid_token("copilot"), None);
    }

    #[test]
    fn test_auth_storage_save_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("auth.json");

        let mut storage = AuthStorage::default();
        storage.set("anthropic", Credential::api_key("sk-ant-123"));
        storage.set(
            "github_copilot",
            Credential::oauth("gho_token", Some("refresh".to_string()), None),
        );

        storage.save_to(&path).unwrap();

        let loaded = AuthStorage::load_from(&path).unwrap();
        assert!(loaded.has("anthropic"));
        assert!(loaded.has("github_copilot"));
        assert_eq!(loaded.get("anthropic").unwrap().token(), "sk-ant-123");
    }

    #[test]
    fn test_auth_storage_load_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.json");

        let storage = AuthStorage::load_from(&path).unwrap();
        assert!(storage.credentials.is_empty());
    }

    #[test]
    fn test_credential_serialization() {
        let cred = Credential::api_key("test-key");
        let json = serde_json::to_string(&cred).unwrap();
        assert!(json.contains("\"type\":\"api_key\""));
        assert!(json.contains("\"key\":\"test-key\""));
    }

    #[test]
    fn test_oauth_credential_serialization() {
        let expires = DateTime::parse_from_rfc3339("2025-01-03T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let cred = Credential::oauth("access", Some("refresh".to_string()), Some(expires));
        let json = serde_json::to_string(&cred).unwrap();
        assert!(json.contains("\"type\":\"oauth\""));
        assert!(json.contains("\"access_token\":\"access\""));
        assert!(json.contains("\"refresh_token\":\"refresh\""));
    }

    #[cfg(unix)]
    #[test]
    fn test_auth_storage_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("auth.json");

        let storage = AuthStorage::default();
        storage.save_to(&path).unwrap();

        let metadata = std::fs::metadata(&path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "File should have 0600 permissions");
    }
}
