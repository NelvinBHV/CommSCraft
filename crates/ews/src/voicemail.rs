//! The voicemail list from Exchange (function 38; L-15 closed by OB-48): voice messages are the
//! messages of class `IPM.Note.Microsoft.Voicemail.UM.CA` and `IPM.Note.Microsoft.Voicemail.UM`
//! ([MS-OXOUM] 20250520 §2.2.2.1, p. 10), found with FindItem ([MS-OXWSSRCH] 20250520
//! §3.1.4.2, pp. 46–49) in the Inbox (where Exchange delivers them – inference) by a prefix
//! restriction on the item class. Caller and length come from PidTagVoiceMessageSenderName
//! (0x6803), PidTagSenderTelephoneNumber (0x6802) and PidTagVoiceMessageDuration (0x6801,
//! seconds; [MS-OXOUM] §2.2.5.1–2.2.5.5, pp. 24–25; IDs [MS-OXPROPS] §2.1009, §2.1062, §2.1063).
//!
//! Security by design: the answer is read with the bounded XML reader, at most [`MAX_ITEMS`]
//! messages; texts are stripped of control characters and bounded; only items whose class is
//! a voicemail class are taken.

use super::{printable, Error, NS_SOAP};
use commscraft_xml::{parse_xs_datetime, Element, Limits};

const NS_TYPES: &str = "http://schemas.microsoft.com/exchange/services/2006/types";
const NS_MESSAGES: &str = "http://schemas.microsoft.com/exchange/services/2006/messages";
pub const FIND_ITEM_ACTION: &str =
    "\"http://schemas.microsoft.com/exchange/services/2006/messages/FindItem\"";
/// Most messages asked for and kept (design).
pub const MAX_ITEMS: usize = 50;
/// The class prefix of voice messages (MS-OXOUM §2.2.2.1).
pub const CLASS_PREFIX: &str = "IPM.Note.Microsoft.Voicemail";

/// One voice message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voicemail {
    pub received: Option<u64>,
    /// The caller's name as the voice network's caller ID gave it, else the sender's name.
    pub caller: String,
    pub phone: Option<String>,
    pub duration: Option<u32>,
    pub read: bool,
    pub subject: String,
}

/// The FindItem request: the newest [`MAX_ITEMS`] voice messages in the Inbox.
pub fn list_request() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"{NS_SOAP}\" xmlns:t=\"{NS_TYPES}\" xmlns:m=\"{NS_MESSAGES}\">\
<soap:Header><t:RequestServerVersion Version=\"Exchange2013\"/></soap:Header><soap:Body>\
<m:FindItem Traversal=\"Shallow\"><m:ItemShape><t:BaseShape>IdOnly</t:BaseShape><t:AdditionalProperties>\
<t:FieldURI FieldURI=\"item:Subject\"/><t:FieldURI FieldURI=\"item:DateTimeReceived\"/><t:FieldURI FieldURI=\"item:ItemClass\"/>\
<t:FieldURI FieldURI=\"message:IsRead\"/><t:FieldURI FieldURI=\"message:From\"/>\
<t:ExtendedFieldURI PropertyTag=\"0x6801\" PropertyType=\"Integer\"/>\
<t:ExtendedFieldURI PropertyTag=\"0x6802\" PropertyType=\"String\"/>\
<t:ExtendedFieldURI PropertyTag=\"0x6803\" PropertyType=\"String\"/>\
</t:AdditionalProperties></m:ItemShape>\
<m:IndexedPageItemView MaxEntriesReturned=\"{MAX_ITEMS}\" Offset=\"0\" BasePoint=\"Beginning\"/>\
<m:Restriction><t:Contains ContainmentMode=\"Prefixed\" ContainmentComparison=\"IgnoreCase\"><t:FieldURI FieldURI=\"item:ItemClass\"/>\
<t:Constant Value=\"{CLASS_PREFIX}\"/></t:Contains></m:Restriction>\
<m:SortOrder><t:FieldOrder Order=\"Descending\"><t:FieldURI FieldURI=\"item:DateTimeReceived\"/></t:FieldOrder></m:SortOrder>\
<m:ParentFolderIds><t:DistinguishedFolderId Id=\"inbox\"/></m:ParentFolderIds></m:FindItem></soap:Body></soap:Envelope>"
    )
}

fn extended(m: &Element, tag: &str) -> Option<String> {
    m.children_named("ExtendedProperty")
        .find(|p| {
            p.child("ExtendedFieldURI")
                .and_then(|u| u.attr("PropertyTag"))
                .is_some_and(|t| t.eq_ignore_ascii_case(tag))
        })
        .and_then(|p| p.child("Value"))
        .map(|v| printable(&v.text()))
        .filter(|v| !v.is_empty())
}

/// Reads the FindItem answer: the voice messages, newest first as the server sorted them.
pub fn parse_list(xml: &str) -> Result<Vec<Voicemail>, Error> {
    let root = commscraft_xml::parse(
        xml,
        &Limits {
            max_input: 1024 * 1024,
            ..Limits::default()
        },
    )
    .map_err(Error::Xml)?;
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
    let m = body
        .child("FindItemResponse")
        .filter(|r| r.namespace.as_deref() == Some(NS_MESSAGES))
        .and_then(|r| r.child("ResponseMessages"))
        .and_then(|r| r.child("FindItemResponseMessage"))
        .ok_or(Error::WrongDocument("FindItemResponseMessage"))?;
    let code = m
        .child("ResponseCode")
        .map(|c| printable(&c.text()))
        .unwrap_or_default();
    if m.attr("ResponseClass") != Some("Success") || (code != "NoError" && !code.is_empty()) {
        return Err(Error::Server(if code.is_empty() {
            "Error".into()
        } else {
            code
        }));
    }
    let Some(items) = m.child("RootFolder").and_then(|r| r.child("Items")) else {
        return Ok(Vec::new());
    };
    Ok(items
        .elements()
        .filter(|e| {
            e.child("ItemClass").is_some_and(|c| {
                c.text()
                    .trim()
                    .get(..CLASS_PREFIX.len())
                    .is_some_and(|p| p.eq_ignore_ascii_case(CLASS_PREFIX))
            })
        })
        .take(MAX_ITEMS)
        .map(|e| {
            let sender = e
                .child("From")
                .and_then(|f| f.child("Mailbox"))
                .and_then(|b| b.child("Name"))
                .map(|n| printable(&n.text()))
                .unwrap_or_default();
            Voicemail {
                received: e
                    .child("DateTimeReceived")
                    .and_then(|d| parse_xs_datetime(&d.text())),
                caller: extended(e, "0x6803").unwrap_or(sender),
                phone: extended(e, "0x6802"),
                duration: extended(e, "0x6801").and_then(|d| d.parse().ok()),
                read: e.child("IsRead").map(|r| r.text().trim().to_string()) == Some("true".into()),
                subject: e
                    .child("Subject")
                    .map(|s| printable(&s.text()))
                    .unwrap_or_default(),
            }
        })
        .collect())
}
