use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Terminal};

use crate::app::{App, ConnectState, MenuItem};
use crate::config::Config;
use crate::llm::{Provider, ANTHROPIC_MODELS, COPILOT_MODELS};
use crate::ui;
use crate::ui::AuthDialogResult;

/// Result of handling a key event.
pub enum HandleResult {
    /// Continue running the app
    Continue,
    /// Exit the app
    Exit,
}

/// Cursor blink interval in milliseconds.
const CURSOR_BLINK_MS: u64 = 530;

/// OAuth timer tick interval in milliseconds.
const OAUTH_TICK_MS: u64 = 1000;

/// Run the main application loop.
pub fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    config: &Config,
) -> io::Result<()> {
    let behavior = &config.behavior;
    let mut last_cursor_toggle = Instant::now();
    let mut last_oauth_tick = Instant::now();

    loop {
        // Process any streaming events first
        app.process_stream();
        
        // Process async validation results
        app.process_validation();

        // Process device code results (OAuth step 1)
        app.process_device_code();

        // Process async OAuth results (OAuth step 2)
        app.process_oauth();

        // Process authorization code exchange (Anthropic OAuth)
        app.process_auth_code_exchange();

        // Tick OAuth dialog timer
        if last_oauth_tick.elapsed() >= Duration::from_millis(OAUTH_TICK_MS) {
            app.tick_oauth_dialog();
            last_oauth_tick = Instant::now();
        }
        
        // Tick toast notifications to expire old ones
        app.tick_toasts();
        
        terminal.draw(|f| ui::ui(f, app, config))?;

        // Toggle cursor blink
        if last_cursor_toggle.elapsed() >= Duration::from_millis(CURSOR_BLINK_MS) {
            app.toggle_cursor();
            last_cursor_toggle = Instant::now();
        }

        // Use timeout for animation: fast polling during animation/streaming/validation/oauth, slower when idle
        let timeout = if !app.animation.banner_complete || app.is_streaming() || app.validation_rx.is_some() || app.oauth_rx.is_some() || app.device_code_rx.is_some() || app.auth_code_rx.is_some() {
            Duration::from_millis(behavior.animation_frame_ms)
        } else {
            // Use shorter timeout to keep cursor blinking smooth
            Duration::from_millis(50)
        };

        // Poll for events with timeout
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Reset cursor to visible on any keypress
                    app.animation.cursor_visible = true;
                    last_cursor_toggle = Instant::now();
                    
                    match handle_key_event(app, key.code, key.modifiers, config) {
                        HandleResult::Exit => return Ok(()),
                        HandleResult::Continue => {}
                    }
                }
            }
        }
        // If no event, loop continues and redraws (for animation/cursor blink/streaming)
    }
}

/// Handle a key event and return whether to continue or exit.
fn handle_key_event(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    config: &Config,
) -> HandleResult {
    let page_size = config.behavior.scroll_page_size;

    // Global shortcuts (work in all modes)
    match code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            return HandleResult::Exit;
        }
        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
            return HandleResult::Exit;
        }
        KeyCode::Char('p') if modifiers.contains(KeyModifiers::CONTROL) => {
            // Only toggle menu if not in connection dialog
            if !app.connect.is_active() {
                app.toggle_menu();
            }
            return HandleResult::Continue;
        }
        _ => {}
    }

    // Handle connection dialog first (takes priority over menu)
    if app.connect.is_active() {
        return handle_connect_keys(app, code);
    }

    // Handle menu-specific or normal-mode keys
    if app.menu.visible {
        handle_menu_keys(app, code)
    } else {
        handle_normal_keys(app, code, page_size)
    }
}

/// Handle key events when the menu is open.
fn handle_menu_keys(app: &mut App, code: KeyCode) -> HandleResult {
    if app.menu.in_submenu {
        handle_submenu_keys(app, code)
    } else {
        handle_main_menu_keys(app, code)
    }
}

