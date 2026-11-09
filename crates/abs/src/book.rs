//! The decompressed address book file: header, phone normalization rules, attribute table,
//! contacts and trailer ([MS-ABS] 20250520 §2.2.3–§2.2.14, pp. 18–24; AB-05 … AB-09), and the
//! client's use of it – searching users and reverse lookup of phone numbers (§1.3, p. 10).
//!
//! Field order: ABF_ATTRIBUTE and ABF_CONTACT start with their Length, then the Id (K-AB-2, as the
//! field descriptions and the hex example §3.1 show, not the bit diagrams); an
//! ABF_CONTACT_ATTRIBUTE starts with its Id, followed by a Length only for a binary attribute (the
//! Id decides whether the Length is there – inference from the field description, the examples
//! carry no binary contact attribute).
//!
//! Security by design: every length is checked against the remaining bytes before use; at most
//! [`MAX_ATTRIBUTES`] attributes, [`MAX_CONTACTS`] contacts and [`MAX_VALUES`] values per contact;
//! strings are UTF-8 checked and stripped of control characters; a contact referring to an
//! unknown attribute is kept without that value.
//!
//! Delta files (D-XXXX-YYYY, §1.3 p. 10, §2.2.4, §2.2.9, §2.2.13; AB-10) carry the new and
//! changed contacts in full and the deleted ones with NumberOfAttributes 0 in the same list
//! before the sentinel (as the example §3.1.4–§3.1.7, pp. 46–58, shows); compact delta files
//! (C-XXXX-YYYY, §2.2.15–§2.2.16) carry only the changed attributes, a deleted attribute as an
//! empty value. A delta is applied only to the full file it was made from (BaseCreationDate, and
//! BaseFileHash against the base file's Hash).

use crate::Error as BlockError;

/// FileType GUID of a full file (§2.2.3, p. 18).
pub const FULL_FILE: [u8; 16] = [
    0x76, 0x6c, 0xe1, 0x44, 0xfd, 0x0a, 0xa9, 0x40, 0x8b, 0x63, 0x5f, 0xe9, 0xb0, 0x81, 0x73, 0x8f,
];
/// FileType GUIDs of delta and compact delta files (§2.2.4, p. 19).
pub const DELTA_FILE: [u8; 16] = [
    0x16, 0xc1, 0x4b, 0xb5, 0x08, 0x90, 0xc7, 0x47, 0xb9, 0xbd, 0xf3, 0xbb, 0x1a, 0x0a, 0xb6, 0xeb,
];
pub const COMPACT_DELTA_FILE: [u8; 16] = [
    0x34, 0x17, 0x7d, 0xf7, 0x87, 0xae, 0x2b, 0x4d, 0x09, 0xa0, 0x8e, 0xe9, 0xba, 0x89, 0x4a, 0x04,
];

/// Bounds (design).
pub const MAX_ATTRIBUTES: usize = 1024;
pub const MAX_CONTACTS: usize = 500_000;
pub const MAX_VALUES: usize = 256;
/// Largest decompressed file accepted (design: far above the files of large organizations).
pub const MAX_FILE: usize = 256 * 1024 * 1024;

