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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    /// Normal chat input
    Chat,
    /// Entering API key in menu
    ApiKey,
    /// Entering API base URL in menu
    ApiBase,
    /// Entering model name in menu
    Model,
}

/// Application state for the chat CLI.
pub struct App {
    /// Chat message history
    pub messages: Vec<Message>,
    /// Current input text
    pub input: String,
    /// Cursor position in input
    pub cursor_position: usize,
    /// Cursor blink visibility state
    pub cursor_visible: bool,
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
    /// LLM client for API calls
    pub llm_client: Option<LlmClient>,
    /// Current connection status
    pub connection_status: ConnectionStatus,
    /// Receiver for streaming events
    pub stream_rx: Option<mpsc::Receiver<StreamEvent>>,
    /// Current input mode
    pub input_mode: InputMode,
    /// Temporary input for menu fields
    pub menu_input: String,
    /// Current LLM configuration (mutable for menu editing)
    pub llm_config: LlmConfig,
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
        let is_configured = llm_config.is_configured();
        let llm_client = LlmClient::new(llm_config.clone());

        Self {
            messages: vec![Message::assistant(banner)],
            input: String::new(),
            cursor_position: 0,
            cursor_visible: true,
            scroll_offset: 0,
            scroll_state: ScrollbarState::default(),
            show_menu: false,
            menu_selected: 0,
            banner_animation_frame: 0,
            banner_animation_complete: false,
            llm_client: Some(llm_client),
            connection_status: if is_configured {
                ConnectionStatus::Ready
            } else {
                ConnectionStatus::NotConfigured
            },
            stream_rx: None,
            input_mode: InputMode::Chat,
            menu_input: String::new(),
            llm_config,
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
        let is_configured = llm_config.is_configured();
        let llm_client = LlmClient::new(llm_config.clone());

        Self {
            messages: vec![Message::assistant(
                "Welcome! Type a message and press Enter to chat. Press Ctrl+P for menu.".to_string(),
            )],
            input: String::new(),
            cursor_position: 0,
            cursor_visible: true,
            scroll_offset: 0,
            scroll_state: ScrollbarState::default(),
            show_menu: false,
            menu_selected: 0,
            banner_animation_frame: 0,
            banner_animation_complete: true, // No animation needed
            llm_client: Some(llm_client),
            connection_status: if is_configured {
                ConnectionStatus::Ready
            } else {
                ConnectionStatus::NotConfigured
            },
            stream_rx: None,
            input_mode: InputMode::Chat,
            menu_input: String::new(),
            llm_config,
        }
    }

    /// Save current LLM configuration to file.
    pub fn save_config(&self) -> anyhow::Result<()> {
        let mut config = Config::load();
        config.update_llm(
            self.llm_config.api_base.clone(),
            self.llm_config.api_key.clone(),
            self.llm_config.model.clone(),
        );
        config.save()
    }

    /// Toggle cursor visibility for blinking effect.
    pub fn toggle_cursor(&mut self) {
        self.cursor_visible = !self.cursor_visible;
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
        if self.input.trim().is_empty() {
            return;
        }

        // Add user message
        self.messages.push(Message::user(self.input.clone()));

        // Check if LLM is configured
        if let Some(client) = &self.llm_client {
            if client.is_configured() {
                // Convert message history to API format
                let api_messages: Vec<ChatMessage> = self
                    .messages
                    .iter()
                    .filter(|m| !m.content.contains("WELCOME TO")) // Skip banner
                    .map(|m| ChatMessage {
                        role: match m.role {
                            Role::User => "user".to_string(),
                            Role::Assistant => "assistant".to_string(),
                        },
                        content: m.content.clone(),
                    })
                    .collect();

                // Start streaming
                self.stream_rx = Some(client.stream_chat(api_messages));
                self.connection_status = ConnectionStatus::Streaming;
                
                // Add empty assistant message that will be filled by streaming
                self.messages.push(Message::assistant(String::new()));
            } else {
                // Not configured - show helpful message
                self.messages.push(Message::assistant(
                    "No API key configured. Set OPENAI_API_KEY environment variable or use the menu to configure.".to_string()
                ));
            }
        } else {
            // Fallback echo
            self.messages
                .push(Message::assistant(format!("You said: {}", self.input)));
        }

        // Clear input
        self.input.clear();
        self.cursor_position = 0;
    }

