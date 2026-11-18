//! The client runtime shared by the command-line tool and the desktop client: discovery with
//! the trust model and the endpoint ID (`setup`), the media thread of a call (`calls`),
//! ringing, notification sounds and music on hold (`cues`), and the address book web service
//! (`directory`), the video of a call with its sandboxed window process (`video_call`,
//! `video_window`), viewing a window shared over RDP (`rdp_call`) and the content of a meeting
//! (`meeting_content`). These are adapters with threads, devices and sockets; the protocol state
//! machines stay in their own crates.

pub mod address_book;
pub mod calls;
pub mod cues;
pub mod directory;
pub mod exchange;
pub mod meeting_content;
pub mod photos;
pub mod rdp_call;
pub mod setup;
pub mod tlsdsk;
pub mod video_call;
pub mod video_window;