/// Low byte of the attribute flags: the value type (§2.1.2, p. 15).
pub const TYPE_BINARY: u32 = 0x01;
/// Client field values (high byte of the flags, §2.1.4, p. 16).
pub mod field {
    pub const GIVEN_NAME: u8 = 0x01;
    pub const SURNAME: u8 = 0x02;
    pub const DISPLAY_NAME: u8 = 0x03;
    pub const TITLE: u8 = 0x04;
    pub const COMPANY: u8 = 0x06;
    pub const OFFICE: u8 = 0x07;
    pub const SIP_ADDRESS: u8 = 0x08;
    pub const TELEPHONE: u8 = 0x09;
    pub const HOME: u8 = 0x0A;
    pub const MOBILE: u8 = 0x0B;
    pub const OTHER_TELEPHONE: u8 = 0x0C;
    pub const IP_PHONE: u8 = 0x0D;
    pub const MAIL: u8 = 0x0E;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Block(BlockError),
    /// The data ends inside a structure.
    Truncated,
    /// Not a full address book file (other FileType GUID).
    NotFullFile,
    /// A delta or compact delta file where a full file is expected.
    DeltaFile,
    /// Not a delta or compact delta file.
    NotDeltaFile,
    /// The delta was not made from this address book (other base date or base file hash).
    WrongBase,
    /// A length, count or bound does not fit.
    Bad(&'static str),
}

impl From<BlockError> for Error {
    fn from(e: BlockError) -> Self {
        Error::Block(e)
    }
}

/// One attribute of the table (ABF_ATTRIBUTE, §2.2.7, p. 21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub id: u16,
    pub flags: u32,
    pub name: String,
}

impl Attribute {
    pub fn is_binary(&self) -> bool {
        self.flags & 0xff == TYPE_BINARY
    }
    /// The client field (high byte of the flags).
    pub fn client_field(&self) -> u8 {
        (self.flags >> 24) as u8
    }
}

/// A phone number normalization rule (§2.2.5, p. 20): a regular expression in [MC-RegEx] syntax
/// and its replacement, or the built-in `E164` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub pattern: String,
    pub replacement: String,
}

/// One value of a contact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    Binary(Vec<u8>),
}

/// An address book contact (ABF_CONTACT, §2.2.10, p. 22): the objectGUID and its attribute
/// values by attribute Id. A phone attribute may appear twice: as stored and normalized
/// (`tel:+…`, example §3.1.4.4, p. 47).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    pub guid: [u8; 16],
    pub values: Vec<(u16, Value)>,
}

/// A full address book file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressBook {
    /// The XXXX of F-XXXX (days since 2001-01-01).
    pub created: u16,
    /// The server normalized the phone numbers with the rules.
    pub normalized: bool,
    pub rules: Vec<Rule>,
    pub attributes: Vec<Attribute>,
    pub contacts: Vec<Contact>,
    /// The Hash of the trailer, which a delta names as its BaseFileHash.
    pub hash: Option<u16>,
}

/// A delta or compact delta file (§2.2.4, §2.2.13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    /// The XXXX of D-XXXX-YYYY: the full file it was made from.
    pub base: u16,
    /// The YYYY: the full file it leads to.
    pub created: u16,
    /// A compact delta: contacts carry only their changed attributes.
    pub compact: bool,
    pub normalized: bool,
    pub rules: Vec<Rule>,
    pub attributes: Vec<Attribute>,
    /// New and changed contacts.
    pub changed: Vec<Contact>,
    /// The objectGUIDs of deleted contacts.
    pub deleted: Vec<[u8; 16]>,
    pub hash: Option<u16>,
    pub base_hash: Option<u16>,
}

/// One search result: the fields a client shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    pub display_name: Option<String>,
    pub sip: Option<String>,
    pub mail: Option<String>,
    pub title: Option<String>,
    pub company: Option<String>,
    pub office: Option<String>,
    /// Phone numbers as stored and normalized.
    pub phones: Vec<String>,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated)?;
        let s = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, Error> {
        Ok(*self.take(1)?.first().ok_or(Error::Truncated)?)
    }
    fn u16(&mut self) -> Result<u16, Error> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([
            *b.first().ok_or(Error::Truncated)?,
            *b.get(1).ok_or(Error::Truncated)?,
        ]))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        let b: [u8; 4] = self.take(4)?.try_into().map_err(|_| Error::Truncated)?;
        Ok(u32::from_le_bytes(b))
    }
    /// A zero-terminated UTF-8 string, cleaned of control characters.
    fn cstr(&mut self) -> Result<String, Error> {
        let rest = self.data.get(self.pos..).ok_or(Error::Truncated)?;
        let end = rest.iter().position(|b| *b == 0).ok_or(Error::Truncated)?;
        let s = self.take(end)?;
        self.take(1)?;
        Ok(clean(s))
    }
}

