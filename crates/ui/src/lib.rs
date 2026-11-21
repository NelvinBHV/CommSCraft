//! CommSCraft's desktop interface (egui): theme, view model, views and demo fixtures.
//!
//! This crate draws state and reports intents; it has no protocol, network or file code. The
//! protocol core never depends on it. Adapters turn session events into [`model::AppEvent`]s and
//! [`app::UiIntent`]s into session commands.

pub mod app;
pub mod content;
pub mod demo;
pub mod emoticons;
pub mod format;
pub mod model;
pub mod text;
pub mod theme;
mod views;
pub mod widgets;

pub use app::{CommsUi, UiIntent};
/// Installs the Lucide icon font into font definitions (see `views::icons`).
pub use views::icons::install as install_icons;

/// `name@domain` or `sip:name@domain` → `sip:name@domain` (the check of the sign-in form).
pub fn normalise_address(text: &str) -> Option<String> {
    views::chats::normalise_address(text)
}
