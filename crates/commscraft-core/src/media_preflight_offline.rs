//! Offline-only safe UI staging. No calls, location lookup, media traffic, ICE or SRTP.
const MAX: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Duplicate,
    Capacity,
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
#[derive(Default)]
pub struct PstnNumberDraft {
    number: Option<String>,
}
impl PstnNumberDraft {
    pub fn stage(&mut self, number: &str) -> Result<(), Error> {
        if number.len() < 4
            || number.len() > 16
            || !number.starts_with('+')
            || !number[1..].bytes().all(|b| b.is_ascii_digit())
        {
            return Err(Error::Invalid);
        }
        self.number = Some(number.into());
        Ok(())
    }
    pub fn number(&self) -> Option<&str> {
        self.number.as_deref()
    }
}
#[derive(Default)]
pub struct EmergencyBannerDraft {
    acknowledged: bool,
}
impl EmergencyBannerDraft {
    pub fn acknowledge_local_warning(&mut self) {
        self.acknowledged = true;
    }
    pub fn acknowledged(&self) -> bool {
        self.acknowledged
    }
}
#[derive(Default)]
pub struct VideoDeviceDraft {
    devices: Vec<String>,
}
impl VideoDeviceDraft {
    pub fn stage_label(&mut self, label: &str) -> Result<(), Error> {
        if !token(label) {
            return Err(Error::Invalid);
        }
        if self.devices.iter().any(|x| x.eq_ignore_ascii_case(label)) {
            return Err(Error::Duplicate);
        }
        if self.devices.len() >= MAX {
            return Err(Error::Capacity);
        }
        self.devices.push(label.into());
        Ok(())
    }
    pub fn devices(&self) -> &[String] {
        &self.devices
    }
}
#[derive(Default)]
pub struct IceCandidateDraft {
    priorities: Vec<u32>,
}
impl IceCandidateDraft {
    pub fn stage_priority(&mut self, priority: u32) -> Result<(), Error> {
        if priority == 0 {
            return Err(Error::Invalid);
        }
        if self.priorities.contains(&priority) {
            return Err(Error::Duplicate);
        }
        if self.priorities.len() >= MAX {
            return Err(Error::Capacity);
        }
        self.priorities.push(priority);
        Ok(())
    }
    pub fn priorities(&self) -> &[u32] {
        &self.priorities
    }
}
#[derive(Default)]
pub struct SrtpSuiteDraft {
    suite: Option<String>,
}
impl SrtpSuiteDraft {
    pub fn stage_label(&mut self, suite: &str) -> Result<(), Error> {
        if !matches!(suite, "AES_CM_128_HMAC_SHA1_80" | "AES_CM_128_HMAC_SHA1_32") {
            return Err(Error::Invalid);
        }
        self.suite = Some(suite.into());
        Ok(())
    }
    pub fn label(&self) -> Option<&str> {
        self.suite.as_deref()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pstn_draft_rejects_bad_numbers() {
        let mut x = PstnNumberDraft::default();
        assert_eq!(x.stage("+49123456"), Ok(()));
        assert_eq!(x.stage("911"), Err(Error::Invalid));
        assert_eq!(x.stage("+49\r23"), Err(Error::Invalid));
    }
    #[test]
    fn emergency_draft_does_not_silently_acknowledge() {
        let mut x = EmergencyBannerDraft::default();
        assert!(!x.acknowledged());
        x.acknowledge_local_warning();
        assert!(x.acknowledged());
    }
    #[test]
    fn video_device_draft_rejects_duplicate_and_controls() {
        let mut x = VideoDeviceDraft::default();
        assert_eq!(x.stage_label("camera_1"), Ok(()));
        assert_eq!(x.stage_label("CAMERA_1"), Err(Error::Duplicate));
        assert_eq!(x.stage_label("bad\n"), Err(Error::Invalid));
    }
    #[test]
    fn ice_priority_draft_rejects_duplicate_and_zero() {
        let mut x = IceCandidateDraft::default();
        assert_eq!(x.stage_priority(123), Ok(()));
        assert_eq!(x.stage_priority(123), Err(Error::Duplicate));
        assert_eq!(x.stage_priority(0), Err(Error::Invalid));
    }
    #[test]
    fn srtp_suite_draft_rejects_unrecognized_labels() {
        let mut x = SrtpSuiteDraft::default();
        assert_eq!(x.stage_label("AES_CM_128_HMAC_SHA1_80"), Ok(()));
        assert_eq!(x.stage_label("NULL_CIPHER"), Err(Error::Invalid));
    }
}
