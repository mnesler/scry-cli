//! UI module for chat-cli.
//!
//! This module contains all UI rendering logic including:
//! - Main UI layout and rendering
//! - Menu overlay
//! - Modal dialogs
//! - Auth dialogs for OAuth
//! - Toast notifications
//! - Gradient utilities
//! - Text processing

pub mod anthropic_dialogs;
mod auth_dialog;
mod dialog;
mod gradient;
mod menu;
mod render;
pub mod text;
mod toast;

pub use auth_dialog::{AuthDialog, AuthDialogResult, AuthDialogState};
pub use dialog::{Dialog, DialogAction, DialogContent, DialogResult, DialogState};
pub use render::ui;
pub use toast::{render_toasts, Toast, ToastLevel, ToastState};
