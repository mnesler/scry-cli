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
    show_menu: bool,
    menu_selected: usize,
}

impl App {
    fn new() -> App {
        App {
            messages: vec![
                Message {
                    role: "assistant".to_string(),
                    content: "Hello! I'm an echo bot. Type something and I'll repeat it back to you.".to_string(),
                }
            ],
            input: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            scroll_state: ScrollbarState::default(),
            show_menu: false,
            menu_selected: 0,
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

    fn toggle_menu(&mut self) {
        self.show_menu = !self.show_menu;
        if self.show_menu {
            self.menu_selected = 0;
        }
    }

    fn menu_up(&mut self) {
        if self.menu_selected > 0 {
            self.menu_selected -= 1;
        }
    }

    fn menu_down(&mut self, menu_items_count: usize) {
        if self.menu_selected < menu_items_count - 1 {
            self.menu_selected += 1;
        }
    }

    fn menu_items() -> Vec<&'static str> {
        vec!["Link Model", "Open Dashboard", "Config Orcs", "Exit"]
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
                // Global shortcuts (work in both menu and normal mode)
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char('p') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.toggle_menu();
                    }
                    _ => {
                        // Handle menu-specific or normal-mode keys
                        if app.show_menu {
                            // Menu mode key handlers
                            match key.code {
                                KeyCode::Up => {
                                    app.menu_up();
                                }
                                KeyCode::Down => {
                                    let menu_count = App::menu_items().len();
                                    app.menu_down(menu_count);
                                }
                                KeyCode::Enter => {
                                    // Handle menu selection
                                    let menu_items = App::menu_items();
                                    if let Some(selected) = menu_items.get(app.menu_selected) {
                                        match *selected {
                                            "Exit" => {
                                                return Ok(());
                                            }
                                            _ => {
                                                // Other menu items don't do anything yet
                                                app.show_menu = false;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Esc => {
                                    app.show_menu = false;
                                }
                                _ => {}
                            }
                        } else {
                            // Normal mode key handlers
                            match key.code {
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

            // Wrap long messages
            let wrapped_lines = wrap_text(&msg.content, chunks[0].width.saturating_sub(4) as usize);

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

    // Render menu overlay if visible
    if app.show_menu {
        render_menu(f, app);
    }
}

fn render_menu(f: &mut ratatui::Frame, app: &App) {
    use ratatui::layout::{Alignment};

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

    // Miami gradient colors: Pink -> Purple -> Cyan -> Orange
    let miami_pink = (255, 0, 128);      // Hot pink
    let miami_purple = (138, 43, 226);   // Blue violet
    let miami_cyan = (0, 255, 255);      // Cyan
    let miami_orange = (255, 140, 0);    // Dark orange

    // Create menu items with selection highlight
    let mut menu_lines = vec![
        Line::from(""),  // Empty line for spacing
    ];

    for (i, item) in menu_items.iter().enumerate() {
        let is_selected = i == app.menu_selected;

        if is_selected {
            // Selected item: Miami gradient with bold
            let gradient_pos = i as f32 / menu_items.len() as f32;
            let selected_color = if gradient_pos < 0.33 {
                gradient_color(miami_pink, miami_purple, gradient_pos * 3.0)
            } else if gradient_pos < 0.66 {
                gradient_color(miami_purple, miami_cyan, (gradient_pos - 0.33) * 3.0)
            } else {
                gradient_color(miami_cyan, miami_orange, (gradient_pos - 0.66) * 3.0)
            };

            menu_lines.push(Line::from(vec![
                Span::styled("  ▶ ", Style::default().fg(selected_color).add_modifier(Modifier::BOLD)),
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
                gradient_color(miami_purple, miami_cyan, gradient_pos * 2.0)
            } else {
                gradient_color(miami_cyan, miami_purple, (gradient_pos - 0.5) * 2.0)
            };

            menu_lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("{:<30}", item),
                    Style::default().fg(item_color),
                ),
            ]));
        }
    }

    menu_lines.push(Line::from(""));  // Empty line for spacing

    // Create the menu paragraph
    let menu_text = Paragraph::new(menu_lines)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(gradient_color(miami_pink, miami_cyan, 0.5)))
                .title(Span::styled(
                    " 🎛️  MENU (Ctrl+P) ",
                    Style::default()
                        .fg(gradient_color(miami_pink, miami_orange, 0.7))
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
        Span::styled("↑↓", Style::default().fg(Color::Rgb(miami_cyan.0, miami_cyan.1, miami_cyan.2)).add_modifier(Modifier::BOLD)),
        Span::styled(" Navigate  ", Style::default().fg(Color::White)),
        Span::styled("Enter", Style::default().fg(Color::Rgb(miami_pink.0, miami_pink.1, miami_pink.2)).add_modifier(Modifier::BOLD)),
        Span::styled(" Select  ", Style::default().fg(Color::White)),
        Span::styled("Esc", Style::default().fg(Color::Rgb(miami_orange.0, miami_orange.1, miami_orange.2)).add_modifier(Modifier::BOLD)),
        Span::styled(" Close", Style::default().fg(Color::White)),
    ]))
    .alignment(Alignment::Center);

    f.render_widget(hint, hint_area);
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
