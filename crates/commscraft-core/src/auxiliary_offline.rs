//! Bounded, offline-only calendar availability, contact photos and distribution lists.
//! No Exchange, SfB server, directory sync or authorization functionality.

const MAX_ENTRY: usize = 128;
const MAX_LABEL: usize = 512;

fn valid(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_LABEL && !s.chars().any(char::is_control)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftError { Invalid, Duplicate, Capacity, Missing }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarAvailability { Free, Busy, OutOfOffice }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarSlot {
    pub start_utc_seconds: i64,
    pub end_utc_seconds: i64,
    pub availability: CalendarAvailability,
}
#[derive(Default)]
pub struct OfflineCalendar { slots: Vec<CalendarSlot> }
impl OfflineCalendar {
    pub fn add(&mut self, slot: CalendarSlot) -> Result<(), DraftError> {
        if slot.start_utc_seconds >= slot.end_utc_seconds { return Err(DraftError::Invalid); }
        if self.slots.len() >= MAX_ENTRY { return Err(DraftError::Capacity); }
        if self.slots.iter().any(|s| slot.start_utc_seconds < s.end_utc_seconds
            && s.start_utc_seconds < slot.end_utc_seconds) { return Err(DraftError::Duplicate); }
        self.slots.push(slot);
        Ok(())
    }
    pub fn at(&self, instant: i64) -> CalendarAvailability {
        self.slots.iter().find(|s| s.start_utc_seconds <= instant && instant < s.end_utc_seconds)
            .map_or(CalendarAvailability::Free, |s| s.availability)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactPhotoDraft { mime: String, bytes: Vec<u8> }
impl ContactPhotoDraft {
    pub fn new(mime: &str, data: &[u8]) -> Result<Self, DraftError> {
        // Offline preview container only: these bytes are NEVER decoded or executed here.
        if !matches!(mime, "image/png" | "image/jpeg")
            || data.is_empty() || data.len() > 256 * 1024 { return Err(DraftError::Invalid); }
        let signature_ok = match mime {
            "image/png" => data.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]),
            "image/jpeg" => data.starts_with(&[255, 216, 255]),
            _ => false,
        };
        if !signature_ok { return Err(DraftError::Invalid); }
        Ok(Self { mime: mime.into(), bytes: data.into() })
    }
    pub fn mime(&self) -> &str { &self.mime }
    pub fn bytes(&self) -> &[u8] { &self.bytes }
}
#[derive(Default)]
pub struct DistributionListDraft { members: Vec<String> }
impl DistributionListDraft {
    pub fn add_member(&mut self, uri: &str) -> Result<(), DraftError> {
        if !valid(uri) || !uri.starts_with("sip:") || uri.matches('@').count() != 1
            || uri.bytes().any(|b| b.is_ascii_whitespace()) { return Err(DraftError::Invalid); }
        if self.members.iter().any(|s| s.eq_ignore_ascii_case(uri)) {
            return Err(DraftError::Duplicate);
        }
        if self.members.len() >= MAX_ENTRY { return Err(DraftError::Capacity); }
        self.members.push(uri.into());
        Ok(())
    }
    pub fn members(&self) -> &[String] { &self.members }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calendar_rejects_overlaps_and_uses_half_open_interval() {
        let mut c = OfflineCalendar::default();
        let slot = CalendarSlot { start_utc_seconds: 10, end_utc_seconds: 20,
            availability: CalendarAvailability::Busy };
        assert_eq!(c.add(slot.clone()), Ok(()));
        assert_eq!(c.add(slot), Err(DraftError::Duplicate));
        assert_eq!(c.at(10), CalendarAvailability::Busy);
        assert_eq!(c.at(20), CalendarAvailability::Free);
        assert_eq!(c.add(CalendarSlot { start_utc_seconds: 20, end_utc_seconds: 30,
            availability: CalendarAvailability::OutOfOffice }), Ok(()));
        assert_eq!(c.add(CalendarSlot { start_utc_seconds: 1, end_utc_seconds: 1,
            availability: CalendarAvailability::Free }), Err(DraftError::Invalid));
    }
    #[test]
    fn photos_are_size_and_mime_bounded() {
        let png = [137, 80, 78, 71, 13, 10, 26, 10];
        assert_eq!(ContactPhotoDraft::new("image/png", &png).unwrap().mime(), "image/png");
        assert_eq!(ContactPhotoDraft::new("text/html", b"<script>"), Err(DraftError::Invalid));
        assert_eq!(ContactPhotoDraft::new("image/png", b"bad"), Err(DraftError::Invalid));
        assert_eq!(ContactPhotoDraft::new("image/png", &vec![0; 256 * 1024 + 1]), Err(DraftError::Invalid));
    }
    #[test]
    fn distribution_members_are_unique_and_bounded() {
        let mut d = DistributionListDraft::default();
        assert_eq!(d.add_member("sip:a@example.com"), Ok(()));
        assert_eq!(d.add_member("sip:A@example.com"), Err(DraftError::Duplicate));
        assert_eq!(d.add_member("sip:a@example.com\r\n"), Err(DraftError::Invalid));
        assert_eq!(d.members().len(), 1);
    }
}
