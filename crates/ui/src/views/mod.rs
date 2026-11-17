//! The views of the interface. Each draws state from [`crate::app::CommsUi`] and pushes
//! intents; none of them does I/O.

pub mod calls;
pub mod chats;
pub mod content;
pub mod conv_window;
pub mod icons;
pub mod main_window;
pub mod meetings;
pub mod options;
pub mod search;
pub mod shell;
pub mod signin;
