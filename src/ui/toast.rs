//! Toast notification system for displaying transient status messages.
//!
//! Toasts appear in the top-right corner and auto-dismiss after a configurable duration.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Toast notification level determining color and behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    /// Informational message (blue/cyan)
    Info,
    /// Success message (green)
    Success,
    /// Warning message (yellow/orange)
    Warning,
    /// Error message (red)
    Error,
}

impl ToastLevel {
    /// Get the color for this toast level.
    pub fn color(&self) -> Color {
        match self {
            ToastLevel::Info => Color::Cyan,
            ToastLevel::Success => Color::Green,
            ToastLevel::Warning => Color::Yellow,
            ToastLevel::Error => Color::Red,
        }
    }

    /// Get the prefix icon for this toast level.
    pub fn prefix(&self) -> &'static str {
        match self {
            ToastLevel::Info => "[i]",
            ToastLevel::Success => "[+]",
            ToastLevel::Warning => "[!]",
            ToastLevel::Error => "[x]",
        }
    }

    /// Get the default duration for this toast level.
    pub fn default_duration(&self) -> Duration {
        match self {
            ToastLevel::Info => Duration::from_secs(3),
            ToastLevel::Success => Duration::from_secs(3),
            ToastLevel::Warning => Duration::from_secs(5),
            ToastLevel::Error => Duration::from_secs(8),
        }
    }
}

/// A single toast notification.
#[derive(Debug, Clone)]
pub struct Toast {
    /// Unique identifier for this toast.
    pub id: u64,
    /// The message to display.
    pub message: String,
    /// The severity level.
    pub level: ToastLevel,
    /// When this toast was created.
    pub created_at: Instant,
    /// How long this toast should be displayed.
    pub duration: Duration,
}

impl Toast {
    /// Create a new toast with the default duration for its level.
    pub fn new(id: u64, message: impl Into<String>, level: ToastLevel) -> Self {
        let duration = level.default_duration();
        Self {
            id,
            message: message.into(),
            level,
            created_at: Instant::now(),
            duration,
        }
    }

    /// Create a new toast with a custom duration.
    pub fn with_duration(id: u64, message: impl Into<String>, level: ToastLevel, duration: Duration) -> Self {
        Self {
            id,
            message: message.into(),
            level,
            created_at: Instant::now(),
            duration,
        }
    }

    /// Check if this toast has expired.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    /// Get the remaining time as a fraction (1.0 = full, 0.0 = expired).
    pub fn time_remaining_fraction(&self) -> f32 {
        let elapsed = self.created_at.elapsed().as_secs_f32();
        let total = self.duration.as_secs_f32();
        if total == 0.0 {
            return 0.0;
        }
        (1.0 - elapsed / total).max(0.0)
    }
}

/// State manager for toast notifications.
#[derive(Debug)]
pub struct ToastState {
    /// Queue of active toasts.
    pub toasts: VecDeque<Toast>,
    /// Next toast ID to assign.
    next_id: u64,
    /// Maximum number of toasts to display at once.
    pub max_visible: usize,
}

impl Default for ToastState {
    fn default() -> Self {
        Self {
            toasts: VecDeque::new(),
            next_id: 0,
            max_visible: 3,
        }
    }
}

impl ToastState {
    /// Create a new ToastState with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new ToastState with a custom max visible count.
    pub fn with_max_visible(max_visible: usize) -> Self {
        Self {
            toasts: VecDeque::new(),
            next_id: 0,
            max_visible,
        }
    }

    /// Add a new toast notification.
    pub fn push(&mut self, message: impl Into<String>, level: ToastLevel) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let toast = Toast::new(id, message, level);
        self.toasts.push_back(toast);
        
        // Remove oldest toasts if we exceed max visible
        while self.toasts.len() > self.max_visible {
            self.toasts.pop_front();
        }
        
