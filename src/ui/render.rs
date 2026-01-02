use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
    Frame,
};

use crate::app::{App, ConnectState};
use crate::config::Config;
use crate::llm::COPILOT_MODELS;
use crate::message::Role;

use super::gradient::gradient_color;
use super::menu::render_menu;
use super::text::{apply_miami_gradient_to_line, wrap_text};
use super::toast::render_toasts;

/// Main UI rendering function.
pub fn ui(f: &mut Frame, app: &mut App, config: &Config) {
    let colors = &config.colors;
    let behavior = &config.behavior;
    let theme = &config.theme;
    let miami = colors.miami_colors();
    let (chat_start, chat_end) = colors.chat_gradient();
    let (input_start, input_end) = colors.input_gradient();

    let border_color = Color::Black;
    let bg_color = theme.bg_primary();

    // Fill entire background with border color to create thick border effect
    let background = Block::default()
        .style(Style::default().bg(border_color));
    f.render_widget(background, f.size());

    // Inner area with margin to create thick border (2 chars on sides, 1 on top/bottom)
    let inner_area = f.size().inner(&Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Dark background for inner content area
    let inner_bg = Block::default()
        .style(Style::default().bg(bg_color));
    f.render_widget(inner_bg, inner_area);

    // Create layout: chat area (top) and input area (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Chat messages
            Constraint::Length(3), // Input box
        ])
        .split(inner_area);

    // Update scroll state with total message count
    let total_messages = app.chat.messages.len();
    app.update_scroll_state(total_messages);

    // Increment animation frame if banner animation is not complete
    if !app.animation.banner_complete && !app.chat.messages.is_empty() {
        let banner_len = app.chat.messages[0].content.len();
        if app.animation.banner_frame < banner_len {
            app.animation.banner_frame += behavior.animation_chars_per_frame;
        } else {
            app.animation.banner_complete = true;
        }
    }

    // Render chat messages (skip based on scroll offset)
    let messages: Vec<ListItem> = app
        .chat
        .messages
        .iter()
        .enumerate()
        .skip(app.scroll.offset)
        .flat_map(|(_msg_idx, msg)| {
            let is_banner = msg.is_system_banner();

            // Apply Miami gradient to banner, regular colors to other messages
            let message_content = if is_banner && !app.animation.banner_complete {
                // Animated reveal: only show characters up to current frame
                msg.content
                    .chars()
                    .take(app.animation.banner_frame)
                    .collect::<String>()
            } else {
                msg.content.clone()
            };

            let role_prefix = msg.role.prefix();

            // Wrap long messages
            let wrapped_lines =
                wrap_text(&message_content, chunks[0].width.saturating_sub(4) as usize);

            let mut items = Vec::new();
            for (i, line) in wrapped_lines.iter().enumerate() {
                if is_banner {
                    // Apply Miami gradient to banner (no role prefix)
                    let miami_line = apply_miami_gradient_to_line(line, i, &miami);
                    items.push(ListItem::new(miami_line));
                } else {
                    // Regular message styling
                    let style = match msg.role {
                        Role::User => Style::default().fg(Color::Cyan),
                        Role::Assistant => Style::default().fg(Color::Green),
                    };

                    if i == 0 {
                        items.push(ListItem::new(Line::from(vec![
                            Span::styled(role_prefix, style.add_modifier(Modifier::BOLD)),
                            Span::styled(line.clone(), style),
                        ])));
                    } else {
                        items.push(ListItem::new(Line::from(Span::styled(
                            format!("         {}", line),
                            style,
                        ))));
                    }
                }
            }

            // Add empty line between messages
            if !is_banner {
                items.push(ListItem::new(Line::from("")));
            }
            items
        })
        .collect();

    // Purple to Blue gradient for chat area
    let mid_color = gradient_color(chat_start, chat_end, 0.5);
    let messages_list = List::new(messages).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(mid_color))
    );

    f.render_widget(messages_list, chunks[0]);

    // Render scrollbar with smooth Unicode characters and gradient
    let scroll_position = if total_messages > 0 {
        app.scroll.offset as f32 / total_messages as f32
    } else {
        0.0
    };
    
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .track_symbol(Some("░"))
        .thumb_symbol("█")
        .style(Style::default().fg(gradient_color(chat_start, chat_end, scroll_position)));

    f.render_stateful_widget(
        scrollbar,
        chunks[0].inner(&Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.scroll.scrollbar,
    );

    // Render input box with left border only, dark grey background, blinking cursor
    let cursor_char = if app.animation.cursor_visible { "▎" } else { " " };
    
    let input_text = if app.chat.cursor_position < app.chat.input.len() {
        Line::from(vec![
            Span::raw(&app.chat.input[..app.chat.cursor_position]),
            Span::styled(cursor_char, Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK)),
            Span::raw(&app.chat.input[app.chat.cursor_position..]),
        ])
    } else {
        Line::from(vec![
            Span::raw(&app.chat.input),
            Span::styled(cursor_char, Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK)),
        ])
    };

    // Dark grey background, left border only with gradient color
    let input_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(gradient_color(input_start, input_end, 0.5)))
        .style(Style::default().bg(theme.bg_secondary()));

    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::White))
        .block(input_block)
        .wrap(Wrap { trim: false });

    f.render_widget(input, chunks[1]);

    // Render menu overlay if visible
    if app.menu.visible {
        render_menu(f, app, &miami, config);
    }

    // Render toast notifications (above main content, but below dialogs)
    render_toasts(f, &app.toasts);

    // Render connection dialog if active (on top of everything)
    if app.connect.is_active() {
        render_connect_dialog(f, app);
    }
}

