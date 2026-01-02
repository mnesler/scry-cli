//! LLM Provider trait for extensible provider architecture.
//!
//! This module defines the `LlmProvider` trait that all LLM providers must implement.
//! It enables a unified interface for interacting with different LLM backends.

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::{ChatMessage, Provider, StreamEvent};

/// Trait for LLM providers.
///
/// All LLM providers (Anthropic, Copilot, Ollama, etc.) implement this trait
/// to provide a unified interface for chat completions.
///
/// # Example
///
/// ```ignore
/// use scry_cli::llm::{LlmProvider, ChatMessage};
///
/// async fn chat(provider: &dyn LlmProvider) {
///     let messages = vec![ChatMessage {
///         role: "user".to_string(),
///         content: "Hello!".to_string(),
///     }];
///     
///     let mut rx = provider.stream_chat(messages);
///     while let Some(event) = rx.recv().await {
///         // Handle events...
///     }
/// }
/// ```
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Returns the provider type.
    fn provider(&self) -> Provider;

    /// Returns the current model name.
    fn model(&self) -> &str;

    /// Returns whether the provider is configured and ready to use.
    ///
    /// For API-key based providers, this checks if the key is set.
    /// For OAuth providers, this checks if valid credentials exist.
    /// For local providers (like Ollama), this may check connectivity.
    fn is_configured(&self) -> bool;

    /// Returns the display name for this provider instance.
    ///
    /// Defaults to the provider's standard display name.
    fn display_name(&self) -> &str {
        self.provider().display_name()
    }

    /// Send a streaming chat completion request.
    ///
    /// Returns a channel receiver that yields `StreamEvent`s:
    /// - `StreamEvent::Token(String)` - A chunk of generated text
    /// - `StreamEvent::Done` - Stream completed successfully
    /// - `StreamEvent::Error(String)` - An error occurred
    ///
    /// The returned receiver should be polled until `Done` or `Error` is received.
    fn stream_chat(&self, messages: Vec<ChatMessage>) -> mpsc::Receiver<StreamEvent>;

    /// Cancel any ongoing request.
    ///
    /// Default implementation does nothing. Providers that support
    /// cancellation should override this.
    fn cancel(&self) {
        // Default: no-op
    }
}

/// Result type for provider operations.
pub type ProviderResult<T> = Result<T, ProviderError>;

/// Errors that can occur during provider operations.
#[derive(Debug, Clone)]
pub enum ProviderError {
    /// Provider is not configured (missing API key, etc.)
    NotConfigured(String),

    /// Authentication failed
    AuthenticationFailed(String),

    /// Rate limit exceeded
    RateLimited {
        message: String,
        retry_after: Option<std::time::Duration>,
    },

    /// Network or connection error
    NetworkError(String),

    /// API returned an error
    ApiError {
        status: u16,
        message: String,
    },

    /// Invalid request (bad parameters, etc.)
    InvalidRequest(String),

    /// Provider-specific error
    Other(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured(msg) => write!(f, "Provider not configured: {}", msg),
            Self::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            Self::RateLimited { message, retry_after } => {
                if let Some(duration) = retry_after {
                    write!(f, "Rate limited: {} (retry after {:?})", message, duration)
                } else {
                    write!(f, "Rate limited: {}", message)
                }
            }
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::ApiError { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ProviderError {}

impl From<reqwest::Error> for ProviderError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::NetworkError("Request timed out".to_string())
        } else if err.is_connect() {
            Self::NetworkError(format!("Connection failed: {}", err))
        } else {
            Self::NetworkError(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_error_display() {
        let err = ProviderError::NotConfigured("missing API key".to_string());
        assert_eq!(err.to_string(), "Provider not configured: missing API key");

        let err = ProviderError::AuthenticationFailed("invalid token".to_string());
        assert_eq!(err.to_string(), "Authentication failed: invalid token");

        let err = ProviderError::RateLimited {
            message: "too many requests".to_string(),
            retry_after: Some(std::time::Duration::from_secs(60)),
        };
        assert!(err.to_string().contains("Rate limited"));
        assert!(err.to_string().contains("retry after"));

        let err = ProviderError::ApiError {
            status: 400,
            message: "bad request".to_string(),
        };
        assert_eq!(err.to_string(), "API error (400): bad request");
    }

    #[test]
    fn test_provider_error_from_reqwest() {
        // We can't easily create reqwest errors, but we can test the From impl exists
        // by checking the trait bounds
        fn assert_from<T: From<reqwest::Error>>() {}
        assert_from::<ProviderError>();
    }
}
