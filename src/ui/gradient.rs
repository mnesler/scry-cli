use ratatui::style::Color;

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