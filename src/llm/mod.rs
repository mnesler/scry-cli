//! LLM client module for API interactions.
//!
//! This module provides a unified interface for interacting with LLM providers.
//! Currently supports:
//! - Anthropic (Claude)
//! - GitHub Copilot
//! - Ollama (local models)
//! - OpenRouter (multi-model access)

mod anthropic;
mod copilot;
mod ollama;
mod openrouter;
mod provider;

pub use provider::{LlmProvider, ProviderError, ProviderResult};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

pub use anthropic::AnthropicClient;
pub use copilot::CopilotProvider;
pub use ollama::OllamaProvider;
pub use openrouter::OpenRouterProvider;

/// Available models for GitHub Copilot.
///
/// Each tuple contains (display_name, api_model_id).
/// These are the models available through GitHub Copilot Chat API.
pub const COPILOT_MODELS: &[(&str, &str)] = &[
    // Claude models (Anthropic)
    ("Claude Sonnet 4.5", "claude-sonnet-4.5"),
    ("Claude Sonnet 4", "claude-sonnet-4"),
    ("Claude Opus 4.5", "claude-opus-4.5"),
    ("Claude Opus 4.1", "claude-opus-4.1"),
    ("Claude Haiku 4.5", "claude-haiku-4.5"),
    // GPT models (OpenAI)
    ("GPT-5.2", "gpt-5.2"),
    ("GPT-5.1-Codex-Max", "gpt-5.1-codex-max"),
    ("GPT-5.1-Codex", "gpt-5.1-codex"),
    ("GPT-5.1-Codex-Mini", "gpt-5.1-codex-mini"),
    ("GPT-5.1", "gpt-5.1"),
    ("GPT-5-Codex", "gpt-5-codex"),
    ("GPT-5 mini", "gpt-5-mini"),
    ("GPT-5", "gpt-5"),
    ("GPT-4.1", "gpt-4.1"),
    // Gemini models (Google)
    ("Gemini 3 Pro", "gemini-3-pro"),
    ("Gemini 3 Flash", "gemini-3-flash"),
    ("Gemini 2.5 Pro", "gemini-2.5-pro"),
    // Other
    ("Grok Code Fast 1", "grok-code-fast-1"),
    ("Raptor mini", "raptor-mini"),
];

/// Supported LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Anthropic,
    Ollama,
    OpenRouter,
    GitHubCopilot,
}

