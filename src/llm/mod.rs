//! LLM client module for API interactions.
//!
//! This module provides a unified interface for interacting with LLM providers.
//! Currently supports:
//! - Anthropic (Claude)
//! - GitHub Copilot

mod anthropic;
mod copilot;
mod provider;

pub use provider::{LlmProvider, ProviderError, ProviderResult};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

pub use anthropic::AnthropicClient;
pub use copilot::CopilotProvider;

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
            Provider::Ollama => "http://localhost:11434/api",
            Provider::OpenRouter => "https://openrouter.ai/api/v1",
            Provider::GitHubCopilot => "https://api.githubcopilot.com",
        }
    }

    /// Get the default model for this provider.
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-sonnet-4-5",
            Provider::Ollama => "llama3.2",
            Provider::OpenRouter => "anthropic/claude-sonnet-4-5",
            Provider::GitHubCopilot => "gpt-4o",
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
            Provider::Anthropic => true,
            Provider::Ollama => false,
            Provider::OpenRouter => true,
            Provider::GitHubCopilot => true,
        }
    }

    /// Check if this provider uses OAuth device flow.
    pub const fn uses_oauth(&self) -> bool {
        matches!(self, Provider::GitHubCopilot)
    }
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
            // Other providers use a placeholder that returns an error
            provider @ (Provider::Ollama | Provider::OpenRouter) => {
                Arc::new(NotImplementedProvider::new(provider, config.model))
            }
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

/// Placeholder provider for not-yet-implemented providers.
struct NotImplementedProvider {
    provider_type: Provider,
    model: String,
}

impl NotImplementedProvider {
    fn new(provider: Provider, model: String) -> Self {
        Self {
            provider_type: provider,
            model,
        }
    }
}

impl LlmProvider for NotImplementedProvider {
    fn provider(&self) -> Provider {
        self.provider_type
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn is_configured(&self) -> bool {
        // Not implemented providers are never configured
        false
    }

    fn stream_chat(&self, _messages: Vec<ChatMessage>) -> mpsc::Receiver<StreamEvent> {
        let (tx, rx) = mpsc::channel(1);
        let provider_name = self.provider_type.display_name().to_string();
        tokio::spawn(async move {
            let _ = tx
                .send(StreamEvent::Error(format!(
                    "{} provider is not yet implemented. Coming soon!",
                    provider_name
                )))
                .await;
        });
        rx
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
        config.model = "claude-opus-4".to_string();
        let client = LlmClient::new(config);
        assert_eq!(client.model(), "claude-opus-4");
    }

    #[test]
    fn test_llm_client_not_configured_without_key() {
        let config = LlmConfig::default();
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
    fn test_not_implemented_provider() {
        let mut config = LlmConfig::default();
        config.provider = Provider::Ollama;
        let client = LlmClient::new(config);
        assert_eq!(client.provider_type(), Provider::Ollama);
        assert!(!client.is_configured());
    }
}
