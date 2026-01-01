// Example of animated gradients - replace gradient_block function with this

use std::time::{SystemTime, UNIX_EPOCH};

// Add to App struct:
// animation_offset: f32,

// Add this to App::new():
// animation_offset: 0.0,

// In run_app, before terminal.draw:
// app.animation_offset = (app.animation_offset + 0.05) % 1.0;

fn gradient_block_animated(
    title: &str,
    _area: Rect,
    start_color: (u8, u8, u8),
    end_color: (u8, u8, u8),
    animation_offset: f32
) -> Block<'_> {
    // Shift the gradient position based on time
    let position = (animation_offset.sin() + 1.0) / 2.0; // 0.0 to 1.0

    let border_color = gradient_color(start_color, end_color, position);
    let title_color = gradient_color(start_color, end_color, 1.0 - position);

    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        ))
}

// Usage in ui():
// .block(gradient_block_animated(
//     " Chat (Ctrl+C or Esc to quit) ",
//     chunks[0],
//     (147, 51, 234),
//     (59, 130, 246),
//     app.animation_offset
// ))
