use ratatui::widgets::ScrollbarState;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::llm::{ChatMessage, LlmClient, LlmConfig, StreamEvent};
use crate::message::{Message, Role};

/// Connection status for the LLM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Not configured (no API key)
    NotConfigured,
    /// Ready to send messages
    Ready,
    /// Currently streaming a response
    Streaming,
    /// An error occurred
    Error(String),
}

/// Input mode for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal chat input
    #[default]
    Chat,
}

/// Menu items available in the settings menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Exit,
}

impl MenuItem {
    /// Returns all menu items in display order.
    pub const fn all() -> &'static [MenuItem] {
        &[
            MenuItem::Exit,
        ]
    }

    /// Returns the display label for this menu item.
    pub const fn label(&self) -> &'static str {
        match self {
            MenuItem::Exit => "Exit",
        }
    }
}

/// Chat-related state: messages and input.
#[derive(Debug, Default)]
pub struct ChatState {
    /// Chat message history
    pub messages: Vec<Message>,
    /// Current input text
    pub input: String,
    /// Cursor position in input
    pub cursor_position: usize,
}

impl ChatState {
    /// Create a new ChatState with initial messages.
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            input: String::new(),
            cursor_position: 0,
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

    /// Clear input and reset cursor.
    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
    }

    /// Get max scroll offset based on message count.
    pub fn max_scroll(&self) -> usize {
        self.messages.len().saturating_sub(1)
    }
}

/// Scroll-related state for the message list.
#[derive(Debug, Default)]
pub struct ScrollState {
    /// Current scroll offset in message list
    pub offset: usize,
    /// Scrollbar state for ratatui
    pub scrollbar: ScrollbarState,
}

impl ScrollState {
    /// Scroll up one line.
    pub fn scroll_up(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }

    /// Scroll down one line.
    pub fn scroll_down(&mut self, max_scroll: usize) {
        if self.offset < max_scroll {
            self.offset += 1;
        }
    }

    /// Scroll up by page size.
    pub fn scroll_page_up(&mut self, page_size: usize) {
        self.offset = self.offset.saturating_sub(page_size);
    }

    /// Scroll down by page size.
    pub fn scroll_page_down(&mut self, max_scroll: usize, page_size: usize) {
        self.offset = (self.offset + page_size).min(max_scroll);
    }

    /// Scroll to top.
    pub fn scroll_to_top(&mut self) {
        self.offset = 0;
    }

    /// Scroll to bottom.
    pub fn scroll_to_bottom(&mut self, max_scroll: usize) {
        self.offset = max_scroll;
    }

    /// Update scrollbar state.
    pub fn update(&mut self, total_items: usize) {
        self.scrollbar = self.scrollbar.content_length(total_items);
        self.scrollbar = self.scrollbar.position(self.offset);
    }
}

/// Menu-related state for the settings overlay.
#[derive(Debug, Default)]
pub struct MenuState {
    /// Whether the menu overlay is visible
    pub visible: bool,
    /// Currently selected menu item index
    pub selected: usize,
}

impl MenuState {
    /// Toggle menu visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.selected = 0;
        }
    }

    /// Move menu selection up.
    pub fn up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move menu selection down.
    pub fn down(&mut self, menu_items_count: usize) {
        if self.selected < menu_items_count - 1 {
            self.selected += 1;
        }
    }
}

/// Animation-related state for UI effects.
#[derive(Debug)]
pub struct AnimationState {
    /// Cursor blink visibility state
    pub cursor_visible: bool,
    /// Current frame of banner animation
    pub banner_frame: usize,
    /// Whether banner animation has completed
    pub banner_complete: bool,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            cursor_visible: true,
            banner_frame: 0,
            banner_complete: false,
        }
    }
}

impl AnimationState {
    /// Create a new AnimationState with banner animation complete.
    pub fn no_banner() -> Self {
        Self {
            cursor_visible: true,
            banner_frame: 0,
            banner_complete: true,
        }
    }

    /// Toggle cursor visibility for blinking effect.
    pub fn toggle_cursor(&mut self) {
        self.cursor_visible = !self.cursor_visible;
    }
}

/// LLM-related state for API interactions.
pub struct LlmState {
    /// LLM client for API calls
    pub client: Option<LlmClient>,
    /// Current connection status
    pub status: ConnectionStatus,
    /// Receiver for streaming events
    pub stream_rx: Option<mpsc::Receiver<StreamEvent>>,
    /// Current LLM configuration
    pub config: LlmConfig,
}

impl LlmState {
    /// Create a new LlmState from config.
    pub fn new(llm_config: LlmConfig) -> Self {
        let is_configured = llm_config.is_configured();
        let client = LlmClient::new(llm_config.clone());

        Self {
            client: Some(client),
            status: if is_configured {
                ConnectionStatus::Ready
            } else {
                ConnectionStatus::NotConfigured
            },
            stream_rx: None,
            config: llm_config,
        }
    }

