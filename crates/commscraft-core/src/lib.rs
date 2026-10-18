//! Clean-room, offline-first Skype for Business protocol primitives.
//!
//! This crate deliberately does NOT implement complete authentication, live SIP networking,
//! server registration, presence publication, IM interoperability or media transport.
//! The transaction and stream components operate only on offline data.
//! Its inputs must be treated as untrusted when integrated into a network client.

pub mod discovery;
pub mod registration;
pub mod challenge;
pub mod session;
pub mod sip;
pub mod stream;
pub mod transaction;
pub mod security;
pub mod presence;
pub mod im;
pub mod sdp;

pub mod kerberos;
pub mod kerberos_mic;

// Native SSPI is Windows-only and deliberately not linked on other platforms.
#[cfg(windows)]
pub mod windows_sspi;

pub mod inbound;

pub mod client_offline;

pub mod local_features;

pub mod auxiliary_offline;

pub mod interaction_offline;

pub mod call_drafts;

pub mod usability_offline;

pub mod contacts_ui_offline;

pub mod client_preferences_offline;

pub mod meeting_controls_offline;

pub mod additional_meeting_offline;

pub mod communication_drafts_offline;

pub mod client_operations_offline;

pub mod client_admin_offline;

pub mod meeting_entry_offline;

pub mod advanced_client_offline;

pub mod delegate_meetings_offline;

pub mod enterprise_preview_offline;

pub mod collaboration_staging_offline;
