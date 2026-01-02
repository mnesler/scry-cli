use ratatui::widgets::ScrollbarState;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::llm::{ChatMessage, LlmClient, LlmConfig, Provider, StreamEvent};
use crate::message::{Message, Role};
use crate::ui::{AuthDialog, ToastLevel, ToastState};

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

/// State of the interactive connection flow.
///
/// This enum tracks the user's progress through the connection dialog,
/// which allows them to enter API keys or authenticate via OAuth.
#[derive(Debug, Clone)]
pub enum ConnectState {
    /// No connection dialog is active.
    None,
    /// User has existing credentials; offer to use them or enter new ones.
    ExistingCredential {
        provider: Provider,
        masked_key: String,
        selected: usize,
    },
    /// User is selecting how to authenticate (enter key, open browser, cancel).
    SelectingMethod {
        provider: Provider,
        selected: usize,
    },
    /// User is typing an API key.
    EnteringApiKey {
        provider: Provider,
        input: String,
        cursor: usize,
        error: Option<String>,
    },
    /// Validating the API key with the provider.
    ValidatingKey {
        provider: Provider,
        key: String,
    },
    /// OAuth device code flow pending (waiting for device code).
    OAuthPending {
        provider: Provider,
        auth_dialog: AuthDialog,
    },
    /// OAuth device code flow polling (device code received, polling for token).
    OAuthPolling {
        provider: Provider,
        auth_dialog: AuthDialog,
    },
}

impl Default for ConnectState {
    fn default() -> Self {
        Self::None
    }
}

impl ConnectState {
    /// Check if a connection dialog is active.
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Get the provider being connected to, if any.
    pub fn provider(&self) -> Option<Provider> {
        match self {
            Self::None => None,
            Self::ExistingCredential { provider, .. }
            | Self::SelectingMethod { provider, .. }
            | Self::EnteringApiKey { provider, .. }
            | Self::ValidatingKey { provider, .. }
            | Self::OAuthPending { provider, .. }
            | Self::OAuthPolling { provider, .. } => Some(*provider),
        }
    }
}

/// Mask an API key for display, showing only first and last 4 characters.
///
/// Examples:
/// - "sk-ant-api03-abc123xyz789" -> "sk-a...9789"
/// - "short" -> "*****"
/// - "" -> ""
pub fn mask_api_key(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    format!("{}...{}", &key[..4], &key[key.len() - 4..])
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
    ConnectProvider,
    Exit,
}

impl MenuItem {
    /// Returns all menu items in display order.
    pub const fn all() -> &'static [MenuItem] {
        &[
            MenuItem::ConnectProvider,
            MenuItem::Exit,
        ]
    }

    /// Returns the display label for this menu item.
    pub const fn label(&self) -> &'static str {
        match self {
            MenuItem::ConnectProvider => "Connect Provider",
            MenuItem::Exit => "Exit",
        }
    }

    /// Check if this menu item has a submenu.
    pub const fn has_submenu(&self) -> bool {
        matches!(self, MenuItem::ConnectProvider)
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
    /// Whether we're in a submenu
    pub in_submenu: bool,
    /// Currently selected submenu item index
    pub submenu_selected: usize,
}

