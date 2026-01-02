//! Welcome screen module using Terminal Text Effects (TTE).
//!
//! This module handles displaying an animated welcome screen using the `tte` CLI tool
//! if it's available, otherwise falls back gracefully.

use std::io::{self, Write};
use std::process::{Command, Stdio};

use crossterm::{
    event::{self, Event, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::config::WelcomeConfig;

/// The welcome text to display with TTE effects.
const WELCOME_TEXT: &str = r#"
 _____/\\\\\\\\\\\___________/\\\\\\\\\_____/\\\\\\\\\_______/\\\________/\\\____________________/\\\\\\\\\______________/\\\__________________________/\\\\\\\\\\\_        
  ___/\\\/////////\\\______/\\\////////____/\\\///////\\\____\///\\\____/\\\/__________________/\\\////////______________\/\\\_________________________\/////\\\///__       
   __\//\\\______\///_____/\\\/____________\/\\\_____\/\\\______\///\\\/\\\/__________________/\\\/_______________________\/\\\_____________________________\/\\\_____      
    ___\////\\\___________/\\\______________\/\\\\\\\\\\\/_________\///\\\/___________________/\\\_________________________\/\\\_____________________________\/\\\_____     
     ______\////\\\_______\/\\\______________\/\\\//////\\\___________\/\\\___________________\/\\\_________________________\/\\\_____________________________\/\\\_____    
      _________\////\\\____\//\\\_____________\/\\\____\//\\\__________\/\\\___________________\//\\\________________________\/\\\_____________________________\/\\\_____   
       __/\\\______\//\\\____\///\\\___________\/\\\_____\//\\\_________\/\\\____________________\///\\\______________________\/\\\_____________________________\/\\\_____  
        _\///\\\\\\\\\\\/_______\////\\\\\\\\\__\/\\\______\//\\\________\/\\\______________________\////\\\\\\\\\_____________\/\\\\\\\\\\\\\\\______________/\\\\\\\\\\\_ 
         ___\///////////____________\/////////___\///________\///_________\///__________________________\/////////______________\///////////////______________\///////////__

                                                            
                                                    Built with Rust & Ratatui & TTE

"#;

/// Get the tte command, checking common install locations.
fn get_tte_command() -> Option<Command> {
    // First try PATH
    if Command::new("tte")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some(Command::new("tte"));
    }

    // Try ~/.local/bin/tte (common pip --user install location)
    if let Some(home) = dirs::home_dir() {
        let local_tte = home.join(".local").join("bin").join("tte");
        if local_tte.exists() {
            return Some(Command::new(local_tte));
        }
    }

    None
}

/// Run the TTE effect on the welcome text using config settings.
///
/// Returns `Ok(true)` if TTE ran successfully, `Ok(false)` if TTE is not available,
/// or an error if something went wrong.
pub fn run_welcome_effect(config: &WelcomeConfig) -> io::Result<bool> {
    let mut cmd = match get_tte_command() {
        Some(cmd) => cmd,
        None => return Ok(false),
    };

    // Clear the screen first
    print!("\x1B[2J\x1B[H");
    io::stdout().flush()?;

    // Build arguments based on effect type
    let args = build_tte_args(config);

    let mut child = cmd
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    // Write the welcome text to TTE's stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(WELCOME_TEXT.as_bytes())?;
    }

    // Wait for TTE to complete
    let status = child.wait()?;

    if status.success() {
        // Show "press any key" prompt and wait
        wait_for_keypress()?;
    }

    Ok(status.success())
}

