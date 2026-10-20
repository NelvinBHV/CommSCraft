//! Offline, bounded client-side contact groups, publication drafts and IM history.
//! No network publication, server sync, authorization or Windows interop.

const MAX_TEXT: usize = 512;
const MAX_ITEMS: usize = 128;

fn safe_text(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_TEXT &&
    !text.chars().any(|c| c.is_control())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientDataError { Invalid, Duplicate, Capacity, Missing }

#[derive(Debug, Default)]
pub struct ContactGroups {
    groups: Vec<(String, Vec<String>)>,
}
impl ContactGroups {
    pub fn add_group(&mut self, name: &str) -> Result<(), ClientDataError> {
        if !safe_text(name) { return Err(ClientDataError::Invalid); }
        if self.groups.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)) {
            return Err(ClientDataError::Duplicate);
        }
        if self.groups.len() >= MAX_ITEMS { return Err(ClientDataError::Capacity); }
        self.groups.push((name.to_owned(), Vec::new()));
        Ok(())
    }
    pub fn add_member(&mut self, group: &str, sip_uri: &str) -> Result<(), ClientDataError> {
        if !safe_text(sip_uri) || !sip_uri.starts_with("sip:") ||
            sip_uri.matches('@').count() != 1 ||
            sip_uri.contains([' ', '\r', '\n']) {
            return Err(ClientDataError::Invalid);
        }
        let (_, members) = self.groups.iter_mut().find(|(name, _)| name == group)
            .ok_or(ClientDataError::Missing)?;
        if members.iter().any(|m| m.eq_ignore_ascii_case(sip_uri)) {
            return Err(ClientDataError::Duplicate);
        }
        if members.len() >= MAX_ITEMS { return Err(ClientDataError::Capacity); }
        members.push(sip_uri.to_owned());
        Ok(())
    }
    pub fn members(&self, group: &str) -> Option<&[String]> {
        self.groups.iter().find(|(name, _)| name == group)
            .map(|(_, members)| members.as_slice())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability { Available, Busy, DoNotDisturb, Away, Offline }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceDraft {
    pub availability: Availability,
    pub note: Option<String>,
}
impl PresenceDraft {
    pub fn new(availability: Availability, note: Option<&str>) -> Result<Self, ClientDataError> {
        if note.is_some_and(|s| !safe_text(s)) { return Err(ClientDataError::Invalid); }
        Ok(Self { availability, note: note.map(str::to_owned) })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub peer: String,
    pub message: String,
}
#[derive(Debug, Default)]
pub struct ConversationHistory { entries: Vec<HistoryEntry> }
impl ConversationHistory {
    pub fn push(&mut self, peer: &str, message: &str) -> Result<(), ClientDataError> {
        if !safe_text(peer) || !peer.starts_with("sip:") || !peer.contains('@')
            || !safe_text(message) { return Err(ClientDataError::Invalid); }
        if self.entries.len() >= MAX_ITEMS { return Err(ClientDataError::Capacity); }
        self.entries.push(HistoryEntry { peer: peer.to_owned(), message: message.to_owned() });
        Ok(())
    }
    pub fn entries(&self) -> &[HistoryEntry] { &self.entries }
    pub fn clear(&mut self) { self.entries.clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contact_group_rejects_duplicate_injection_and_missing_group() {
        let mut groups = ContactGroups::default();
        assert_eq!(groups.add_group("Team"), Ok(()));
        assert_eq!(groups.add_group("team"), Err(ClientDataError::Duplicate));
        assert_eq!(groups.add_group("bad\r\n"), Err(ClientDataError::Invalid));
        assert_eq!(groups.add_member("unknown", "sip:a@example.com"), Err(ClientDataError::Missing));
        assert_eq!(groups.add_member("Team", "sip:a@example.com"), Ok(()));
        assert_eq!(groups.add_member("Team", "sip:A@example.com"), Err(ClientDataError::Duplicate));
        assert_eq!(groups.add_member("Team", "sip:a@example.com\r\n"), Err(ClientDataError::Invalid));
        assert_eq!(groups.members("Team").unwrap().len(), 1);
    }
    #[test]
    fn presence_draft_is_not_a_server_publication() {
        let draft = PresenceDraft::new(Availability::Busy, Some("In a meeting")).unwrap();
        assert_eq!(draft.note.as_deref(), Some("In a meeting"));
        assert_eq!(PresenceDraft::new(Availability::Away, Some("x\nInjected")), Err(ClientDataError::Invalid));
        assert_eq!(PresenceDraft::new(Availability::Offline, Some("")), Err(ClientDataError::Invalid));
    }
    #[test]
    fn history_is_bounded_and_can_be_wiped() {
        let mut history = ConversationHistory::default();
        assert_eq!(history.push("sip:a@example.com", "hello"), Ok(()));
        assert_eq!(history.push("sip:a@example.com", "x\r\nInjected"), Err(ClientDataError::Invalid));
        for _ in 1..MAX_ITEMS { history.push("sip:a@example.com", "hello").unwrap(); }
        assert_eq!(history.push("sip:a@example.com", "overflow"), Err(ClientDataError::Capacity));
        history.clear();
        assert!(history.entries().is_empty());
    }
}
