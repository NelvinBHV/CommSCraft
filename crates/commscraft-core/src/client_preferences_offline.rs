//! Bounded offline presentation prototypes. No SIP, Microsoft services, or OS integrations.
const MAX: usize = 128;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftError { Invalid, Duplicate, Capacity, Missing }
fn valid(s: &str) -> bool { !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control) }
fn uri(s: &str) -> bool { valid(s) && s.starts_with("sip:") && s.matches('@').count() == 1 && !s.bytes().any(|b| b.is_ascii_whitespace()) }
#[derive(Default)]
pub struct AvailabilitySortDraft { contacts: Vec<(String, u8)> }
impl AvailabilitySortDraft {
    pub fn add(&mut self, address: &str, rank: u8) -> Result<(), DraftError> {
        if !uri(address) || rank > 5 { return Err(DraftError::Invalid); }
        if self.contacts.iter().any(|(a, _)| a.eq_ignore_ascii_case(address)) { return Err(DraftError::Duplicate); }
        if self.contacts.len() >= MAX { return Err(DraftError::Capacity); }
        self.contacts.push((address.into(), rank)); Ok(())
    }
    pub fn ordered(&self) -> Vec<&str> {
        let mut v: Vec<_> = self.contacts.iter().collect();
        v.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        v.into_iter().map(|x| x.0.as_str()).collect()
    }
}
#[derive(Default)]
pub struct ContactLayoutDraft { compact: bool, show_presence: bool }
impl ContactLayoutDraft {
    pub fn set_compact(&mut self, enabled: bool) { self.compact = enabled; }
    pub fn set_presence(&mut self, enabled: bool) { self.show_presence = enabled; }
    pub fn settings(&self) -> (bool, bool) { (self.compact, self.show_presence) }
}
#[derive(Default)]
pub struct InviteDraft { recipients: Vec<String> }
impl InviteDraft {
    pub fn add(&mut self, address: &str) -> Result<(), DraftError> {
        if !uri(address) { return Err(DraftError::Invalid); }
        if self.recipients.iter().any(|x| x.eq_ignore_ascii_case(address)) { return Err(DraftError::Duplicate); }
        if self.recipients.len() >= MAX { return Err(DraftError::Capacity); }
        self.recipients.push(address.into()); Ok(())
    }
    pub fn recipients(&self) -> &[String] { &self.recipients }
}
#[derive(Default)]
pub struct SpellcheckDraft { flagged: Vec<String> }
impl SpellcheckDraft {
    pub fn flag(&mut self, word: &str) -> Result<(), DraftError> {
        if !valid(word) || word.bytes().any(|x| x.is_ascii_whitespace()) { return Err(DraftError::Invalid); }
        if self.flagged.iter().any(|x| x == word) { return Err(DraftError::Duplicate); }
        if self.flagged.len() >= MAX { return Err(DraftError::Capacity); }
        self.flagged.push(word.into()); Ok(())
    }
    pub fn flagged(&self) -> &[String] { &self.flagged }
}
#[derive(Default)]
pub struct ContactPrepopulationDraft { addresses: Vec<String> }
impl ContactPrepopulationDraft {
    pub fn stage(&mut self, address: &str) -> Result<(), DraftError> {
        if !uri(address) { return Err(DraftError::Invalid); }
        if self.addresses.iter().any(|x| x.eq_ignore_ascii_case(address)) { return Err(DraftError::Duplicate); }
        if self.addresses.len() >= MAX { return Err(DraftError::Capacity); }
        self.addresses.push(address.into()); Ok(())
    }
    pub fn staged(&self) -> &[String] { &self.addresses }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn availability_sort_is_local_and_bounded() {
        let mut s = AvailabilitySortDraft::default();
        assert_eq!(s.add("sip:b@example.com", 3), Ok(()));
        assert_eq!(s.add("sip:a@example.com", 1), Ok(()));
        assert_eq!(s.ordered(), vec!["sip:a@example.com", "sip:b@example.com"]);
        assert_eq!(s.add("sip:a@example.com", 5), Err(DraftError::Duplicate));
        assert_eq!(s.add("sip:x@example.com", 6), Err(DraftError::Invalid));
    }
    #[test] fn layout_is_just_local_preferences() {
        let mut s = ContactLayoutDraft::default();
        s.set_compact(true); s.set_presence(true);
        assert_eq!(s.settings(), (true, true));
        s.set_compact(false); assert_eq!(s.settings(), (false, true));
    }
    #[test] fn invite_draft_rejects_injection_and_duplicates() {
        let mut i = InviteDraft::default();
        assert_eq!(i.add("sip:a@example.com"), Ok(()));
        assert_eq!(i.add("sip:A@example.com"), Err(DraftError::Duplicate));
        assert_eq!(i.add("sip:bad@example.com\r"), Err(DraftError::Invalid));
    }
    #[test] fn spelling_flags_do_not_claim_dictionary_check() {
        let mut s = SpellcheckDraft::default();
        assert_eq!(s.flag("typoo"), Ok(()));
        assert_eq!(s.flag("bad word"), Err(DraftError::Invalid));
        assert_eq!(s.flag("typoo"), Err(DraftError::Duplicate));
    }
    #[test] fn contact_prepopulation_is_deduplicated() {
        let mut s = ContactPrepopulationDraft::default();
        assert_eq!(s.stage("sip:a@example.com"), Ok(()));
        assert_eq!(s.stage("sip:A@example.com"), Err(DraftError::Duplicate));
        assert_eq!(s.stage("sip:x@example.com\n"), Err(DraftError::Invalid));
    }
}
