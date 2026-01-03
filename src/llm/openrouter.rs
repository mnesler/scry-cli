//! OpenRouter provider for multi-model access.
//!
//! OpenRouter provides access to multiple LLM models through a single API.
//! Uses the OpenAI-compatible API format.

use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

use super::{ChatMessage, LlmConfig, LlmProvider, Provider, StreamEvent};

/// Default OpenRouter API base URL.
const DEFAULT_API_BASE: &str = "https://openrouter.ai/api/v1";

/// OpenRouter provider for multi-model access.
pub struct OpenRouterProvider {
    client: Client,
    config: Arc<LlmConfig>,
}

impl OpenRouterProvider {
    /// Create a new OpenRouter provider with the given configuration.
    pub fn new(config: LlmConfig) -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(config),
        }
    }

    /// Create a new OpenRouter provider with default settings.
    #[allow(dead_code)]
    pub fn with_defaults() -> Self {
        let mut config = LlmConfig::default();
        config.provider = Provider::OpenRouter;
        config.api_base = DEFAULT_API_BASE.to_string();
        config.model = Provider::OpenRouter.default_model().to_string();
        Self::new(config)
    }

    /// Get the API base URL.
    fn api_base(&self) -> &str {
        if self.config.api_base.is_empty() {
            DEFAULT_API_BASE
        } else {
            &self.config.api_base
        }
    }
}

impl LlmProvider for OpenRouterProvider {
    fn provider(&self) -> Provider {
        Provider::OpenRouter
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn is_configured(&self) -> bool {
        !self.config.api_key.is_empty()
    }

    fn stream_chat(&self, messages: Vec<ChatMessage>) -> mpsc::Receiver<StreamEvent> {
        let (tx, rx) = mpsc::channel(32);

        let client = self.client.clone();
        let api_base = self.api_base().to_string();
        let api_key = self.config.api_key.clone();
        let model = self.config.model.clone();
        let temperature = self.config.temperature;
        let max_tokens = self.config.max_tokens;

        tokio::spawn(async move {
            if api_key.is_empty() {
                let _ = tx
                    .send(StreamEvent::Error(
                        "OpenRouter API key not configured. Set OPENROUTER_API_KEY environment variable.".to_string(),
                    ))
                    .await;
                return;
            }

            if let Err(e) = stream_openrouter_chat(
                client,
                api_base,
                api_key,
                model,
                temperature,
                max_tokens,
                messages,
                tx.clone(),
            )
            .await
            {
                let _ = tx.send(StreamEvent::Error(e)).await;
            }
        });

        rx
    }
}

/// OpenRouter chat request format (OpenAI-compatible).
#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// OpenRouter message format.
#[derive(Debug, Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

/// OpenRouter streaming response chunk (OpenAI SSE format).
#[derive(Debug, Deserialize)]
struct OpenRouterStreamChunk {
    #[serde(default)]
    choices: Vec<OpenRouterChoice>,
    #[serde(default)]
    error: Option<OpenRouterError>,
}

/// OpenRouter choice in streaming response.
#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    #[serde(default)]
    delta: OpenRouterDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

/// OpenRouter delta content.
#[derive(Debug, Default, Deserialize)]
struct OpenRouterDelta {
    #[serde(default)]
    content: Option<String>,
}

/// OpenRouter error response.
#[derive(Debug, Deserialize)]
struct OpenRouterError {
    message: String,
    #[serde(default)]
    #[allow(dead_code)]
    code: Option<String>,
}

/// Perform streaming chat with OpenRouter.
async fn stream_openrouter_chat(
    client: Client,
    api_base: String,
    api_key: String,
    model: String,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    messages: Vec<ChatMessage>,
    tx: mpsc::Sender<StreamEvent>,
) -> Result<(), String> {
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    // Convert messages to OpenRouter format
    let openrouter_messages: Vec<OpenRouterMessage> = messages
        .into_iter()
        .map(|m| OpenRouterMessage {
            role: m.role,
            content: m.content,
        })
        .collect();

    let request = OpenRouterRequest {
        model,
        messages: openrouter_messages,
        stream: true,
        temperature,
        max_tokens,
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("HTTP-Referer", "https://github.com/mnesler/scry-cli")
        .header("X-Title", "scry-cli")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        
        // Try to parse error message
        if let Ok(error_resp) = serde_json::from_str::<OpenRouterErrorResponse>(&body) {
            return Err(format!("OpenRouter error: {}", error_resp.error.message));
        }
        
        return Err(format!("OpenRouter error ({}): {}", status, body));
    }

    // Process SSE stream
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete lines (SSE format: "data: {...}")
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim();

            if line.starts_with("data: ") {
                let data = &line[6..];
                
                // Check for stream end
                if data == "[DONE]" {
                    let _ = tx.send(StreamEvent::Done).await;
                    return Ok(());
                }

                // Parse JSON chunk
                match serde_json::from_str::<OpenRouterStreamChunk>(data) {
                    Ok(chunk) => {
                        // Check for error
                        if let Some(error) = chunk.error {
                            let _ = tx.send(StreamEvent::Error(error.message)).await;
                            return Ok(());
                        }

                        // Send content if present
                        for choice in chunk.choices {
                            if let Some(content) = choice.delta.content {
                                if !content.is_empty() {
                                    if tx.send(StreamEvent::Token(content)).await.is_err() {
                                        return Ok(()); // Receiver dropped
                                    }
                                }
                            }

                            // Check for finish
                            if choice.finish_reason.is_some() {
                                let _ = tx.send(StreamEvent::Done).await;
                                return Ok(());
                            }
                        }
                    }
                    Err(e) => {
                        // Log parse error but continue
                        eprintln!("OpenRouter parse warning: {} for data: {}", e, data);
                    }
                }
            }

            buffer = buffer[newline_pos + 1..].to_string();
        }
    }

    // Send done if we haven't already
    let _ = tx.send(StreamEvent::Done).await;
    Ok(())
}

