use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Terminal};

use crate::app::{App, InputMode, MenuItem};
use crate::config::Config;
use crate::ui;

/// Result of handling a key event.
pub enum HandleResult {
    /// Continue running the app
    Continue,
    /// Exit the app
    Exit,
}

/// Cursor blink interval in milliseconds.
const CURSOR_BLINK_MS: u64 = 530;

/// Run the main application loop.
pub fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    config: &Config,
) -> io::Result<()> {
    let behavior = &config.behavior;
    let mut last_cursor_toggle = Instant::now();

    loop {
        // Process any streaming events first
        app.process_stream();
        
        terminal.draw(|f| ui::ui(f, app, config))?;

        // Toggle cursor blink
        if last_cursor_toggle.elapsed() >= Duration::from_millis(CURSOR_BLINK_MS) {
            app.toggle_cursor();
            last_cursor_toggle = Instant::now();
        }

        // Use timeout for animation: fast polling during animation/streaming, slower when idle
        let timeout = if !app.animation.banner_complete || app.is_streaming() {
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

    // Global shortcuts (work in both menu and normal mode)
    match code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            return HandleResult::Exit;
        }
        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
            return HandleResult::Exit;
        }
        KeyCode::Char('p') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_menu();
            return HandleResult::Continue;
        }
        _ => {}
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
    // Check if we're in a menu input mode
    if app.is_menu_input_mode() {
        match code {
            KeyCode::Enter => {
                app.confirm_menu_input();
            }
            KeyCode::Esc => {
                app.cancel_menu_input();
            }
            KeyCode::Backspace => {
                app.handle_menu_backspace();
            }
            KeyCode::Char(c) => {
                app.handle_menu_char(c);
            }
            _ => {}
        }
        return HandleResult::Continue;
    }

    // Normal menu navigation
    match code {
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
            if let Some(&selected) = menu_items.get(app.menu.selected) {
                match selected {
                    MenuItem::ApiKey => {
                        app.start_menu_input(InputMode::ApiKey);
                    }
                    MenuItem::ApiBase => {
                        app.start_menu_input(InputMode::ApiBase);
                    }
                    MenuItem::Model => {
                        app.start_menu_input(InputMode::Model);
                    }
                    MenuItem::SaveConfig => {
                        // Save config to file
                        if let Err(e) = app.save_config() {
                            app.chat.messages.push(crate::message::Message::assistant(
                                format!("Failed to save config: {}", e)
                            ));
                        } else {
                            app.chat.messages.push(crate::message::Message::assistant(
                                "Configuration saved successfully!".to_string()
                            ));
                        }
                        app.menu.visible = false;
                    }
                    MenuItem::Exit => {
                        return HandleResult::Exit;
                    }
                }
            }
        }
        KeyCode::Esc => {
            app.menu.visible = false;
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
