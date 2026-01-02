//! Modal dialog system for user interaction.
//!
//! This module provides a reusable dialog system for displaying
//! overlays that require user interaction (confirmations, selections, etc.).

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Result of handling a key event in a dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResult {
    /// Continue showing the dialog.
    Continue,
    /// Close the dialog (cancelled).
    Cancel,
    /// Dialog confirmed with an optional value.
    Confirm(Option<String>),
    /// A selection was made (index into options).
    Select(usize),
}

/// A dialog action with label and key binding.
#[derive(Debug, Clone)]
pub struct DialogAction {
    /// Display label for the action.
    pub label: String,
    /// Key that triggers this action.
    pub key: KeyCode,
    /// Result when this action is triggered.
    pub result: DialogResult,
}

impl DialogAction {
    /// Create a new action.
    pub fn new(label: impl Into<String>, key: KeyCode, result: DialogResult) -> Self {
        Self {
            label: label.into(),
            key,
            result,
        }
    }

    /// Create a confirm action (Enter key).
    pub fn confirm(label: impl Into<String>) -> Self {
        Self::new(label, KeyCode::Enter, DialogResult::Confirm(None))
    }

    /// Create a cancel action (Esc key).
    pub fn cancel(label: impl Into<String>) -> Self {
        Self::new(label, KeyCode::Esc, DialogResult::Cancel)
    }

    /// Create a Yes action (y key).
    pub fn yes() -> Self {
        Self::new("Yes", KeyCode::Char('y'), DialogResult::Confirm(None))
    }

    /// Create a No action (n key).
    pub fn no() -> Self {
        Self::new("No", KeyCode::Char('n'), DialogResult::Cancel)
    }
}

/// Content to display in a dialog.
#[derive(Debug, Clone)]
pub enum DialogContent {
    /// Simple text message.
    Text(String),
    /// List of selectable options.
    Selection {
        items: Vec<String>,
        selected: usize,
    },
    /// Custom lines of text with formatting.
    Lines(Vec<Line<'static>>),
}

impl DialogContent {
    /// Create text content.
    pub fn text(message: impl Into<String>) -> Self {
        Self::Text(message.into())
    }

    /// Create selection content.
    pub fn selection(items: Vec<String>) -> Self {
        Self::Selection { items, selected: 0 }
    }

    /// Create selection with initial selection.
    pub fn selection_with_index(items: Vec<String>, selected: usize) -> Self {
        let max_index = items.len().saturating_sub(1);
        Self::Selection {
            items,
            selected: selected.min(max_index),
        }
    }
}

/// A modal dialog.
#[derive(Debug, Clone)]
pub struct Dialog {
    /// Dialog title.
    pub title: String,
    /// Content to display.
    pub content: DialogContent,
    /// Available actions.
    pub actions: Vec<DialogAction>,
    /// Width as percentage of screen (0.0 - 1.0).
    pub width_percent: u16,
    /// Height as percentage of screen (0.0 - 1.0).
    pub height_percent: u16,
}

impl Dialog {
    /// Create a new dialog.
    pub fn new(title: impl Into<String>, content: DialogContent) -> Self {
        Self {
            title: title.into(),
            content,
            actions: vec![DialogAction::confirm("OK"), DialogAction::cancel("Cancel")],
            width_percent: 60,
            height_percent: 40,
        }
    }

    /// Set the dialog actions.
    pub fn with_actions(mut self, actions: Vec<DialogAction>) -> Self {
        self.actions = actions;
        self
    }

    /// Set dialog size as percentage of screen.
    pub fn with_size(mut self, width_percent: u16, height_percent: u16) -> Self {
        self.width_percent = width_percent.min(100);
        self.height_percent = height_percent.min(100);
        self
    }

    /// Create an alert dialog with just OK button.
    pub fn alert(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, DialogContent::text(message))
            .with_actions(vec![DialogAction::confirm("OK")])
    }

