//! Snapshot tests for Anthropic OAuth UI dialogs.
//!
//! These tests verify that the TUI renders correctly for various Anthropic
//! authentication states. Uses ratatui's TestBackend to render to a virtual
//! terminal buffer, then compares the output against saved snapshots.

use insta::assert_snapshot;
use ratatui::{backend::TestBackend, Terminal};
use scry_cli::auth::AnthropicAuthMethod;
use scry_cli::ui::anthropic_dialogs::{
    render_anthropic_method_dialog, render_auth_code_entry_dialog, render_exchanging_code_dialog,
};

/// Terminal size for snapshots: 173x64 (user's preferred size)
const TERMINAL_WIDTH: u16 = 173;
const TERMINAL_HEIGHT: u16 = 64;

/// Helper to convert a ratatui Buffer to a plain text string (no ANSI codes).
fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let mut output = String::new();
    for y in 0..buffer.area().height {
        for x in 0..buffer.area().width {
            let cell = buffer.get(x, y);
            // Get just the symbol, strip color information
            output.push_str(cell.symbol());
        }
        output.push('\n');
    }
    output
}

// ─────────────────────────────────────────────────────────────────────────────
// Anthropic Method Selection Dialog Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn anthropic_method_dialog_first_option_selected() {
    let backend = TestBackend::new(TERMINAL_WIDTH, TERMINAL_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render_anthropic_method_dialog(f, 0); // Claude Pro/Max selected
        })
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn anthropic_method_dialog_second_option_selected() {
    let backend = TestBackend::new(TERMINAL_WIDTH, TERMINAL_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render_anthropic_method_dialog(f, 1); // Create API Key selected
        })
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn anthropic_method_dialog_third_option_selected() {
    let backend = TestBackend::new(TERMINAL_WIDTH, TERMINAL_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render_anthropic_method_dialog(f, 2); // Manual API Key selected
        })
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

// ─────────────────────────────────────────────────────────────────────────────
// Authorization Code Entry Dialog Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn auth_code_entry_claude_pro_empty() {
    let backend = TestBackend::new(TERMINAL_WIDTH, TERMINAL_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render_auth_code_entry_dialog(
                f,
                AnthropicAuthMethod::ClaudeProMax,
                "", // empty input
                0,  // cursor position
                None, // no error
            );
        })
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn auth_code_entry_claude_pro_with_code() {
    let backend = TestBackend::new(TERMINAL_WIDTH, TERMINAL_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render_auth_code_entry_dialog(
                f,
                AnthropicAuthMethod::ClaudeProMax,
                "abc123xyz789", // sample code
                12,             // cursor at end
                None,
            );
        })
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn auth_code_entry_claude_pro_with_error() {
    let backend = TestBackend::new(TERMINAL_WIDTH, TERMINAL_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render_auth_code_entry_dialog(
                f,
                AnthropicAuthMethod::ClaudeProMax,
                "invalid_code",
                12,
                Some("Invalid authorization code"), // error message
            );
        })
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn auth_code_entry_create_api_key_empty() {
    let backend = TestBackend::new(TERMINAL_WIDTH, TERMINAL_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render_auth_code_entry_dialog(
                f,
                AnthropicAuthMethod::CreateApiKey,
                "",
                0,
                None,
            );
        })
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn auth_code_entry_create_api_key_with_code() {
    let backend = TestBackend::new(TERMINAL_WIDTH, TERMINAL_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render_auth_code_entry_dialog(
                f,
                AnthropicAuthMethod::CreateApiKey,
                "xyz987abc456",
                12,
                None,
            );
        })
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

// ─────────────────────────────────────────────────────────────────────────────
// Exchanging Code Dialog Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn exchanging_code_dialog() {
    let backend = TestBackend::new(TERMINAL_WIDTH, TERMINAL_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render_exchanging_code_dialog(f);
        })
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}
