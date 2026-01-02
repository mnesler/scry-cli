//! UI module for chat-cli.
//!
//! This module contains all UI rendering logic including:
//! - Main UI layout and rendering
//! - Menu overlay
//! - Modal dialogs
//! - Gradient utilities
//! - Text processing

mod dialog;
mod gradient;
mod menu;
mod render;
pub mod text;

pub use dialog::{Dialog, DialogAction, DialogContent, DialogResult, DialogState};
pub use render::ui;
