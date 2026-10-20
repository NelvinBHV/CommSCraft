//! Local-only, bounded address book, privacy relation and rich text preview.
//! Neither data here nor its tests establish Microsoft server interoperability.

const LIMIT: usize = 128;
const MAX_FIELD: usize = 512;

fn safe(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_FIELD && !s.chars().any(char::is_control)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalError {
    Invalid,
    Duplicate,
    Capacity,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactCard {
    pub sip_uri: String,
    pub display_name: String,
}

#[derive(Default)]
pub struct AddressBook {
    cards: Vec<ContactCard>,
}
impl AddressBook {
    pub fn insert(&mut self, uri: &str, name: &str) -> Result<(), LocalError> {
        if !safe(uri) || !uri.starts_with("sip:") || uri.matches('@').count() != 1 || !safe(name) {
            return Err(LocalError::Invalid);
        }
        if self
            .cards
            .iter()
            .any(|card| card.sip_uri.eq_ignore_ascii_case(uri))
        {
            return Err(LocalError::Duplicate);
        }
        if self.cards.len() >= LIMIT {
            return Err(LocalError::Capacity);
        }
        self.cards.push(ContactCard {
            sip_uri: uri.into(),
            display_name: name.into(),
        });
        Ok(())
    }
    pub fn search(&self, query: &str) -> Result<Vec<&ContactCard>, LocalError> {
        if !safe(query) {
            return Err(LocalError::Invalid);
        }
        let q = query.to_lowercase();
        Ok(self
            .cards
            .iter()
            .filter(|card| {
                card.sip_uri.to_lowercase().contains(&q)
                    || card.display_name.to_lowercase().contains(&q)
            })
            .collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalVisibility {
    Blocked,
    Contacts,
    Everyone,
}
#[derive(Default)]
pub struct PrivacyDraft {
    rules: Vec<(String, LocalVisibility)>,
}
impl PrivacyDraft {
    pub fn set(&mut self, uri: &str, visibility: LocalVisibility) -> Result<(), LocalError> {
        if !safe(uri) || !uri.starts_with("sip:") || uri.matches('@').count() != 1 {
            return Err(LocalError::Invalid);
        }
        if let Some(entry) = self
            .rules
            .iter_mut()
            .find(|(existing, _)| existing.eq_ignore_ascii_case(uri))
        {
            entry.1 = visibility;
            return Ok(());
        }
        if self.rules.len() >= LIMIT {
            return Err(LocalError::Capacity);
        }
        self.rules.push((uri.into(), visibility));
        Ok(())
    }
    pub fn visibility(&self, uri: &str) -> Option<LocalVisibility> {
        self.rules
            .iter()
            .find(|(existing, _)| existing.eq_ignore_ascii_case(uri))
            .map(|x| x.1)
    }
}

/// Safe display text: never interpret user-provided HTML, scripts or markdown.
/// A frontend MUST render this as text, not HTML or a DOM fragment.
pub fn safe_message_preview(raw: &str) -> Result<String, LocalError> {
    if !safe(raw) {
        return Err(LocalError::Invalid);
    }
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directory_filters_case_insensitively_and_rejects_injection() {
        let mut book = AddressBook::default();
        assert_eq!(book.insert("sip:alice@example.com", "Alice"), Ok(()));
        assert_eq!(
            book.insert("sip:ALICE@example.com", "Other"),
            Err(LocalError::Duplicate)
        );
        assert_eq!(
            book.insert("sip:bob@example.com", "Bo\nb"),
            Err(LocalError::Invalid)
        );
        assert_eq!(book.search("ALI").unwrap().len(), 1);
        assert_eq!(book.search("\r"), Err(LocalError::Invalid));
    }
    #[test]
    fn privacy_draft_preserves_block_and_does_not_claim_server_policy() {
        let mut rules = PrivacyDraft::default();
        assert_eq!(
            rules.set("sip:alice@example.com", LocalVisibility::Blocked),
            Ok(())
        );
        assert_eq!(
            rules.visibility("sip:ALICE@example.com"),
            Some(LocalVisibility::Blocked)
        );
        assert_eq!(
            rules.set("sip:alice@example.com", LocalVisibility::Contacts),
            Ok(())
        );
        assert_eq!(
            rules.visibility("sip:alice@example.com"),
            Some(LocalVisibility::Contacts)
        );
        assert_eq!(
            rules.set("sip:bob@example.com\n", LocalVisibility::Everyone),
            Err(LocalError::Invalid)
        );
    }
    #[test]
    fn preview_escapes_markup_and_rejects_control_characters() {
        assert_eq!(
            safe_message_preview("<b>Tom & 'Jerry'</b>").unwrap(),
            "&lt;b&gt;Tom &amp; &#39;Jerry&#39;&lt;/b&gt;"
        );
        assert_eq!(
            safe_message_preview("<script>alert(1)</script>").unwrap(),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
        assert_eq!(safe_message_preview("hello\0"), Err(LocalError::Invalid));
        assert_eq!(
            safe_message_preview(&"a".repeat(MAX_FIELD + 1)),
            Err(LocalError::Invalid)
        );
    }
}
