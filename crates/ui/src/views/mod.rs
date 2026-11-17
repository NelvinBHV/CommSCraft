//! The views of the interface. Each draws state from [`crate::app::CommsUi`] and pushes
//! intents; none of them does I/O.

pub mod calls;
pub mod chats;
pub mod contacts;
pub mod content;
pub mod meetings;
pub mod search;
pub mod settings;
pub mod shell;
pub mod signin;
