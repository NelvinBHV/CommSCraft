//! Desktop notifications: the application hands incoming calls, invitations and new messages
//! to the desktop's notification service (`org.freedesktop.Notifications`); the interface keeps
//! its own notification window only where there is none (`CommsUi::system_notifications`).

use crate::app::CommsUi;
use crate::model::Presence;

impl CommsUi {
    /// Message notifications are off (option), or quiet while the own status is Do Not Disturb.
    pub fn quiet_for_messages(&self) -> bool {
        !self.prefs.message_alerts
            || (self.prefs.quiet_when_dnd && self.state.me.presence == Presence::DoNotDisturb)
    }
}
