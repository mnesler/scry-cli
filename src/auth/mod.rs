//! Authentication module for credential storage and OAuth flows.
//!
//! This module provides secure storage for API keys and OAuth tokens,
//! as well as implementations for authentication flows like OAuth device code.

mod anthropic;
mod oauth;
mod pkce;
mod storage;

pub use anthropic::{AnthropicAuthMethod, AnthropicOAuth};
pub use oauth::{DeviceCode, DeviceCodeConfig, DeviceCodeFlow, OAuthToken, PollResult};
pub use pkce::Pkce;
pub use storage::{AuthStorage, Credential};