impl MenuState {
    /// Toggle menu visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.selected = 0;
            self.in_submenu = false;
            self.submenu_selected = 0;
        }
    }

    /// Move menu selection up.
    pub fn up(&mut self) {
        if self.in_submenu {
            if self.submenu_selected > 0 {
                self.submenu_selected -= 1;
            }
        } else if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move menu selection down.
    pub fn down(&mut self, menu_items_count: usize, submenu_items_count: usize) {
        if self.in_submenu {
            if self.submenu_selected < submenu_items_count.saturating_sub(1) {
                self.submenu_selected += 1;
            }
        } else if self.selected < menu_items_count.saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Enter submenu if current item has one.
    pub fn enter_submenu(&mut self) {
        self.in_submenu = true;
        self.submenu_selected = 0;
    }

    /// Exit submenu back to main menu.
    pub fn exit_submenu(&mut self) {
        self.in_submenu = false;
        self.submenu_selected = 0;
    }

    /// Close the menu entirely.
    pub fn close(&mut self) {
        self.visible = false;
        self.in_submenu = false;
        self.selected = 0;
        self.submenu_selected = 0;
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
    /// Toast notification state
    pub toasts: ToastState,
    /// Connection dialog state
    pub connect: ConnectState,
    /// Receiver for async API key validation results
    pub validation_rx: Option<tokio::sync::oneshot::Receiver<Result<(), String>>>,
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
            toasts: ToastState::default(),
            connect: ConnectState::default(),
            validation_rx: None,
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
            toasts: ToastState::default(),
            connect: ConnectState::default(),
            validation_rx: None,
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
                    "No API key configured. Set ANTHROPIC_API_KEY environment variable or add it to your config file.".to_string()
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
    pub fn menu_down(&mut self, menu_items_count: usize, submenu_items_count: usize) {
        self.menu.down(menu_items_count, submenu_items_count);
    }

    /// Get the currently selected menu item.
    pub fn selected_menu_item(&self) -> Option<&MenuItem> {
        App::menu_items().get(self.menu.selected)
    }

    /// Get the currently selected provider (when in provider submenu).
    pub fn selected_provider(&self) -> Option<Provider> {
        if self.menu.in_submenu {
            Provider::all().get(self.menu.submenu_selected).copied()
        } else {
            None
        }
    }

    /// Switch to a new provider.
    pub fn switch_provider(&mut self, provider: Provider) {
        self.llm.config.provider = provider;
        self.llm.config.api_base = provider.default_api_base().to_string();
        self.llm.config.model = provider.default_model().to_string();
        
        // Try to load API key from environment
        let env_var = provider.env_var_name();
        if !env_var.is_empty() {
            if let Ok(key) = std::env::var(env_var) {
                self.llm.config.api_key = key;
            } else {
                self.llm.config.api_key.clear();
            }
        } else {
            // Provider doesn't need an API key (e.g., Ollama)
            self.llm.config.api_key.clear();
        }
        
        self.llm.apply_config();
        self.menu.close();
        
        // Add a message about the provider switch
        let status = if self.llm.config.is_configured() || !provider.requires_api_key() {
            format!("Switched to {}. Ready to chat!", provider.display_name())
        } else {
            format!(
                "Switched to {}. Set {} to connect.",
                provider.display_name(),
                provider.env_var_name()
            )
        };
        self.chat.messages.push(Message::assistant(status));
    }

    /// Get the list of menu items.
    pub fn menu_items() -> &'static [MenuItem] {
        MenuItem::all()
    }

    /// Get max scroll offset based on message count.
    pub fn max_scroll(&self) -> usize {
        self.chat.max_scroll()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Toast notification methods
    // ─────────────────────────────────────────────────────────────────────────────

    /// Add a toast notification.
    pub fn toast(&mut self, message: impl Into<String>, level: ToastLevel) -> u64 {
        self.toasts.push(message, level)
    }

    /// Add an info toast.
    pub fn toast_info(&mut self, message: impl Into<String>) -> u64 {
        self.toasts.info(message)
    }

    /// Add a success toast.
    pub fn toast_success(&mut self, message: impl Into<String>) -> u64 {
        self.toasts.success(message)
    }

    /// Add a warning toast.
    pub fn toast_warning(&mut self, message: impl Into<String>) -> u64 {
        self.toasts.warning(message)
    }

    /// Add an error toast.
    pub fn toast_error(&mut self, message: impl Into<String>) -> u64 {
        self.toasts.error(message)
    }

    /// Dismiss a toast by ID.
    pub fn dismiss_toast(&mut self, id: u64) {
        self.toasts.dismiss(id)
    }

    /// Tick the toast system to remove expired toasts.
    /// Call this on each frame/tick.
    pub fn tick_toasts(&mut self) {
        self.toasts.tick()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Connection flow methods
    // ─────────────────────────────────────────────────────────────────────────────

    /// Start the connection flow for a provider.
    ///
    /// This checks for existing credentials and shows the appropriate dialog:
    /// - If credentials exist: Show ExistingCredential dialog
    /// - If OAuth provider (Copilot): Start OAuth device flow
    /// - Otherwise: Show SelectingMethod dialog
    pub fn start_connection(&mut self, provider: Provider) {
        use crate::auth::AuthStorage;

        // Close the menu first
        self.menu.close();

        // Check for existing credentials
        if let Ok(storage) = AuthStorage::load() {
            if let Some(cred) = storage.get(provider.storage_key()) {
                if !cred.is_expired() {
                    let masked = mask_api_key(cred.token());
                    self.connect = ConnectState::ExistingCredential {
                        provider,
                        masked_key: masked,
                        selected: 0,
                    };
                    return;
                }
            }
        }

        // No existing credentials - determine how to connect
        if provider.uses_oauth() {
            // OAuth providers (Copilot) - will be handled in a later issue
            // For now, show a message
            self.chat.messages.push(Message::assistant(
                format!("{} uses OAuth authentication. This feature is coming soon.", provider.display_name())
            ));
        } else if !provider.requires_api_key() {
            // Provider doesn't need auth (e.g., Ollama) - connect directly
            self.complete_connection(provider, None);
        } else {
            // Show method selection dialog
            self.connect = ConnectState::SelectingMethod {
                provider,
                selected: 0,
            };
        }
    }

    /// Cancel the connection flow and return to normal state.
    pub fn cancel_connection(&mut self) {
        self.connect = ConnectState::None;
    }

    /// Complete the connection successfully.
    ///
    /// Saves credentials (if provided) and switches to the provider.
    pub fn complete_connection(&mut self, provider: Provider, api_key: Option<String>) {
        use crate::auth::{AuthStorage, Credential};

        // Save credential if provided
        if let Some(key) = &api_key {
            if let Ok(mut storage) = AuthStorage::load() {
                storage.set(provider.storage_key(), Credential::api_key(key));
                if let Err(e) = storage.save() {
                    self.toast_warning(format!("Could not save credentials: {}", e));
                }
            }
        }

        // Configure and switch to the provider
        self.llm.config.provider = provider;
        self.llm.config.api_base = provider.default_api_base().to_string();
        self.llm.config.model = provider.default_model().to_string();

        if let Some(key) = api_key {
            self.llm.config.api_key = key;
        } else if !provider.requires_api_key() {
            self.llm.config.api_key.clear();
        }

        self.llm.apply_config();
        self.connect = ConnectState::None;

        self.toast_success(format!("Connected to {}", provider.display_name()));
    }

    /// Handle a connection error.
    ///
    /// Shows the error in the EnteringApiKey state so the user can try again.
    pub fn connection_error(&mut self, error: String) {
        if let ConnectState::ValidatingKey { provider, key } = &self.connect {
            self.connect = ConnectState::EnteringApiKey {
                provider: *provider,
                input: key.clone(),
                cursor: key.len(),
                error: Some(error),
            };
        } else {
            // Fallback: show toast
            self.toast_error(error);
            self.connect = ConnectState::None;
        }
    }

    /// Use existing credentials to connect.
    pub fn use_existing_credentials(&mut self) {
        use crate::auth::AuthStorage;

        if let ConnectState::ExistingCredential { provider, .. } = self.connect {
            if let Ok(storage) = AuthStorage::load() {
                if let Some(cred) = storage.get(provider.storage_key()) {
                    let key = cred.token().to_string();
                    self.complete_connection(provider, Some(key));
                    return;
                }
            }
            // Fallback if credential disappeared
            self.toast_error("Credential not found");
            self.connect = ConnectState::None;
        }
    }

    /// Enter new credentials (from ExistingCredential or SelectingMethod state).
    pub fn enter_new_credentials(&mut self) {
        let provider = match &self.connect {
            ConnectState::ExistingCredential { provider, .. } => *provider,
            ConnectState::SelectingMethod { provider, .. } => *provider,
            _ => return,
        };

        self.connect = ConnectState::EnteringApiKey {
            provider,
            input: String::new(),
            cursor: 0,
            error: None,
        };
    }

    /// Start async validation of an API key.
    ///
    /// Spawns a background task to validate the key and stores the receiver.
    pub fn start_validation(&mut self, provider: Provider, key: String) {
        use crate::llm::validate_api_key;

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.validation_rx = Some(rx);
        self.connect = ConnectState::ValidatingKey {
            provider,
            key: key.clone(),
        };

        tokio::spawn(async move {
            let result = validate_api_key(provider, &key).await;
            let _ = tx.send(result);
        });
    }

    /// Process async validation results.
    ///
    /// Call this in the event loop to check for completed validations.
    /// Returns true if a validation completed (success or failure).
    pub fn process_validation(&mut self) -> bool {
        if let Some(mut rx) = self.validation_rx.take() {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    // Validation succeeded
                    if let ConnectState::ValidatingKey { provider, key } = &self.connect {
                        let provider = *provider;
                        let key = key.clone();
                        self.complete_connection(provider, Some(key));
                    }
                    return true;
                }
                Ok(Err(e)) => {
                    // Validation failed
                    self.connection_error(e);
                    return true;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                    // Still pending, put the receiver back
                    self.validation_rx = Some(rx);
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    // Channel closed unexpectedly
                    self.connection_error("Validation task failed".to_string());
                    return true;
                }
            }
        }
        false
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key_normal() {
        assert_eq!(mask_api_key("sk-ant-api03-abcdefghijklmnop"), "sk-a...mnop");
    }

    #[test]
    fn test_mask_api_key_short() {
        assert_eq!(mask_api_key("short"), "*****");
        assert_eq!(mask_api_key("12345678"), "********");
    }

    #[test]
    fn test_mask_api_key_empty() {
        assert_eq!(mask_api_key(""), "");
    }

    #[test]
    fn test_mask_api_key_exactly_nine_chars() {
        // 9 chars should show first 4 and last 4, which overlaps but works
        assert_eq!(mask_api_key("123456789"), "1234...6789");
    }

    #[test]
    fn test_connect_state_default() {
        let state = ConnectState::default();
        assert!(!state.is_active());
        assert!(state.provider().is_none());
    }

    #[test]
    fn test_connect_state_is_active() {
        let state = ConnectState::SelectingMethod {
            provider: Provider::Anthropic,
            selected: 0,
        };
        assert!(state.is_active());
        assert_eq!(state.provider(), Some(Provider::Anthropic));
    }

    #[test]
    fn test_connect_state_entering_api_key() {
        let state = ConnectState::EnteringApiKey {
            provider: Provider::OpenRouter,
            input: "sk-or-test".to_string(),
            cursor: 10,
            error: None,
        };
        assert!(state.is_active());
        assert_eq!(state.provider(), Some(Provider::OpenRouter));
    }

    #[test]
    fn test_connect_state_validating() {
        let state = ConnectState::ValidatingKey {
            provider: Provider::Anthropic,
            key: "sk-ant-test".to_string(),
        };
        assert!(state.is_active());
    }

    #[test]
    fn test_connect_state_clone() {
        let state = ConnectState::ExistingCredential {
            provider: Provider::Anthropic,
            masked_key: "sk-a...xyz".to_string(),
            selected: 1,
        };
        let cloned = state.clone();
        assert!(matches!(cloned, ConnectState::ExistingCredential { .. }));
    }

    #[test]
    fn test_start_connection_shows_dialog() {
        let mut app = App::new_without_banner();
        app.start_connection(Provider::Anthropic);

        // Should go to either SelectingMethod or ExistingCredential
        // depending on whether credentials already exist
        assert!(
            matches!(
                app.connect,
                ConnectState::SelectingMethod {
                    provider: Provider::Anthropic,
                    ..
                }
            ) || matches!(
                app.connect,
                ConnectState::ExistingCredential {
                    provider: Provider::Anthropic,
                    ..
                }
            )
        );
    }

    #[test]
    fn test_start_connection_ollama() {
        let mut app = App::new_without_banner();
        app.start_connection(Provider::Ollama);

        // Ollama doesn't need credentials, should complete immediately
        assert!(matches!(app.connect, ConnectState::None));
        assert_eq!(app.llm.config.provider, Provider::Ollama);
    }

    #[test]
    fn test_cancel_connection() {
        let mut app = App::new_without_banner();
        app.connect = ConnectState::SelectingMethod {
            provider: Provider::Anthropic,
            selected: 0,
        };

        app.cancel_connection();
        assert!(matches!(app.connect, ConnectState::None));
    }

    #[test]
    fn test_enter_new_credentials() {
        let mut app = App::new_without_banner();
        app.connect = ConnectState::SelectingMethod {
            provider: Provider::Anthropic,
            selected: 0,
        };

        app.enter_new_credentials();

        assert!(matches!(
            app.connect,
            ConnectState::EnteringApiKey {
                provider: Provider::Anthropic,
                ..
            }
        ));
    }

    #[test]
    fn test_connection_error() {
        let mut app = App::new_without_banner();
        app.connect = ConnectState::ValidatingKey {
            provider: Provider::Anthropic,
            key: "sk-ant-test-key".to_string(),
        };

        app.connection_error("Invalid API key".to_string());

        match &app.connect {
            ConnectState::EnteringApiKey {
                provider,
                input,
                error,
                ..
            } => {
                assert_eq!(*provider, Provider::Anthropic);
                assert_eq!(input, "sk-ant-test-key");
                assert_eq!(error, &Some("Invalid API key".to_string()));
            }
            _ => panic!("Expected EnteringApiKey state"),
        }
    }

    #[test]
    fn test_complete_connection() {
        let mut app = App::new_without_banner();
        app.connect = ConnectState::ValidatingKey {
            provider: Provider::Anthropic,
            key: "sk-ant-test".to_string(),
        };

        app.complete_connection(Provider::Anthropic, Some("sk-ant-test".to_string()));

        assert!(matches!(app.connect, ConnectState::None));
        assert_eq!(app.llm.config.provider, Provider::Anthropic);
        assert_eq!(app.llm.config.api_key, "sk-ant-test");
    }
}
