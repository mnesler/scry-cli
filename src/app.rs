use ratatui::widgets::ScrollbarState;

use crate::message::Message;

/// Application state for the chat CLI.
pub struct App {
    /// Chat message history
    pub messages: Vec<Message>,
    /// Current input text
    pub input: String,
    /// Cursor position in input
    pub cursor_position: usize,
    /// Current scroll offset in message list
    pub scroll_offset: usize,
    /// Scrollbar state for ratatui
    pub scroll_state: ScrollbarState,
    /// Whether the menu overlay is visible
    pub show_menu: bool,
    /// Currently selected menu item index
    pub menu_selected: usize,
    /// Current frame of banner animation
    pub banner_animation_frame: usize,
    /// Whether banner animation has completed
    pub banner_animation_complete: bool,
}

impl App {
    /// Create a new App instance with the welcome banner.
    pub fn new() -> Self {
        let banner = Self::get_banner();

        Self {
            messages: vec![Message::assistant(banner)],
            input: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            scroll_state: ScrollbarState::default(),
            show_menu: false,
            menu_selected: 0,
            banner_animation_frame: 0,
            banner_animation_complete: false,
        }
    }

    /// Get the welcome banner ASCII art.
    pub fn get_banner() -> String {
        r#"
╔═══════════════════════════════════════════════════════════════════════════╗
║                                                                           ║
║   ██╗    ██╗███████╗██╗      ██████╗ ██████╗ ███╗   ███╗███████╗        ║
║   ██║    ██║██╔════╝██║     ██╔════╝██╔═══██╗████╗ ████║██╔════╝        ║
║   ██║ █╗ ██║█████╗  ██║     ██║     ██║   ██║██╔████╔██║█████╗          ║
║   ██║███╗██║██╔══╝  ██║     ██║     ██║   ██║██║╚██╔╝██║██╔══╝          ║
║   ╚███╔███╔╝███████╗███████╗╚██████╗╚██████╔╝██║ ╚═╝ ██║███████╗        ║
║    ╚══╝╚══╝ ╚══════╝╚══════╝ ╚═════╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝        ║
║                                                                           ║
║                ████████╗ ██████╗     ███╗   ███╗██╗                      ║
║                ╚══██╔══╝██╔═══██╗    ████╗ ████║██║                      ║
║                   ██║   ██║   ██║    ██╔████╔██║██║                      ║
║                   ██║   ██║   ██║    ██║╚██╔╝██║██║                      ║
║                   ██║   ╚██████╔╝    ██║ ╚═╝ ██║██║                      ║
║                   ╚═╝    ╚═════╝     ╚═╝     ╚═╝╚═╝                      ║
║                                                                           ║
║              ███╗   ███╗██╗ █████╗ ███╗   ███╗██╗                        ║
║              ████╗ ████║██║██╔══██╗████╗ ████║██║                        ║
║              ██╔████╔██║██║███████║██╔████╔██║██║                        ║
║              ██║╚██╔╝██║██║██╔══██║██║╚██╔╝██║██║                        ║
║              ██║ ╚═╝ ██║██║██║  ██║██║ ╚═╝ ██║██║                        ║
║              ╚═╝     ╚═╝╚═╝╚═╝  ╚═╝╚═╝     ╚═╝╚═╝                        ║
║                                                                           ║
║   ════════════════════════════════════════════════════════════════════   ║
║                                                                           ║
║        A Beautiful Chat CLI with Gradient Borders & Scrolling            ║
║                                                                           ║
║        Press Ctrl+P to open the menu                                     ║
║        Type a message and press Enter to chat                            ║
║        Use Up/Down to scroll through history                             ║
║        Built with Rust, Ratatui, and Miami vibes                         ║
║                                                                           ║
╚═══════════════════════════════════════════════════════════════════════════╝
"#
        .to_string()
    }

    /// Submit the current input as a message.
    pub fn submit_message(&mut self) {
        if !self.input.trim().is_empty() {
            // Add user message
            self.messages.push(Message::user(self.input.clone()));

            // Echo it back as assistant
            self.messages
                .push(Message::assistant(format!("You said: {}", self.input)));

            // Clear input
            self.input.clear();
            self.cursor_position = 0;
        }
    }

    /// Handle a character input.
    pub fn handle_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    /// Handle backspace key.
    pub fn handle_backspace(&mut self) {
        if self.cursor_position > 0 {
            self.input.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    /// Move cursor left.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right.
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position += 1;
        }
    }

    /// Scroll up one line.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down one line.
    pub fn scroll_down(&mut self, max_scroll: usize) {
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Scroll up by page size.
    pub fn scroll_page_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    /// Scroll down by page size.
    pub fn scroll_page_down(&mut self, max_scroll: usize, page_size: usize) {
        self.scroll_offset = (self.scroll_offset + page_size).min(max_scroll);
    }

    /// Scroll to top.
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll to bottom.
    pub fn scroll_to_bottom(&mut self, max_scroll: usize) {
        self.scroll_offset = max_scroll;
    }

    /// Update scrollbar state.
    pub fn update_scroll_state(&mut self, total_items: usize) {
        self.scroll_state = self.scroll_state.content_length(total_items);
        self.scroll_state = self.scroll_state.position(self.scroll_offset);
    }

    /// Toggle menu visibility.
    pub fn toggle_menu(&mut self) {
        self.show_menu = !self.show_menu;
        if self.show_menu {
            self.menu_selected = 0;
        }
    }

    /// Move menu selection up.
    pub fn menu_up(&mut self) {
        if self.menu_selected > 0 {
            self.menu_selected -= 1;
        }
    }

    /// Move menu selection down.
    pub fn menu_down(&mut self, menu_items_count: usize) {
        if self.menu_selected < menu_items_count - 1 {
            self.menu_selected += 1;
        }
    }

    /// Get the list of menu items.
    pub fn menu_items() -> Vec<&'static str> {
        vec!["Link Model", "Open Dashboard", "Config Orcs", "Exit"]
    }

    /// Get max scroll offset based on message count.
    pub fn max_scroll(&self) -> usize {
        self.messages.len().saturating_sub(1)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