/// Calculate a centered rectangle within an area.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = (area.width as u32 * percent_x as u32 / 100) as u16;
    let height = (area.height as u32 * percent_y as u32 / 100) as u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

/// Render the connection dialog based on current state.
pub fn render_connect_dialog(f: &mut Frame, app: &App) {
    match &app.connect {
        ConnectState::None => {}
        ConnectState::ExistingCredential {
            provider,
            masked_key,
            current_model,
            selected,
        } => {
            render_existing_credential_dialog(
                f,
                provider.display_name(),
                masked_key,
                current_model.as_deref(),
                *selected,
            );
        }
        ConnectState::SelectingMethod { provider, selected } => {
            render_selecting_method_dialog(f, provider.display_name(), *selected);
        }
        ConnectState::EnteringApiKey {
            provider,
            input,
            cursor,
            error,
        } => {
            render_entering_api_key_dialog(
                f,
                provider.display_name(),
                input,
                *cursor,
                error.as_deref(),
            );
        }
        ConnectState::ValidatingKey { provider, .. } => {
            render_validating_dialog(f, provider.display_name());
        }
        ConnectState::OAuthPending { auth_dialog, .. }
        | ConnectState::OAuthPolling { auth_dialog, .. } => {
            auth_dialog.render(f, f.size());
        }
        ConnectState::SelectingModel { selected, .. } => {
            render_model_selection_dialog(f, *selected);
        }
    }
}

/// Render the "existing credential" dialog.
fn render_existing_credential_dialog(
    f: &mut Frame,
    provider_name: &str,
    masked_key: &str,
    current_model: Option<&str>,
    selected: usize,
) {
    let area = centered_rect(50, 40, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" Already Connected to {} ", provider_name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Layout: info line, spacer, options, spacer, hints
    let chunks = Layout::vertical([
        Constraint::Length(2), // Current key info
        Constraint::Length(1), // Spacer
        Constraint::Min(3),    // Options
        Constraint::Length(1), // Hints
    ])
    .split(inner);

    // Current key info
    let info = Paragraph::new(format!("Current key: {}", masked_key))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(info, chunks[0]);

    // Options
    let options = vec!["Use existing key", "Enter new key", "Cancel"];
    let lines: Vec<Line> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let style = if i == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if i == selected { "> " } else { "  " };
            Line::from(Span::styled(format!("{}{}", prefix, opt), style))
        })
        .collect();
    let options_widget = Paragraph::new(lines);
    f.render_widget(options_widget, chunks[2]);

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
    f.render_widget(hints_widget, chunks[3]);
}

