//! The contact list in Exchange, the unified contact store (function 11; EX-01 … EX-05): the
//! EWS operations of [MS-OXWSCOS] 20250520 the client needs to read and change its contact
//! list once the server moved it to Exchange (MS-PRES §3.10.4, PR-51).
//!
//! Security by design: names and addresses are checked and XML-escaped, item identifiers are
//! taken over only in their base64 form; the answer is read with the bounded XML reader, at
//! most [`MAX_GROUPS`] groups and [`MAX_PERSONAS`] contacts, values stripped of control
//! characters and bounded.

use super::{printable, Error, NS_SOAP};
use commscraft_xml::{escape, Element, Limits};

const NS_TYPES: &str = "http://schemas.microsoft.com/exchange/services/2006/types";
const NS_MESSAGES: &str = "http://schemas.microsoft.com/exchange/services/2006/messages";
/// Most groups and contacts read from a list (design).
pub const MAX_GROUPS: usize = 256;
pub const MAX_PERSONAS: usize = 2000;
/// Longest item identifier and display name accepted (design).
pub const MAX_ID: usize = 512;
pub const MAX_NAME: usize = 256;

/// An Exchange item identifier ([MS-OXWSCORE] ItemIdType): `Id` and, if sent, `ChangeKey`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemId {
    pub id: String,
    pub change_key: Option<String>,
}

/// Identifiers are base64 (the examples of §4, pp. 73–75); anything else is refused.
fn base64ish(v: &str) -> Option<String> {
    let v = v.trim();
    (!v.is_empty()
        && v.len() <= MAX_ID
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "+/=-_".contains(c)))
    .then(|| v.to_string())
}

impl ItemId {
    fn from_element(e: &Element) -> Option<ItemId> {
        Some(ItemId {
            id: base64ish(e.attr("Id")?)?,
            change_key: e.attr("ChangeKey").and_then(base64ish),
        })
    }

    fn xml(&self, element: &str) -> String {
        match &self.change_key {
            Some(k) => format!("<m:{element} Id=\"{}\" ChangeKey=\"{}\"/>", self.id, k),
            None => format!("<m:{element} Id=\"{}\"/>", self.id),
        }
    }
}

/// The kind of a group ([MS-OXWSCOS] §2.2.4.2, pp. 14–15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    Tagged,
    OtherContacts,
    UserGroup,
    DistributionGroup,
    Favorites,
    Other,
}

impl GroupKind {
    fn parse(v: &str) -> GroupKind {
        match v.trim() {
            "IPM.DistList.MOC.Tagged" => GroupKind::Tagged,
            "IPM.DistList.MOC.OtherContacts" => GroupKind::OtherContacts,
            "IPM.DistList.MOC.UserGroup" => GroupKind::UserGroup,
            "IPM.DistList.MOC.DG" => GroupKind::DistributionGroup,
            "IPM.DistList.MOC.Favorites" => GroupKind::Favorites,
            _ => GroupKind::Other,
        }
    }
}

/// A group of the list (ImGroupType, §2.2.4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImGroup {
    pub name: String,
    pub kind: GroupKind,
    pub id: Option<ItemId>,
    /// The identifiers of the member contacts (MemberCorrelationKey).
    pub members: Vec<String>,
}

/// A contact of the list (PersonaType, [MS-OXWSPERS] §2.2.4.19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Persona {
    pub id: ItemId,
    pub name: String,
    pub im_address: Option<String>,
    pub email: Option<String>,
    /// The contact items behind the persona (Attributions/Attribution/SourceId, §2.2.4.1 of
    /// [MS-OXWSPERS]). K-EX-2: the groups' MemberCorrelationKey names these items, not the
    /// PersonaId (`tested-in-lab`); they are also the ContactId of the contact operations.
    pub sources: Vec<ItemId>,
}

impl Persona {
    /// The identifier for ContactId: the first contact item, else the persona.
    pub fn contact_id(&self) -> &ItemId {
        self.sources.first().unwrap_or(&self.id)
    }

    /// Whether a group with these members (MemberCorrelationKey) holds this contact.
    pub fn in_group(&self, group: &ImGroup) -> bool {
        group
            .members
            .iter()
            .any(|m| *m == self.id.id || self.sources.iter().any(|s| s.id == *m))
    }
}

/// The whole list (ImItemListType, §2.2.4.3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImList {
    pub groups: Vec<ImGroup>,
    pub personas: Vec<Persona>,
}

