//! Ollama provider for local LLM models.
//!
//! Ollama runs locally and provides an OpenAI-compatible API.
//! No authentication is required.

use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

use super::{ChatMessage, LlmConfig, LlmProvider, Provider, StreamEvent};

/// Default Ollama API base URL.
const DEFAULT_API_BASE: &str = "http://localhost:11434";

/// Ollama provider for local models.
pub struct OllamaProvider {
    client: Client,
    config: Arc<LlmConfig>,
}

impl OllamaProvider {
    /// Create a new Ollama provider with the given configuration.
    pub fn new(config: LlmConfig) -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(config),
        }
    }

    /// Create a new Ollama provider with default settings.
    pub fn with_defaults() -> Self {
        let mut config = LlmConfig::default();
        config.provider = Provider::Ollama;
        config.api_base = DEFAULT_API_BASE.to_string();
        config.model = Provider::Ollama.default_model().to_string();
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

impl LlmProvider for OllamaProvider {
    fn provider(&self) -> Provider {
        Provider::Ollama
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn is_configured(&self) -> bool {
        // Ollama doesn't need an API key, so it's always "configured"
        // In a more sophisticated implementation, we might check if Ollama is actually running
        true
    }

    fn stream_chat(&self, messages: Vec<ChatMessage>) -> mpsc::Receiver<StreamEvent> {
        let (tx, rx) = mpsc::channel(32);

        let client = self.client.clone();
        let api_base = self.api_base().to_string();
        let model = self.config.model.clone();
        let temperature = self.config.temperature;

        tokio::spawn(async move {
            if let Err(e) = stream_ollama_chat(client, api_base, model, temperature, messages, tx.clone()).await {
                let _ = tx.send(StreamEvent::Error(e)).await;
            }
        });

        rx
    }
}

/// Ollama chat request format.
#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

/// Ollama message format.
#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

/// Ollama options for generation.
#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

/// Ollama streaming response chunk.
#[derive(Debug, Deserialize)]
struct OllamaStreamChunk {
    #[serde(default)]
    message: Option<OllamaResponseMessage>,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    error: Option<String>,
}

/// Ollama response message.
#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    #[allow(dead_code)]
    role: String,
    content: String,
}

/// Perform streaming chat with Ollama.
async fn stream_ollama_chat(
    client: Client,
    api_base: String,
    model: String,
    temperature: Option<f32>,
    messages: Vec<ChatMessage>,
    tx: mpsc::Sender<StreamEvent>,
) -> Result<(), String> {
    let url = format!("{}/api/chat", api_base.trim_end_matches('/'));

    // Convert messages to Ollama format
    let ollama_messages: Vec<OllamaMessage> = messages
        .into_iter()
        .map(|m| OllamaMessage {
            role: m.role,
            content: m.content,
        })
        .collect();

    let options = temperature.map(|t| OllamaOptions {
        temperature: Some(t),
    });

    let request = OllamaChatRequest {
        model,
        messages: ollama_messages,
        stream: true,
        options,
    };

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                "Failed to connect to Ollama. Is it running? Start with: ollama serve".to_string()
            } else {
                format!("Request failed: {}", e)
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Ollama error ({}): {}", status, body));
    }

    // Process streaming response (newline-delimited JSON)
    let mut stream = response.bytes_stream();

    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete lines
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim();
            
            if !line.is_empty() {
                match serde_json::from_str::<OllamaStreamChunk>(line) {
                    Ok(chunk) => {
                        // Check for error
                        if let Some(error) = chunk.error {
                            let _ = tx.send(StreamEvent::Error(error)).await;
                            return Ok(());
                        }

                        // Send content if present
                        if let Some(message) = chunk.message {
                            if !message.content.is_empty() {
                                if tx.send(StreamEvent::Token(message.content)).await.is_err() {
                                    return Ok(()); // Receiver dropped
                                }
                            }
                        }

                        // Check if done
                        if chunk.done {
                            let _ = tx.send(StreamEvent::Done).await;
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        // Log parse error but continue (might be incomplete JSON)
                        eprintln!("Parse warning: {} for line: {}", e, line);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_provider_new() {
        let config = LlmConfig {
            provider: Provider::Ollama,
            api_base: "http://localhost:11434".to_string(),
            api_key: String::new(),
            model: "qwen3:4b".to_string(),
            temperature: Some(0.7),
            max_tokens: None,
        };
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.provider(), Provider::Ollama);
        assert_eq!(provider.model(), "qwen3:4b");
    }

    #[test]
    fn test_ollama_provider_with_defaults() {
        let provider = OllamaProvider::with_defaults();
        assert_eq!(provider.provider(), Provider::Ollama);
        assert_eq!(provider.model(), "qwen3:4b");
        assert!(provider.is_configured());
    }

    #[test]
    fn test_ollama_provider_is_always_configured() {
        let provider = OllamaProvider::with_defaults();
        // Ollama doesn't need an API key, so it's always configured
        assert!(provider.is_configured());
    }

    #[test]
    fn test_ollama_provider_display_name() {
        let provider = OllamaProvider::with_defaults();
        assert_eq!(provider.display_name(), "Ollama (Local)");
    }

    #[test]
    fn test_ollama_api_base_default() {
        let mut config = LlmConfig::default();
        config.provider = Provider::Ollama;
        config.api_base = String::new(); // Empty
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.api_base(), DEFAULT_API_BASE);
    }

    #[test]
    fn test_ollama_api_base_custom() {
        let mut config = LlmConfig::default();
        config.provider = Provider::Ollama;
        config.api_base = "http://192.168.1.100:11434".to_string();
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.api_base(), "http://192.168.1.100:11434");
    }

    #[test]
    fn test_ollama_message_serialization() {
        let msg = OllamaMessage {
            role: "user".to_string(),
            content: "Hello!".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello!\""));
    }

    #[test]
    fn test_ollama_request_serialization() {
        let request = OllamaChatRequest {
            model: "qwen3:4b".to_string(),
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: true,
            options: Some(OllamaOptions {
                temperature: Some(0.7),
            }),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"qwen3:4b\""));
        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"temperature\":0.7"));
    }

    #[test]
    fn test_ollama_request_no_options() {
        let request = OllamaChatRequest {
            model: "qwen3:4b".to_string(),
            messages: vec![],
            stream: true,
            options: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("options"));
    }

    #[test]
    fn test_ollama_stream_chunk_deserialization() {
        let json = r#"{"message":{"role":"assistant","content":"Hello"},"done":false}"#;
        let chunk: OllamaStreamChunk = serde_json::from_str(json).unwrap();
        assert!(!chunk.done);
        assert!(chunk.message.is_some());
        assert_eq!(chunk.message.unwrap().content, "Hello");
    }

    #[test]
    fn test_ollama_stream_chunk_done() {
        let json = r#"{"done":true}"#;
        let chunk: OllamaStreamChunk = serde_json::from_str(json).unwrap();
        assert!(chunk.done);
        assert!(chunk.message.is_none());
    }

    #[test]
    fn test_ollama_stream_chunk_error() {
        let json = r#"{"error":"model not found","done":false}"#;
        let chunk: OllamaStreamChunk = serde_json::from_str(json).unwrap();
        assert!(!chunk.done);
        assert_eq!(chunk.error, Some("model not found".to_string()));
    }
}
