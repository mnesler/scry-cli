//! GitHub Copilot provider implementation.
//!
//! This module implements the GitHub Copilot LLM provider with OAuth device
//! code authentication. Copilot uses OpenAI-compatible chat completions API.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::{ChatMessage, LlmProvider, Provider, StreamEvent};
use crate::auth::{AuthStorage, Credential, DeviceCodeFlow, OAuthToken};

/// GitHub Copilot token response.
#[derive(Debug, Clone, Deserialize)]
struct CopilotToken {
    token: String,
    expires_at: i64,
}

/// Request body for Copilot chat completions.
#[derive(Debug, Serialize)]
struct CopilotRequest {
    model: String,
    messages: Vec<CopilotMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// Message format for Copilot API.
#[derive(Debug, Serialize)]
struct CopilotMessage {
    role: String,
    content: String,
}

/// SSE delta for streaming responses.
#[derive(Debug, Deserialize)]
struct StreamDelta {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: DeltaContent,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeltaContent {
    #[serde(default)]
    content: Option<String>,
}

/// Copilot token state for caching.
#[derive(Debug, Clone)]
struct TokenState {
    /// The Copilot API token.
    token: String,
    /// When the token expires.
    expires_at: DateTime<Utc>,
}

impl TokenState {
    #[allow(dead_code)]
    fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    fn needs_refresh(&self) -> bool {
        // Refresh if expiring in less than 5 minutes
        Utc::now() + chrono::Duration::minutes(5) >= self.expires_at
    }
}

/// GitHub Copilot LLM provider.
pub struct CopilotProvider {
    client: Client,
    model: String,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    /// Cached OAuth token from GitHub.
    oauth_token: Arc<RwLock<Option<String>>>,
    /// Cached Copilot API token.
    copilot_token: Arc<RwLock<Option<TokenState>>>,
}

impl Clone for CopilotProvider {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            model: self.model.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            oauth_token: Arc::clone(&self.oauth_token),
            copilot_token: Arc::clone(&self.copilot_token),
        }
    }
}

impl CopilotProvider {
    /// Create a new Copilot provider.
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            model: "claude-sonnet-4.5".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            oauth_token: Arc::new(RwLock::new(None)),
            copilot_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a provider with a specific model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Create a provider with temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Create a provider with max tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Load credentials from storage.
    pub async fn load_credentials(&self) -> Result<bool> {
        let storage = AuthStorage::load()?;
        if let Some(cred) = storage.get("github_copilot") {
            if !cred.is_expired() {
                let token = cred.token().to_string();
                *self.oauth_token.write().await = Some(token);
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Save credentials to storage.
    pub async fn save_credentials(&self, oauth_token: &OAuthToken) -> Result<()> {
        let mut storage = AuthStorage::load().unwrap_or_default();
        let expires_at = oauth_token.expires_at();
        storage.set(
            "github_copilot",
            Credential::oauth(
                &oauth_token.access_token,
                oauth_token.refresh_token.clone(),
                expires_at,
                None, // Model will be saved by app.rs after selection
            ),
        );
        storage.save()?;
        *self.oauth_token.write().await = Some(oauth_token.access_token.clone());
        Ok(())
    }

    /// Clear stored credentials.
    pub async fn clear_credentials(&self) -> Result<()> {
        let mut storage = AuthStorage::load().unwrap_or_default();
        storage.remove("github_copilot");
        storage.save()?;
        *self.oauth_token.write().await = None;
        *self.copilot_token.write().await = None;
        Ok(())
    }

    /// Check if we have a valid OAuth token.
    pub async fn has_oauth_token(&self) -> bool {
        self.oauth_token.read().await.is_some()
    }

    /// Get the device code flow for authentication.
    pub fn device_code_flow() -> DeviceCodeFlow {
        DeviceCodeFlow::github_copilot()
    }

    /// Exchange OAuth token for Copilot API token.
    async fn get_copilot_token(&self) -> Result<String> {
        // Check if we have a valid cached token
        {
            let cached = self.copilot_token.read().await;
            if let Some(ref state) = *cached {
                if !state.needs_refresh() {
                    return Ok(state.token.clone());
                }
            }
        }

        // Try to load OAuth token from storage if not already present
        if self.oauth_token.read().await.is_none() {
            let _ = self.load_credentials().await;
        }

        // Get the OAuth token
        let oauth_token = self
            .oauth_token
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("Not authenticated - run OAuth flow first"))?;

        let response = self
            .client
            .get("https://api.github.com/copilot_internal/v2/token")
            .header("Authorization", format!("Bearer {}", oauth_token))
            .header("User-Agent", "scry-cli/0.1.0")
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to get Copilot token")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get Copilot token ({}): {}", status, body));
        }

        let copilot_token: CopilotToken = response
            .json()
            .await
            .context("Failed to parse Copilot token response")?;

        let expires_at = DateTime::from_timestamp(copilot_token.expires_at, 0)
            .unwrap_or_else(|| Utc::now() + chrono::Duration::minutes(30));

        let token = copilot_token.token.clone();
        *self.copilot_token.write().await = Some(TokenState {
            token: copilot_token.token,
            expires_at,
        });

