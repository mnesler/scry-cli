use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::config::MiamiColors;
use super::gradient::gradient_color;

/// Render the popup menu overlay.
pub fn render_menu(f: &mut Frame, app: &App, miami: &MiamiColors) {
    let menu_items = App::menu_items();

    // Calculate menu size
    let menu_width = 40;
    let menu_height = (menu_items.len() + 4) as u16; // +4 for borders and padding

    // Center the menu
    let area = f.size();
    let menu_x = (area.width.saturating_sub(menu_width)) / 2;
    let menu_y = (area.height.saturating_sub(menu_height)) / 2;

    let menu_area = Rect {
        x: menu_x,
        y: menu_y,
        width: menu_width,
        height: menu_height,
    };

    // Create menu items with selection highlight
    let mut menu_lines = vec![
        Line::from(""), // Empty line for spacing
    ];

    for (i, item) in menu_items.iter().enumerate() {
        let is_selected = i == app.menu_selected;

        if is_selected {
            // Selected item: Miami gradient with bold
            let gradient_pos = i as f32 / menu_items.len() as f32;
            let selected_color = if gradient_pos < 0.33 {
                gradient_color(miami.pink, miami.purple, gradient_pos * 3.0)
            } else if gradient_pos < 0.66 {
                gradient_color(miami.purple, miami.cyan, (gradient_pos - 0.33) * 3.0)
            } else {
                gradient_color(miami.cyan, miami.orange, (gradient_pos - 0.66) * 3.0)
            };

            menu_lines.push(Line::from(vec![
                Span::styled(
                    "  > ",
                    Style::default()
                        .fg(selected_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<30}", item),
                    Style::default()
                        .fg(selected_color)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
            ]));
        } else {
            // Unselected items: subtle gradient
            let gradient_pos = i as f32 / menu_items.len() as f32;
            let item_color = if gradient_pos < 0.5 {
                gradient_color(miami.purple, miami.cyan, gradient_pos * 2.0)
            } else {
                gradient_color(miami.cyan, miami.purple, (gradient_pos - 0.5) * 2.0)
            };

            menu_lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(format!("{:<30}", item), Style::default().fg(item_color)),
            ]));
        }
    }

    menu_lines.push(Line::from("")); // Empty line for spacing

    // Create the menu paragraph
    let menu_text = Paragraph::new(menu_lines).alignment(Alignment::Left).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(gradient_color(miami.pink, miami.cyan, 0.5)))
            .title(Span::styled(
                " MENU (Ctrl+P) ",
                Style::default()
                    .fg(gradient_color(miami.pink, miami.orange, 0.7))
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Color::Rgb(20, 20, 30))), // Dark background
    );

    f.render_widget(menu_text, menu_area);

    // Add hint at the bottom
    let hint_y = menu_area.y + menu_area.height;
    let hint_area = Rect {
        x: menu_x,
        y: hint_y,
        width: menu_width,
        height: 1,
    };

    let hint = Paragraph::new(Line::from(vec![
        Span::styled(
            "Up/Down",
            Style::default()
                .fg(Color::Rgb(miami.cyan.0, miami.cyan.1, miami.cyan.2))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Navigate  ", Style::default().fg(Color::White)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Rgb(miami.pink.0, miami.pink.1, miami.pink.2))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Select  ", Style::default().fg(Color::White)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Rgb(miami.orange.0, miami.orange.1, miami.orange.2))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Close", Style::default().fg(Color::White)),
    ]))
    .alignment(Alignment::Center);

    f.render_widget(hint, hint_area);
}