    /// Check if currently streaming a response.
    pub fn is_streaming(&self) -> bool {
        self.stream_rx.is_some()
    }

    /// Apply the current config and recreate the client.
    pub fn apply_config(&mut self) {
        let is_configured = self.config.is_configured();
        self.client = Some(LlmClient::new(self.config.clone()));
        self.status = if is_configured {
            ConnectionStatus::Ready
        } else {
            ConnectionStatus::NotConfigured
        };
    }
}

/// Application state for the chat CLI.
pub struct App {
    /// Chat state: messages, input, cursor
    pub chat: ChatState,
    /// Scroll state: offset and scrollbar
    pub scroll: ScrollState,
    /// Menu state: visibility, selection, input
    pub menu: MenuState,
    /// Animation state: cursor blink, banner animation
    pub animation: AnimationState,
    /// LLM state: client, config, status, streaming
    pub llm: LlmState,
}

impl App {
    /// Create a new App instance with the welcome banner.
    pub fn new() -> Self {
        Self::new_with_config(&Config::load())
    }

    /// Create a new App instance from config.
    pub fn new_with_config(config: &Config) -> Self {
        let banner = Self::get_banner();
        let llm_config = LlmConfig::from_env_and_config(Some(&config.llm));

        Self {
            chat: ChatState::new(vec![Message::system_banner(banner)]),
            scroll: ScrollState::default(),
            menu: MenuState::default(),
            animation: AnimationState::default(),
            llm: LlmState::new(llm_config),
        }
    }

    /// Create a new App instance without the welcome banner.
    /// Used when TTE welcome screen was shown instead.
    #[allow(dead_code)]
    pub fn new_without_banner() -> Self {
        Self::new_without_banner_with_config(&Config::load())
    }

    /// Create a new App instance without the welcome banner, from config.
    pub fn new_without_banner_with_config(config: &Config) -> Self {
        let llm_config = LlmConfig::from_env_and_config(Some(&config.llm));

        Self {
            chat: ChatState::new(vec![Message::assistant(
                "Welcome! Type a message and press Enter to chat. Press Ctrl+P for menu.".to_string(),
            )]),
            scroll: ScrollState::default(),
            menu: MenuState::default(),
            animation: AnimationState::no_banner(),
            llm: LlmState::new(llm_config),
        }
    }

    /// Toggle cursor visibility for blinking effect.
    pub fn toggle_cursor(&mut self) {
        self.animation.toggle_cursor();
    }

    /// Get the welcome banner ASCII art.
    pub fn get_banner() -> String {
        r#"
╔═══════════════════════════════════════════════════════════════════════════╗
║                                                                           ║
║   ██╗    ██╗███████╗██╗      ██████╗ ██████╗ ███╗   ███╗███████╗          ║
║   ██║    ██║██╔════╝██║     ██╔════╝██╔═══██╗████╗ ████║██╔════╝          ║
║   ██║ █╗ ██║█████╗  ██║     ██║     ██║   ██║██╔████╔██║█████╗            ║
║   ██║███╗██║██╔══╝  ██║     ██║     ██║   ██║██║╚██╔╝██║██╔══╝            ║
║   ╚███╔███╔╝███████╗███████╗╚██████╗╚██████╔╝██║ ╚═╝ ██║███████╗          ║
║    ╚══╝╚══╝ ╚══════╝╚══════╝ ╚═════╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝          ║
║                                                                           ║
║                           ████████╗ ██████╗                               ║
║                           ╚══██╔══╝██╔═══██╗                              ║
║                              ██║   ██║   ██║                              ║
║                              ██║   ██║   ██║                              ║
║                              ██║   ╚██████╔╝                              ║
║                              ╚═╝    ╚═════╝                               ║
║                                                                           ║
║              ███╗   ███╗██╗ █████╗ ███╗   ███╗██╗                         ║
║              ████╗ ████║██║██╔══██╗████╗ ████║██║                         ║
║              ██╔████╔██║██║███████║██╔████╔██║██║                         ║
║              ██║╚██╔╝██║██║██╔══██║██║╚██╔╝██║██║                         ║
║              ██║ ╚═╝ ██║██║██║  ██║██║ ╚═╝ ██║██║     Bro!                ║
║              ╚═╝     ╚═╝╚═╝╚═╝  ╚═╝╚═╝     ╚═╝╚═╝                         ║
║                                                                           ║
║   ════════════════════════════════════════════════════════════════════    ║
║                                                                           ║
║        A Beautiful Chat CLI with Gradient Borders & Scrolling             ║
║                                                                           ║
║        Press Ctrl+P to open the menu                                      ║
║        Type a message and press Enter to chat                             ║
║        Use Up/Down to scroll through history                              ║
║        Built with Rust, Ratatui, and Miami vibes                          ║
║                                                                           ║
╚═══════════════════════════════════════════════════════════════════════════╝
"#
        .to_string()
    }