    /// Create a confirmation dialog with Yes/No buttons.
    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, DialogContent::text(message))
            .with_actions(vec![DialogAction::yes(), DialogAction::no()])
    }

    /// Create a selection dialog.
    pub fn selection(title: impl Into<String>, options: Vec<String>) -> Self {
        Self::new(title, DialogContent::selection(options))
            .with_actions(vec![
                DialogAction::confirm("Select"),
                DialogAction::cancel("Cancel"),
            ])
    }

    /// Handle a key event. Returns the result of the action if one matches.
    pub fn handle_key(&mut self, key: KeyCode) -> DialogResult {
        // Handle selection navigation
        if let DialogContent::Selection { items, selected } = &mut self.content {
            match key {
                KeyCode::Up => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                    return DialogResult::Continue;
                }
                KeyCode::Down => {
                    if *selected < items.len().saturating_sub(1) {
                        *selected += 1;
                    }
                    return DialogResult::Continue;
                }
                KeyCode::Enter => {
                    return DialogResult::Select(*selected);
                }
                _ => {}
            }
        }

        // Check action keys
        for action in &self.actions {
            if action.key == key {
                return action.result.clone();
            }
        }

        DialogResult::Continue
    }

    /// Get the currently selected index for selection dialogs.
    pub fn selected_index(&self) -> Option<usize> {
        if let DialogContent::Selection { selected, .. } = &self.content {
            Some(*selected)
        } else {
            None
        }
    }

    /// Calculate the dialog area within the given frame area.
    pub fn area(&self, frame_area: Rect) -> Rect {
        let width = (frame_area.width as u32 * self.width_percent as u32 / 100) as u16;
        let height = (frame_area.height as u32 * self.height_percent as u32 / 100) as u16;
        let x = (frame_area.width.saturating_sub(width)) / 2;
        let y = (frame_area.height.saturating_sub(height)) / 2;
        Rect::new(x, y, width, height)
    }

    /// Render the dialog to the frame.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let dialog_area = self.area(area);

        // Clear the area behind the dialog
        frame.render_widget(Clear, dialog_area);

        // Create the dialog block
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Layout: content area and action hints
        let chunks = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

        // Render content
        match &self.content {
            DialogContent::Text(text) => {
                let paragraph = Paragraph::new(text.as_str())
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(Color::White));
                frame.render_widget(paragraph, chunks[0]);
            }
            DialogContent::Selection { items, selected } => {
                let lines: Vec<Line> = items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let style = if i == *selected {
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };
                        let prefix = if i == *selected { "> " } else { "  " };
                        Line::from(Span::styled(format!("{}{}", prefix, item), style))
                    })
                    .collect();
                let paragraph = Paragraph::new(lines);
                frame.render_widget(paragraph, chunks[0]);
            }
            DialogContent::Lines(lines) => {
                let paragraph = Paragraph::new(lines.clone())
                    .wrap(Wrap { trim: true });
                frame.render_widget(paragraph, chunks[0]);
            }
        }

        // Render action hints
        let hints: Vec<Span> = self
            .actions
            .iter()
            .flat_map(|action| {
                let key_name = match action.key {
                    KeyCode::Enter => "Enter".to_string(),
                    KeyCode::Esc => "Esc".to_string(),
                    KeyCode::Char(c) => c.to_string(),
                    _ => "?".to_string(),
                };
                vec![
                    Span::styled(
                        format!("[{}]", key_name),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(format!(" {}  ", action.label)),
                ]
            })
            .collect();
        let hints_line = Line::from(hints);
        let hints_paragraph = Paragraph::new(hints_line)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(hints_paragraph, chunks[1]);
    }
}

/// Dialog state for the application.
#[derive(Debug, Default)]
pub struct DialogState {
    /// Currently active dialog, if any.
    pub active: Option<Dialog>,
}

impl DialogState {
    /// Show a dialog.
    pub fn show(&mut self, dialog: Dialog) {
        self.active = Some(dialog);
    }

    /// Close the current dialog.
    pub fn close(&mut self) {
        self.active = None;
    }

    /// Check if a dialog is active.
    pub fn has_dialog(&self) -> bool {
        self.active.is_some()
    }

    /// Handle a key event if a dialog is active.
    /// Returns Some(result) if the dialog handled the key, None if no dialog is active.
    pub fn handle_key(&mut self, key: KeyCode) -> Option<DialogResult> {
        if let Some(dialog) = &mut self.active {
            let result = dialog.handle_key(key);
            match &result {
                DialogResult::Continue => Some(result),
                _ => {
                    // Dialog is closing
                    Some(result)
                }
            }
        } else {
            None
        }
    }