        id
    }

    /// Add a new toast with a custom duration.
    pub fn push_with_duration(&mut self, message: impl Into<String>, level: ToastLevel, duration: Duration) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let toast = Toast::with_duration(id, message, level, duration);
        self.toasts.push_back(toast);
        
        // Remove oldest toasts if we exceed max visible
        while self.toasts.len() > self.max_visible {
            self.toasts.pop_front();
        }
        
        id
    }

    /// Add an info toast.
    pub fn info(&mut self, message: impl Into<String>) -> u64 {
        self.push(message, ToastLevel::Info)
    }

    /// Add a success toast.
    pub fn success(&mut self, message: impl Into<String>) -> u64 {
        self.push(message, ToastLevel::Success)
    }

    /// Add a warning toast.
    pub fn warning(&mut self, message: impl Into<String>) -> u64 {
        self.push(message, ToastLevel::Warning)
    }

    /// Add an error toast.
    pub fn error(&mut self, message: impl Into<String>) -> u64 {
        self.push(message, ToastLevel::Error)
    }

    /// Remove a toast by ID.
    pub fn dismiss(&mut self, id: u64) {
        self.toasts.retain(|t| t.id != id);
    }

    /// Remove all expired toasts. Call this on each tick.
    pub fn tick(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Check if there are any toasts to display.
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// Get the number of active toasts.
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    /// Clear all toasts.
    pub fn clear(&mut self) {
        self.toasts.clear();
    }
}

