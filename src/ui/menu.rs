use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, InputMode};
use crate::config::MiamiColors;

/// Render the popup menu overlay with modal effect.
pub fn render_menu(f: &mut Frame, app: &App, _miami: &MiamiColors) {
    let menu_items = App::menu_items();
    let area = f.size();

    // Semi-transparent backdrop overlay (darken the background)
    let backdrop = Block::default()
        .style(Style::default().bg(Color::Rgb(0, 0, 0)));
    f.render_widget(backdrop, area);

    // Calculate menu size
    let menu_width = 54u16;
    let menu_height = (menu_items.len() as u16) + 8; // +8 for borders, padding, title, hints

    // Center the menu
    let menu_x = (area.width.saturating_sub(menu_width)) / 2;
    let menu_y = (area.height.saturating_sub(menu_height)) / 2;

    // Shadow layer (offset by 2,1)
    let shadow_area = Rect {
        x: menu_x + 2,
        y: menu_y + 1,
        width: menu_width,
        height: menu_height,
    };
    let shadow = Block::default()
        .style(Style::default().bg(Color::Rgb(10, 10, 15)));
    f.render_widget(shadow, shadow_area);

    // Main menu area
    let menu_area = Rect {
        x: menu_x,
        y: menu_y,
        width: menu_width,
        height: menu_height,
    };

    // Clear the menu area first
    f.render_widget(Clear, menu_area);

    // Build menu content
    let mut menu_lines = vec![
        Line::from(""), // Top padding
    ];

    let is_editing = app.is_menu_input_mode();

    for (i, item) in menu_items.iter().enumerate() {
        let is_selected = i == app.menu_selected;
        let is_config_field = matches!(*item, "API Key" | "API Base URL" | "Model");
        
        // Check if this field is being edited
        let is_being_edited = is_selected && match (app.input_mode.clone(), *item) {
            (InputMode::ApiKey, "API Key") => true,
            (InputMode::ApiBase, "API Base URL") => true,
            (InputMode::Model, "Model") => true,
            _ => false,
        };

        if is_being_edited {
            // Show input field
            let display_value = if *item == "API Key" {
                // Mask the input for API key
                "*".repeat(app.menu_input.len())
            } else {
                app.menu_input.clone()
            };
            
            menu_lines.push(Line::from(vec![
                Span::styled(
                    "  ▸ ",
                    Style::default()
                        .fg(Color::Rgb(0, 255, 255))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}: ", item),
                    Style::default()
                        .fg(Color::Rgb(0, 255, 255))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}_", display_value),
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(50, 50, 60)),
                ),
            ]));
        } else if is_selected {
            // Selected: highlighted row with accent color
            let bg_color = Color::Rgb(60, 60, 80);
            let fg_color = Color::Rgb(0, 255, 255); // Cyan

            let mut spans = vec![
                Span::styled(
                    "  ▸ ",
                    Style::default()
                        .fg(fg_color)
                        .bg(bg_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}", item),
                    Style::default()
                        .fg(fg_color)
                        .bg(bg_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ];

            // Add current value for config fields
            if is_config_field {
                let value = app.get_config_display(item);
                let remaining_width = 46 - item.len() - value.len();
                spans.push(Span::styled(
                    format!("{:>width$}", "", width = remaining_width.max(1)),
                    Style::default().bg(bg_color),
                ));
                spans.push(Span::styled(
                    format!("{}", value),
                    Style::default()
                        .fg(Color::Rgb(150, 150, 170))
                        .bg(bg_color),
                ));
            } else {
                let remaining = 50 - item.len() - 4;
                spans.push(Span::styled(
                    format!("{:width$}", "", width = remaining),
                    Style::default().bg(bg_color),
                ));
            }

            menu_lines.push(Line::from(spans));
        } else {
            // Unselected: dimmer text
            let fg_color = Color::Rgb(140, 140, 160);

            let mut spans = vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("{}", item),
                    Style::default().fg(fg_color),
                ),
            ];

            // Add current value for config fields
            if is_config_field {
                let value = app.get_config_display(item);
                let remaining_width = 46 - item.len() - value.len();
                spans.push(Span::styled(
                    format!("{:>width$}", "", width = remaining_width.max(1)),
                    Style::default(),
                ));
                spans.push(Span::styled(
                    format!("{}", value),
                    Style::default().fg(Color::Rgb(100, 100, 120)),
                ));
            }

            menu_lines.push(Line::from(spans));
        }
    }

    menu_lines.push(Line::from("")); // Bottom padding

    // Separator line
    menu_lines.push(Line::from(Span::styled(
        "  ────────────────────────────────────────────────",
        Style::default().fg(Color::Rgb(60, 60, 80)),
    )));

    // Keyboard hints
    if is_editing {
        menu_lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("⏎", Style::default().fg(Color::Rgb(0, 255, 128)).add_modifier(Modifier::BOLD)),
            Span::styled(" Confirm  ", Style::default().fg(Color::Rgb(100, 100, 120))),
            Span::styled("Esc", Style::default().fg(Color::Rgb(255, 100, 100)).add_modifier(Modifier::BOLD)),
            Span::styled(" Cancel", Style::default().fg(Color::Rgb(100, 100, 120))),
        ]));
    } else {
        menu_lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("↑↓", Style::default().fg(Color::Rgb(0, 255, 255)).add_modifier(Modifier::BOLD)),
            Span::styled(" Navigate  ", Style::default().fg(Color::Rgb(100, 100, 120))),
            Span::styled("⏎", Style::default().fg(Color::Rgb(0, 255, 128)).add_modifier(Modifier::BOLD)),
            Span::styled(" Edit/Select  ", Style::default().fg(Color::Rgb(100, 100, 120))),
            Span::styled("Esc", Style::default().fg(Color::Rgb(255, 100, 100)).add_modifier(Modifier::BOLD)),
            Span::styled(" Close", Style::default().fg(Color::Rgb(100, 100, 120))),
        ]));
    }

    // Menu block with double border for modal effect
    let menu_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(80, 80, 100)))
        .title(Span::styled(
            " Settings ",
            Style::default()
                .fg(Color::Rgb(0, 255, 255))
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(Color::Rgb(25, 25, 35)));

    let menu_text = Paragraph::new(menu_lines)
        .alignment(Alignment::Left)
        .block(menu_block);

    f.render_widget(menu_text, menu_area);
}
