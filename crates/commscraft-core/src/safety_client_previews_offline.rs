//! In-memory safety-oriented UI placeholders only. NOT emergency signaling, recording,
//! external calling, MFA, or an authenticated session. No I/O, secrets or persistence.
const CAP: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Duplicate,
    Capacity,
}
fn safe(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
}
fn identifier(s: &str) -> bool {
    safe(s)
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn sip(s: &str) -> bool {
    safe(s)
        && s.starts_with("sip:")
        && s.matches('@').count() == 1
        && !s.bytes().any(|b| b.is_ascii_whitespace())
}
#[derive(Default)]
pub struct RecordingConsentPreview {
    consented: bool,
}
impl RecordingConsentPreview {
    pub fn acknowledge_local_notice(&mut self) {
        self.consented = true;
    }
    pub fn consented(&self) -> bool {
        self.consented
    }
}
#[derive(Default)]
pub struct EmergencySipNoticePreview {
    labels: Vec<String>,
}
impl EmergencySipNoticePreview {
    pub fn stage_display_label(&mut self, label: &str) -> Result<(), Error> {
        if !identifier(label) {
            return Err(Error::Invalid);
        }
        if self.labels.iter().any(|x| x.eq_ignore_ascii_case(label)) {
            return Err(Error::Duplicate);
        }
        if self.labels.len() >= CAP {
            return Err(Error::Capacity);
        }
        self.labels.push(label.into());
        Ok(())
    }
    pub fn labels(&self) -> &[String] {
        &self.labels
    }
}
#[derive(Default)]
pub struct EmergencyDistributionPreview {
    destinations: Vec<String>,
}
impl EmergencyDistributionPreview {
    pub fn stage_display_destination(&mut self, address: &str) -> Result<(), Error> {
        if !sip(address) {
            return Err(Error::Invalid);
        }
        if self
            .destinations
            .iter()
            .any(|x| x.eq_ignore_ascii_case(address))
        {
            return Err(Error::Duplicate);
        }
        if self.destinations.len() >= CAP {
            return Err(Error::Capacity);
        }
        self.destinations.push(address.into());
        Ok(())
    }
    pub fn destinations(&self) -> &[String] {
        &self.destinations
    }
}
#[derive(Default)]
pub struct ExternalCallPreview {
    destination: Option<String>,
}
impl ExternalCallPreview {
    pub fn stage_address(&mut self, address: &str) -> Result<(), Error> {
        if !sip(address) {
            return Err(Error::Invalid);
        }
        self.destination = Some(address.into());
        Ok(())
    }
    pub fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }
}
#[derive(Default)]
pub struct MfaMethodDisplayPreview {
    method: Option<String>,
}
impl MfaMethodDisplayPreview {
    pub fn stage_method_name(&mut self, name: &str) -> Result<(), Error> {
        if !identifier(name) {
            return Err(Error::Invalid);
        }
        self.method = Some(name.into());
        Ok(())
    }
    pub fn method(&self) -> Option<&str> {
        self.method.as_deref()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_notice_defaults_to_unacknowledged() {
        let mut x = RecordingConsentPreview::default();
        assert!(!x.consented());
        x.acknowledge_local_notice();
        assert!(x.consented());
    }
    #[test]
    fn e911_sip_notice_rejects_duplicate_and_control_chars() {
        let mut x = EmergencySipNoticePreview::default();
        assert_eq!(x.stage_display_label("notice_1"), Ok(()));
        assert_eq!(x.stage_display_label("NOTICE_1"), Err(Error::Duplicate));
        assert_eq!(x.stage_display_label("bad\n"), Err(Error::Invalid));
    }
    #[test]
    fn e911_distribution_rejects_duplicate_and_injected_uri() {
        let mut x = EmergencyDistributionPreview::default();
        assert_eq!(
            x.stage_display_destination("sip:safety@example.test"),
            Ok(())
        );
        assert_eq!(
            x.stage_display_destination("sip:SAFETY@example.test"),
            Err(Error::Duplicate)
        );
        assert_eq!(
            x.stage_display_destination("sip:bad@example.test\r"),
            Err(Error::Invalid)
        );
    }
    #[test]
    fn external_call_preview_rejects_bad_uri() {
        let mut x = ExternalCallPreview::default();
        assert_eq!(x.stage_address("sip:contact@example.test"), Ok(()));
        assert_eq!(x.stage_address("tel:+49123\n"), Err(Error::Invalid));
    }
    #[test]
    fn mfa_display_rejects_controls_and_path_names() {
        let mut x = MfaMethodDisplayPreview::default();
        assert_eq!(x.stage_method_name("security_key"), Ok(()));
        assert_eq!(x.stage_method_name("../secrets"), Err(Error::Invalid));
        assert_eq!(x.stage_method_name("bad\n"), Err(Error::Invalid));
    }
}