/// Handle key events in the main menu.
fn handle_main_menu_keys(app: &mut App, code: KeyCode) -> HandleResult {
    match code {
        KeyCode::Up => {
            app.menu_up();
        }
        KeyCode::Down => {
            let menu_count = App::menu_items().len();
            app.menu_down(menu_count, 0);
        }
        KeyCode::Enter | KeyCode::Right => {
            // Handle menu selection
            let menu_items = App::menu_items();
            if let Some(&selected) = menu_items.get(app.menu.selected) {
                match selected {
                    MenuItem::ConnectProvider => {
                        // Enter the provider submenu
                        app.menu.enter_submenu();
                    }
                    MenuItem::Exit => {
                        return HandleResult::Exit;
                    }
                }
            }
        }
        KeyCode::Esc => {
            app.menu.close();
        }
        _ => {}
    }
    HandleResult::Continue
}

/// Handle key events in a submenu (e.g., provider selection).
fn handle_submenu_keys(app: &mut App, code: KeyCode) -> HandleResult {
    match code {
        KeyCode::Up => {
            app.menu_up();
        }
        KeyCode::Down => {
            let submenu_count = Provider::all().len();
            app.menu_down(0, submenu_count);
        }
        KeyCode::Enter => {
            // Start connection flow for selected provider
            if let Some(provider) = app.selected_provider() {
                app.start_connection(provider);
            }
        }
        KeyCode::Esc | KeyCode::Left => {
            // Go back to main menu
            app.menu.exit_submenu();
        }
        _ => {}
    }
    HandleResult::Continue
}

/// Handle key events in normal (non-menu) mode.
fn handle_normal_keys(app: &mut App, code: KeyCode, page_size: usize) -> HandleResult {
    let max_scroll = app.max_scroll();

    match code {
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
            app.scroll_down(max_scroll);
        }
        KeyCode::PageUp => {
            app.scroll_page_up(page_size);
        }
        KeyCode::PageDown => {
            app.scroll_page_down(max_scroll, page_size);
        }
        KeyCode::Home => {
            app.scroll_to_top();
        }
        KeyCode::End => {
            app.scroll_to_bottom(max_scroll);
        }
        KeyCode::Esc => {
            return HandleResult::Exit;
        }
        _ => {}
    }
    HandleResult::Continue
}

// ─────────────────────────────────────────────────────────────────────────────
// Connection dialog handlers
// ─────────────────────────────────────────────────────────────────────────────

/// Handle key events when the connection dialog is active.
fn handle_connect_keys(app: &mut App, code: KeyCode) -> HandleResult {
    match &app.connect {
        ConnectState::None => HandleResult::Continue,
        ConnectState::ExistingCredential { selected, .. } => {
            handle_existing_credential_keys(app, code, *selected)
        }
        ConnectState::SelectingMethod { selected, .. } => {
            handle_selecting_method_keys(app, code, *selected)
        }
        ConnectState::EnteringApiKey { input, cursor, .. } => {
            let input = input.clone();
            let cursor = *cursor;
            handle_entering_api_key_keys(app, code, &input, cursor)
        }
        ConnectState::ValidatingKey { .. } => {
            // No input during validation, but allow Esc to cancel
            if code == KeyCode::Esc {
                app.cancel_connection();
            }
            HandleResult::Continue
        }
        ConnectState::OAuthPending { auth_dialog, .. } | ConnectState::OAuthPolling { auth_dialog, .. } => {
            // Handle OAuth dialog keys
            let mut dialog = auth_dialog.clone();
            match dialog.handle_key(code) {
                AuthDialogResult::OpenBrowser => {
                    // Open browser to the verification URL
                    let url = dialog.verification_url().to_string();
                    if open::that(&url).is_err() {
                        app.toast_error("Could not open browser");
                    }
                }
                AuthDialogResult::Cancel => {
                    app.cancel_connection();
                }
                AuthDialogResult::Continue => {}
            }
            HandleResult::Continue
        }
        ConnectState::SelectingAnthropicMethod { selected } => {
            handle_selecting_anthropic_method_keys(app, code, *selected)
        }
        ConnectState::EnteringAuthCode { input, cursor, .. } => {
            let input = input.clone();
            let cursor = *cursor;
            handle_entering_auth_code_keys(app, code, &input, cursor)
        }
        ConnectState::ExchangingCode { .. } => {
            // No input during code exchange, but allow Esc to cancel
            if code == KeyCode::Esc {
                app.cancel_connection();
            }
            HandleResult::Continue
        }
        ConnectState::SelectingModel { selected, .. } => {
            handle_model_selection_keys(app, code, *selected)
        }
    }
}

