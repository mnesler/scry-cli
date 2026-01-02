use anyhow::{anyhow, Result};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

/// OpenAI-compatible chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Request body for chat completions.
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// Streaming response chunk (SSE format).
#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Delta,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
}

/// LLM client configuration.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_base: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(2048),
        }
    }
}

impl LlmConfig {
    /// Check if the client is configured with an API key.
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Load API key from environment variable, with optional file config fallback.
    #[allow(dead_code)]
    pub fn from_env() -> Self {
        Self::from_env_and_config(None)
    }

    /// Load from environment variables, with file config as fallback.
    pub fn from_env_and_config(file_config: Option<&crate::config::LlmConfigFile>) -> Self {
        let mut config = Self::default();
        
        // First apply file config if present
        if let Some(fc) = file_config {
            config.api_base = fc.api_base.clone();
            if let Some(ref key) = fc.api_key {
                config.api_key = key.clone();
            }
            config.model = fc.model.clone();
            config.temperature = fc.temperature;
            config.max_tokens = fc.max_tokens;
        }
        
        // Environment variables override file config
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            config.api_key = key;
        }
        
        if let Ok(base) = std::env::var("OPENAI_API_BASE") {
            config.api_base = base;
        }
        
        if let Ok(model) = std::env::var("OPENAI_MODEL") {
            config.model = model;
        }
        
        config
    }
}

/// LLM client for making API calls.
#[derive(Clone)]
pub struct LlmClient {
    client: Client,
    config: Arc<LlmConfig>,
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

impl LlmClient {
    /// Create a new LLM client with the given configuration.
    pub fn new(config: LlmConfig) -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(config),
        }
    }

    /// Check if the client is configured.
    pub fn is_configured(&self) -> bool {
        self.config.is_configured()
    }

    /// Get the current model name.
    #[allow(dead_code)]
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Send a streaming chat completion request.
    /// Returns a channel receiver that yields StreamEvents.
    pub fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
    ) -> mpsc::Receiver<StreamEvent> {
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

/// Internal streaming implementation.
async fn stream_chat_inner(
    client: &Client,
    config: &LlmConfig,
    messages: Vec<ChatMessage>,
    tx: mpsc::Sender<StreamEvent>,
) -> Result<()> {
    let url = format!("{}/chat/completions", config.api_base);

    let request_body = ChatRequest {
        model: config.model.clone(),
        messages,
        stream: true,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("API error {}: {}", status, body));
    }

    let mut stream = response.bytes_stream();

    // Buffer for incomplete SSE lines
    let mut buffer = String::new();

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

            // SSE format: "data: {...}"
            if let Some(json_str) = line.strip_prefix("data: ") {
                if json_str == "[DONE]" {
                    tx.send(StreamEvent::Done).await.ok();
                    return Ok(());
                }

                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) {
                    for choice in chunk.choices {
                        if let Some(content) = choice.delta.content {
                            if !content.is_empty() {
                                tx.send(StreamEvent::Token(content)).await.ok();
                            }
                        }
                    }
                }
            }
        }
    }

    tx.send(StreamEvent::Done).await.ok();
    Ok(())
}
