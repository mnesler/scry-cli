//! LLM client module for API interactions.
//!
//! This module provides a unified interface for interacting with LLM providers.
//! Currently supports:
//! - Anthropic (Claude)

mod anthropic;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

pub use anthropic::AnthropicClient;

/// Supported LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Anthropic,
    // Future providers:
    // Ollama,
    // OpenRouter,
    // GitHubCopilot,
}

impl Provider {
    /// Get the default API base URL for this provider.
    pub fn default_api_base(&self) -> &'static str {
        match self {
            Provider::Anthropic => "https://api.anthropic.com/v1",
        }
    }

    /// Get the default model for this provider.
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-sonnet-4-5",
        }
    }

    /// Get the environment variable name for the API key.
    pub fn env_var_name(&self) -> &'static str {
        match self {
            Provider::Anthropic => "ANTHROPIC_API_KEY",
        }
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
    /// Check if the client is configured with an API key.
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
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
/// This is a unified client that dispatches to the appropriate provider.
#[derive(Clone)]
pub struct LlmClient {
    inner: Arc<LlmClientInner>,
}

enum LlmClientInner {
    Anthropic(AnthropicClient),
}

impl LlmClient {
    /// Create a new LLM client with the given configuration.
    pub fn new(config: LlmConfig) -> Self {
        let inner = match config.provider {
            Provider::Anthropic => LlmClientInner::Anthropic(AnthropicClient::new(config)),
        };

        Self {
            inner: Arc::new(inner),
        }
    }

    /// Check if the client is configured.
    pub fn is_configured(&self) -> bool {
        match self.inner.as_ref() {
            LlmClientInner::Anthropic(client) => client.is_configured(),
        }
    }

    /// Get the current model name.
    #[allow(dead_code)]
    pub fn model(&self) -> &str {
        match self.inner.as_ref() {
            LlmClientInner::Anthropic(client) => client.model(),
        }
    }

    /// Send a streaming chat completion request.
    /// Returns a channel receiver that yields StreamEvents.
    pub fn stream_chat(&self, messages: Vec<ChatMessage>) -> mpsc::Receiver<StreamEvent> {
        match self.inner.as_ref() {
            LlmClientInner::Anthropic(client) => client.stream_chat(messages),
        }
    }
}
