//! The client runtime shared by the command-line tool and the desktop client: discovery with
//! the trust model and the endpoint ID (`setup`), the media thread of a call (`calls`),
//! ringing, notification sounds and music on hold (`cues`), and the address book web service
//! (`directory`). These are adapters with threads,
//! devices and sockets; the protocol state machines stay in their own crates.

pub mod calls;
pub mod cues;
pub mod directory;
pub mod setup;
