//! Anthropic (Claude) API client implementation.
//!
//! This module implements streaming chat completions for Anthropic's Messages API.
//! See: https://docs.anthropic.com/en/api/messages-streaming

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

use super::{ChatMessage, LlmConfig, LlmProvider, Provider, StreamEvent};

/// Anthropic API version header value.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Request body for Anthropic Messages API.
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

/// Message format for Anthropic API.
/// Note: Anthropic only supports "user" and "assistant" roles.
/// System prompts are passed as a separate field.
#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// SSE event data for content_block_delta.
#[derive(Debug, Deserialize)]
struct ContentBlockDelta {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    event_type: String,
    #[allow(dead_code)]
    index: usize,
    delta: TextDelta,
}

/// Delta containing text content.
#[derive(Debug, Deserialize)]
struct TextDelta {
    #[serde(rename = "type")]
    delta_type: String,
    #[serde(default)]
    text: String,
}

/// Error response from Anthropic API.
#[derive(Debug, Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: String,
    error: AnthropicErrorDetail,
}

#[derive(Debug, Deserialize)]
struct AnthropicErrorDetail {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

/// Anthropic API client.
#[derive(Clone)]
pub struct AnthropicClient {
    client: Client,
    config: Arc<LlmConfig>,
}

impl AnthropicClient {
    /// Create a new Anthropic client with the given configuration.
    pub fn new(config: LlmConfig) -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(config),
        }
    }

    /// Get a reference to the internal HTTP client (for testing).
    #[cfg(test)]
    pub fn http_client(&self) -> &Client {
        &self.client
    }
}

#[async_trait]
impl LlmProvider for AnthropicClient {
    fn provider(&self) -> Provider {
        Provider::Anthropic
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn is_configured(&self) -> bool {
        self.config.is_configured()
    }

    fn stream_chat(&self, messages: Vec<ChatMessage>) -> mpsc::Receiver<StreamEvent> {
        let (tx, rx) = mpsc::channel(100);
        let client = self.client.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            if let Err(e) = stream_chat_inner(&client, &config, messages, tx.clone()).await {
                let _ = tx.send(StreamEvent::Error(e.to_string())).await;
            }
        });

        rx
    }
}

/// Convert generic ChatMessages to Anthropic format.
/// Extracts system messages into a separate field.
fn convert_messages(messages: Vec<ChatMessage>) -> (Option<String>, Vec<AnthropicMessage>) {
    let mut system = None;
    let mut anthropic_messages = Vec::new();

    for msg in messages {
        if msg.role == "system" {
            // Anthropic uses a separate system field, not in messages array
            system = Some(msg.content);
        } else {
            anthropic_messages.push(AnthropicMessage {
                role: msg.role,
                content: msg.content,
            });
        }
    }

    (system, anthropic_messages)
}

