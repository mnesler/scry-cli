use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::config::{Config, MiamiColors};
use crate::llm::Provider;
use super::gradient::gradient_color;

/// Render the popup menu overlay with modal effect.
pub fn render_menu(f: &mut Frame, app: &App, miami: &MiamiColors, config: &Config) {
    if app.menu.in_submenu {
        render_provider_submenu(f, app, miami, config);
    } else {
        render_main_menu(f, app, miami, config);
    }
}

/// Render the main menu.
fn render_main_menu(f: &mut Frame, app: &App, miami: &MiamiColors, config: &Config) {
    let menu_items = App::menu_items();
    let area = f.size();
    let theme = &config.theme;

    // Semi-transparent backdrop overlay (darken the background)
    let backdrop = Block::default()
        .style(Style::default().bg(Color::Rgb(0, 0, 0)));
    f.render_widget(backdrop, area);

    // Calculate menu size
    let menu_width = 54u16;
    let menu_height = (menu_items.len() as u16) + 8; // +8 for borders, padding, hints

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
        .style(Style::default().bg(theme.menu_shadow()));
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

    // Use Miami colors for menu accent
    let accent_color = Color::Rgb(miami.cyan.0, miami.cyan.1, miami.cyan.2);
    let highlight_color = gradient_color(miami.pink, miami.cyan, 0.5);
    let selected_bg = theme.menu_selected_bg();
    let unselected_fg = theme.menu_unselected_fg();

    // Build menu content
    let mut menu_lines = vec![
        Line::from(""), // Top padding
    ];

    for (i, &item) in menu_items.iter().enumerate() {
        let is_selected = i == app.menu.selected;
        let label = item.label();
        let has_submenu = item.has_submenu();

        if is_selected {
            // Selected: highlighted row with accent color
            let suffix = if has_submenu { " \u{25b6}" } else { "" }; // Right arrow for submenu
            let remaining = 50 - label.len() - 4 - suffix.len();
            let spans = vec![
                Span::styled(
                    "  \u{25b8} ",
                    Style::default()
                        .fg(highlight_color)
                        .bg(selected_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(highlight_color)
                        .bg(selected_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    suffix.to_string(),
                    Style::default()
                        .fg(highlight_color)
                        .bg(selected_bg),
                ),
                Span::styled(
                    format!("{:width$}", "", width = remaining),
                    Style::default().bg(selected_bg),
                ),
            ];

            menu_lines.push(Line::from(spans));
        } else {
            // Unselected: dimmer text
            let suffix = if has_submenu { " \u{25b6}" } else { "" };
            let spans = vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    label.to_string(),
                    Style::default().fg(unselected_fg),
                ),
                Span::styled(
                    suffix.to_string(),
                    Style::default().fg(unselected_fg),
                ),
            ];

            menu_lines.push(Line::from(spans));
        }
    }

    menu_lines.push(Line::from("")); // Bottom padding

    // Separator line
    menu_lines.push(Line::from(Span::styled(
        "  ────────────────────────────────────────────────",
        Style::default().fg(theme.menu_separator()),
    )));

    // Keyboard hints
    menu_lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled("\u{2191}\u{2193}", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
        Span::styled(" Navigate  ", Style::default().fg(Color::Rgb(100, 100, 120))),
        Span::styled("\u{23ce}", Style::default().fg(Color::Rgb(0, 255, 128)).add_modifier(Modifier::BOLD)),
        Span::styled(" Select  ", Style::default().fg(Color::Rgb(100, 100, 120))),
        Span::styled("Esc", Style::default().fg(Color::Rgb(255, 100, 100)).add_modifier(Modifier::BOLD)),
        Span::styled(" Close", Style::default().fg(Color::Rgb(100, 100, 120))),
    ]));

    // Menu block with double border for modal effect
    let menu_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.menu_border()))
        .style(Style::default().bg(theme.menu_bg()));

    let menu_text = Paragraph::new(menu_lines)
        .alignment(Alignment::Left)
        .block(menu_block);

    f.render_widget(menu_text, menu_area);
}