fn clean(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}

/// Decompresses an address book file with an upper bound on its size.
pub fn decompress(file: &[u8]) -> Result<Vec<u8>, Error> {
    let out = crate::decompress_file_bounded(file, MAX_FILE)?;
    Ok(out)
}

/// The phone normalization rules: pairs of CRLF-terminated lines, the last byte zero (§2.2.5).
/// Empty lines between pairs are skipped (an empty line is no regular expression; robustness);
/// the line after a pattern is always its replacement – for `E164` it is ignored (p. 20).
fn parse_rules(raw: &[u8]) -> Vec<Rule> {
    let text = clean_lines(raw);
    let mut lines = text.split("\r\n");
    let mut rules = Vec::new();
    while let Some(pattern) = lines.next() {
        if pattern.is_empty() {
            continue;
        }
        let replacement = lines.next().unwrap_or_default();
        if rules.len() < MAX_ATTRIBUTES {
            rules.push(Rule {
                pattern: pattern.to_string(),
                replacement: if pattern == "E164" {
                    String::new()
                } else {
                    replacement.to_string()
                },
            });
        }
    }
    rules
}

/// Keeps CR and LF, drops the terminating zero and other control characters.
fn clean_lines(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw)
        .chars()
        .filter(|c| *c == '\r' || *c == '\n' || !c.is_control())
        .collect()
}

/// The rules and the attribute table after a header (§2.2.5–§2.2.7).
fn read_tables(
    r: &mut Reader<'_>,
    count: usize,
    max_id: u16,
) -> Result<(Vec<Rule>, Vec<Attribute>), Error> {
    if count > MAX_ATTRIBUTES {
        return Err(Error::Bad("attribute count"));
    }
    let rules_len = usize::try_from(r.u32()?).map_err(|_| Error::Bad("rules length"))?;
    let rules = parse_rules(r.take(rules_len)?);
    let mut attributes = Vec::with_capacity(count);
    for _ in 0..count {
        let len = usize::from(r.u16()?);
        let body = r.take(len)?;
        let mut a = Reader { data: body, pos: 0 };
        let id = a.u16()?;
        let flags = a.u32()?;
        let name = a.cstr()?;
        if id > max_id {
            return Err(Error::Bad("attribute id above MaximumAttributeId"));
        }
        attributes.push(Attribute { id, flags, name });
    }
    Ok((rules, attributes))
}

/// The contacts up to the sentinel (§2.2.8–§2.2.11). In a delta a contact without attributes
/// is a deleted one (§2.2.9/§2.2.10); its NumberOfDeletedAttributes and the rest are skipped.
fn read_contacts(
    r: &mut Reader<'_>,
    attributes: &[Attribute],
    max_id: u16,
    delta: bool,
) -> Result<(Vec<Contact>, Vec<[u8; 16]>), Error> {
    let wide_ids = max_id >= 256;
    let mut contacts = Vec::new();
    let mut deleted = Vec::new();
    loop {
        let len = usize::from(r.u16()?);
        let body = r.take(len)?;
        let mut c = Reader { data: body, pos: 0 };
        let guid: [u8; 16] = c.take(16)?.try_into().map_err(|_| Error::Truncated)?;
        let n = usize::from(c.u16()?);
        if guid == [0u8; 16] {
            // The sentinel: no attributes, Length 0x12 (§2.2.10).
            if n != 0 {
                return Err(Error::Bad("sentinel with attributes"));
            }
            break;
        }
        if n > MAX_VALUES {
            return Err(Error::Bad("attribute values per contact"));
        }
        if contacts.len() + deleted.len() >= MAX_CONTACTS {
            return Err(Error::Bad("contact count"));
        }
        if delta && n == 0 {
            deleted.push(guid);
            continue;
        }
        let mut values = Vec::with_capacity(n);
        for _ in 0..n {
            let id = if wide_ids {
                c.u16()?
            } else {
                u16::from(c.u8()?)
            };
            let binary = attributes
                .iter()
                .find(|a| a.id == id)
                .is_some_and(Attribute::is_binary);
            let value = if binary {
                let l = usize::from(c.u16()?);
                Value::Binary(c.take(l)?.to_vec())
            } else {
                Value::Text(c.cstr()?)
            };
            values.push((id, value));
        }
        contacts.push(Contact { guid, values });
    }
    Ok((contacts, deleted))
}