    /// Render the dialog if active.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if let Some(dialog) = &self.active {
            dialog.render(frame, area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_action_confirm() {
        let action = DialogAction::confirm("OK");
        assert_eq!(action.key, KeyCode::Enter);
        assert_eq!(action.result, DialogResult::Confirm(None));
    }

    #[test]
    fn test_dialog_action_cancel() {
        let action = DialogAction::cancel("Cancel");
        assert_eq!(action.key, KeyCode::Esc);
        assert_eq!(action.result, DialogResult::Cancel);
    }

    #[test]
    fn test_dialog_action_yes_no() {
        let yes = DialogAction::yes();
        let no = DialogAction::no();
        assert_eq!(yes.key, KeyCode::Char('y'));
        assert_eq!(no.key, KeyCode::Char('n'));
    }

    #[test]
    fn test_dialog_content_text() {
        let content = DialogContent::text("Hello");
        match content {
            DialogContent::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected Text content"),
        }
    }

    #[test]
    fn test_dialog_content_selection() {
        let content = DialogContent::selection(vec!["A".to_string(), "B".to_string()]);
        match content {
            DialogContent::Selection { items, selected } => {
                assert_eq!(items.len(), 2);
                assert_eq!(selected, 0);
            }
            _ => panic!("Expected Selection content"),
        }
    }

    #[test]
    fn test_dialog_content_selection_with_index() {
        let content = DialogContent::selection_with_index(
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            1,
        );
        match content {
            DialogContent::Selection { selected, .. } => assert_eq!(selected, 1),
            _ => panic!("Expected Selection content"),
        }
    }

    #[test]
    fn test_dialog_content_selection_index_clamped() {
        let content = DialogContent::selection_with_index(
            vec!["A".to_string(), "B".to_string()],
            10, // Out of bounds
        );
        match content {
            DialogContent::Selection { selected, .. } => assert_eq!(selected, 1), // Clamped to last
            _ => panic!("Expected Selection content"),
        }
    }

    #[test]
    fn test_dialog_new() {
        let dialog = Dialog::new("Test", DialogContent::text("Message"));
        assert_eq!(dialog.title, "Test");
        assert_eq!(dialog.actions.len(), 2); // Default OK + Cancel
    }

    #[test]
    fn test_dialog_alert() {
        let dialog = Dialog::alert("Alert", "Something happened");
        assert_eq!(dialog.title, "Alert");
        assert_eq!(dialog.actions.len(), 1); // Just OK
    }

    #[test]
    fn test_dialog_confirm() {
        let dialog = Dialog::confirm("Confirm", "Are you sure?");
        assert_eq!(dialog.title, "Confirm");
        assert_eq!(dialog.actions.len(), 2); // Yes + No
    }

    #[test]
    fn test_dialog_selection() {
        let dialog = Dialog::selection("Choose", vec!["Option 1".to_string(), "Option 2".to_string()]);
        assert_eq!(dialog.title, "Choose");
        assert!(matches!(dialog.content, DialogContent::Selection { .. }));
    }

    #[test]
    fn test_dialog_with_size() {
        let dialog = Dialog::alert("Test", "Message").with_size(80, 50);
        assert_eq!(dialog.width_percent, 80);
        assert_eq!(dialog.height_percent, 50);
    }

    #[test]
    fn test_dialog_with_size_clamped() {
        let dialog = Dialog::alert("Test", "Message").with_size(150, 200);
        assert_eq!(dialog.width_percent, 100);
        assert_eq!(dialog.height_percent, 100);
    }

    #[test]
    fn test_dialog_handle_key_action() {
        let mut dialog = Dialog::alert("Test", "Message");
        
        // Enter should confirm
        let result = dialog.handle_key(KeyCode::Enter);
        assert_eq!(result, DialogResult::Confirm(None));
    }

    #[test]
    fn test_dialog_handle_key_no_match() {
        let mut dialog = Dialog::alert("Test", "Message");
        
        // Random key should continue
        let result = dialog.handle_key(KeyCode::Char('x'));
        assert_eq!(result, DialogResult::Continue);
    }

    #[test]
    fn test_dialog_selection_navigation() {
        let mut dialog = Dialog::selection(
            "Choose",
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
        );

        assert_eq!(dialog.selected_index(), Some(0));

        // Navigate down
        dialog.handle_key(KeyCode::Down);
        assert_eq!(dialog.selected_index(), Some(1));

        dialog.handle_key(KeyCode::Down);
        assert_eq!(dialog.selected_index(), Some(2));

        // Can't go past end
        dialog.handle_key(KeyCode::Down);
        assert_eq!(dialog.selected_index(), Some(2));

        // Navigate up
        dialog.handle_key(KeyCode::Up);
        assert_eq!(dialog.selected_index(), Some(1));
    }

    #[test]
    fn test_dialog_selection_enter() {
        let mut dialog = Dialog::selection(
            "Choose",
            vec!["A".to_string(), "B".to_string()],
        );

        dialog.handle_key(KeyCode::Down); // Select index 1
        let result = dialog.handle_key(KeyCode::Enter);
        assert_eq!(result, DialogResult::Select(1));
    }

    #[test]
    fn test_dialog_selected_index_text() {
        let dialog = Dialog::alert("Test", "Message");
        assert_eq!(dialog.selected_index(), None);
    }

    #[test]
    fn test_dialog_area_calculation() {
        let dialog = Dialog::alert("Test", "Message").with_size(50, 50);
        let frame_area = Rect::new(0, 0, 100, 100);
        let area = dialog.area(frame_area);
        
        assert_eq!(area.width, 50);
        assert_eq!(area.height, 50);
        assert_eq!(area.x, 25); // Centered
        assert_eq!(area.y, 25); // Centered
    }

    #[test]
    fn test_dialog_state_default() {
        let state = DialogState::default();
        assert!(!state.has_dialog());
    }

    #[test]
    fn test_dialog_state_show_close() {
        let mut state = DialogState::default();
        
        state.show(Dialog::alert("Test", "Message"));
        assert!(state.has_dialog());
        
        state.close();
        assert!(!state.has_dialog());
    }

    #[test]
    fn test_dialog_state_handle_key_no_dialog() {
        let mut state = DialogState::default();
        let result = state.handle_key(KeyCode::Enter);
        assert!(result.is_none());
    }

    #[test]
    fn test_dialog_state_handle_key_with_dialog() {
        let mut state = DialogState::default();
        state.show(Dialog::alert("Test", "Message"));
        
        let result = state.handle_key(KeyCode::Char('x'));
        assert_eq!(result, Some(DialogResult::Continue));
        
        let result = state.handle_key(KeyCode::Enter);
        assert_eq!(result, Some(DialogResult::Confirm(None)));
    }
}