    /// Process streaming events. Call this in the event loop.
    pub fn process_stream(&mut self) {
        if let Some(rx) = &mut self.stream_rx {
            // Try to receive without blocking
            match rx.try_recv() {
                Ok(event) => match event {
                    StreamEvent::Token(token) => {
                        // Append token to the last message
                        if let Some(last) = self.messages.last_mut() {
                            if last.role == Role::Assistant {
                                last.content.push_str(&token);
                            }
                        }
                    }
                    StreamEvent::Done => {
                        self.stream_rx = None;
                        self.connection_status = ConnectionStatus::Ready;
                    }
                    StreamEvent::Error(e) => {
                        // Append error to the last message or create new one
                        if let Some(last) = self.messages.last_mut() {
                            if last.role == Role::Assistant && last.content.is_empty() {
                                last.content = format!("Error: {}", e);
                            }
                        }
                        self.stream_rx = None;
                        self.connection_status = ConnectionStatus::Error(e);
                    }
                },
                Err(mpsc::error::TryRecvError::Empty) => {
                    // No events yet, continue
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Channel closed
                    self.stream_rx = None;
                    if self.connection_status == ConnectionStatus::Streaming {
                        self.connection_status = ConnectionStatus::Ready;
                    }
                }
            }
        }
    }

    /// Check if currently streaming a response.
    pub fn is_streaming(&self) -> bool {
        self.stream_rx.is_some()
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
        vec!["API Key", "API Base URL", "Model", "Save Config", "Exit"]
    }

    /// Apply the current LLM config and recreate the client.
    pub fn apply_llm_config(&mut self) {
        let is_configured = self.llm_config.is_configured();
        self.llm_client = Some(LlmClient::new(self.llm_config.clone()));
        self.connection_status = if is_configured {
            ConnectionStatus::Ready
        } else {
            ConnectionStatus::NotConfigured
        };
    }

    /// Start editing a menu field.
    pub fn start_menu_input(&mut self, mode: InputMode) {
        self.input_mode = mode.clone();
        // Pre-fill with current value
        self.menu_input = match mode {
            InputMode::ApiKey => self.llm_config.api_key.clone(),
            InputMode::ApiBase => self.llm_config.api_base.clone(),
            InputMode::Model => self.llm_config.model.clone(),
            InputMode::Chat => String::new(),
        };
    }

    /// Confirm menu input and apply the value.
    pub fn confirm_menu_input(&mut self) {
        match self.input_mode {
            InputMode::ApiKey => {
                self.llm_config.api_key = self.menu_input.clone();
                self.apply_llm_config();
            }
            InputMode::ApiBase => {
                self.llm_config.api_base = self.menu_input.clone();
                self.apply_llm_config();
            }
            InputMode::Model => {
                self.llm_config.model = self.menu_input.clone();
                self.apply_llm_config();
            }
            InputMode::Chat => {}
        }
        self.menu_input.clear();
        self.input_mode = InputMode::Chat;
    }

    /// Cancel menu input.
    pub fn cancel_menu_input(&mut self) {
        self.menu_input.clear();
        self.input_mode = InputMode::Chat;
    }

    /// Handle character input for menu fields.
    pub fn handle_menu_char(&mut self, c: char) {
        self.menu_input.push(c);
    }

    /// Handle backspace for menu fields.
    pub fn handle_menu_backspace(&mut self) {
        self.menu_input.pop();
    }

    /// Check if in a menu input mode.
    pub fn is_menu_input_mode(&self) -> bool {
        self.input_mode != InputMode::Chat
    }

    /// Get display value for a config field (masked for API key).
    pub fn get_config_display(&self, field: &str) -> String {
        match field {
            "API Key" => {
                if self.llm_config.api_key.is_empty() {
                    "(not set)".to_string()
                } else {
                    let key = &self.llm_config.api_key;
                    if key.len() > 8 {
                        format!("{}...{}", &key[..4], &key[key.len()-4..])
                    } else {
                        "****".to_string()
                    }
                }
            }
            "API Base URL" => self.llm_config.api_base.clone(),
            "Model" => self.llm_config.model.clone(),
            _ => String::new(),
        }
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