/// Internal streaming implementation.
async fn stream_chat_inner(
    client: &Client,
    config: &LlmConfig,
    messages: Vec<ChatMessage>,
    tx: mpsc::Sender<StreamEvent>,
) -> Result<()> {
    let url = format!("{}/messages", config.api_base);
    let (system, anthropic_messages) = convert_messages(messages);

    // Anthropic requires max_tokens
    let max_tokens = config.max_tokens.unwrap_or(4096);

    let request_body = AnthropicRequest {
        model: config.model.clone(),
        messages: anthropic_messages,
        max_tokens,
        stream: true,
        temperature: config.temperature,
        system,
    };

    let response = client
        .post(&url)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse as Anthropic error format
        if let Ok(error) = serde_json::from_str::<AnthropicError>(&body) {
            return Err(anyhow!(
                "Anthropic API error ({}): {} - {}",
                status,
                error.error.error_type,
                error.error.message
            ));
        }

        return Err(anyhow!("Anthropic API error {}: {}", status, body));
    }

    let mut stream = response.bytes_stream();

    // Buffer for incomplete SSE lines
    let mut buffer = String::new();
    // Track current event type (Anthropic uses named events)
    let mut current_event = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete lines
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim().to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            // Anthropic SSE format uses "event:" lines to identify event type
            if let Some(event_name) = line.strip_prefix("event: ") {
                current_event = event_name.to_string();
                continue;
            }

            // Process data lines based on current event type
            if let Some(json_str) = line.strip_prefix("data: ") {
                match current_event.as_str() {
                    "content_block_delta" => {
                        if let Ok(delta) = serde_json::from_str::<ContentBlockDelta>(json_str) {
                            // Only process text deltas
                            if delta.delta.delta_type == "text_delta" && !delta.delta.text.is_empty()
                            {
                                tx.send(StreamEvent::Token(delta.delta.text)).await.ok();
                            }
                        }
                    }
                    "message_stop" => {
                        tx.send(StreamEvent::Done).await.ok();
                        return Ok(());
                    }
                    "error" => {
                        // Handle streaming errors
                        if let Ok(error) = serde_json::from_str::<AnthropicError>(json_str) {
                            return Err(anyhow!(
                                "Stream error: {} - {}",
                                error.error.error_type,
                                error.error.message
                            ));
                        }
                    }
                    // Ignore other events: message_start, content_block_start,
                    // content_block_stop, message_delta, ping
                    _ => {}
                }
            }
        }
    }

    // If we get here without message_stop, still send Done
    tx.send(StreamEvent::Done).await.ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_messages_basic() {
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: "Hi there!".to_string(),
            },
        ];

        let (system, anthropic_msgs) = convert_messages(messages);

        assert!(system.is_none());
        assert_eq!(anthropic_msgs.len(), 2);
        assert_eq!(anthropic_msgs[0].role, "user");
        assert_eq!(anthropic_msgs[0].content, "Hello");
        assert_eq!(anthropic_msgs[1].role, "assistant");
        assert_eq!(anthropic_msgs[1].content, "Hi there!");
    }

    #[test]
    fn test_convert_messages_with_system() {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
        ];

        let (system, anthropic_msgs) = convert_messages(messages);

        assert_eq!(system, Some("You are a helpful assistant.".to_string()));
        assert_eq!(anthropic_msgs.len(), 1);
        assert_eq!(anthropic_msgs[0].role, "user");
    }

    #[test]
    fn test_request_serialization() {
        let request = AnthropicRequest {
            model: "claude-sonnet-4-5".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: 1024,
            stream: true,
            temperature: Some(0.7),
            system: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"claude-sonnet-4-5\""));
        assert!(json.contains("\"max_tokens\":1024"));
        assert!(json.contains("\"stream\":true"));
        // system should be omitted when None
        assert!(!json.contains("\"system\""));
    }

    #[test]
    fn test_request_serialization_with_system() {
        let request = AnthropicRequest {
            model: "claude-sonnet-4-5".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: 1024,
            stream: true,
            temperature: None,
            system: Some("Be helpful.".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"system\":\"Be helpful.\""));
        // temperature should be omitted when None
        assert!(!json.contains("\"temperature\""));
    }

    #[test]
    fn test_parse_content_block_delta() {
        let json = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let delta: ContentBlockDelta = serde_json::from_str(json).unwrap();
        assert_eq!(delta.delta.delta_type, "text_delta");
        assert_eq!(delta.delta.text, "Hello");
    }

    #[test]
    fn test_parse_error_response() {
        let json = r#"{"type":"error","error":{"type":"invalid_api_key","message":"Invalid API key provided"}}"#;
        let error: AnthropicError = serde_json::from_str(json).unwrap();
        assert_eq!(error.error.error_type, "invalid_api_key");
        assert_eq!(error.error.message, "Invalid API key provided");
    }

    #[test]
    fn test_client_not_configured() {
        let config = LlmConfig::default();
        let client = AnthropicClient::new(config);
        // Anthropic now uses OAuth, so it's always "configured" from requires_api_key perspective
        // but may not have a valid token yet
        assert!(client.is_configured());
    }

    #[test]
    fn test_client_configured() {
        let mut config = LlmConfig::default();
        config.api_key = "test-oauth-token".to_string(); // Now an OAuth token, not API key
        let client = AnthropicClient::new(config);
        assert!(client.is_configured());
    }

    #[test]
    fn test_model_name() {
        let mut config = LlmConfig::default();
        config.model = "claude-opus-4".to_string();
        let client = AnthropicClient::new(config);
        assert_eq!(client.model(), "claude-opus-4");
    }

    #[test]
    fn test_provider_type() {
        let config = LlmConfig::default();
        let client = AnthropicClient::new(config);
        assert_eq!(client.provider(), Provider::Anthropic);
    }

    #[test]
    fn test_display_name() {
        let config = LlmConfig::default();
        let client = AnthropicClient::new(config);
        assert_eq!(client.display_name(), "Anthropic");
    }
}
