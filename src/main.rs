use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Terminal,
};
use std::io;
use std::env;

#[derive(Debug, Clone, Copy, PartialEq)]
enum TerminalCapabilities {
    Kitty,      // Kitty terminal with all features
    Modern,     // GNOME Terminal, Alacritty, iTerm2, etc.
    Basic,      // Standard xterm/fallback
}

impl TerminalCapabilities {
    fn detect() -> Self {
        let term = env::var("TERM").unwrap_or_default();
        let term_program = env::var("TERM_PROGRAM").unwrap_or_default();

        if term.contains("kitty") {
            TerminalCapabilities::Kitty
        } else if term.contains("256color")
               || term.contains("xterm-256")
               || term_program.contains("iTerm")
               || term_program.contains("gnome-terminal")
               || term_program.contains("vscode") {
            TerminalCapabilities::Modern
        } else {
            TerminalCapabilities::Basic
        }
    }

    fn supports_hyperlinks(&self) -> bool {
        matches!(self, TerminalCapabilities::Kitty | TerminalCapabilities::Modern)
    }

    fn name(&self) -> &str {
        match self {
            TerminalCapabilities::Kitty => "Kitty",
            TerminalCapabilities::Modern => "Modern",
            TerminalCapabilities::Basic => "Basic",
        }
    }
}

#[derive(Clone)]
struct Message {
    role: String,  // "user" or "assistant"
    content: String,
}

struct App {
    messages: Vec<Message>,
    input: String,
    cursor_position: usize,
    scroll_offset: usize,
    scroll_state: ScrollbarState,
    terminal_caps: TerminalCapabilities,
}

impl App {
    fn new() -> App {
        let terminal_caps = TerminalCapabilities::detect();

        let welcome_msg = if terminal_caps.supports_hyperlinks() {
            format!(
                "Hello! I'm an echo bot. Type something and I'll repeat it back to you.\n\n\
                Terminal: {} (hyperlinks enabled ✓)\n\
                Try typing a URL like https://github.com to see clickable links!",
                terminal_caps.name()
            )
        } else {
            format!(
                "Hello! I'm an echo bot. Type something and I'll repeat it back to you.\n\n\
                Terminal: {} (basic mode)",
                terminal_caps.name()
            )
        };

        App {
            messages: vec![
                Message {
                    role: "assistant".to_string(),
                    content: welcome_msg,
                }
            ],
            input: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            scroll_state: ScrollbarState::default(),
            terminal_caps,
        }
    }

    fn submit_message(&mut self) {
        if !self.input.trim().is_empty() {
            // Add user message
            self.messages.push(Message {
                role: "user".to_string(),
                content: self.input.clone(),
            });

            // Echo it back as assistant
            self.messages.push(Message {
                role: "assistant".to_string(),
                content: format!("You said: {}", self.input),
            });

            // Clear input
            self.input.clear();
            self.cursor_position = 0;
        }
    }

    fn handle_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    fn handle_backspace(&mut self) {
        if self.cursor_position > 0 {
            self.input.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position += 1;
        }
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    fn scroll_down(&mut self, max_scroll: usize) {
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    fn scroll_page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }

    fn scroll_page_down(&mut self, max_scroll: usize) {
        self.scroll_offset = (self.scroll_offset + 10).min(max_scroll);
    }

    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    fn scroll_to_bottom(&mut self, max_scroll: usize) {
        self.scroll_offset = max_scroll;
    }

    fn update_scroll_state(&mut self, total_items: usize) {
        self.scroll_state = self.scroll_state.content_length(total_items);
        self.scroll_state = self.scroll_state.position(self.scroll_offset);
    }
}

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Run app
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Enter => {
                        app.submit_message();
                    }
                    KeyCode::Char(c) => {
                        app.handle_char(c);
                    }
                    KeyCode::Backspace => {
                        app.handle_backspace();
                    }
                    KeyCode::Left => {
                        app.move_cursor_left();
                    }
                    KeyCode::Right => {
                        app.move_cursor_right();
                    }
                    KeyCode::Up => {
                        app.scroll_up();
                    }
                    KeyCode::Down => {
                        // Calculate max scroll based on message count
                        let max_scroll = app.messages.len().saturating_sub(1);
                        app.scroll_down(max_scroll);
                    }
                    KeyCode::PageUp => {
                        app.scroll_page_up();
                    }
                    KeyCode::PageDown => {
                        let max_scroll = app.messages.len().saturating_sub(1);
                        app.scroll_page_down(max_scroll);
                    }
                    KeyCode::Home => {
                        app.scroll_to_top();
                    }
                    KeyCode::End => {
                        let max_scroll = app.messages.len().saturating_sub(1);
                        app.scroll_to_bottom(max_scroll);
                    }
                    KeyCode::Esc => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }
}