impl Provider {
    /// Returns all available providers in display order.
    pub const fn all() -> &'static [Provider] {
        &[
            Provider::Anthropic,
            Provider::GitHubCopilot,
            Provider::OpenRouter,
            Provider::Ollama,
        ]
    }

    /// Get the display name for this provider.
    pub const fn display_name(&self) -> &'static str {
        match self {
            Provider::Anthropic => "Anthropic",
            Provider::GitHubCopilot => "GitHub Copilot",
            Provider::OpenRouter => "OpenRouter",
            Provider::Ollama => "Ollama (Local)",
        }
    }

    /// Get the default API base URL for this provider.
    pub fn default_api_base(&self) -> &'static str {
        match self {
            Provider::Anthropic => "https://api.anthropic.com/v1",
            Provider::Ollama => "http://localhost:11434",
            Provider::OpenRouter => "https://openrouter.ai/api/v1",
            Provider::GitHubCopilot => "https://api.githubcopilot.com",
        }
    }

    /// Get the default model for this provider.
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-sonnet-4-5",
            Provider::Ollama => "qwen3:4b",
            Provider::OpenRouter => "anthropic/claude-sonnet-4-5",
            Provider::GitHubCopilot => "claude-sonnet-4.5",
        }
    }

    /// Get the environment variable name for the API key.
    pub fn env_var_name(&self) -> &'static str {
        match self {
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::Ollama => "", // No API key needed for local Ollama
            Provider::OpenRouter => "OPENROUTER_API_KEY",
            Provider::GitHubCopilot => "GITHUB_COPILOT_TOKEN",
        }
    }

    /// Check if this provider requires an API key.
    pub const fn requires_api_key(&self) -> bool {
        match self {
            Provider::Anthropic => false, // Uses OAuth
            Provider::Ollama => false,
            Provider::OpenRouter => true,
            Provider::GitHubCopilot => false, // Uses OAuth
        }
    }

    /// Check if this provider uses OAuth device flow.
    pub const fn uses_oauth(&self) -> bool {
        matches!(self, Provider::Anthropic | Provider::GitHubCopilot)
    }

    /// Get the storage key used in AuthStorage.
    ///
    /// This key is used to store and retrieve credentials from `auth.json`.
    pub const fn storage_key(&self) -> &'static str {
        match self {
            Provider::Anthropic => "anthropic",
            Provider::Ollama => "ollama",
            Provider::OpenRouter => "openrouter",
            Provider::GitHubCopilot => "github_copilot",
        }
    }

    /// Get the URL where users can create API keys for this provider.
    ///
    /// Returns `None` for providers that don't use API keys (e.g., Ollama, OAuth providers).
    pub const fn api_key_url(&self) -> Option<&'static str> {
        match self {
            Provider::Anthropic => None,    // Uses OAuth, not API keys
            Provider::OpenRouter => Some("https://openrouter.ai/keys"),
            Provider::Ollama => None,       // Local, no API key needed
            Provider::GitHubCopilot => None, // Uses OAuth, not API keys
        }
    }

    /// Validate the format of an API key for this provider.
    ///
    /// Returns `Ok(())` if the format is valid, or an error message describing the issue.
    /// This only validates the format, not whether the key is actually valid.
    pub fn validate_api_key_format(&self, key: &str) -> Result<(), &'static str> {
        if key.is_empty() {
            return Err("API key cannot be empty");
        }

        match self {
            Provider::Anthropic => {
                // Anthropic keys start with "sk-ant-"
                if !key.starts_with("sk-ant-") {
                    return Err("Anthropic keys must start with 'sk-ant-'");
                }
                if key.len() < 20 {
                    return Err("API key is too short");
                }
                Ok(())
            }
            Provider::OpenRouter => {
                // OpenRouter keys start with "sk-or-"
                if !key.starts_with("sk-or-") {
                    return Err("OpenRouter keys must start with 'sk-or-'");
                }
                if key.len() < 20 {
                    return Err("API key is too short");
                }
                Ok(())
            }
            Provider::Ollama => {
                // Ollama doesn't need an API key
                Err("Ollama does not require an API key")
            }
            Provider::GitHubCopilot => {
                // Copilot uses OAuth, not API keys
                Err("GitHub Copilot uses OAuth, not API keys")
            }
        }
    }
}

/// Validate an API key by making a minimal request to the provider.
///
/// This function makes a real API request to verify the key is valid.
/// Returns `Ok(())` if the key is valid (including rate limited - 429).
/// Returns an error message if the key is invalid or the request fails.
///
/// # Note
/// This may incur minimal API costs (~1 token for most providers).
pub async fn validate_api_key(provider: Provider, api_key: &str) -> Result<(), String> {
    use reqwest::Client;

    // Handle providers that don't use API keys first
    match provider {
        Provider::Ollama => {
            // Ollama doesn't need API key validation
            return Ok(());
        }
        Provider::Anthropic | Provider::GitHubCopilot => {
            // OAuth providers don't use API keys
            return Err(format!("{} uses OAuth authentication, not API keys", provider.display_name()));
        }
        _ => {}
    }

    // Check format for API key providers
    provider
        .validate_api_key_format(api_key)
        .map_err(|e| e.to_string())?;

    let client = Client::new();

    match provider {
        Provider::OpenRouter => validate_openrouter_key(&client, api_key).await,
        Provider::Anthropic | Provider::Ollama | Provider::GitHubCopilot => {
            // Already handled above
            unreachable!()
        }
    }
}

/// Validate an Anthropic API key by making a minimal request.
async fn validate_anthropic_key(client: &reqwest::Client, api_key: &str) -> Result<(), String> {
    // Use the messages endpoint with max_tokens=1 for minimal cost
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-5",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hi"}]
        }))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let status = response.status();

    // 200 = valid key, working
    // 429 = valid key, rate limited (treat as valid)
    // 400 = might be valid but bad request (treat as valid for format check)
    if status.is_success() || status.as_u16() == 429 || status.as_u16() == 400 {
        return Ok(());
    }

    // 401 = invalid key
    if status.as_u16() == 401 {
        return Err("Invalid API key".to_string());
    }

    // 403 = forbidden (might be permissions issue)
    if status.as_u16() == 403 {
        return Err("API key forbidden - check permissions".to_string());
    }

    // Other errors
    let body = response.text().await.unwrap_or_default();
    Err(format!("API error ({}): {}", status, body))
}

