//! Bounded local-only user interaction models. Never a SIP/server session.
const MAX: usize = 128;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflineError {
    Invalid,
    Duplicate,
    Capacity,
    Missing,
}
fn identity(s: &str) -> bool {
    s.len() <= 256
        && s.starts_with("sip:")
        && s.matches('@').count() == 1
        && !s.chars().any(char::is_control)
        && !s.bytes().any(|b| b.is_ascii_whitespace())
}
#[derive(Default)]
pub struct GroupChatDraft {
    participants: Vec<String>,
}
impl GroupChatDraft {
    pub fn invite(&mut self, user: &str) -> Result<(), OfflineError> {
        if !identity(user) {
            return Err(OfflineError::Invalid);
        }
        if self
            .participants
            .iter()
            .any(|s| s.eq_ignore_ascii_case(user))
        {
            return Err(OfflineError::Duplicate);
        }
        if self.participants.len() == MAX {
            return Err(OfflineError::Capacity);
        }
        self.participants.push(user.to_owned());
        Ok(())
    }
    pub fn participants(&self) -> &[String] {
        &self.participants
    }
    pub fn remove(&mut self, user: &str) -> Result<(), OfflineError> {
        let index = self
            .participants
            .iter()
            .position(|s| s.eq_ignore_ascii_case(user))
            .ok_or(OfflineError::Missing)?;
        self.participants.remove(index);
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOfferDraft {
    pub filename: String,
    pub bytes: u64,
}
impl FileOfferDraft {
    pub fn new(filename: &str, bytes: u64) -> Result<Self, OfflineError> {
        if filename.is_empty()
            || filename.len() > 255
            || bytes == 0
            || bytes > 25 * 1024 * 1024
            || filename.contains(['/', '\\', ':'])
            || filename == "."
            || filename == ".."
            || filename.chars().any(char::is_control)
        {
            return Err(OfflineError::Invalid);
        }
        Ok(Self {
            filename: filename.into(),
            bytes,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallPhase {
    Idle,
    Ringing,
    Connected,
    Ended,
}
#[derive(Debug)]
pub struct LocalCallDraft {
    phase: CallPhase,
    muted: bool,
    held: bool,
}
impl Default for LocalCallDraft {
    fn default() -> Self {
        Self {
            phase: CallPhase::Idle,
            muted: false,
            held: false,
        }
    }
}
impl LocalCallDraft {
    pub fn ring(&mut self) -> Result<(), OfflineError> {
        if self.phase != CallPhase::Idle {
            return Err(OfflineError::Invalid);
        }
        self.phase = CallPhase::Ringing;
        Ok(())
    }
    pub fn answer(&mut self) -> Result<(), OfflineError> {
        if self.phase != CallPhase::Ringing {
            return Err(OfflineError::Invalid);
        }
        self.phase = CallPhase::Connected;
        Ok(())
    }
    pub fn mute(&mut self, enabled: bool) -> Result<(), OfflineError> {
        if self.phase != CallPhase::Connected {
            return Err(OfflineError::Invalid);
        }
        self.muted = enabled;
        Ok(())
    }
    pub fn hold(&mut self, enabled: bool) -> Result<(), OfflineError> {
        if self.phase != CallPhase::Connected {
            return Err(OfflineError::Invalid);
        }
        self.held = enabled;
        Ok(())
    }
    pub fn end(&mut self) -> Result<(), OfflineError> {
        if matches!(self.phase, CallPhase::Ended | CallPhase::Idle) {
            return Err(OfflineError::Invalid);
        }
        self.phase = CallPhase::Ended;
        self.held = false;
        self.muted = false;
        Ok(())
    }
    pub fn state(&self) -> (CallPhase, bool, bool) {
        (self.phase, self.muted, self.held)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_multiparty_roster() {
        let mut chat = GroupChatDraft::default();
        assert_eq!(chat.invite("sip:a@example.com"), Ok(()));
        assert_eq!(
            chat.invite("sip:A@example.com"),
            Err(OfflineError::Duplicate)
        );
        assert_eq!(
            chat.invite("sip:a@example.com\n"),
            Err(OfflineError::Invalid)
        );
        assert_eq!(chat.remove("sip:a@example.com"), Ok(()));
        assert_eq!(chat.remove("sip:a@example.com"), Err(OfflineError::Missing));
    }
    #[test]
    fn file_drafts_do_not_move_files() {
        assert_eq!(FileOfferDraft::new("report.txt", 100).unwrap().bytes, 100);
        for name in ["../test", "x\\file", "x:y", "bad\rfile"] {
            assert_eq!(FileOfferDraft::new(name, 100), Err(OfflineError::Invalid));
        }
        assert_eq!(FileOfferDraft::new("x", 0), Err(OfflineError::Invalid));
    }
    #[test]
    fn call_controls_require_connected_state_and_clear_on_end() {
        let mut call = LocalCallDraft::default();
        assert_eq!(call.mute(true), Err(OfflineError::Invalid));
        assert_eq!(call.ring(), Ok(()));
        assert_eq!(call.answer(), Ok(()));
        assert_eq!(call.hold(true), Ok(()));
        assert_eq!(call.mute(true), Ok(()));
        assert_eq!(call.end(), Ok(()));
        assert_eq!(call.state(), (CallPhase::Ended, false, false));
        assert_eq!(call.answer(), Err(OfflineError::Invalid));
    }
}
