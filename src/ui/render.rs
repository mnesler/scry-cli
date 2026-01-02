use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
    Frame,
};

use crate::app::App;
use crate::config::Config;
use crate::message::Role;

use super::gradient::gradient_color;
use super::menu::render_menu;
use super::text::{apply_miami_gradient_to_line, wrap_text};

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
}
