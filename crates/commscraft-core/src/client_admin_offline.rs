//! Offline-only drafts: never transport, authenticate, save files or contact a server.
const LIMIT: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftError {
    Invalid,
    Duplicate,
    Capacity,
    Missing,
}
fn safe(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
}
fn sip(s: &str) -> bool {
    safe(s)
        && s.starts_with("sip:")
        && s.matches('@').count() == 1
        && !s.bytes().any(|b| b.is_ascii_whitespace())
}
#[derive(Default)]
pub struct CallMonitorDraft {
    active: Vec<String>,
}
impl CallMonitorDraft {
    pub fn start(&mut self, uri: &str) -> Result<(), DraftError> {
        if !sip(uri) {
            return Err(DraftError::Invalid);
        }
        if self.active.iter().any(|x| x.eq_ignore_ascii_case(uri)) {
            return Err(DraftError::Duplicate);
        }
        if self.active.len() >= LIMIT {
            return Err(DraftError::Capacity);
        }
        self.active.push(uri.into());
        Ok(())
    }
    pub fn end(&mut self, uri: &str) -> Result<(), DraftError> {
        let i = self
            .active
            .iter()
            .position(|x| x.eq_ignore_ascii_case(uri))
            .ok_or(DraftError::Missing)?;
        self.active.remove(i);
        Ok(())
    }
    pub fn active(&self) -> &[String] {
        &self.active
    }
}
#[derive(Default)]
pub struct ReliabilityDraft {
    last_failure: Option<&'static str>,
}
impl ReliabilityDraft {
    pub fn record(&mut self, reason: &'static str) -> Result<(), DraftError> {
        if !matches!(reason, "offline" | "timeout" | "invalid-input") {
            return Err(DraftError::Invalid);
        }
        self.last_failure = Some(reason);
        Ok(())
    }
    pub fn clear(&mut self) {
        self.last_failure = None;
    }
    pub fn reason(&self) -> Option<&'static str> {
        self.last_failure
    }
}
#[derive(Default)]
pub struct PolicyPreviewDraft {
    allow_calls: bool,
    allow_messages: bool,
}
impl PolicyPreviewDraft {
    pub fn set(&mut self, calls: bool, messages: bool) {
        self.allow_calls = calls;
        self.allow_messages = messages;
    }
    pub fn preview(&self) -> (bool, bool) {
        (self.allow_calls, self.allow_messages)
    }
}
#[derive(Default)]
pub struct RecordingArchiveDraft {
    labels: Vec<String>,
}
impl RecordingArchiveDraft {
    pub fn stage(&mut self, label: &str) -> Result<(), DraftError> {
        if !safe(label)
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(DraftError::Invalid);
        }
        if self.labels.iter().any(|x| x == label) {
            return Err(DraftError::Duplicate);
        }
        if self.labels.len() >= LIMIT {
            return Err(DraftError::Capacity);
        }
        self.labels.push(label.into());
        Ok(())
    }
    pub fn entries(&self) -> &[String] {
        &self.labels
    }
}
#[derive(Default)]
pub struct ContactImportPreviewDraft {
    entries: Vec<String>,
}
impl ContactImportPreviewDraft {
    pub fn preview(&mut self, uri: &str) -> Result<(), DraftError> {
        if !sip(uri) {
            return Err(DraftError::Invalid);
        }
        if self.entries.iter().any(|x| x.eq_ignore_ascii_case(uri)) {
            return Err(DraftError::Duplicate);
        }
        if self.entries.len() >= LIMIT {
            return Err(DraftError::Capacity);
        }
        self.entries.push(uri.into());
        Ok(())
    }
    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn call_monitor_rejects_duplicates_and_invalid_sip() {
        let mut x = CallMonitorDraft::default();
        assert_eq!(x.start("sip:a@example.test"), Ok(()));
        assert_eq!(x.start("sip:A@example.test"), Err(DraftError::Duplicate));
        assert_eq!(x.start("sip:a@example.test\r"), Err(DraftError::Invalid));
        assert_eq!(x.end("sip:a@example.test"), Ok(()));
        assert_eq!(x.end("sip:a@example.test"), Err(DraftError::Missing));
    }
    #[test]
    fn reliability_only_allows_static_safe_failures() {
        let mut x = ReliabilityDraft::default();
        assert_eq!(x.record("timeout"), Ok(()));
        assert_eq!(x.record("password"), Err(DraftError::Invalid));
        x.clear();
        assert_eq!(x.reason(), None);
    }
    #[test]
    fn policy_preview_never_enforces_server_policy() {
        let mut x = PolicyPreviewDraft::default();
        x.set(false, true);
        assert_eq!(x.preview(), (false, true));
    }
    #[test]
    fn archive_metadata_rejects_paths_and_duplicates() {
        let mut x = RecordingArchiveDraft::default();
        assert_eq!(x.stage("rec_01"), Ok(()));
        assert_eq!(x.stage("../secret"), Err(DraftError::Invalid));
        assert_eq!(x.stage("rec_01"), Err(DraftError::Duplicate));
        assert_eq!(x.entries(), &["rec_01"]);
    }
    #[test]
    fn import_preview_deduplicates_and_validates() {
        let mut x = ContactImportPreviewDraft::default();
        assert_eq!(x.preview("sip:a@example.test"), Ok(()));
        assert_eq!(x.preview("sip:A@example.test"), Err(DraftError::Duplicate));
        assert_eq!(x.preview("sip:a@example.test\n"), Err(DraftError::Invalid));
    }
}
