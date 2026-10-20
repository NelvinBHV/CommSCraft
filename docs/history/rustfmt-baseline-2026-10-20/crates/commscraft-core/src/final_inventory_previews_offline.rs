//! Offline metadata-only previews; no certificate authentication or call processing.
const MAX: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error { Invalid, Duplicate, Capacity }
fn valid_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
#[derive(Default)]
pub struct CertificateAuthDisplayDraft { device_label: Option<String> }
impl CertificateAuthDisplayDraft {
    pub fn stage_device_label(&mut self, label: &str) -> Result<(), Error> {
        if !valid_id(label) { return Err(Error::Invalid); }
        self.device_label = Some(label.into());
        Ok(())
    }
    pub fn device_label(&self) -> Option<&str> { self.device_label.as_deref() }
}
#[derive(Default)]
pub struct ConcurrentCallLoadPreview { call_labels: Vec<String> }
impl ConcurrentCallLoadPreview {
    pub fn stage_call_label(&mut self, label: &str) -> Result<(), Error> {
        if !valid_id(label) { return Err(Error::Invalid); }
        if self.call_labels.iter().any(|x| x.eq_ignore_ascii_case(label)) {
            return Err(Error::Duplicate);
        }
        if self.call_labels.len() >= MAX { return Err(Error::Capacity); }
        self.call_labels.push(label.into());
        Ok(())
    }
    pub fn staged_count(&self) -> usize { self.call_labels.len() }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn certificate_auth_display_rejects_malformed_device_labels() {
        let mut x = CertificateAuthDisplayDraft::default();
        assert_eq!(x.stage_device_label("device_01"), Ok(()));
        assert_eq!(x.stage_device_label("../secret"), Err(Error::Invalid));
        assert_eq!(x.stage_device_label("bad\r"), Err(Error::Invalid));
        assert_eq!(x.device_label(), Some("device_01"));
    }
    #[test]
    fn call_load_preview_rejects_duplicates_and_bounded_overflow() {
        let mut x = ConcurrentCallLoadPreview::default();
        for i in 0..MAX {
            assert_eq!(x.stage_call_label(&format!("call_{i}")), Ok(()));
        }
        assert_eq!(x.staged_count(), MAX);
        assert_eq!(x.stage_call_label("CALL_0"), Err(Error::Duplicate));
        assert_eq!(x.stage_call_label("call_new"), Err(Error::Capacity));
        assert_eq!(x.stage_call_label("bad\n"), Err(Error::Invalid));
    }
}
