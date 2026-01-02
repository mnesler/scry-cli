//! Auth dialog for OAuth device code flow.
//!
//! This module provides a specialized dialog for displaying OAuth device code
//! information and handling user interaction during authentication.

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::auth::DeviceCode;

/// State of the auth dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthDialogState {
    /// Waiting for user to authenticate.
    Pending,
    /// Authentication successful.
    Success,
    /// Authentication failed with error message.
    Error(String),
    /// User cancelled authentication.
    Cancelled,
}

/// Result of handling a key event in the auth dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthDialogResult {
    /// Continue showing the dialog.
    Continue,
    /// User requested to open the URL in browser.
    OpenBrowser,
    /// User cancelled the dialog.
    Cancel,
}

/// Auth dialog for OAuth device code flow.
#[derive(Debug, Clone)]
pub struct AuthDialog {
    /// Provider name (e.g., "GitHub Copilot").
    pub provider_name: String,
    /// The device code information.
    pub device_code: DeviceCode,
    /// Current state of the dialog.
    pub state: AuthDialogState,
    /// Seconds remaining until expiration.
    pub seconds_remaining: u64,
    /// Status message to display.
    pub status_message: String,
}

impl AuthDialog {
    /// Create a new auth dialog.
    pub fn new(provider_name: impl Into<String>, device_code: DeviceCode) -> Self {
        let seconds_remaining = device_code.expires_in;
        Self {
            provider_name: provider_name.into(),
            device_code,
            state: AuthDialogState::Pending,
            seconds_remaining,
            status_message: "Waiting for authentication...".to_string(),
        }
    }

    /// Update the countdown timer.
    pub fn tick(&mut self) {
        if self.seconds_remaining > 0 {
            self.seconds_remaining -= 1;
        }
    }

    /// Check if the dialog has expired.
    pub fn is_expired(&self) -> bool {
        self.seconds_remaining == 0
    }

    /// Set the dialog to success state.
    pub fn set_success(&mut self) {
        self.state = AuthDialogState::Success;
        self.status_message = "Authentication successful!".to_string();
    }

