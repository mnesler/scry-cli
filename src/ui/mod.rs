//! UI module for chat-cli.
//!
//! This module contains all UI rendering logic including:
//! - Main UI layout and rendering
//! - Menu overlay
//! - Modal dialogs
//! - Auth dialogs for OAuth
//! - Gradient utilities
//! - Text processing

mod auth_dialog;
mod dialog;
mod gradient;
mod menu;
mod render;
pub mod text;

pub use auth_dialog::{AuthDialog, AuthDialogResult, AuthDialogState};
pub use dialog::{Dialog, DialogAction, DialogContent, DialogResult, DialogState};
pub use render::ui;
