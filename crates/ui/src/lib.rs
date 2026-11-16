//! CommSCraft's desktop interface (egui): theme, view model, views and demo fixtures.
//!
//! This crate draws state and reports intents; it has no protocol, network or file code. The
//! protocol core never depends on it. Adapters turn session events into [`model::AppEvent`]s and
//! [`app::UiIntent`]s into session commands.

pub mod app;
pub mod demo;
pub mod format;
pub mod model;
pub mod text;
pub mod theme;
mod views;
pub mod widgets;

pub use app::{CommsUi, UiIntent};