impl AddressBook {
    /// Reads a decompressed full file.
    pub fn parse(data: &[u8]) -> Result<AddressBook, Error> {
        let mut r = Reader { data, pos: 0 };
        let file_type = r.take(16)?;
        if file_type == DELTA_FILE || file_type == COMPACT_DELTA_FILE {
            return Err(Error::DeltaFile);
        }
        if file_type != FULL_FILE {
            return Err(Error::NotFullFile);
        }
        // §2.2.3: CreationDate, NumberOfAttributes, MaximumAttributeId, UseNormalizationRules
        // (order of the field descriptions; AB-05), then 128 bytes SourceStream.
        let created = r.u16()?;
        let count = usize::from(r.u16()?);
        let max_id = r.u16()?;
        let normalized = r.u16()? != 0;
        r.take(128)?;
        let (rules, attributes) = read_tables(&mut r, count, max_id)?;
        let (contacts, _) = read_contacts(&mut r, &attributes, max_id, false)?;
        // §2.2.12/§2.2.14: Hash, then NumberOfContacts; checked when present.
        let t = trailer(data);
        if let Some(count) = t.and_then(|t| le32(t, 2)) {
            if count != contacts.len() {
                return Err(Error::Bad("trailer contact count"));
            }
        }
        Ok(AddressBook {
            created,
            normalized,
            rules,
            attributes,
            contacts,
            hash: t.and_then(|t| le16(t, 0)),
        })
    }

    /// Brings the address book to the state of the delta's target file (AB-10): only a delta made
    /// from this file is accepted. The tables are replaced; new and changed contacts replace the
    /// old ones (in a compact delta only their attributes, an empty value removes one); deleted
    /// contacts are removed.
    ///
    /// Security by design (issues #125, #126): every check runs before anything changes, so a
    /// refused delta leaves the book as it was; contacts are found through hash tables, so the
    /// work grows linearly with the book and the delta.
    pub fn apply(&mut self, d: &Delta) -> Result<(), Error> {
        use std::collections::{HashMap, HashSet};
        let hash_differs = matches!((self.hash, d.base_hash), (Some(a), Some(b)) if a != b);
        if d.base != self.created || hash_differs {
            return Err(Error::WrongBase);
        }
        let deleted: HashSet<&[u8; 16]> = d.deleted.iter().collect();
        let kept = self
            .contacts
            .iter()
            .filter(|c| !deleted.contains(&c.guid))
            .count();
        let known: HashSet<&[u8; 16]> = self
            .contacts
            .iter()
            .filter(|c| !deleted.contains(&c.guid))
            .map(|c| &c.guid)
            .collect();
        let new: HashSet<&[u8; 16]> = d
            .changed
            .iter()
            .map(|c| &c.guid)
            .filter(|g| !known.contains(g))
            .collect();
        if kept.saturating_add(new.len()) > MAX_CONTACTS {
            return Err(Error::Bad("contact count"));
        }
        // From here on nothing can fail.
        self.contacts.retain(|c| !deleted.contains(&c.guid));
        let mut index: HashMap<[u8; 16], usize> = self
            .contacts
            .iter()
            .enumerate()
            .map(|(i, c)| (c.guid, i))
            .collect();
        for c in &d.changed {
            match index.get(&c.guid).and_then(|&i| self.contacts.get_mut(i)) {
                Some(old) if d.compact => {
                    let changed: HashSet<u16> = c.values.iter().map(|(n, _)| *n).collect();
                    old.values.retain(|(id, _)| !changed.contains(id));
                    old.values.extend(
                        c.values
                            .iter()
                            .filter(|(_, v)| !matches!(v, Value::Text(t) if t.is_empty()))
                            .cloned(),
                    );
                    old.values.truncate(MAX_VALUES);
                }
                Some(old) => *old = c.clone(),
                None => {
                    index.insert(c.guid, self.contacts.len());
                    self.contacts.push(c.clone());
                }
            }
        }
        self.created = d.created;
        self.normalized = d.normalized;
        self.rules = d.rules.clone();
        self.attributes = d.attributes.clone();
        self.hash = d.hash;
        Ok(())
    }