/// Validate an OpenRouter API key by making a minimal request.
async fn validate_openrouter_key(client: &reqwest::Client, api_key: &str) -> Result<(), String> {
    // OpenRouter uses OpenAI-compatible API format
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "openai/claude-haiku-4.5",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hi"}]
        }))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let status = response.status();

    // 200 = valid key, working
    // 429 = valid key, rate limited (treat as valid)
    // 400 = might be valid but bad request (treat as valid for format check)
    if status.is_success() || status.as_u16() == 429 || status.as_u16() == 400 {
        return Ok(());
    }

    // 401 = invalid key
    if status.as_u16() == 401 {
        return Err("Invalid API key".to_string());
    }

    // 403 = forbidden
    if status.as_u16() == 403 {
        return Err("API key forbidden - check permissions".to_string());
    }

    // Other errors
    let body = response.text().await.unwrap_or_default();
    Err(format!("API error ({}): {}", status, body))
}

/// Chat message for API requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Events sent during streaming.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A chunk of text was received.
    Token(String),
    /// Stream completed successfully.
    Done,
    /// An error occurred.
    Error(String),
    /// Authentication error - token is invalid or expired.
    /// The app should clear credentials and prompt for re-authentication.
    AuthError,
}

/// LLM client configuration.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: Provider,
    pub api_base: String,
    pub api_key: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        let provider = Provider::default();
        Self {
            provider,
            api_base: provider.default_api_base().to_string(),
            api_key: String::new(),
            model: provider.default_model().to_string(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
        }
    }
}

impl LlmConfig {
    /// Check if the client is configured with an API key (or doesn't need one).
    pub fn is_configured(&self) -> bool {
        !self.provider.requires_api_key() || !self.api_key.is_empty()
    }

    /// Load from environment variables, with file config as fallback.
    pub fn from_env_and_config(file_config: Option<&crate::config::LlmConfigFile>) -> Self {
        let mut config = Self::default();

        // First apply file config if present
        if let Some(fc) = file_config {
            // Check for deprecated OpenAI config and warn
            if fc.api_base.contains("openai.com") || fc.model.starts_with("gpt-") {
                eprintln!(
                    "\x1b[33mWarning: OpenAI configuration detected but is no longer supported.\x1b[0m"
                );
                eprintln!(
                    "\x1b[33mPlease update your config to use Anthropic. See docs for migration.\x1b[0m"
                );
            }

            config.api_base = fc.api_base.clone();
            if let Some(ref key) = fc.api_key {
                config.api_key = key.clone();
            }
            config.model = fc.model.clone();
            config.temperature = fc.temperature;
            config.max_tokens = fc.max_tokens;
        }

        // Environment variable overrides file config
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            config.api_key = key;
        }

        // Also check for ANTHROPIC_MODEL
        if let Ok(model) = std::env::var("ANTHROPIC_MODEL") {
            config.model = model;
        }

        config
    }
}

/// LLM client for making API calls.
///
/// This is a unified client that wraps a provider implementing `LlmProvider`.
/// It provides a simple interface for the rest of the application.
#[derive(Clone)]
pub struct LlmClient {
    inner: Arc<dyn LlmProvider>,
}

impl LlmClient {
    /// Create a new LLM client with the given configuration.
    ///
    /// This will create the appropriate provider based on the config.
    pub fn new(config: LlmConfig) -> Self {
        let provider: Arc<dyn LlmProvider> = match config.provider {
            Provider::Anthropic => Arc::new(AnthropicClient::new(config)),
            Provider::GitHubCopilot => {
                // Copilot provider - will load credentials on first use
                let mut copilot = CopilotProvider::new();
                copilot = copilot.with_model(config.model);
                if let Some(temp) = config.temperature {
                    copilot = copilot.with_temperature(temp);
                }
                if let Some(max) = config.max_tokens {
                    copilot = copilot.with_max_tokens(max);
                }
                Arc::new(copilot)
            }
            Provider::Ollama => Arc::new(OllamaProvider::new(config)),
            Provider::OpenRouter => Arc::new(OpenRouterProvider::new(config)),
        };

        Self { inner: provider }
    }

