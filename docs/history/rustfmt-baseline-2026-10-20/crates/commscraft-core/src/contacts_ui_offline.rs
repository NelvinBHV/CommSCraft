//! Explicitly local Windows-client presentation drafts, not SIP presence synchronization.
//! All collections and untrusted text are bounded; no network, files or policy I/O.
const LIMIT: usize = 128;
const TEXT_LIMIT: usize = 256;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftError { Invalid, Duplicate, Capacity, Missing }
fn text_valid(s: &str) -> bool {
    !s.is_empty() && s.len() <= TEXT_LIMIT && !s.chars().any(char::is_control)
}
fn uri_valid(s: &str) -> bool {
    text_valid(s) && s.starts_with("sip:") && s.matches('@').count() == 1
        && !s.bytes().any(|b| b.is_ascii_whitespace())
}
#[derive(Debug, Default)]
pub struct PresenceNoteDraft { note: Option<String> }
impl PresenceNoteDraft {
    pub fn set(&mut self, note: &str) -> Result<(), DraftError> {
        if !text_valid(note) { return Err(DraftError::Invalid); }
        self.note = Some(note.into()); Ok(())
    }
    pub fn clear(&mut self) { self.note = None; }
    pub fn value(&self) -> Option<&str> { self.note.as_deref() }
}
#[derive(Debug, Default)]
pub struct PresenceLocationDraft { label: Option<String> }
impl PresenceLocationDraft {
    pub fn set(&mut self, label: &str) -> Result<(), DraftError> {
        if !text_valid(label) { return Err(DraftError::Invalid); }
        self.label = Some(label.into()); Ok(())
    }
    pub fn clear(&mut self) { self.label = None; }
    pub fn value(&self) -> Option<&str> { self.label.as_deref() }
}
#[derive(Debug, Default)]
pub struct RecentContactsDraft { contacts: Vec<String> }
impl RecentContactsDraft {
    pub fn visit(&mut self, uri: &str) -> Result<(), DraftError> {
        if !uri_valid(uri) { return Err(DraftError::Invalid); }
        if let Some(index) = self.contacts.iter().position(|x| x.eq_ignore_ascii_case(uri)) {
            self.contacts.remove(index);
        }
        if self.contacts.len() == LIMIT { self.contacts.pop(); }
        self.contacts.insert(0, uri.into()); Ok(())
    }
    pub fn entries(&self) -> &[String] { &self.contacts }
    pub fn clear(&mut self) { self.contacts.clear(); }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactDisplay { pub uri: String, pub display_name: String }
#[derive(Debug, Default)]
pub struct ContactSortDraft { contacts: Vec<ContactDisplay> }
impl ContactSortDraft {
    pub fn insert(&mut self, uri: &str, name: &str) -> Result<(), DraftError> {
        if !uri_valid(uri) || !text_valid(name) { return Err(DraftError::Invalid); }
        if self.contacts.iter().any(|x| x.uri.eq_ignore_ascii_case(uri)) { return Err(DraftError::Duplicate); }
        if self.contacts.len() == LIMIT { return Err(DraftError::Capacity); }
        self.contacts.push(ContactDisplay { uri: uri.into(), display_name: name.into() });
        Ok(())
    }
    pub fn sorted(&self) -> Vec<&ContactDisplay> {
        let mut result: Vec<_> = self.contacts.iter().collect();
        result.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase())
            .then_with(|| a.uri.cmp(&b.uri)));
        result
    }
}
#[derive(Debug, Default)]
pub struct CurrentConversationsDraft { sessions: Vec<String> }
impl CurrentConversationsDraft {
    pub fn open(&mut self, uri: &str) -> Result<(), DraftError> {
        if !uri_valid(uri) { return Err(DraftError::Invalid); }
        if self.sessions.iter().any(|x| x.eq_ignore_ascii_case(uri)) { return Err(DraftError::Duplicate); }
        if self.sessions.len() == LIMIT { return Err(DraftError::Capacity); }
        self.sessions.push(uri.into()); Ok(())
    }
    pub fn close(&mut self, uri: &str) -> Result<(), DraftError> {
        let i = self.sessions.iter().position(|x| x.eq_ignore_ascii_case(uri)).ok_or(DraftError::Missing)?;
        self.sessions.remove(i); Ok(())
    }
    pub fn entries(&self) -> &[String] { &self.sessions }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn notes_reject_controls_and_clear() {
        let mut n = PresenceNoteDraft::default();
        assert_eq!(n.set("Working"), Ok(()));
        assert_eq!(n.set("bad\r\n"), Err(DraftError::Invalid));
        assert_eq!(n.value(), Some("Working")); n.clear(); assert_eq!(n.value(), None);
    }
    #[test] fn locations_reject_untrusted_controls() {
        let mut l = PresenceLocationDraft::default();
        assert_eq!(l.set("Office"), Ok(()));
        assert_eq!(l.set("Office\0"), Err(DraftError::Invalid));
        assert_eq!(l.value(), Some("Office"));
    }
    #[test] fn recents_are_unique_most_recent_first_and_bounded() {
        let mut r = RecentContactsDraft::default();
        assert_eq!(r.visit("sip:a@example.com"), Ok(()));
        assert_eq!(r.visit("sip:b@example.com"), Ok(()));
        assert_eq!(r.visit("sip:A@example.com"), Ok(()));
        assert_eq!(r.entries().len(), 2);
        assert_eq!(r.entries()[0], "sip:A@example.com");
        assert_eq!(r.visit("sip:bad@example.com\n"), Err(DraftError::Invalid));
        r.clear(); assert!(r.entries().is_empty());
    }
    #[test] fn contacts_sorted_stably_and_duplicates_denied() {
        let mut s = ContactSortDraft::default();
        assert_eq!(s.insert("sip:z@example.com", "Zoë"), Ok(()));
        assert_eq!(s.insert("sip:a@example.com", "Anna"), Ok(()));
        assert_eq!(s.insert("sip:A@example.com", "Other"), Err(DraftError::Duplicate));
        assert_eq!(s.sorted()[0].display_name, "Anna");
        assert_eq!(s.insert("sip:c@example.com", "bad\n"), Err(DraftError::Invalid));
    }
    #[test] fn current_conversations_guard_state() {
        let mut c = CurrentConversationsDraft::default();
        assert_eq!(c.open("sip:a@example.com"), Ok(()));
        assert_eq!(c.open("sip:A@example.com"), Err(DraftError::Duplicate));
        assert_eq!(c.close("sip:a@example.com"), Ok(()));
        assert_eq!(c.close("sip:a@example.com"), Err(DraftError::Missing));
        assert!(c.entries().is_empty());
    }
}
