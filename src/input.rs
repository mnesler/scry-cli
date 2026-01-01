use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Terminal};

use crate::app::App;
use crate::config::Config;
use crate::ui;

/// Result of handling a key event.
pub enum HandleResult {
    /// Continue running the app
    Continue,
    /// Exit the app
    Exit,
}

/// Run the main application loop.
pub fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    config: &Config,
) -> io::Result<()> {
    let behavior = &config.behavior;

    loop {
        terminal.draw(|f| ui::ui(f, app, config))?;

        // Use timeout for animation: fast polling during animation, blocking when done
        let timeout = if !app.banner_animation_complete {
            Duration::from_millis(behavior.animation_frame_ms)
        } else {
            Duration::from_millis(behavior.idle_poll_ms)
        };

        // Poll for events with timeout
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match handle_key_event(app, key.code, key.modifiers, config) {
                        HandleResult::Exit => return Ok(()),
                        HandleResult::Continue => {}
                    }
                }
            }
        }
        // If no event, loop continues and redraws (for animation)
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
    if app.show_menu {
        handle_menu_keys(app, code)
    } else {
        handle_normal_keys(app, code, page_size)
    }
}

/// Handle key events when the menu is open.
fn handle_menu_keys(app: &mut App, code: KeyCode) -> HandleResult {
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
            if let Some(selected) = menu_items.get(app.menu_selected) {
                match *selected {
                    "Exit" => {
                        return HandleResult::Exit;
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