/// Build TTE command arguments from config.
fn build_tte_args(config: &WelcomeConfig) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // Fullscreen canvas options (must come before effect name)
    if config.fullscreen {
        args.extend([
            "--canvas-width".to_string(),
            "0".to_string(), // 0 = match terminal width
            "--canvas-height".to_string(),
            "0".to_string(), // 0 = match terminal height
            "--anchor-canvas".to_string(),
            "c".to_string(), // center canvas in terminal
            "--anchor-text".to_string(),
            "c".to_string(), // center text within canvas
        ]);
    }

    // Effect name
    args.push(config.effect.clone());

    match config.effect.as_str() {
        "beams" => {
            // Beam appearance settings
            args.extend([
                "--beam-row-symbols".to_string(),
                "▂".to_string(),
                "▁".to_string(),
                "_".to_string(),
            ]);
            args.extend([
                "--beam-column-symbols".to_string(),
                "▌".to_string(),
                "▍".to_string(),
                "▎".to_string(),
                "▏".to_string(),
            ]);

            // Speed settings from config
            args.extend([
                "--beam-row-speed-range".to_string(),
                format!("{}-{}", config.beam_row_speed_min, config.beam_row_speed_max),
            ]);
            args.extend([
                "--beam-column-speed-range".to_string(),
                format!("{}-{}", config.beam_column_speed_min, config.beam_column_speed_max),
            ]);
            args.extend(["--beam-delay".to_string(), config.beam_delay.to_string()]);

            // Beam gradient colors from config
            args.push("--beam-gradient-stops".to_string());
            args.extend(config.gradient_stops.iter().cloned());
            args.extend(["--beam-gradient-steps".to_string(), "6".to_string(), "12".to_string()]);
            args.extend(["--beam-gradient-frames".to_string(), "4".to_string()]);

            // Final wipe settings from config
            args.push("--final-gradient-stops".to_string());
            args.extend(config.final_gradient_stops.iter().cloned());
            args.extend(["--final-gradient-steps".to_string(), "12".to_string()]);
            args.extend(["--final-gradient-frames".to_string(), "6".to_string()]);
            args.extend(["--final-gradient-direction".to_string(), "vertical".to_string()]);
            args.extend([
                "--final-wipe-speed".to_string(),
                config.final_wipe_speed.to_string(),
            ]);
        }
        "decrypt" => {
            // Decrypt effect - simpler config
            args.push("--typing-speed".to_string());
            args.push("2".to_string());
            args.push("--final-gradient-stops".to_string());
            args.extend(config.final_gradient_stops.iter().cloned());
        }
        "rain" => {
            // Rain effect
            args.extend(["--rain-symbols".to_string(), "o".to_string(), ".".to_string()]);
            args.push("--final-gradient-stops".to_string());
            args.extend(config.final_gradient_stops.iter().cloned());
        }
        "slide" => {
            // Slide effect
            args.push("--final-gradient-stops".to_string());
            args.extend(config.final_gradient_stops.iter().cloned());
        }
        "waves" => {
            // Waves effect
            args.push("--wave-gradient-stops".to_string());
            args.extend(config.gradient_stops.iter().cloned());
            args.push("--final-gradient-stops".to_string());
            args.extend(config.final_gradient_stops.iter().cloned());
        }
        _ => {
            // Generic: just apply final gradient if supported
            args.push("--final-gradient-stops".to_string());
            args.extend(config.final_gradient_stops.iter().cloned());
        }
    }

    args
}

/// Wait for user to press any key to continue.
fn wait_for_keypress() -> io::Result<()> {
    // Print prompt with Miami colors
    println!();
    print!("\x1B[38;2;0;255;255m        Press any key to continue...\x1B[0m");
    io::stdout().flush()?;

    // Enable raw mode to capture single keypress
    enable_raw_mode()?;

    // Wait for a key press
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    break;
                }
            }
        }
    }

    // Disable raw mode before returning
    disable_raw_mode()?;

    Ok(())
}

/// Display a simple welcome message without TTE (fallback).
pub fn run_simple_welcome() -> io::Result<()> {
    // Clear screen
    print!("\x1B[2J\x1B[H");

    // Print with basic ANSI colors (cyan)
    println!("\x1B[36m{}\x1B[0m", WELCOME_TEXT);

    // Show prompt and wait for key
    print!("\x1B[36m        Press any key to continue...\x1B[0m");
    io::stdout().flush()?;

    // Simple blocking read for fallback
    enable_raw_mode()?;
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    break;
                }
            }
        }
    }
    disable_raw_mode()?;

    Ok(())
}

/// Run the welcome screen based on config settings.
pub fn show_welcome(config: &WelcomeConfig) -> io::Result<()> {
    // Skip welcome screen entirely if disabled
    if !config.enabled {
        return Ok(());
    }

    // If TTE is disabled in config, go straight to simple welcome
    if !config.use_tte {
        return run_simple_welcome();
    }

    // Try TTE, fall back to simple if not available or fails
    match run_welcome_effect(config) {
        Ok(true) => Ok(()), // TTE worked
        Ok(false) => {
            // TTE not available, use simple welcome
            run_simple_welcome()
        }
        Err(_) => {
            // TTE failed, use simple welcome
            run_simple_welcome()
        }
    }
}
