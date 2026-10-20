//! Local-only meeting control drafts. No conferencing protocol or server policy is implemented.
const LIMIT: usize = 128;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftError {
    Invalid,
    Duplicate,
    Capacity,
    Missing,
    Denied,
}
fn safe(s: &str) -> bool {
    !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
}
fn sip(s: &str) -> bool {
    safe(s)
        && s.starts_with("sip:")
        && s.matches('@').count() == 1
        && !s.bytes().any(|b| b.is_ascii_whitespace())
}
#[derive(Default)]
pub struct QuestionQueue {
    questions: Vec<String>,
}
impl QuestionQueue {
    pub fn submit(&mut self, q: &str) -> Result<(), DraftError> {
        if !safe(q) {
            return Err(DraftError::Invalid);
        }
        if self.questions.len() >= LIMIT {
            return Err(DraftError::Capacity);
        }
        self.questions.push(q.into());
        Ok(())
    }
    pub fn pending(&self) -> &[String] {
        &self.questions
    }
    pub fn clear(&mut self) {
        self.questions.clear();
    }
}
#[derive(Default)]
pub struct MeetingMediaPermissions {
    video_disabled: bool,
    im_disabled: bool,
}
impl MeetingMediaPermissions {
    pub fn disable_video(&mut self, disabled: bool) {
        self.video_disabled = disabled;
    }
    pub fn disable_im(&mut self, disabled: bool) {
        self.im_disabled = disabled;
    }
    pub fn can_send_video(&self) -> bool {
        !self.video_disabled
    }
    pub fn can_send_im(&self) -> bool {
        !self.im_disabled
    }
}
#[derive(Default)]
pub struct AudienceMuteDraft {
    muted: bool,
}
impl AudienceMuteDraft {
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }
    pub fn can_speak(&self) -> bool {
        !self.muted
    }
}
#[derive(Default)]
pub struct SpotlightDraft {
    speaker: Option<String>,
}
impl SpotlightDraft {
    pub fn focus(&mut self, uri: &str) -> Result<(), DraftError> {
        if !sip(uri) {
            return Err(DraftError::Invalid);
        }
        self.speaker = Some(uri.into());
        Ok(())
    }
    pub fn clear(&mut self) {
        self.speaker = None;
    }
    pub fn speaker(&self) -> Option<&str> {
        self.speaker.as_deref()
    }
}
#[derive(Default)]
pub struct ConferenceGalleryDraft {
    members: Vec<String>,
}
impl ConferenceGalleryDraft {
    pub fn add(&mut self, uri: &str) -> Result<(), DraftError> {
        if !sip(uri) {
            return Err(DraftError::Invalid);
        }
        if self.members.iter().any(|m| m.eq_ignore_ascii_case(uri)) {
            return Err(DraftError::Duplicate);
        }
        if self.members.len() >= LIMIT {
            return Err(DraftError::Capacity);
        }
        self.members.push(uri.into());
        Ok(())
    }
    pub fn remove(&mut self, uri: &str) -> Result<(), DraftError> {
        let index = self
            .members
            .iter()
            .position(|m| m.eq_ignore_ascii_case(uri))
            .ok_or(DraftError::Missing)?;
        self.members.remove(index);
        Ok(())
    }
    pub fn members(&self) -> &[String] {
        &self.members
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qa_rejects_control_and_bounds_queue() {
        let mut q = QuestionQueue::default();
        assert_eq!(q.submit("question?"), Ok(()));
        assert_eq!(q.submit("bad\n"), Err(DraftError::Invalid));
        for _ in 1..LIMIT {
            q.submit("ok").unwrap();
        }
        assert_eq!(q.submit("overflow"), Err(DraftError::Capacity));
        q.clear();
        assert!(q.pending().is_empty());
    }
    #[test]
    fn media_disable_flags_fail_closed_locally() {
        let mut p = MeetingMediaPermissions::default();
        p.disable_video(true);
        p.disable_im(true);
        assert!(!p.can_send_video());
        assert!(!p.can_send_im());
        p.disable_video(false);
        assert!(p.can_send_video());
        assert!(!p.can_send_im());
    }
    #[test]
    fn audience_mute_switches_local_state() {
        let mut m = AudienceMuteDraft::default();
        m.set_muted(true);
        assert!(!m.can_speak());
        m.set_muted(false);
        assert!(m.can_speak());
    }
    #[test]
    fn spotlight_refuses_sip_injection() {
        let mut s = SpotlightDraft::default();
        assert_eq!(s.focus("sip:a@example.com"), Ok(()));
        assert_eq!(s.focus("sip:bad@example.com\r"), Err(DraftError::Invalid));
        assert_eq!(s.speaker(), Some("sip:a@example.com"));
        s.clear();
        assert_eq!(s.speaker(), None);
    }
    #[test]
    fn gallery_rejects_duplicate_and_missing_user() {
        let mut g = ConferenceGalleryDraft::default();
        assert_eq!(g.add("sip:a@example.com"), Ok(()));
        assert_eq!(g.add("sip:A@example.com"), Err(DraftError::Duplicate));
        assert_eq!(g.remove("sip:a@example.com"), Ok(()));
        assert_eq!(g.remove("sip:a@example.com"), Err(DraftError::Missing));
    }
}
