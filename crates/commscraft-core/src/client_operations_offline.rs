//! Local-only accessibility, diagnostics and recording-artifact metadata prototypes.
//! Never collects credentials, recordings or network traffic.
const MAX: usize = 64;
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
#[derive(Default)]
pub struct AccessibilityDraft {
    high_contrast: bool,
    reduced_motion: bool,
}
impl AccessibilityDraft {
    pub fn set_high_contrast(&mut self, value: bool) {
        self.high_contrast = value;
    }
    pub fn set_reduced_motion(&mut self, value: bool) {
        self.reduced_motion = value;
    }
    pub fn settings(&self) -> (bool, bool) {
        (self.high_contrast, self.reduced_motion)
    }
}
#[derive(Default)]
pub struct LocalLogDraft {
    events: Vec<&'static str>,
}
impl LocalLogDraft {
    pub fn record(&mut self, event: &'static str) -> Result<(), DraftError> {
        if !matches!(event, "startup" | "shutdown" | "ui-error" | "offline-test") {
            return Err(DraftError::Invalid);
        }
        if self.events.len() >= MAX {
            return Err(DraftError::Capacity);
        }
        self.events.push(event);
        Ok(())
    }
    pub fn events(&self) -> &[&'static str] {
        &self.events
    }
}
#[derive(Default)]
pub struct SignInAttemptDraft {
    results: Vec<bool>,
}
impl SignInAttemptDraft {
    pub fn record_offline_result(&mut self, success: bool) -> Result<(), DraftError> {
        if self.results.len() >= MAX {
            return Err(DraftError::Capacity);
        }
        self.results.push(success);
        Ok(())
    }
    pub fn count(&self) -> usize {
        self.results.len()
    }
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|&&x| x).count()
    }
}
#[derive(Default)]
pub struct RecordingResolutionDraft {
    width: u16,
    height: u16,
}
impl RecordingResolutionDraft {
    pub fn set(&mut self, width: u16, height: u16) -> Result<(), DraftError> {
        if width == 0 || height == 0 || width > 3840 || height > 2160 {
            return Err(DraftError::Invalid);
        }
        self.width = width;
        self.height = height;
        Ok(())
    }
    pub fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }
}
#[derive(Default)]
pub struct RecordingArtifactDraft {
    names: Vec<String>,
}
impl RecordingArtifactDraft {
    pub fn stage_name(&mut self, name: &str) -> Result<(), DraftError> {
        if !safe(name)
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(DraftError::Invalid);
        }
        if self.names.iter().any(|x| x == name) {
            return Err(DraftError::Duplicate);
        }
        if self.names.len() >= MAX {
            return Err(DraftError::Capacity);
        }
        self.names.push(name.into());
        Ok(())
    }
    pub fn names(&self) -> &[String] {
        &self.names
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accessibility_settings_are_local_only() {
        let mut a = AccessibilityDraft::default();
        a.set_high_contrast(true);
        a.set_reduced_motion(true);
        assert_eq!(a.settings(), (true, true));
        a.set_reduced_motion(false);
        assert_eq!(a.settings(), (true, false));
    }
    #[test]
    fn logging_rejects_arbitrary_sensitive_values() {
        let mut l = LocalLogDraft::default();
        assert_eq!(l.record("startup"), Ok(()));
        assert_eq!(l.record("secret-password"), Err(DraftError::Invalid));
        assert_eq!(l.events(), &["startup"]);
    }
    #[test]
    fn sign_in_logs_store_only_boolean_results() {
        let mut s = SignInAttemptDraft::default();
        assert_eq!(s.record_offline_result(false), Ok(()));
        assert_eq!(s.record_offline_result(true), Ok(()));
        assert_eq!(s.count(), 2);
        assert_eq!(s.success_count(), 1);
    }
    #[test]
    fn recording_resolution_rejects_unsupported_bounds() {
        let mut r = RecordingResolutionDraft::default();
        assert_eq!(r.set(1920, 1080), Ok(()));
        assert_eq!(r.set(0, 1080), Err(DraftError::Invalid));
        assert_eq!(r.set(8000, 8000), Err(DraftError::Invalid));
        assert_eq!(r.dimensions(), (1920, 1080));
    }
    #[test]
    fn recording_artifacts_reject_paths_and_duplicates() {
        let mut a = RecordingArtifactDraft::default();
        assert_eq!(a.stage_name("session_01"), Ok(()));
        assert_eq!(a.stage_name("../secret"), Err(DraftError::Invalid));
        assert_eq!(a.stage_name("session_01"), Err(DraftError::Duplicate));
        assert_eq!(a.names(), &["session_01"]);
    }
}