    /// Submit the current input as a message.
    pub fn submit_message(&mut self) {
        if self.chat.input.trim().is_empty() {
            return;
        }

        // Add user message
        self.chat.messages.push(Message::user(self.chat.input.clone()));

        // Check if LLM is configured
        if let Some(client) = &self.llm.client {
            if client.is_configured() {
                // Convert message history to API format (skip system banners)
                let api_messages: Vec<ChatMessage> = self
                    .chat
                    .messages
                    .iter()
                    .filter(|m| !m.is_system_banner())
                    .map(|m| ChatMessage {
                        role: match m.role {
                            Role::User => "user".to_string(),
                            Role::Assistant => "assistant".to_string(),
                        },
                        content: m.content.clone(),
                    })
                    .collect();

                // Start streaming
                self.llm.stream_rx = Some(client.stream_chat(api_messages));
                self.llm.status = ConnectionStatus::Streaming;
                
                // Add empty assistant message that will be filled by streaming
                self.chat.messages.push(Message::assistant(String::new()));
            } else {
                // Not configured - show helpful message
                self.chat.messages.push(Message::assistant(
                    "No API key configured. Set OPENAI_API_KEY environment variable or use the menu to configure.".to_string()
                ));
            }
        } else {
            // Fallback echo
            self.chat
                .messages
                .push(Message::assistant(format!("You said: {}", self.chat.input)));
        }

        // Clear input
        self.chat.clear_input();
    }

    /// Process streaming events. Call this in the event loop.
    pub fn process_stream(&mut self) {
        if let Some(rx) = &mut self.llm.stream_rx {
            // Try to receive without blocking
            match rx.try_recv() {
                Ok(event) => match event {
                    StreamEvent::Token(token) => {
                        // Append token to the last message
                        if let Some(last) = self.chat.messages.last_mut() {
                            if last.role == Role::Assistant {
                                last.content.push_str(&token);
                            }
                        }
                    }
                    StreamEvent::Done => {
                        self.llm.stream_rx = None;
                        self.llm.status = ConnectionStatus::Ready;
                    }
                    StreamEvent::Error(e) => {
                        // Append error to the last message or create new one
                        if let Some(last) = self.chat.messages.last_mut() {
                            if last.role == Role::Assistant && last.content.is_empty() {
                                last.content = format!("Error: {}", e);
                            }
                        }
                        self.llm.stream_rx = None;
                        self.llm.status = ConnectionStatus::Error(e);
                    }
                },
                Err(mpsc::error::TryRecvError::Empty) => {
                    // No events yet, continue
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Channel closed
                    self.llm.stream_rx = None;
                    if self.llm.status == ConnectionStatus::Streaming {
                        self.llm.status = ConnectionStatus::Ready;
                    }
                }
            }
        }
    }

    /// Check if currently streaming a response.
    pub fn is_streaming(&self) -> bool {
        self.llm.is_streaming()
    }

    /// Handle a character input.
    pub fn handle_char(&mut self, c: char) {
        self.chat.handle_char(c);
    }

    /// Handle backspace key.
    pub fn handle_backspace(&mut self) {
        self.chat.handle_backspace();
    }

    /// Move cursor left.
    pub fn move_cursor_left(&mut self) {
        self.chat.move_cursor_left();
    }

    /// Move cursor right.
    pub fn move_cursor_right(&mut self) {
        self.chat.move_cursor_right();
    }

    /// Scroll up one line.
    pub fn scroll_up(&mut self) {
        self.scroll.scroll_up();
    }

    /// Scroll down one line.
    pub fn scroll_down(&mut self, max_scroll: usize) {
        self.scroll.scroll_down(max_scroll);
    }

    /// Scroll up by page size.
    pub fn scroll_page_up(&mut self, page_size: usize) {
        self.scroll.scroll_page_up(page_size);
    }

    /// Scroll down by page size.
    pub fn scroll_page_down(&mut self, max_scroll: usize, page_size: usize) {
        self.scroll.scroll_page_down(max_scroll, page_size);
    }

    /// Scroll to top.
    pub fn scroll_to_top(&mut self) {
        self.scroll.scroll_to_top();
    }

    /// Scroll to bottom.
    pub fn scroll_to_bottom(&mut self, max_scroll: usize) {
        self.scroll.scroll_to_bottom(max_scroll);
    }

    /// Update scrollbar state.
    pub fn update_scroll_state(&mut self, total_items: usize) {
        self.scroll.update(total_items);
    }

    /// Toggle menu visibility.
    pub fn toggle_menu(&mut self) {
        self.menu.toggle();
    }

    /// Move menu selection up.
    pub fn menu_up(&mut self) {
        self.menu.up();
    }

    /// Move menu selection down.
    pub fn menu_down(&mut self, menu_items_count: usize) {
        self.menu.down(menu_items_count);
    }

    /// Get the list of menu items.
    pub fn menu_items() -> &'static [MenuItem] {
        MenuItem::all()
    }

    /// Get max scroll offset based on message count.
    pub fn max_scroll(&self) -> usize {
        self.chat.max_scroll()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