    fn field_of(&self, id: u16) -> Option<u8> {
        self.attributes
            .iter()
            .find(|a| a.id == id)
            .map(Attribute::client_field)
    }

    /// The fields of a contact a client shows.
    pub fn entry(&self, c: &Contact) -> Entry {
        let mut e = Entry::default();
        for (id, v) in &c.values {
            let Value::Text(t) = v else { continue };
            if t.is_empty() {
                continue;
            }
            let set = |slot: &mut Option<String>| {
                if slot.is_none() {
                    *slot = Some(t.clone());
                }
            };
            match self.field_of(*id) {
                Some(field::DISPLAY_NAME) => set(&mut e.display_name),
                Some(field::SIP_ADDRESS) => set(&mut e.sip),
                Some(field::MAIL) => set(&mut e.mail),
                Some(field::TITLE) => set(&mut e.title),
                Some(field::COMPANY) => set(&mut e.company),
                Some(field::OFFICE) => set(&mut e.office),
                Some(
                    field::TELEPHONE
                    | field::HOME
                    | field::MOBILE
                    | field::OTHER_TELEPHONE
                    | field::IP_PHONE,
                ) if !e.phones.contains(t) => e.phones.push(t.clone()),
                _ => {}
            }
        }
        e
    }

    /// Contacts whose display name, SIP address, mail address, given name or surname contains
    /// every word of `query` (case-insensitive), at most `limit`.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Entry> {
        let words: Vec<String> = query
            .split_whitespace()
            .map(str::to_lowercase)
            .filter(|w| !w.is_empty())
            .collect();
        if words.is_empty() {
            return Vec::new();
        }
        let searchable = [
            field::DISPLAY_NAME,
            field::SIP_ADDRESS,
            field::MAIL,
            field::GIVEN_NAME,
            field::SURNAME,
        ];
        self.contacts
            .iter()
            .filter(|c| {
                let text: String = c
                    .values
                    .iter()
                    .filter(|(id, _)| self.field_of(*id).is_some_and(|f| searchable.contains(&f)))
                    .filter_map(|(_, v)| match v {
                        Value::Text(t) => Some(t.to_lowercase()),
                        Value::Binary(_) => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                words.iter().all(|w| text.contains(w.as_str()))
            })
            .take(limit)
            .map(|c| self.entry(c))
            .collect()
    }

    /// Reverse lookup (§1.3, p. 10): the contact one of whose phone numbers is `number`,
    /// compared by digits (see [`digits`]).
    pub fn lookup_number(&self, number: &str) -> Option<Entry> {
        let normalizer = crate::normalize::Normalizer::new(&self.rules);
        let mut wanted: Vec<String> = digits(number).into_iter().collect();
        // AB-11: the caller's number in the form the rules give, too.
        if let Some(d) = normalizer.normalize(number).as_deref().and_then(digits) {
            if !wanted.contains(&d) {
                wanted.push(d);
            }
        }
        if wanted.is_empty() {
            return None;
        }
        let matches = |t: &str| {
            digits(t).is_some_and(|d| wanted.contains(&d))
                // The server left the numbers as they are: the client normalizes them (§1.3.1).
                || (!self.normalized
                    && normalizer
                        .normalize(t)
                        .as_deref()
                        .and_then(digits)
                        .is_some_and(|d| wanted.contains(&d)))
        };
        self.contacts
            .iter()
            .find(|c| {
                c.values.iter().any(|(id, v)| match v {
                    Value::Text(t) => {
                        matches!(
                            self.field_of(*id),
                            Some(
                                field::TELEPHONE
                                    | field::HOME
                                    | field::MOBILE
                                    | field::OTHER_TELEPHONE
                                    | field::IP_PHONE
                            )
                        ) && matches(t)
                    }
                    Value::Binary(_) => false,
                })
            })
            .map(|c| self.entry(c))
    }
}

/// The digits of a phone number (`tel:+49 40 1234;ext=12` → `49401234`): the part before the
/// first `;` (parameters such as `ext=` are not compared) without `tel:`, `+` and separators;
/// `None` if anything but digits and separators is there or there are more than 20 digits.
pub fn digits(number: &str) -> Option<String> {
    let n = number.trim();
    let n = n.strip_prefix("tel:").unwrap_or(n);
    let main = n.split(';').next().unwrap_or(n);
    let mut out = String::new();
    for c in main.chars() {
        match c {
            '0'..='9' => out.push(c),
            '+' | ' ' | '-' | '.' | '(' | ')' | '/' => {}
            _ => return None,
        }
    }
    (!out.is_empty() && out.len() <= 20).then_some(out)
}

impl Delta {
    /// Reads a decompressed delta or compact delta file (§2.2.4: BaseCreationDate,
    /// CreationDate, NumberOfAttributes, MaximumAttributeId, UseNormalizationRules – the order
    /// of the field descriptions and the example §3.1.1, p. 32).
    pub fn parse(data: &[u8]) -> Result<Delta, Error> {
        let mut r = Reader { data, pos: 0 };
        let file_type = r.take(16)?;
        let compact = if file_type == DELTA_FILE {
            false
        } else if file_type == COMPACT_DELTA_FILE {
            true
        } else {
            return Err(Error::NotDeltaFile);
        };
        let base = r.u16()?;
        let created = r.u16()?;
        let count = usize::from(r.u16()?);
        let max_id = r.u16()?;
        let normalized = r.u16()? != 0;
        r.take(128)?;
        if base >= created {
            return Err(Error::Bad("delta dates"));
        }
        let (rules, attributes) = read_tables(&mut r, count, max_id)?;
        let (changed, deleted) = read_contacts(&mut r, &attributes, max_id, true)?;
        // §2.2.13: Hash, BaseFileHash, NumberOfContacts (without the deleted ones, example
        // §3.1.8), NumberOfDeletedContacts; checked when present.
        let t = trailer(data);
        if let Some(t) = t {
            if le32(t, 4).is_some_and(|n| n != changed.len())
                || le32(t, 8).is_some_and(|n| n != deleted.len())
            {
                return Err(Error::Bad("trailer contact count"));
            }
        }
        Ok(Delta {
            base,
            created,
            compact,
            normalized,
            rules,
            attributes,
            changed,
            deleted,
            hash: t.and_then(|t| le16(t, 0)),
            base_hash: t.and_then(|t| le16(t, 2)),
        })
    }
}

/// The trailer before ABF_TRAILER_LENGTH (§2.2.14), if it can be found.
fn trailer(data: &[u8]) -> Option<&[u8]> {
    let len_at = data.len().checked_sub(4)?;
    let len = u32::from_le_bytes(data.get(len_at..)?.try_into().ok()?);
    let len = usize::try_from(len).ok()?;
    let start = len_at.checked_sub(len)?;
    data.get(start..len_at)
}

fn le16(t: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(t.get(at..at + 2)?.try_into().ok()?))
}

fn le32(t: &[u8], at: usize) -> Option<usize> {
    let v = u32::from_le_bytes(t.get(at..at + 4)?.try_into().ok()?);
    usize::try_from(v).ok()
}