/// Render the provider selection submenu.
fn render_provider_submenu(f: &mut Frame, app: &App, miami: &MiamiColors, config: &Config) {
    let providers = Provider::all();
    let area = f.size();
    let theme = &config.theme;

    // Semi-transparent backdrop overlay (darken the background)
    let backdrop = Block::default()
        .style(Style::default().bg(Color::Rgb(0, 0, 0)));
    f.render_widget(backdrop, area);

    // Calculate menu size
    let menu_width = 54u16;
    let menu_height = (providers.len() as u16) + 9; // +9 for borders, padding, title, hints

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
        .style(Style::default().bg(theme.menu_shadow()));
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

    // Use Miami colors for menu accent
    let accent_color = Color::Rgb(miami.cyan.0, miami.cyan.1, miami.cyan.2);
    let highlight_color = gradient_color(miami.pink, miami.cyan, 0.5);
    let selected_bg = theme.menu_selected_bg();
    let unselected_fg = theme.menu_unselected_fg();
    let current_provider = app.llm.config.provider;

    // Build menu content
    let mut menu_lines = vec![
        Line::from(""), // Top padding
        Line::from(Span::styled(
            "  Select Provider",
            Style::default().fg(accent_color).add_modifier(Modifier::BOLD),
        )),
        Line::from(""), // Spacing after title
    ];

    for (i, &provider) in providers.iter().enumerate() {
        let is_selected = i == app.menu.submenu_selected;
        let is_current = provider == current_provider;
        let label = provider.display_name();
        let check = if is_current { "\u{2713} " } else { "  " }; // Checkmark for current

        if is_selected {
            // Selected: highlighted row with accent color
            let remaining = 50 - label.len() - 4 - check.len();
            let spans = vec![
                Span::styled(
                    format!("  \u{25b8} {}", check),
                    Style::default()
                        .fg(highlight_color)
                        .bg(selected_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(highlight_color)
                        .bg(selected_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:width$}", "", width = remaining),
                    Style::default().bg(selected_bg),
                ),
            ];

            menu_lines.push(Line::from(spans));
        } else {
            // Unselected: dimmer text, but highlight current provider slightly
            let fg = if is_current { accent_color } else { unselected_fg };
            let spans = vec![
                Span::styled(format!("    {}", check), Style::default().fg(fg)),
                Span::styled(
                    label.to_string(),
                    Style::default().fg(fg),
                ),
            ];

            menu_lines.push(Line::from(spans));
        }
    }

    menu_lines.push(Line::from("")); // Bottom padding

    // Separator line
    menu_lines.push(Line::from(Span::styled(
        "  ────────────────────────────────────────────────",
        Style::default().fg(theme.menu_separator()),
    )));

    // Keyboard hints - different for submenu
    menu_lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled("\u{2191}\u{2193}", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
        Span::styled(" Navigate  ", Style::default().fg(Color::Rgb(100, 100, 120))),
        Span::styled("\u{23ce}", Style::default().fg(Color::Rgb(0, 255, 128)).add_modifier(Modifier::BOLD)),
        Span::styled(" Select  ", Style::default().fg(Color::Rgb(100, 100, 120))),
        Span::styled("Esc", Style::default().fg(Color::Rgb(255, 100, 100)).add_modifier(Modifier::BOLD)),
        Span::styled(" Back", Style::default().fg(Color::Rgb(100, 100, 120))),
    ]));

    // Menu block with double border for modal effect
    let menu_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.menu_border()))
        .style(Style::default().bg(theme.menu_bg()));

    let menu_text = Paragraph::new(menu_lines)
        .alignment(Alignment::Left)
        .block(menu_block);

    f.render_widget(menu_text, menu_area);
}