    /// Create a new LLM client from an existing provider.
    ///
    /// Use this when you have a custom or pre-configured provider.
    pub fn from_provider(provider: Arc<dyn LlmProvider>) -> Self {
        Self { inner: provider }
    }

    /// Get a reference to the underlying provider.
    pub fn provider(&self) -> &dyn LlmProvider {
        self.inner.as_ref()
    }

    /// Get the provider type.
    pub fn provider_type(&self) -> Provider {
        self.inner.provider()
    }

    /// Check if the client is configured.
    pub fn is_configured(&self) -> bool {
        self.inner.is_configured()
    }

    /// Get the current model name.
    #[allow(dead_code)]
    pub fn model(&self) -> &str {
        self.inner.model()
    }

    /// Get the display name for this provider.
    pub fn display_name(&self) -> &str {
        self.inner.display_name()
    }

    /// Send a streaming chat completion request.
    /// Returns a channel receiver that yields StreamEvents.
    pub fn stream_chat(&self, messages: Vec<ChatMessage>) -> mpsc::Receiver<StreamEvent> {
        self.inner.stream_chat(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_client_anthropic_provider() {
        let config = LlmConfig::default();
        let client = LlmClient::new(config);
        assert_eq!(client.provider_type(), Provider::Anthropic);
        assert_eq!(client.display_name(), "Anthropic");
    }

    #[test]
    fn test_llm_client_model() {
        let mut config = LlmConfig::default();
        config.model = "claude-3-opus-20240229".to_string();
        let client = LlmClient::new(config);
        assert_eq!(client.model(), "claude-3-opus-20240229");
    }

    #[test]
    fn test_llm_client_not_configured_without_key() {
        let mut config = LlmConfig::default();
        config.provider = Provider::OpenRouter; // OpenRouter requires API key
        let client = LlmClient::new(config);
        assert!(!client.is_configured());
    }

    #[test]
    fn test_llm_client_configured_with_key() {
        let mut config = LlmConfig::default();
        config.api_key = "test-key".to_string();
        let client = LlmClient::new(config);
        assert!(client.is_configured());
    }

    #[test]
    fn test_llm_client_from_provider() {
        let config = LlmConfig::default();
        let anthropic = Arc::new(AnthropicClient::new(config));
        let client = LlmClient::from_provider(anthropic);
        assert_eq!(client.provider_type(), Provider::Anthropic);
    }

    #[test]
    fn test_openrouter_provider() {
        let mut config = LlmConfig::default();
        config.provider = Provider::OpenRouter;
        config.api_base = Provider::OpenRouter.default_api_base().to_string();
        config.model = Provider::OpenRouter.default_model().to_string();
        let client = LlmClient::new(config);
        assert_eq!(client.provider_type(), Provider::OpenRouter);
        // OpenRouter requires an API key, so it's not configured without one
        assert!(!client.is_configured());
    }

    #[test]
    fn test_openrouter_provider_configured() {
        let mut config = LlmConfig::default();
        config.provider = Provider::OpenRouter;
        config.api_key = "test-key".to_string();
        let client = LlmClient::new(config);
        assert_eq!(client.provider_type(), Provider::OpenRouter);
        assert!(client.is_configured());
    }

    #[test]
    fn test_ollama_provider() {
        let mut config = LlmConfig::default();
        config.provider = Provider::Ollama;
        config.api_base = Provider::Ollama.default_api_base().to_string();
        config.model = Provider::Ollama.default_model().to_string();
        let client = LlmClient::new(config);
        assert_eq!(client.provider_type(), Provider::Ollama);
        // Ollama doesn't need an API key, so it's always configured
        assert!(client.is_configured());
    }

    #[test]
    fn test_provider_storage_key() {
        assert_eq!(Provider::Anthropic.storage_key(), "anthropic");
        assert_eq!(Provider::OpenRouter.storage_key(), "openrouter");
        assert_eq!(Provider::Ollama.storage_key(), "ollama");
        assert_eq!(Provider::GitHubCopilot.storage_key(), "github_copilot");
    }

    #[test]
    fn test_provider_api_key_url() {
        // Only OpenRouter has API key URL now
        assert!(Provider::OpenRouter.api_key_url().is_some());
        assert!(Provider::OpenRouter
            .api_key_url()
            .unwrap()
            .contains("openrouter"));

        // Anthropic, Ollama, and Copilot don't use API keys
        assert!(Provider::Anthropic.api_key_url().is_none()); // Uses OAuth
        assert!(Provider::Ollama.api_key_url().is_none());
        assert!(Provider::GitHubCopilot.api_key_url().is_none()); // Uses OAuth
    }

    #[test]
    fn test_provider_validate_api_key_format_anthropic() {
        // Valid Anthropic key
        assert!(Provider::Anthropic
            .validate_api_key_format("sk-ant-api03-abcdefghijklmnopqrstuvwxyz")
            .is_ok());

        // Invalid: wrong prefix
        assert!(Provider::Anthropic
            .validate_api_key_format("sk-xyz-abcdefghijklmnopqrstuvwxyz")
            .is_err());

        // Invalid: empty
        assert!(Provider::Anthropic.validate_api_key_format("").is_err());

        // Invalid: too short
        assert!(Provider::Anthropic
            .validate_api_key_format("sk-ant-abc")
            .is_err());
    }

    #[test]
    fn test_provider_validate_api_key_format_openrouter() {
        // Valid OpenRouter key
        assert!(Provider::OpenRouter
            .validate_api_key_format("sk-or-v1-abcdefghijklmnopqrstuvwxyz")
            .is_ok());

        // Invalid: wrong prefix
        assert!(Provider::OpenRouter
            .validate_api_key_format("sk-xyz-abcdefghijklmnopqrstuvwxyz")
            .is_err());

        // Invalid: empty
        assert!(Provider::OpenRouter.validate_api_key_format("").is_err());
    }

    #[test]
    fn test_provider_validate_api_key_format_no_key_providers() {
        // Ollama doesn't need API keys
        assert!(Provider::Ollama
            .validate_api_key_format("anything")
            .is_err());

        // OAuth providers don't use API keys
        assert!(Provider::Anthropic
            .validate_api_key_format("anything")
            .is_err());
        assert!(Provider::GitHubCopilot
            .validate_api_key_format("anything")
            .is_err());
    }

    #[tokio::test]
    async fn test_validate_api_key_format_check_anthropic() {
        // Anthropic uses OAuth, not API keys - should fail
        let result = validate_api_key(Provider::Anthropic, "invalid-key").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("OAuth"));
    }

    #[tokio::test]
    async fn test_validate_api_key_format_check_openrouter() {
        // Invalid format should fail before network request
        let result = validate_api_key(Provider::OpenRouter, "invalid-key").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("sk-or-"));
    }

    #[tokio::test]
    async fn test_validate_api_key_ollama() {
        // Ollama doesn't need validation - should always succeed
        let result = validate_api_key(Provider::Ollama, "anything").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_api_key_copilot_fails() {
        // Copilot uses OAuth, not API keys
        let result = validate_api_key(Provider::GitHubCopilot, "anything").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("OAuth"));
    }

    #[tokio::test]
    async fn test_validate_api_key_empty() {
        // Empty key should fail - use OpenRouter since Anthropic now uses OAuth
        let result = validate_api_key(Provider::OpenRouter, "").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_copilot_models_not_empty() {
        assert!(!COPILOT_MODELS.is_empty());
    }

    #[test]
    fn test_copilot_models_have_display_and_api_names() {
        for (display_name, api_id) in COPILOT_MODELS {
            assert!(!display_name.is_empty(), "Display name should not be empty");
            assert!(!api_id.is_empty(), "API ID should not be empty");
        }
    }

    #[test]
    fn test_copilot_models_first_is_default() {
        // First model should match the default
        let (_, first_api_id) = COPILOT_MODELS[0];
        assert_eq!(first_api_id, "claude-sonnet-4.5");
        assert_eq!(first_api_id, Provider::GitHubCopilot.default_model());
    }
}
