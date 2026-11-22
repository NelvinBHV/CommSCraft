//! Screen 3: calls – the active call with its controls and the call history.

use crate::app::CommsUi;
use crate::format;
use crate::model::{CallKind, CallState};

/// The call state in words; "connected" only after the session reported the answer.
pub fn state_text(app: &CommsUi, s: CallState, held: bool, kind: CallKind) -> String {
    let t = app.t;
    let connected = match kind {
        CallKind::Audio => t.call_connected,
        CallKind::Video => t.call_connected_video,
        CallKind::Screen => t.call_connected_screen,
        CallKind::Sharing => t.call_presenting,
    };
    match s {
        CallState::Dialing => t.call_dialing.to_string(),
        CallState::Ringing => t.call_ringing.to_string(),
        CallState::Incoming => t.call_incoming.to_string(),
        CallState::Connected { since } if held => format!(
            "{} · {}",
            t.call_on_hold,
            format::duration(app.now.saturating_sub(since))
        ),
        CallState::Connected { since } => format!(
            "{} · {}",
            connected,
            format::duration(app.now.saturating_sub(since))
        ),
        CallState::Ended { status: 0 } => t.call_ended.to_string(),
        CallState::Ended { status } => format!("{} ({status})", t.call_ended),
    }
}
