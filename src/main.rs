//! Scry CLI - A beautiful terminal-based chat interface.
//!
//! Built with Rust, Ratatui, and Miami vibes.

use std::io;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use scry_cli::app::App;
use scry_cli::config::Config;
use scry_cli::input;
use scry_cli::welcome;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load();

    // Show welcome screen with TTE effects (if available)
    if let Err(e) = welcome::show_welcome(&config.welcome) {
        eprintln!("Warning: Welcome screen failed: {}", e);
    }

    // Setup terminal for TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app (without the old banner since we showed TTE welcome)
    let mut app = App::new_without_banner_with_config(&config);

    // Run app
    let res = input::run_app(&mut terminal, &mut app, &config);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Handle any errors
    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}
