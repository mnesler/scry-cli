//! Anthropic OAuth 2.0 Authorization Code flow with PKCE.
//!
//! Anthropic uses the standard OAuth Authorization Code flow with PKCE,
//! which requires the user to visit a URL in their browser and paste back
//! an authorization code.

use super::oauth::OAuthToken;
use super::pkce::Pkce;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Serialize;

/// Anthropic OAuth client ID (public identifier from Claude CLI / OpenCode).
const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

/// Authorization URL for Claude Pro/Max users.
const AUTH_URL_CLAUDE: &str = "https://claude.ai/oauth/authorize";

/// Authorization URL for Console users (API key creation).
const AUTH_URL_CONSOLE: &str = "https://console.anthropic.com/oauth/authorize";

/// Token exchange URL.
const TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";

/// OAuth redirect URI.
const REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";

/// OAuth scopes to request.
const SCOPES: &str = "org:create_api_key user:profile user:inference";

/// Anthropic authentication method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnthropicAuthMethod {
    /// Claude Pro/Max subscription OAuth (claude.ai).
    ClaudeProMax,
    /// Create API key via OAuth (console.anthropic.com).
    CreateApiKey,
}

/// Token exchange request.
#[derive(Debug, Serialize)]
struct TokenRequest {
    code: String,
    state: String,
    grant_type: String,
    client_id: String,
    redirect_uri: String,
    code_verifier: String,
}

/// Token refresh request.
#[derive(Debug, Serialize)]
struct RefreshRequest {
    grant_type: String,
    client_id: String,
    refresh_token: String,
}

/// Handler for Anthropic OAuth flow.
#[derive(Debug, Clone)]
pub struct AnthropicOAuth {
    client: Client,
    method: AnthropicAuthMethod,
    pkce: Pkce,
}

impl AnthropicOAuth {
    /// Create a new Anthropic OAuth handler.
    pub fn new(method: AnthropicAuthMethod) -> Result<Self> {
        let pkce = Pkce::new()?;

        Ok(Self {
            client: Client::new(),
            method,
            pkce,
        })
    }

    /// Build the authorization URL for the user to visit.
    ///
    /// The user should open this URL in their browser, authorize the application,
    /// and then paste back the authorization code shown.
    pub fn build_auth_url(&self) -> String {
        let base_url = match self.method {
            AnthropicAuthMethod::ClaudeProMax => AUTH_URL_CLAUDE,
            AnthropicAuthMethod::CreateApiKey => AUTH_URL_CONSOLE,
        };

        // URL-encode parameters per OAuth 2.0 spec
        // Note: scope uses + for spaces (application/x-www-form-urlencoded style)
        let encoded_redirect = urlencoding::encode(REDIRECT_URI);
        let encoded_scope = urlencoding::encode(SCOPES).replace("%20", "+");

        format!(
            "{}?code=true&client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
            base_url,
            CLIENT_ID,
            encoded_redirect,
            encoded_scope,
            self.pkce.challenge,
            self.pkce.verifier  // verifier is used as state (matches OpenCode)
        )
    }

    /// Exchange the authorization code for an access token.
    ///
    /// The authorization code is expected in the format: `{code}#{state}`
    /// as returned by Anthropic's OAuth flow.
    pub async fn exchange_code(&self, auth_code: &str) -> Result<OAuthToken> {
        // Parse the authorization code format: {code}#{state}
        let parts: Vec<&str> = auth_code.split('#').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid authorization code format. Expected: {{code}}#{{state}}"
            ));
        }

        let code = parts[0];
        let state = parts[1];

        // Verify state matches our verifier (basic CSRF protection)
        if state != self.pkce.verifier {
            return Err(anyhow!("State mismatch - possible CSRF attack"));
        }

        let request = TokenRequest {
            code: code.to_string(),
            state: state.to_string(),
            grant_type: "authorization_code".to_string(),
            client_id: CLIENT_ID.to_string(),
            redirect_uri: REDIRECT_URI.to_string(),
            code_verifier: self.pkce.verifier.clone(),
        };

        let response = self
            .client
            .post(TOKEN_URL)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to exchange authorization code")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Token exchange failed ({}): {}",
                status,
                body
            ));
        }

        response
            .json::<OAuthToken>()
            .await
            .context("Failed to parse token response")
    }

    /// Refresh an expired access token using the refresh token.
    pub async fn refresh_token(refresh_token: &str) -> Result<OAuthToken> {
        let client = Client::new();

        let request = RefreshRequest {
            grant_type: "refresh_token".to_string(),
            client_id: CLIENT_ID.to_string(),
            refresh_token: refresh_token.to_string(),
        };

        let response = client
            .post(TOKEN_URL)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to refresh token")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Token refresh failed ({}): {}",
                status,
                body
            ));
        }

        response
            .json::<OAuthToken>()
            .await
            .context("Failed to parse refresh response")
    }

    /// Open the authorization URL in the user's browser.
    pub fn open_browser(&self) -> Result<()> {
        let url = self.build_auth_url();
        open::that(&url).context("Failed to open browser")
    }

    /// Get the PKCE verifier (needed for verification).
    pub fn verifier(&self) -> &str {
        &self.pkce.verifier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_url_claude_pro() {
        let oauth = AnthropicOAuth::new(AnthropicAuthMethod::ClaudeProMax).unwrap();
        let url = oauth.build_auth_url();

        assert!(url.starts_with(AUTH_URL_CLAUDE));
        assert!(url.contains(&format!("client_id={}", CLIENT_ID)));
        assert!(url.contains("code_challenge_method=S256"));
        
        // Check for URL-encoded scope (colons encoded as %3A, spaces as +)
        assert!(url.contains("scope=org%3Acreate_api_key+user%3Aprofile+user%3Ainference"));
        
        // Check for URL-encoded redirect_uri
        assert!(url.contains("redirect_uri=https%3A%2F%2Fconsole.anthropic.com%2Foauth%2Fcode%2Fcallback"));
        
        // Verify state parameter exists and is 86 characters (base64url encoded 64 bytes)
        assert!(url.contains("state="));
        let state_start = url.find("state=").unwrap() + 6;
        let state_end = url[state_start..].find('&').unwrap_or(url[state_start..].len());
        let state = &url[state_start..state_start + state_end];
        assert_eq!(state.len(), 86);
    }

    #[test]
    fn test_auth_url_console() {
        let oauth = AnthropicOAuth::new(AnthropicAuthMethod::CreateApiKey).unwrap();
        let url = oauth.build_auth_url();

        assert!(url.starts_with(AUTH_URL_CONSOLE));
        assert!(url.contains(&format!("client_id={}", CLIENT_ID)));
        
        // Check for URL-encoded parameters
        assert!(url.contains("scope=org%3Acreate_api_key+user%3Aprofile+user%3Ainference"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fconsole.anthropic.com%2Foauth%2Fcode%2Fcallback"));
    }

    #[test]
    fn test_different_instances_have_different_pkce() {
        let oauth1 = AnthropicOAuth::new(AnthropicAuthMethod::ClaudeProMax).unwrap();
        let oauth2 = AnthropicOAuth::new(AnthropicAuthMethod::ClaudeProMax).unwrap();

        assert_ne!(oauth1.pkce.verifier, oauth2.pkce.verifier);
        assert_ne!(oauth1.pkce.challenge, oauth2.pkce.challenge);
    }
}