/// Handle keys in ExistingCredential state.
///
/// Options:
/// - Without saved model: Use existing (0), Enter new (1), Cancel (2)
/// - With saved model: Use existing (0), Change model (1), Enter new (2), Cancel (3)
fn handle_existing_credential_keys(app: &mut App, code: KeyCode, selected: usize) -> HandleResult {
    // Determine if we have a saved model (affects option count)
    let has_saved_model = if let ConnectState::ExistingCredential {
        provider,
        current_model,
        ..
    } = &app.connect
    {
        *provider == Provider::GitHubCopilot && current_model.is_some()
    } else {
        false
    };

    let option_count = if has_saved_model { 4 } else { 3 };

    match code {
        KeyCode::Up => {
            if let ConnectState::ExistingCredential { selected, .. } = &mut app.connect {
                *selected = selected.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if let ConnectState::ExistingCredential { selected, .. } = &mut app.connect {
                if *selected < option_count - 1 {
                    *selected += 1;
                }
            }
        }
        KeyCode::Enter => {
            match selected {
                0 => app.use_existing_credentials(),
                1 if has_saved_model => app.change_copilot_model(),
                1 => app.enter_new_credentials(),
                2 if has_saved_model => app.enter_new_credentials(),
                2 | 3 | _ => app.cancel_connection(),
            }
        }
        KeyCode::Esc => {
            app.cancel_connection();
        }
        _ => {}
    }
    HandleResult::Continue
}

/// Handle keys in SelectingMethod state.
///
/// Options: Enter API Key (0), Create API Key (1), Cancel (2)
fn handle_selecting_method_keys(app: &mut App, code: KeyCode, selected: usize) -> HandleResult {
    const OPTION_COUNT: usize = 3;

    match code {
        KeyCode::Up => {
            if let ConnectState::SelectingMethod { selected, .. } = &mut app.connect {
                *selected = selected.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if let ConnectState::SelectingMethod { selected, .. } = &mut app.connect {
                if *selected < OPTION_COUNT - 1 {
                    *selected += 1;
                }
            }
        }
        KeyCode::Enter => {
            match selected {
                0 => {
                    // Enter API Key manually
                    app.enter_new_credentials();
                }
                1 => {
                    // Open browser to create API key
                    if let ConnectState::SelectingMethod { provider, .. } = app.connect {
                        if let Some(url) = provider.api_key_url() {
                            if open::that(url).is_err() {
                                app.toast_error("Could not open browser");
                            }
                        }
                    }
                }
                2 | _ => {
                    app.cancel_connection();
                }
            }
        }
        KeyCode::Esc => {
            app.cancel_connection();
        }
        _ => {}
    }
    HandleResult::Continue
}

/// Handle keys in EnteringApiKey state.
fn handle_entering_api_key_keys(
    app: &mut App,
    code: KeyCode,
    input: &str,
    _cursor: usize,
) -> HandleResult {
    match code {
        KeyCode::Char(c) => {
            if let ConnectState::EnteringApiKey {
                input,
                cursor,
                error,
                ..
            } = &mut app.connect
            {
                input.insert(*cursor, c);
                *cursor += 1;
                // Clear error when user types
                *error = None;
            }
        }
        KeyCode::Backspace => {
            if let ConnectState::EnteringApiKey {
                input,
                cursor,
                error,
                ..
            } = &mut app.connect
            {
                if *cursor > 0 {
                    input.remove(*cursor - 1);
                    *cursor -= 1;
                    *error = None;
                }
            }
        }
        KeyCode::Left => {
            if let ConnectState::EnteringApiKey { cursor, .. } = &mut app.connect {
                if *cursor > 0 {
                    *cursor -= 1;
                }
            }
        }
        KeyCode::Right => {
            if let ConnectState::EnteringApiKey { input, cursor, .. } = &mut app.connect {
                if *cursor < input.len() {
                    *cursor += 1;
                }
            }
        }
        KeyCode::Enter => {
            // Validate and submit
            if !input.is_empty() {
                if let ConnectState::EnteringApiKey { provider, input, .. } = &app.connect {
                    let provider = *provider;
                    let key = input.clone();

                    // Check format first
                    if let Err(e) = provider.validate_api_key_format(&key) {
                        if let ConnectState::EnteringApiKey { error, .. } = &mut app.connect {
                            *error = Some(e.to_string());
                        }
                    } else {
                        // Format is valid - start async validation
                        app.start_validation(provider, key);
                    }
                }
            }
        }
        KeyCode::Esc => {
            app.cancel_connection();
        }
        _ => {}
    }
    HandleResult::Continue
}

/// Handle keys in SelectingAnthropicMethod state.
fn handle_selecting_anthropic_method_keys(app: &mut App, code: KeyCode, selected: usize) -> HandleResult {
    const OPTION_COUNT: usize = 3;

    match code {
        KeyCode::Up => {
            if let ConnectState::SelectingAnthropicMethod { selected } = &mut app.connect {
                *selected = selected.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if let ConnectState::SelectingAnthropicMethod { selected } = &mut app.connect {
                if *selected < OPTION_COUNT - 1 {
                    *selected += 1;
                }
            }
        }
        KeyCode::Enter => {
            app.select_anthropic_method(selected);
        }
        KeyCode::Esc => {
            app.cancel_connection();
        }
        _ => {}
    }
    HandleResult::Continue
}

/// Handle keys in EnteringAuthCode state.
fn handle_entering_auth_code_keys(
    app: &mut App,
    code: KeyCode,
    input: &str,
    _cursor: usize,
) -> HandleResult {
    match code {
        KeyCode::Left => {
            app.move_cursor_left_auth_code();
        }
        KeyCode::Right => {
            app.move_cursor_right_auth_code();
        }
        KeyCode::Home => {
            app.move_cursor_start_auth_code();
        }
        KeyCode::End => {
            app.move_cursor_end_auth_code();
        }
        KeyCode::Backspace => {
            app.backspace_auth_code();
        }
        KeyCode::Delete => {
            app.delete_auth_code();
        }
        KeyCode::Char(c) => {
            app.insert_char_auth_code(c);
        }
        KeyCode::Enter => {
            // Submit the authorization code
            if !input.is_empty() {
                app.submit_auth_code();
            }
        }
        KeyCode::Esc => {
            app.cancel_connection();
        }
        _ => {}
    }
    HandleResult::Continue
}

/// Handle keys in SelectingModel state.
fn handle_model_selection_keys(app: &mut App, code: KeyCode, selected: usize) -> HandleResult {
    // Get provider to determine which model list to use
    let provider = if let ConnectState::SelectingModel { provider, .. } = &app.connect {
        *provider
    } else {
        return HandleResult::Continue;
    };

    let models: &[(&str, &str)] = match provider {
        Provider::Anthropic => ANTHROPIC_MODELS,
        Provider::GitHubCopilot => COPILOT_MODELS,
        _ => return HandleResult::Continue,
    };

    let model_count = models.len();

    match code {
        KeyCode::Up => {
            if let ConnectState::SelectingModel { selected, .. } = &mut app.connect {
                *selected = selected.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if let ConnectState::SelectingModel { selected, .. } = &mut app.connect {
                if *selected < model_count - 1 {
                    *selected += 1;
                }
            }
        }
        KeyCode::Enter => {
            // Get the API model ID for the selected model
            if let Some((_display_name, api_id)) = models.get(selected) {
                app.complete_model_selection(api_id);
            }
        }
        KeyCode::Esc => {
            app.cancel_connection();
        }
        _ => {}
    }
    HandleResult::Continue
}