/// One change or the read of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UcsOp {
    /// GetImItemList (§3.1.4.6, pp. 40–44).
    GetList,
    /// AddImGroup (§3.1.4.3, p. 27).
    AddGroup { name: String },
    /// SetImGroup (§3.1.4.12, p. 65).
    RenameGroup { group: ItemId, name: String },
    /// RemoveImGroup (§3.1.4.11, p. 61).
    RemoveGroup { group: ItemId },
    /// AddNewImContactToGroup (§3.1.4.4, p. 31): a new contact by IM address.
    AddNewContact {
        im_address: String,
        name: Option<String>,
        group: Option<ItemId>,
    },
    /// AddImContactToGroup (§3.1.4.2): a contact of the list into one more group.
    AddContactToGroup { contact: ItemId, group: ItemId },
    /// RemoveImContactFromGroup (§3.1.4.10, p. 57).
    RemoveContactFromGroup { contact: ItemId, group: ItemId },
    /// RemoveContactFromImList (§3.1.4.8, p. 48).
    RemoveContact { contact: ItemId },
}

impl UcsOp {
    /// The operation name (element and SOAPAction suffix, §1.7, p. 12).
    pub fn name(&self) -> &'static str {
        match self {
            UcsOp::GetList => "GetImItemList",
            UcsOp::AddGroup { .. } => "AddImGroup",
            UcsOp::RenameGroup { .. } => "SetImGroup",
            UcsOp::RemoveGroup { .. } => "RemoveImGroup",
            UcsOp::AddNewContact { .. } => "AddNewImContactToGroup",
            UcsOp::AddContactToGroup { .. } => "AddImContactToGroup",
            UcsOp::RemoveContactFromGroup { .. } => "RemoveImContactFromGroup",
            UcsOp::RemoveContact { .. } => "RemoveContactFromImList",
        }
    }

    /// The SOAPAction header value (quoted).
    pub fn soap_action(&self) -> String {
        format!("\"{NS_MESSAGES}/{}\"", self.name())
    }
}

fn name(v: &str) -> Result<String, Error> {
    let v = v.trim();
    if v.is_empty() || v.len() > MAX_NAME || v.chars().any(char::is_control) {
        return Err(Error::WrongDocument("name"));
    }
    Ok(escape(v))
}

/// A SIP address as IM address: `sip:user@domain`, printable, bounded.
fn im_address(v: &str) -> Result<String, Error> {
    let v = v.trim();
    let ok = v.len() <= 256
        && v.get(..4).is_some_and(|s| s.eq_ignore_ascii_case("sip:"))
        && v.get(4..).is_some_and(|rest| super::mailbox(rest).is_ok());
    ok.then(|| escape(v)).ok_or(Error::BadMailbox)
}

/// The request of an operation (SOAP 1.1 with RequestServerVersion, example §4, pp. 73–74).
pub fn ucs_request(op: &UcsOp) -> Result<String, Error> {
    let inner = match op {
        UcsOp::GetList => String::new(),
        UcsOp::AddGroup { name: n } => format!("<m:DisplayName>{}</m:DisplayName>", name(n)?),
        UcsOp::RenameGroup { group, name: n } => format!(
            "{}<m:NewDisplayName>{}</m:NewDisplayName>",
            group.xml("GroupId"),
            name(n)?
        ),
        UcsOp::RemoveGroup { group } => group.xml("GroupId"),
        UcsOp::AddNewContact {
            im_address: a,
            name: n,
            group,
        } => {
            let mut s = format!("<m:ImAddress>{}</m:ImAddress>", im_address(a)?);
            if let Some(n) = n {
                s.push_str(&format!("<m:DisplayName>{}</m:DisplayName>", name(n)?));
            }
            if let Some(g) = group {
                s.push_str(&g.xml("GroupId"));
            }
            s
        }
        UcsOp::AddContactToGroup { contact, group }
        | UcsOp::RemoveContactFromGroup { contact, group } => {
            format!("{}{}", contact.xml("ContactId"), group.xml("GroupId"))
        }
        UcsOp::RemoveContact { contact } => contact.xml("ContactId"),
    };
    let op_name = op.name();
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<soap:Envelope xmlns:soap=\"{NS_SOAP}\" xmlns:t=\"{NS_TYPES}\" xmlns:m=\"{NS_MESSAGES}\">\
<soap:Header><t:RequestServerVersion Version=\"Exchange2013\"/></soap:Header>\
<soap:Body><m:{op_name}>{inner}</m:{op_name}></soap:Body></soap:Envelope>"
    ))
}

