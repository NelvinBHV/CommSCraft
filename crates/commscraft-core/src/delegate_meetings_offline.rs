//! Pure, bounded local previews. Never opens a connection, controls remote media or authenticates.
const MAX: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Duplicate,
    Capacity,
    Missing,
}
fn text_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
}
fn address_ok(s: &str) -> bool {
    text_ok(s)
        && s.starts_with("sip:")
        && s.matches('@').count() == 1
        && !s.bytes().any(|b| b.is_ascii_whitespace())
}
#[derive(Default)]
pub struct RemoteControlPreview {
    requested: Option<String>,
}
impl RemoteControlPreview {
    pub fn request(&mut self, address: &str) -> Result<(), Error> {
        if !address_ok(address) {
            return Err(Error::Invalid);
        }
        self.requested = Some(address.into());
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.requested = None;
    }
    pub fn target(&self) -> Option<&str> {
        self.requested.as_deref()
    }
}
#[derive(Default)]
pub struct DelegatedOutboundPreview {
    recipients: Vec<String>,
}
impl DelegatedOutboundPreview {
    pub fn stage(&mut self, address: &str) -> Result<(), Error> {
        if !address_ok(address) {
            return Err(Error::Invalid);
        }
        if self
            .recipients
            .iter()
            .any(|x| x.eq_ignore_ascii_case(address))
        {
            return Err(Error::Duplicate);
        }
        if self.recipients.len() >= MAX {
            return Err(Error::Capacity);
        }
        self.recipients.push(address.into());
        Ok(())
    }
    pub fn recipients(&self) -> &[String] {
        &self.recipients
    }
}
#[derive(Default)]
pub struct DelegatedInboundPreview {
    callers: Vec<String>,
}
impl DelegatedInboundPreview {
    pub fn stage(&mut self, address: &str) -> Result<(), Error> {
        if !address_ok(address) {
            return Err(Error::Invalid);
        }
        if self.callers.iter().any(|x| x.eq_ignore_ascii_case(address)) {
            return Err(Error::Duplicate);
        }
        if self.callers.len() >= MAX {
            return Err(Error::Capacity);
        }
        self.callers.push(address.into());
        Ok(())
    }
    pub fn callers(&self) -> &[String] {
        &self.callers
    }
}
#[derive(Default)]
pub struct MeetingRolePreview {
    attendees: Vec<String>,
}
impl MeetingRolePreview {
    pub fn add(&mut self, address: &str) -> Result<(), Error> {
        if !address_ok(address) {
            return Err(Error::Invalid);
        }
        if self
            .attendees
            .iter()
            .any(|x| x.eq_ignore_ascii_case(address))
        {
            return Err(Error::Duplicate);
        }
        if self.attendees.len() >= MAX {
            return Err(Error::Capacity);
        }
        self.attendees.push(address.into());
        Ok(())
    }
    pub fn remove(&mut self, address: &str) -> Result<(), Error> {
        let index = self
            .attendees
            .iter()
            .position(|x| x.eq_ignore_ascii_case(address))
            .ok_or(Error::Missing)?;
        self.attendees.remove(index);
        Ok(())
    }
    pub fn attendees(&self) -> &[String] {
        &self.attendees
    }
}
#[derive(Default)]
pub struct DelegateCalendarPreview {
    labels: Vec<String>,
}
impl DelegateCalendarPreview {
    pub fn stage(&mut self, label: &str) -> Result<(), Error> {
        if !text_ok(label) {
            return Err(Error::Invalid);
        }
        if self.labels.iter().any(|x| x.eq_ignore_ascii_case(label)) {
            return Err(Error::Duplicate);
        }
        if self.labels.len() >= MAX {
            return Err(Error::Capacity);
        }
        self.labels.push(label.into());
        Ok(())
    }
    pub fn labels(&self) -> &[String] {
        &self.labels
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_control_preview_rejects_invalid_address() {
        let mut x = RemoteControlPreview::default();
        assert_eq!(x.request("sip:a@example.test"), Ok(()));
        assert_eq!(x.request("sip:bad@example.test\n"), Err(Error::Invalid));
        x.cancel();
        assert_eq!(x.target(), None);
    }
    #[test]
    fn delegated_outbound_rejects_duplicates_and_controls() {
        let mut x = DelegatedOutboundPreview::default();
        assert_eq!(x.stage("sip:a@example.test"), Ok(()));
        assert_eq!(x.stage("sip:A@example.test"), Err(Error::Duplicate));
        assert_eq!(x.stage("sip:a@example.test\r"), Err(Error::Invalid));
    }
    #[test]
    fn delegated_inbound_rejects_duplicates_and_controls() {
        let mut x = DelegatedInboundPreview::default();
        assert_eq!(x.stage("sip:a@example.test"), Ok(()));
        assert_eq!(x.stage("sip:A@example.test"), Err(Error::Duplicate));
        assert_eq!(x.stage("sip:a@example.test\n"), Err(Error::Invalid));
    }
    #[test]
    fn meeting_roles_reject_missing_and_duplicate_attendees() {
        let mut x = MeetingRolePreview::default();
        assert_eq!(x.add("sip:a@example.test"), Ok(()));
        assert_eq!(x.add("sip:A@example.test"), Err(Error::Duplicate));
        assert_eq!(x.remove("sip:a@example.test"), Ok(()));
        assert_eq!(x.remove("sip:a@example.test"), Err(Error::Missing));
    }
    #[test]
    fn delegate_calendar_rejects_controls_and_duplicate_labels() {
        let mut x = DelegateCalendarPreview::default();
        assert_eq!(x.stage("Team meeting"), Ok(()));
        assert_eq!(x.stage("team meeting"), Err(Error::Duplicate));
        assert_eq!(x.stage("bad\r"), Err(Error::Invalid));
    }
}
