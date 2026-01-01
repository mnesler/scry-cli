//! Chat CLI - A beautiful terminal-based chat interface.
//!
//! Built with Rust, Ratatui, and Miami vibes.

mod app;
mod config;
mod input;
mod message;
mod ui;

use std::io;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;
use config::Config;

fn main() -> Result<()> {
    // Load configuration
    let config = Config::load();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

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