fn text_of(e: &Element, child: &str) -> Option<String> {
    let v = printable(&e.child(child)?.text());
    (!v.is_empty()).then(|| v.chars().take(MAX_NAME).collect())
}

fn group_of(e: &Element) -> Option<ImGroup> {
    Some(ImGroup {
        name: text_of(e, "DisplayName")?,
        kind: GroupKind::parse(&text_of(e, "GroupType").unwrap_or_default()),
        id: e.child("ExchangeStoreId").and_then(ItemId::from_element),
        members: e
            .child("MemberCorrelationKey")
            .map(|k| {
                k.elements()
                    .filter_map(|i| i.attr("Id").and_then(base64ish))
                    .take(MAX_PERSONAS)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn persona_of(e: &Element) -> Option<Persona> {
    Some(Persona {
        id: e.child("PersonaId").and_then(ItemId::from_element)?,
        name: text_of(e, "DisplayName").unwrap_or_default(),
        im_address: text_of(e, "ImAddress"),
        email: e
            .child("EmailAddress")
            .and_then(|m| text_of(m, "EmailAddress")),
        sources: e
            .child("Attributions")
            .map(|a| {
                a.children_named("Attribution")
                    .filter_map(|x| x.child("SourceId").and_then(ItemId::from_element))
                    .take(16)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// The answer of an operation: checks the envelope, ResponseClass and ResponseCode
/// ([MS-OXWSCDATA] §2.2.4.65, named by every operation), and returns the response element.
fn response<'a>(root: &'a Element, op: &UcsOp) -> Result<&'a Element, Error> {
    if root.name != "Envelope" || root.namespace.as_deref() != Some(NS_SOAP) {
        return Err(Error::WrongDocument("soap:Envelope"));
    }
    let body = root
        .child("Body")
        .ok_or(Error::WrongDocument("soap:Body"))?;
    if let Some(f) = body.child("Fault") {
        let text = f.child("faultstring").map(|s| s.text()).unwrap_or_default();
        return Err(Error::Fault(printable(&text)));
    }
    let wanted = format!("{}Response", op.name());
    let r = body
        .child(&wanted)
        .filter(|r| r.namespace.as_deref() == Some(NS_MESSAGES))
        .ok_or(Error::WrongDocument("operation response"))?;
    let code = r
        .child("ResponseCode")
        .map(|c| printable(&c.text()))
        .unwrap_or_default();
    if r.attr("ResponseClass") != Some("Success") || (code != "NoError" && !code.is_empty()) {
        let code = if code.is_empty() {
            "Error".to_string()
        } else {
            code
        };
        return Err(Error::Server(code));
    }
    Ok(r)
}

fn parse(xml: &str) -> Result<Element, Error> {
    commscraft_xml::parse(
        xml,
        &Limits {
            max_input: 4 * 1024 * 1024,
            max_nodes: 400_000,
            ..Limits::default()
        },
    )
    .map_err(Error::Xml)
}

/// Reads the GetImItemList answer (§3.1.4.6.3.2): groups and contacts; a contact without an
/// identifier is left out.
pub fn parse_im_item_list(xml: &str) -> Result<ImList, Error> {
    let root = parse(xml)?;
    let r = response(&root, &UcsOp::GetList)?;
    let Some(list) = r.child("ImItemList") else {
        return Ok(ImList::default());
    };
    Ok(ImList {
        groups: list
            .child("Groups")
            .map(|g| {
                g.children_named("ImGroup")
                    .filter_map(group_of)
                    .take(MAX_GROUPS)
                    .collect()
            })
            .unwrap_or_default(),
        personas: list
            .child("Personas")
            .map(|p| {
                p.children_named("Persona")
                    .filter_map(persona_of)
                    .take(MAX_PERSONAS)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Reads the answer of a change: success, and for AddImGroup the new group, for
/// AddNewImContactToGroup the new contact's identifier.
pub fn parse_ucs_answer(op: &UcsOp, xml: &str) -> Result<Option<ItemId>, Error> {
    let root = parse(xml)?;
    let r = response(&root, op)?;
    Ok(match op {
        UcsOp::AddGroup { .. } => r
            .child("ImGroup")
            .and_then(|g| g.child("ExchangeStoreId"))
            .and_then(ItemId::from_element),
        UcsOp::AddNewContact { .. } => r
            .child("Persona")
            .and_then(|p| p.child("PersonaId"))
            .and_then(ItemId::from_element),
        _ => None,
    })
}