/// OpenRouter error response wrapper.
#[derive(Debug, Deserialize)]
struct OpenRouterErrorResponse {
    error: OpenRouterError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openrouter_provider_new() {
        let config = LlmConfig {
            provider: Provider::OpenRouter,
            api_base: "https://openrouter.ai/api/v1".to_string(),
            api_key: "test-key".to_string(),
            model: "anthropic/claude-sonnet-4-5".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            credential_type: crate::llm::CredentialType::ApiKey,
        };
        let provider = OpenRouterProvider::new(config);
        assert_eq!(provider.provider(), Provider::OpenRouter);
        assert_eq!(provider.model(), "anthropic/claude-sonnet-4-5");
    }

    #[test]
    fn test_openrouter_provider_is_configured() {
        let mut config = LlmConfig::default();
        config.provider = Provider::OpenRouter;
        config.api_key = "test-key".to_string();
        let provider = OpenRouterProvider::new(config);
        assert!(provider.is_configured());
    }

    #[test]
    fn test_openrouter_provider_not_configured() {
        let mut config = LlmConfig::default();
        config.provider = Provider::OpenRouter;
        config.api_key = String::new();
        let provider = OpenRouterProvider::new(config);
        assert!(!provider.is_configured());
    }

    #[test]
    fn test_openrouter_provider_display_name() {
        let config = LlmConfig {
            provider: Provider::OpenRouter,
            api_base: DEFAULT_API_BASE.to_string(),
            api_key: "test".to_string(),
            model: "test".to_string(),
            temperature: None,
            max_tokens: None,
            credential_type: crate::llm::CredentialType::ApiKey,
        };
        let provider = OpenRouterProvider::new(config);
        assert_eq!(provider.display_name(), "OpenRouter");
    }

    #[test]
    fn test_openrouter_api_base_default() {
        let mut config = LlmConfig::default();
        config.provider = Provider::OpenRouter;
        config.api_base = String::new();
        let provider = OpenRouterProvider::new(config);
        assert_eq!(provider.api_base(), DEFAULT_API_BASE);
    }

    #[test]
    fn test_openrouter_api_base_custom() {
        let mut config = LlmConfig::default();
        config.provider = Provider::OpenRouter;
        config.api_base = "https://custom.openrouter.ai/api".to_string();
        let provider = OpenRouterProvider::new(config);
        assert_eq!(provider.api_base(), "https://custom.openrouter.ai/api");
    }

    #[test]
    fn test_openrouter_message_serialization() {
        let msg = OpenRouterMessage {
            role: "user".to_string(),
            content: "Hello!".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello!\""));
    }

    #[test]
    fn test_openrouter_request_serialization() {
        let request = OpenRouterRequest {
            model: "anthropic/claude-sonnet-4-5".to_string(),
            messages: vec![OpenRouterMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: true,
            temperature: Some(0.7),
            max_tokens: Some(4096),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"anthropic/claude-sonnet-4-5\""));
        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_tokens\":4096"));
    }

    #[test]
    fn test_openrouter_request_no_optional_fields() {
        let request = OpenRouterRequest {
            model: "test".to_string(),
            messages: vec![],
            stream: true,
            temperature: None,
            max_tokens: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("temperature"));
        assert!(!json.contains("max_tokens"));
    }

    #[test]
    fn test_openrouter_stream_chunk_deserialization() {
        let json = r#"{"choices":[{"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let chunk: OpenRouterStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_openrouter_stream_chunk_finish() {
        let json = r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#;
        let chunk: OpenRouterStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_openrouter_error_deserialization() {
        let json = r#"{"error":{"message":"Invalid API key","code":"invalid_api_key"}}"#;
        let resp: OpenRouterErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error.message, "Invalid API key");
        assert_eq!(resp.error.code, Some("invalid_api_key".to_string()));
    }
}