        Ok(token)
    }

    /// Send a streaming chat request.
    async fn stream_chat_inner(
        &self,
        messages: Vec<ChatMessage>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<()> {
        let copilot_token = self.get_copilot_token().await?;

        let copilot_messages: Vec<CopilotMessage> = messages
            .into_iter()
            .map(|m| CopilotMessage {
                role: m.role,
                content: m.content,
            })
            .collect();

        let request_body = CopilotRequest {
            model: self.model.clone(),
            messages: copilot_messages,
            stream: true,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        };

        let response = self
            .client
            .post("https://api.githubcopilot.com/chat/completions")
            .header("Authorization", format!("Bearer {}", copilot_token))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("Copilot-Integration-Id", "vscode-chat")
            .header("Editor-Version", "scry-cli/0.1.0")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send chat request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Copilot API error ({}): {}", status, body));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error reading stream")?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            // Process complete lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        tx.send(StreamEvent::Done).await.ok();
                        return Ok(());
                    }

                    if let Ok(delta) = serde_json::from_str::<StreamDelta>(data) {
                        for choice in delta.choices {
                            if let Some(content) = choice.delta.content {
                                if !content.is_empty() {
                                    tx.send(StreamEvent::Token(content)).await.ok();
                                }
                            }
                            if choice.finish_reason.is_some() {
                                tx.send(StreamEvent::Done).await.ok();
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        tx.send(StreamEvent::Done).await.ok();
        Ok(())
    }
}

impl Default for CopilotProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for CopilotProvider {
    fn provider(&self) -> Provider {
        Provider::GitHubCopilot
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn is_configured(&self) -> bool {
        // Check synchronously - we need a blocking check here
        // For a more accurate check, use has_oauth_token() async method
        true // Assume configured if provider exists; actual check happens at runtime
    }

    fn stream_chat(&self, messages: Vec<ChatMessage>) -> mpsc::Receiver<StreamEvent> {
        let (tx, rx) = mpsc::channel(100);
        let provider = self.clone();

        tokio::spawn(async move {
            if let Err(e) = provider.stream_chat_inner(messages, tx.clone()).await {
                let _ = tx.send(StreamEvent::Error(e.to_string())).await;
            }
        });

        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copilot_provider_new() {
        let provider = CopilotProvider::new();
        assert_eq!(provider.model(), "claude-sonnet-4.5");
        assert_eq!(provider.provider(), Provider::GitHubCopilot);
    }

    #[test]
    fn test_copilot_provider_with_model() {
        let provider = CopilotProvider::new().with_model("gpt-4");
        assert_eq!(provider.model(), "gpt-4");
    }

    #[test]
    fn test_copilot_provider_with_temperature() {
        let provider = CopilotProvider::new().with_temperature(0.5);
        assert_eq!(provider.temperature, Some(0.5));
    }

    #[test]
    fn test_copilot_provider_with_max_tokens() {
        let provider = CopilotProvider::new().with_max_tokens(2048);
        assert_eq!(provider.max_tokens, Some(2048));
    }

    #[test]
    fn test_copilot_provider_display_name() {
        let provider = CopilotProvider::new();
        assert_eq!(provider.display_name(), "GitHub Copilot");
    }

    #[test]
    fn test_copilot_request_serialization() {
        let request = CopilotRequest {
            model: "claude-sonnet-4.5".to_string(),
            messages: vec![CopilotMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: true,
            temperature: Some(0.7),
            max_tokens: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"claude-sonnet-4.5\""));
        assert!(json.contains("\"stream\":true"));
        // max_tokens should be omitted when None
        assert!(!json.contains("\"max_tokens\""));
    }

    #[test]
    fn test_copilot_message_serialization() {
        let msg = CopilotMessage {
            role: "user".to_string(),
            content: "Hello world".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello world\""));
    }

    #[test]
    fn test_token_state_expired() {
        let state = TokenState {
            token: "test".to_string(),
            expires_at: Utc::now() - chrono::Duration::hours(1),
        };
        assert!(state.is_expired());
        assert!(state.needs_refresh());
    }

    #[test]
    fn test_token_state_not_expired() {
        let state = TokenState {
            token: "test".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };
        assert!(!state.is_expired());
        assert!(!state.needs_refresh());
    }

    #[test]
    fn test_token_state_needs_refresh() {
        let state = TokenState {
            token: "test".to_string(),
            expires_at: Utc::now() + chrono::Duration::minutes(2),
        };
        assert!(!state.is_expired());
        assert!(state.needs_refresh()); // Less than 5 minutes
    }

    #[test]
    fn test_stream_delta_deserialization() {
        let json = r#"{"choices":[{"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let delta: StreamDelta = serde_json::from_str(json).unwrap();
        assert_eq!(delta.choices.len(), 1);
        assert_eq!(delta.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_stream_delta_with_finish_reason() {
        let json = r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#;
        let delta: StreamDelta = serde_json::from_str(json).unwrap();
        assert_eq!(delta.choices[0].finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_device_code_flow() {
        let _flow = CopilotProvider::device_code_flow();
        // Just verify it creates without panicking
        assert!(true, "Device code flow created successfully");
    }

    #[tokio::test]
    async fn test_copilot_provider_no_oauth_token() {
        let provider = CopilotProvider::new();
        assert!(!provider.has_oauth_token().await);
    }
}
