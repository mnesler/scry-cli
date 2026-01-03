//! Dialog components for Anthropic OAuth authentication.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::auth::AnthropicAuthMethod;

/// Render the Anthropic authentication method selection dialog.
pub fn render_anthropic_method_dialog(f: &mut Frame, selected: usize) {
    let area = centered_rect(60, 50, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Connect to Anthropic ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Title text
            Constraint::Min(6),    // Options
            Constraint::Length(1), // Hints
        ])
        .split(inner);

    // Title
    let title_text = Paragraph::new("Select authentication method:")
        .style(Style::default().fg(Color::Gray));
    f.render_widget(title_text, chunks[0]);

    // Options
    let options = vec![
        ("Claude Pro/Max (OAuth)", "Sign in with Claude Pro or Max subscription"),
        ("Create API Key (OAuth)", "Create a new API key via OAuth"),
        ("Enter API Key", "Enter an existing API key manually"),
    ];

    let lines: Vec<Line> = options
        .iter()
        .enumerate()
        .flat_map(|(i, (title, desc))| {
            let is_selected = i == selected;
            let prefix = if is_selected { "> " } else { "  " };
            let title_style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            vec![
                Line::from(Span::styled(format!("{}{}", prefix, title), title_style)),
                Line::from(Span::styled(
                    format!("  {}", desc),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
            ]
        })
        .collect();

    let options_widget = Paragraph::new(lines);
    f.render_widget(options_widget, chunks[1]);

    // Hints
    let hints = Line::from(vec![
        Span::styled("[↑↓]", Style::default().fg(Color::Yellow)),
        Span::raw(" Navigate  "),
        Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
        Span::raw(" Select  "),
        Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
        Span::raw(" Cancel"),
    ]);
    let hints_widget = Paragraph::new(hints).style(Style::default().fg(Color::Gray));
    f.render_widget(hints_widget, chunks[2]);
}

/// Render the authorization code entry dialog.
pub fn render_auth_code_entry_dialog(
    f: &mut Frame,
    method: AnthropicAuthMethod,
    input: &str,
    _cursor: usize,
    error: Option<&str>,
) {
    let area = centered_rect(60, 45, f.size());
    f.render_widget(Clear, area);

    let method_name = match method {
        AnthropicAuthMethod::ClaudeProMax => "Claude Pro/Max",
        AnthropicAuthMethod::CreateApiKey => "Create API Key",
    };

    let block = Block::default()
        .title(format!(" {} Authentication ", method_name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Instructions
            Constraint::Length(3), // Input field
            Constraint::Length(2), // Error (if any)
            Constraint::Length(1), // Hints
        ])
        .split(inner);

    // Instructions
    let instructions = vec![
        Line::from(Span::styled(
            "The browser has been opened to the authorization page.",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "1. Authorize the application in your browser",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "2. Copy the authorization code shown",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "3. Paste it below and press Enter",
            Style::default().fg(Color::Gray),
        )),
    ];
    let instructions_widget = Paragraph::new(instructions);
    f.render_widget(instructions_widget, chunks[0]);

    // Input field
    let input_lines = vec![
        Line::from(Span::styled(
            "Authorization Code:",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            if input.is_empty() {
                "  (paste code here)".to_string()
            } else {
                format!("  {}", input)
            },
            if input.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            },
        )),
    ];
    let input_widget = Paragraph::new(input_lines);
    f.render_widget(input_widget, chunks[1]);

    // Error message
    if let Some(err) = error {
        let error_text = Paragraph::new(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
        f.render_widget(error_text, chunks[2]);
    }

    // Hints
    let hints = Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
        Span::raw(" Submit  "),
        Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
        Span::raw(" Cancel"),
    ]);
    let hints_widget = Paragraph::new(hints).style(Style::default().fg(Color::Gray));
    f.render_widget(hints_widget, chunks[3]);
}

/// Render the "exchanging code" dialog (loading state).
pub fn render_exchanging_code_dialog(f: &mut Frame) {
    let area = centered_rect(40, 20, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Authenticating ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "Exchanging authorization code...",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Please wait...",
            Style::default().fg(Color::Gray),
        )),
    ])
    .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(text, inner);
}

/// Calculate a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
