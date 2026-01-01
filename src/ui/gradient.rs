use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders},
};

/// Create a gradient color between two RGB values.
///
/// # Arguments
/// * `start` - Starting RGB color
/// * `end` - Ending RGB color
/// * `position` - Position in gradient (0.0 to 1.0)
pub fn gradient_color(start: (u8, u8, u8), end: (u8, u8, u8), position: f32) -> Color {
    let r = (start.0 as f32 + (end.0 as f32 - start.0 as f32) * position) as u8;
    let g = (start.1 as f32 + (end.1 as f32 - start.1 as f32) * position) as u8;
    let b = (start.2 as f32 + (end.2 as f32 - start.2 as f32) * position) as u8;
    Color::Rgb(r, g, b)
}

/// Create a gradient border block with a title.
///
/// # Arguments
/// * `title` - Block title
/// * `_area` - Layout area (unused, kept for API compatibility)
/// * `start_color` - Gradient start RGB color
/// * `end_color` - Gradient end RGB color
pub fn gradient_block<'a>(
    title: &'a str,
    _area: Rect,
    start_color: (u8, u8, u8),
    end_color: (u8, u8, u8),
) -> Block<'a> {
    // Use the middle color of the gradient
    let mid_color = gradient_color(start_color, end_color, 0.5);

    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(mid_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(gradient_color(start_color, end_color, 0.8))
                .add_modifier(Modifier::BOLD),
        ))
}