/// Render the "selecting method" dialog.
fn render_selecting_method_dialog(f: &mut Frame, provider_name: &str, selected: usize) {
    let area = centered_rect(50, 40, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" Connect to {} ", provider_name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Layout: options, hints
    let chunks = Layout::vertical([
        Constraint::Min(3),    // Options
        Constraint::Length(1), // Hints
    ])
    .split(inner);

    // Options
    let options = vec![
        "Enter API Key manually",
        "Create API Key (opens browser)",
        "Cancel",
    ];
    let lines: Vec<Line> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let style = if i == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if i == selected { "> " } else { "  " };
            Line::from(Span::styled(format!("{}{}", prefix, opt), style))
        })
        .collect();
    let options_widget = Paragraph::new(lines);
    f.render_widget(options_widget, chunks[0]);

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
    f.render_widget(hints_widget, chunks[1]);
}

/// Render the "entering API key" dialog.
fn render_entering_api_key_dialog(
    f: &mut Frame,
    provider_name: &str,
    input: &str,
    cursor: usize,
    error: Option<&str>,
) {
    let height_percent = if error.is_some() { 45 } else { 35 };
    let area = centered_rect(60, height_percent, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" Enter {} API Key ", provider_name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Layout with optional error
    let constraints = if error.is_some() {
        vec![
            Constraint::Length(1), // Input label
            Constraint::Length(1), // Input field
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hints
            Constraint::Length(1), // Spacer
            Constraint::Min(1),    // Error
        ]
    } else {
        vec![
            Constraint::Length(1), // Input label
            Constraint::Length(1), // Input field
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hints
            Constraint::Min(0),    // Filler
        ]
    };
    let chunks = Layout::vertical(constraints).split(inner);

    // Input label
    let label = Paragraph::new("API Key:").style(Style::default().fg(Color::Gray));
    f.render_widget(label, chunks[0]);

    // Input field with cursor
    let display_input = if cursor < input.len() {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Cyan)),
            Span::raw(&input[..cursor]),
            Span::styled("▎", Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK)),
            Span::raw(&input[cursor..]),
        ])
    } else {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Cyan)),
            Span::raw(input),
            Span::styled("▎", Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK)),
        ])
    };
    let input_widget = Paragraph::new(display_input).style(Style::default().fg(Color::White));
    f.render_widget(input_widget, chunks[1]);

    // Hints
    let hints = Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
        Span::raw(" Validate & Save  "),
        Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
        Span::raw(" Cancel"),
    ]);
    let hints_widget = Paragraph::new(hints).style(Style::default().fg(Color::Gray));
    f.render_widget(hints_widget, chunks[3]);

    // Error message if present
    if let Some(err) = error {
        let error_widget = Paragraph::new(err)
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: true });
        f.render_widget(error_widget, chunks[5]);
    }
}

/// Render the "validating key" dialog.
fn render_validating_dialog(f: &mut Frame, provider_name: &str) {
    let area = centered_rect(50, 25, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Validating API Key... ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = Paragraph::new(format!("Testing connection to {}", provider_name))
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    f.render_widget(text, inner);
}

/// Render the model selection dialog for GitHub Copilot.
fn render_model_selection_dialog(f: &mut Frame, selected: usize) {
    let area = centered_rect(50, 50, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Select Model ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Layout: models list, hints
    let chunks = Layout::vertical([
        Constraint::Min(3),    // Models
        Constraint::Length(1), // Hints
    ])
    .split(inner);

    // Model options
    let lines: Vec<Line> = COPILOT_MODELS
        .iter()
        .enumerate()
        .map(|(i, (display_name, _api_id))| {
            let style = if i == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if i == selected { "> " } else { "  " };
            Line::from(Span::styled(format!("{}{}", prefix, display_name), style))
        })
        .collect();
    let options_widget = Paragraph::new(lines);
    f.render_widget(options_widget, chunks[0]);

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
    f.render_widget(hints_widget, chunks[1]);
}