    /// Set the dialog to error state.
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.state = AuthDialogState::Error(message.into());
        self.status_message = "Authentication failed.".to_string();
    }

    /// Set the dialog to cancelled state.
    pub fn set_cancelled(&mut self) {
        self.state = AuthDialogState::Cancelled;
    }

    /// Update the status message.
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyCode) -> AuthDialogResult {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.set_cancelled();
                AuthDialogResult::Cancel
            }
            KeyCode::Enter | KeyCode::Char('o') => AuthDialogResult::OpenBrowser,
            _ => AuthDialogResult::Continue,
        }
    }

    /// Format the remaining time as MM:SS.
    pub fn format_time_remaining(&self) -> String {
        let minutes = self.seconds_remaining / 60;
        let seconds = self.seconds_remaining % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    /// Get the verification URL (prefer complete URL if available).
    pub fn verification_url(&self) -> &str {
        self.device_code
            .verification_uri_complete
            .as_deref()
            .unwrap_or(&self.device_code.verification_uri)
    }

    /// Render the auth dialog.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Calculate dialog size (60% width, 50% height)
        let width = (area.width as u32 * 60 / 100) as u16;
        let height = (area.height as u32 * 50 / 100) as u16;
        let height = height.max(12); // Minimum height
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;
        let dialog_area = Rect::new(x, y, width, height);

        // Clear the area
        frame.render_widget(Clear, dialog_area);

        // Dialog border
        let title = format!(" {} Authentication ", self.provider_name);
        let border_color = match &self.state {
            AuthDialogState::Pending => Color::Cyan,
            AuthDialogState::Success => Color::Green,
            AuthDialogState::Error(_) => Color::Red,
            AuthDialogState::Cancelled => Color::Yellow,
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Layout content
        let chunks = Layout::vertical([
            Constraint::Length(2), // URL section
            Constraint::Length(3), // Code section
            Constraint::Length(2), // Timer/status
            Constraint::Min(1),    // Instructions
            Constraint::Length(1), // Key hints
        ])
        .split(inner);

        // URL section
        let url_lines = vec![
            Line::from(Span::styled(
                "Visit:",
                Style::default().fg(Color::Gray),
            )),
            Line::from(Span::styled(
                self.verification_url(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED),
            )),
        ];
        frame.render_widget(Paragraph::new(url_lines), chunks[0]);

        // Code section - prominent display
        let code_lines = vec![
            Line::from(Span::styled(
                "Enter this code:",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}  ", self.device_code.user_code),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
        ];
        frame.render_widget(Paragraph::new(code_lines), chunks[1]);

        // Timer and status
        let timer_color = if self.seconds_remaining < 60 {
            Color::Red
        } else if self.seconds_remaining < 180 {
            Color::Yellow
        } else {
            Color::Green
        };
        let status_lines = vec![
            Line::from(vec![
                Span::styled("Expires in: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    self.format_time_remaining(),
                    Style::default().fg(timer_color),
                ),
            ]),
            Line::from(Span::styled(
                &self.status_message,
                Style::default().fg(Color::White),
            )),
        ];
        frame.render_widget(Paragraph::new(status_lines), chunks[2]);

        // Instructions
        let instructions = match &self.state {
            AuthDialogState::Pending => {
                "Open the URL in your browser and enter the code shown above."
            }
            AuthDialogState::Success => "You can now close this dialog.",
            AuthDialogState::Error(msg) => msg.as_str(),
            AuthDialogState::Cancelled => "Authentication was cancelled.",
        };
        frame.render_widget(
            Paragraph::new(instructions).style(Style::default().fg(Color::Gray)),
            chunks[3],
        );

        // Key hints
        let hints = Line::from(vec![
            Span::styled("[Enter/o]", Style::default().fg(Color::Yellow)),
            Span::raw(" Open URL  "),
            Span::styled("[Esc/q]", Style::default().fg(Color::Yellow)),
            Span::raw(" Cancel"),
        ]);
        frame.render_widget(
            Paragraph::new(hints).style(Style::default().fg(Color::Gray)),
            chunks[4],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device_code() -> DeviceCode {
        DeviceCode {
            device_code: "test_device_code".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://example.com/device".to_string(),
            verification_uri_complete: None,
            expires_in: 900,
            interval: 5,
        }
    }

    #[test]
    fn test_auth_dialog_new() {
        let device_code = create_test_device_code();
        let dialog = AuthDialog::new("Test Provider", device_code);
        
        assert_eq!(dialog.provider_name, "Test Provider");
        assert_eq!(dialog.seconds_remaining, 900);
        assert_eq!(dialog.state, AuthDialogState::Pending);
    }

    #[test]
    fn test_auth_dialog_tick() {
        let device_code = create_test_device_code();
        let mut dialog = AuthDialog::new("Test", device_code);
        
        assert_eq!(dialog.seconds_remaining, 900);
        dialog.tick();
        assert_eq!(dialog.seconds_remaining, 899);
    }

    #[test]
    fn test_auth_dialog_tick_at_zero() {
        let mut device_code = create_test_device_code();
        device_code.expires_in = 1;
        let mut dialog = AuthDialog::new("Test", device_code);
        
        dialog.tick();
        assert_eq!(dialog.seconds_remaining, 0);
        dialog.tick(); // Should not go negative
        assert_eq!(dialog.seconds_remaining, 0);
    }

    #[test]
    fn test_auth_dialog_is_expired() {
        let mut device_code = create_test_device_code();
        device_code.expires_in = 0;
        let dialog = AuthDialog::new("Test", device_code);
        
        assert!(dialog.is_expired());
    }

    #[test]
    fn test_auth_dialog_set_success() {
        let device_code = create_test_device_code();
        let mut dialog = AuthDialog::new("Test", device_code);
        
        dialog.set_success();
        assert_eq!(dialog.state, AuthDialogState::Success);
    }

    #[test]
    fn test_auth_dialog_set_error() {
        let device_code = create_test_device_code();
        let mut dialog = AuthDialog::new("Test", device_code);
        
        dialog.set_error("Something went wrong");
        assert_eq!(dialog.state, AuthDialogState::Error("Something went wrong".to_string()));
    }

    #[test]
    fn test_auth_dialog_set_cancelled() {
        let device_code = create_test_device_code();
        let mut dialog = AuthDialog::new("Test", device_code);
        
        dialog.set_cancelled();
        assert_eq!(dialog.state, AuthDialogState::Cancelled);
    }

    #[test]
    fn test_auth_dialog_format_time_remaining() {
        let device_code = create_test_device_code();
        let dialog = AuthDialog::new("Test", device_code);
        
        assert_eq!(dialog.format_time_remaining(), "15:00");
    }

    #[test]
    fn test_auth_dialog_format_time_remaining_seconds() {
        let mut device_code = create_test_device_code();
        device_code.expires_in = 65;
        let dialog = AuthDialog::new("Test", device_code);
        
        assert_eq!(dialog.format_time_remaining(), "01:05");
    }

    #[test]
    fn test_auth_dialog_verification_url_basic() {
        let device_code = create_test_device_code();
        let dialog = AuthDialog::new("Test", device_code);
        
        assert_eq!(dialog.verification_url(), "https://example.com/device");
    }

    #[test]
    fn test_auth_dialog_verification_url_complete() {
        let mut device_code = create_test_device_code();
        device_code.verification_uri_complete = Some("https://example.com/device?code=ABCD".to_string());
        let dialog = AuthDialog::new("Test", device_code);
        
        assert_eq!(dialog.verification_url(), "https://example.com/device?code=ABCD");
    }

    #[test]
    fn test_auth_dialog_handle_key_escape() {
        let device_code = create_test_device_code();
        let mut dialog = AuthDialog::new("Test", device_code);
        
        let result = dialog.handle_key(KeyCode::Esc);
        assert_eq!(result, AuthDialogResult::Cancel);
        assert_eq!(dialog.state, AuthDialogState::Cancelled);
    }

    #[test]
    fn test_auth_dialog_handle_key_q() {
        let device_code = create_test_device_code();
        let mut dialog = AuthDialog::new("Test", device_code);
        
        let result = dialog.handle_key(KeyCode::Char('q'));
        assert_eq!(result, AuthDialogResult::Cancel);
    }

    #[test]
    fn test_auth_dialog_handle_key_enter() {
        let device_code = create_test_device_code();
        let mut dialog = AuthDialog::new("Test", device_code);
        
        let result = dialog.handle_key(KeyCode::Enter);
        assert_eq!(result, AuthDialogResult::OpenBrowser);
    }

    #[test]
    fn test_auth_dialog_handle_key_o() {
        let device_code = create_test_device_code();
        let mut dialog = AuthDialog::new("Test", device_code);
        
        let result = dialog.handle_key(KeyCode::Char('o'));
        assert_eq!(result, AuthDialogResult::OpenBrowser);
    }

    #[test]
    fn test_auth_dialog_handle_key_other() {
        let device_code = create_test_device_code();
        let mut dialog = AuthDialog::new("Test", device_code);
        
        let result = dialog.handle_key(KeyCode::Char('x'));
        assert_eq!(result, AuthDialogResult::Continue);
    }

    #[test]
    fn test_auth_dialog_set_status() {
        let device_code = create_test_device_code();
        let mut dialog = AuthDialog::new("Test", device_code);
        
        dialog.set_status("Custom status");
        assert_eq!(dialog.status_message, "Custom status");
    }
}
