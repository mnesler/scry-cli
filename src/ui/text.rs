use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::config::MiamiColors;
use super::gradient::gradient_color;

/// Wrap text to fit within a given width.
///
/// # Arguments
/// * `text` - The text to wrap
/// * `width` - Maximum width per line
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > width && !current_line.is_empty() {
            lines.push(current_line.clone());
            current_line.clear();
        }

        if !current_line.is_empty() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Apply Miami gradient colors to a line of text.
///
/// # Arguments
/// * `text` - The text to colorize
/// * `line_index` - Index of the line (affects gradient offset)
/// * `colors` - Miami color palette to use
pub fn apply_miami_gradient_to_line(text: &str, line_index: usize, colors: &MiamiColors) -> Line<'static> {
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();

    if total_chars == 0 {
        return Line::from("");
    }

    let mut spans = Vec::new();

    for (i, ch) in chars.iter().enumerate() {
        // Calculate gradient position based on character position and line
        let position = (i as f32 / total_chars.max(1) as f32 + line_index as f32 * 0.1) % 1.0;

        // Cycle through Miami colors
        let color = if position < 0.25 {
            gradient_color(colors.pink, colors.purple, position * 4.0)
        } else if position < 0.5 {
            gradient_color(colors.purple, colors.cyan, (position - 0.25) * 4.0)
        } else if position < 0.75 {
            gradient_color(colors.cyan, colors.orange, (position - 0.5) * 4.0)
        } else {
            gradient_color(colors.orange, colors.pink, (position - 0.75) * 4.0)
        };

        // Make box drawing and block characters bold for emphasis
        let style = if matches!(ch, '█' | '╔' | '╗' | '╚' | '╝' | '║' | '═' | '╠' | '╣' | '╦' | '╩' | '╬') {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };

        spans.push(Span::styled(ch.to_string(), style));
    }

    Line::from(spans)
}