/// Render toasts in the top-right corner of the frame.
pub fn render_toasts(f: &mut Frame, toast_state: &ToastState) {
    if toast_state.is_empty() {
        return;
    }

    let frame_area = f.size();
    
    // Toast dimensions
    let toast_width = 40u16.min(frame_area.width.saturating_sub(4));
    let toast_height = 3u16; // Border + content + border
    let spacing = 0u16; // Vertical spacing between toasts
    
    // Starting position (top-right corner)
    let start_x = frame_area.width.saturating_sub(toast_width + 2);
    let start_y = 1u16;
    
    for (i, toast) in toast_state.toasts.iter().enumerate() {
        let y = start_y + (i as u16) * (toast_height + spacing);
        
        // Don't render if it would go off screen
        if y + toast_height > frame_area.height {
            break;
        }
        
        let toast_area = Rect::new(start_x, y, toast_width, toast_height);
        
        // Clear the area behind the toast
        f.render_widget(Clear, toast_area);
        
        // Build toast content
        let color = toast.level.color();
        let prefix = toast.level.prefix();
        
        // Truncate message if too long
        let max_msg_len = (toast_width as usize).saturating_sub(prefix.len() + 5);
        let display_msg = if toast.message.len() > max_msg_len {
            format!("{}...", &toast.message[..max_msg_len.saturating_sub(3)])
        } else {
            toast.message.clone()
        };
        
        let content = Line::from(vec![
            Span::styled(prefix, Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(display_msg, Style::default().fg(Color::White)),
        ]);
        
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color))
            .style(Style::default().bg(Color::Black));
        
        let paragraph = Paragraph::new(content).block(block);
        
        f.render_widget(paragraph, toast_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_level_color() {
        assert_eq!(ToastLevel::Info.color(), Color::Cyan);
        assert_eq!(ToastLevel::Success.color(), Color::Green);
        assert_eq!(ToastLevel::Warning.color(), Color::Yellow);
        assert_eq!(ToastLevel::Error.color(), Color::Red);
    }

    #[test]
    fn test_toast_level_prefix() {
        assert_eq!(ToastLevel::Info.prefix(), "[i]");
        assert_eq!(ToastLevel::Success.prefix(), "[+]");
        assert_eq!(ToastLevel::Warning.prefix(), "[!]");
        assert_eq!(ToastLevel::Error.prefix(), "[x]");
    }

    #[test]
    fn test_toast_level_default_duration() {
        assert_eq!(ToastLevel::Info.default_duration(), Duration::from_secs(3));
        assert_eq!(ToastLevel::Success.default_duration(), Duration::from_secs(3));
        assert_eq!(ToastLevel::Warning.default_duration(), Duration::from_secs(5));
        assert_eq!(ToastLevel::Error.default_duration(), Duration::from_secs(8));
    }

    #[test]
    fn test_toast_new() {
        let toast = Toast::new(1, "Test message", ToastLevel::Info);
        assert_eq!(toast.id, 1);
        assert_eq!(toast.message, "Test message");
        assert_eq!(toast.level, ToastLevel::Info);
        assert_eq!(toast.duration, Duration::from_secs(3));
    }

    #[test]
    fn test_toast_with_duration() {
        let toast = Toast::with_duration(2, "Custom duration", ToastLevel::Warning, Duration::from_secs(10));
        assert_eq!(toast.id, 2);
        assert_eq!(toast.duration, Duration::from_secs(10));
    }

    #[test]
    fn test_toast_is_expired() {
        let toast = Toast::with_duration(1, "Instant", ToastLevel::Info, Duration::from_millis(0));
        assert!(toast.is_expired());
    }

    #[test]
    fn test_toast_time_remaining_fraction() {
        let toast = Toast::with_duration(1, "Test", ToastLevel::Info, Duration::from_secs(10));
        let fraction = toast.time_remaining_fraction();
        // Should be close to 1.0 since it was just created
        assert!(fraction > 0.9 && fraction <= 1.0);
    }

    #[test]
    fn test_toast_state_default() {
        let state = ToastState::default();
        assert!(state.is_empty());
        assert_eq!(state.max_visible, 3);
    }

    #[test]
    fn test_toast_state_with_max_visible() {
        let state = ToastState::with_max_visible(5);
        assert_eq!(state.max_visible, 5);
    }

    #[test]
    fn test_toast_state_push() {
        let mut state = ToastState::new();
        let id = state.push("Test", ToastLevel::Info);
        assert_eq!(id, 0);
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn test_toast_state_convenience_methods() {
        let mut state = ToastState::new();
        
        let id1 = state.info("Info message");
        let id2 = state.success("Success message");
        let id3 = state.warning("Warning message");
        
        assert_eq!(state.len(), 3);
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);
    }

    #[test]
    fn test_toast_state_max_visible_enforcement() {
        let mut state = ToastState::with_max_visible(2);
        
        state.push("First", ToastLevel::Info);
        state.push("Second", ToastLevel::Info);
        state.push("Third", ToastLevel::Info);
        
        // Should only have 2 toasts (the newest ones)
        assert_eq!(state.len(), 2);
        assert_eq!(state.toasts.front().unwrap().message, "Second");
        assert_eq!(state.toasts.back().unwrap().message, "Third");
    }

    #[test]
    fn test_toast_state_dismiss() {
        let mut state = ToastState::new();
        let id = state.push("Test", ToastLevel::Info);
        assert_eq!(state.len(), 1);
        
        state.dismiss(id);
        assert!(state.is_empty());
    }

    #[test]
    fn test_toast_state_dismiss_nonexistent() {
        let mut state = ToastState::new();
        state.push("Test", ToastLevel::Info);
        
        state.dismiss(999);
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn test_toast_state_tick_removes_expired() {
        let mut state = ToastState::new();
        state.push_with_duration("Instant", ToastLevel::Info, Duration::from_millis(0));
        
        state.tick();
        assert!(state.is_empty());
    }

    #[test]
    fn test_toast_state_clear() {
        let mut state = ToastState::new();
        state.push("One", ToastLevel::Info);
        state.push("Two", ToastLevel::Success);
        
        state.clear();
        assert!(state.is_empty());
    }

    #[test]
    fn test_toast_state_error_convenience() {
        let mut state = ToastState::new();
        state.error("Error occurred");
        
        assert_eq!(state.len(), 1);
        let toast = state.toasts.front().unwrap();
        assert_eq!(toast.level, ToastLevel::Error);
        assert_eq!(toast.duration, Duration::from_secs(8));
    }
}