// Create a gradient color between two RGB values
fn gradient_color(start: (u8, u8, u8), end: (u8, u8, u8), position: f32) -> Color {
    let r = (start.0 as f32 + (end.0 as f32 - start.0 as f32) * position) as u8;
    let g = (start.1 as f32 + (end.1 as f32 - start.1 as f32) * position) as u8;
    let b = (start.2 as f32 + (end.2 as f32 - start.2 as f32) * position) as u8;
    Color::Rgb(r, g, b)
}

// Create a gradient border block
fn gradient_block(title: &str, _area: Rect, start_color: (u8, u8, u8), end_color: (u8, u8, u8)) -> Block<'_> {
    // For simplicity, use the middle color of the gradient
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

// Create a clickable hyperlink using OSC 8 escape codes
fn make_hyperlink(url: &str, text: &str) -> String {
    format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text)
}

// Process text to make URLs clickable
fn linkify_text(text: &str, supports_hyperlinks: bool) -> String {
    if !supports_hyperlinks {
        return text.to_string();
    }

    let mut result = String::new();
    let mut last_end = 0;

    // Simple URL detection regex pattern (basic implementation)
    let url_patterns = [
        ("https://", "https://"),
        ("http://", "http://"),
    ];

    let mut urls_found: Vec<(usize, usize, String)> = Vec::new();

    // Find all URLs in the text
    for (prefix, _) in &url_patterns {
        let mut search_start = 0;
        while let Some(start) = text[search_start..].find(prefix) {
            let actual_start = search_start + start;
            let remaining = &text[actual_start..];

            // Find the end of the URL (space, newline, or end of string)
            let end_pos = remaining
                .find(|c: char| c.is_whitespace() || c == ')' || c == ']' || c == '>')
                .unwrap_or(remaining.len());

            let url = &remaining[..end_pos];
            urls_found.push((actual_start, actual_start + end_pos, url.to_string()));

            search_start = actual_start + end_pos;
        }
    }

    // Sort URLs by position
    urls_found.sort_by_key(|&(start, _, _)| start);

    // Build the result with clickable links
    for (start, end, url) in urls_found {
        // Add text before the URL
        result.push_str(&text[last_end..start]);

        // Add clickable link
        result.push_str(&make_hyperlink(&url, &url));

        last_end = end;
    }

    // Add remaining text
    result.push_str(&text[last_end..]);

    result
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    // Create layout: chat area (top) and input area (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),      // Chat messages
            Constraint::Length(3),   // Input box
        ])
        .split(f.size());

    // Update scroll state with total message count
    let total_messages = app.messages.len();
    app.update_scroll_state(total_messages);

    // Render chat messages (skip based on scroll offset)
    let supports_hyperlinks = app.terminal_caps.supports_hyperlinks();
    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .skip(app.scroll_offset)
        .flat_map(|msg| {
            let style = if msg.role == "user" {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Green)
            };

            let role_prefix = if msg.role == "user" {
                "You: "
            } else {
                "Assistant: "
            };

            // Linkify URLs in the message content
            let linkified_content = linkify_text(&msg.content, supports_hyperlinks);

            // Wrap long messages
            let wrapped_lines = wrap_text(&linkified_content, chunks[0].width.saturating_sub(4) as usize);

            let mut items = Vec::new();
            for (i, line) in wrapped_lines.iter().enumerate() {
                if i == 0 {
                    items.push(
                        ListItem::new(Line::from(vec![
                            Span::styled(role_prefix, style.add_modifier(Modifier::BOLD)),
                            Span::styled(line.clone(), style),
                        ]))
                    );
                } else {
                    items.push(
                        ListItem::new(Line::from(Span::styled(
                            format!("         {}", line),
                            style,
                        )))
                    );
                }
            }

            // Add empty line between messages
            items.push(ListItem::new(Line::from("")));
            items
        })
        .collect();

    // Purple to Blue gradient for chat area
    let messages_list = List::new(messages)
        .block(gradient_block(
            " Chat (↑↓ PgUp/PgDn Home/End to scroll, Ctrl+C to quit) ",
            chunks[0],
            (147, 51, 234),  // Purple
            (59, 130, 246),  // Blue
        ));

    f.render_widget(messages_list, chunks[0]);

    // Render scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .track_symbol(Some("│"))
        .thumb_symbol("█")
        .style(Style::default().fg(gradient_color(
            (147, 51, 234),
            (59, 130, 246),
            0.5,
        )));

    f.render_stateful_widget(
        scrollbar,
        chunks[0].inner(&ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.scroll_state,
    );

    // Render input box
    let input_text = if app.cursor_position < app.input.len() {
        format!(
            "{}█{}",
            &app.input[..app.cursor_position],
            &app.input[app.cursor_position..]
        )
    } else {
        format!("{}█", app.input)
    };

    // Green to Cyan gradient for input area
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::Yellow))
        .block(gradient_block(
            " Your message ",
            chunks[1],
            (16, 185, 129),  // Green
            (6, 182, 212),   // Cyan
        ))
        .wrap(Wrap { trim: false });

    f.render_widget(input, chunks[1]);
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > width {
            if !current_line.is_empty() {
                lines.push(current_line.clone());
                current_line.clear();
            }
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
