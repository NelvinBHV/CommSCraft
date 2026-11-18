//! Conversation history in Exchange (function 49; EX-07, EX-09, OB-22, OB-49): a finished IM
//! conversation is saved as a message in the user's "Conversation History" folder – the
//! distinguished folder `conversationhistory` ([MS-OXWSCDATA] 20250520 §2.2.5.10, p. 86; not in
//! Exchange 2007/2010, <96>) – with CreateItem ([MS-OXWSCORE] 20250520 §3.1.4.2, pp. 57–61).
//!
//! What the Lync 2010 SDK ("Retrieving Lync-Archived Conversation History with Exchange Web
//! Services", official, secondary) says the entries hold: `Body` the whole conversation text,
//! `DisplayTo` the participants (here from the To recipients), `Subject` the topic. The body
//! layout is this client's own (OB-49, by design). The message is saved, not sent
//! (`MessageDisposition="SaveOnly"`), and marked read and not a draft: PidTagMessageFlags
//! ([MS-OXCMSG] §2.2.1.6, p. 19: mfRead, without mfUnsent; writable until the first save).
//!
//! Reading the folder back: FindItem ([MS-OXWSSRCH] 20250520 §3.1.4.2, pp. 46–49) lists the
//! newest entries (subject, time, `DisplayTo`); GetItem ([MS-OXWSCORE] 20250819 §3.1.4.4,
//! pp. 65–69) fetches one entry's body as text (`ItemResponseShapeType`/`BodyType`,
//! [MS-OXWSCDATA] 20250520 §2.2.4.42, p. 48; `Text`, §2.2.5.1, p. 72).
//!
//! Security by design: every text is stripped of control characters (line breaks between
//! lines are this module's own), bounded and XML-escaped; addresses are checked; at most
//! [`MAX_LINES`] lines and [`MAX_PARTICIPANTS`] participants.

use super::{mailbox, Error, NS_SOAP};
use commscraft_xml::{escape, xs_datetime};

const NS_TYPES: &str = "http://schemas.microsoft.com/exchange/services/2006/types";
const NS_MESSAGES: &str = "http://schemas.microsoft.com/exchange/services/2006/messages";
pub const CREATE_ITEM_ACTION: &str =
    "\"http://schemas.microsoft.com/exchange/services/2006/messages/CreateItem\"";
/// Limits (design).
pub const MAX_LINES: usize = 5000;
pub const MAX_LINE: usize = 4096;
pub const MAX_PARTICIPANTS: usize = 250;
pub const MAX_SUBJECT: usize = 255;

/// One participant: display name and mail address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    pub name: String,
    pub address: String,
}

/// One line of the conversation: Unix time, the sender's display name, the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub time: u64,
    pub sender: String,
    pub text: String,
}

/// A finished conversation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Conversation {
    /// The other participants (the user is the mailbox owner).
    pub participants: Vec<Participant>,
    pub lines: Vec<Line>,
}

fn clean(s: &str, max: usize) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max)
        .collect::<String>()
        .trim()
        .to_string()
}

impl Conversation {
    /// Adds a line while the conversation is open, with the limits of the saved entry applied
    /// in memory too: the text and sender are cut to their bounds and only the newest
    /// [`MAX_LINES`] lines are kept, so a peer sending without end cannot make the transcript
    /// grow without end (security by design).
    pub fn add_line(&mut self, line: Line) {
        if self.lines.len() >= MAX_LINES {
            let excess = self.lines.len() + 1 - MAX_LINES;
            self.lines.drain(..excess);
        }
        self.lines.push(Line {
            time: line.time,
            sender: line.sender.chars().take(256).collect(),
            text: line.text.chars().take(MAX_LINE).collect(),
        });
    }

    /// Adds a participant once (by address, case-insensitive), at most [`MAX_PARTICIPANTS`].
    pub fn add_participant(&mut self, p: Participant) {
        if self.participants.len() >= MAX_PARTICIPANTS
            || self
                .participants
                .iter()
                .any(|q| q.address.eq_ignore_ascii_case(&p.address))
        {
            return;
        }
        self.participants.push(Participant {
            name: p.name.chars().take(256).collect(),
            address: p.address.chars().take(256).collect(),
        });
    }

    /// The subject: the topic, which the Windows client takes from the first message (OB-22:
    /// `Subject` = topic; the first message – inference).
    pub fn subject(&self) -> String {
        self.lines
            .first()
            .map(|l| clean(&l.text, MAX_SUBJECT))
            .unwrap_or_default()
    }

    /// The body as plain text: one line per message, `[YYYY-MM-DD HH:MM UTC] name: text`.
    pub fn body(&self) -> String {
        let mut out = String::new();
        for l in self.lines.iter().take(MAX_LINES) {
            let time = xs_datetime(l.time)
                .map(|t| {
                    format!(
                        "{} {} UTC",
                        t.get(..10).unwrap_or_default(),
                        t.get(11..16).unwrap_or_default()
                    )
                })
                .unwrap_or_default();
            out.push_str(&format!(
                "[{time}] {}: {}\n",
                clean(&l.sender, 256),
                clean(&l.text, MAX_LINE)
            ));
        }
        out
    }
}

/// The CreateItem request saving `c` in the Conversation History folder.
pub fn save_request(c: &Conversation) -> Result<String, Error> {
    if c.lines.is_empty() {
        return Err(Error::WrongDocument("empty conversation"));
    }
    let mut to = String::new();
    // A participant without a usable address (a conference focus, a phone) is left out of the
    // recipients; the lines still name it.
    for p in c.participants.iter().take(MAX_PARTICIPANTS) {
        let Ok(address) = mailbox(&p.address) else {
            continue;
        };
        to.push_str(&format!(
            "<t:Mailbox><t:Name>{}</t:Name><t:EmailAddress>{}</t:EmailAddress></t:Mailbox>",
            escape(&clean(&p.name, 256)),
            escape(address)
        ));
    }
    let recipients = if to.is_empty() {
        String::new()
    } else {
        format!("<t:ToRecipients>{to}</t:ToRecipients>")
    };
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"{NS_SOAP}\" xmlns:t=\"{NS_TYPES}\" xmlns:m=\"{NS_MESSAGES}\">\
<soap:Header><t:RequestServerVersion Version=\"Exchange2013\"/></soap:Header><soap:Body>\
<m:CreateItem MessageDisposition=\"SaveOnly\"><m:SavedItemFolderId><t:DistinguishedFolderId Id=\"conversationhistory\"/></m:SavedItemFolderId>\
<m:Items><t:Message><t:Subject>{}</t:Subject><t:Body BodyType=\"Text\">{}</t:Body>\
<t:ExtendedProperty><t:ExtendedFieldURI PropertyTag=\"0x0E07\" PropertyType=\"Integer\"/><t:Value>1</t:Value></t:ExtendedProperty>\
{recipients}<t:IsRead>true</t:IsRead></t:Message></m:Items></m:CreateItem></soap:Body></soap:Envelope>",
        escape(&c.subject()),
        escape(&c.body())
    ))
}

/// Reads the CreateItem answer: Ok when the item was created.
pub fn parse_save_answer(xml: &str) -> Result<(), Error> {
    let root = commscraft_xml::parse(
        xml,
        &commscraft_xml::Limits {
            max_input: 256 * 1024,
            ..commscraft_xml::Limits::default()
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
        return Err(Error::Fault(super::printable(&text)));
    }
    let m = body
        .child("CreateItemResponse")
        .filter(|r| r.namespace.as_deref() == Some(NS_MESSAGES))
        .and_then(|r| r.child("ResponseMessages"))
        .and_then(|r| r.child("CreateItemResponseMessage"))
        .ok_or(Error::WrongDocument("CreateItemResponseMessage"))?;
    let code = m
        .child("ResponseCode")
        .map(|c| super::printable(&c.text()))
        .unwrap_or_default();
    if m.attr("ResponseClass") != Some("Success") || (code != "NoError" && !code.is_empty()) {
        return Err(Error::Server(if code.is_empty() {
            "Error".into()
        } else {
            code
        }));
    }
    Ok(())
}

pub const FIND_ITEM_ACTION: &str =
    "\"http://schemas.microsoft.com/exchange/services/2006/messages/FindItem\"";
pub const GET_ITEM_ACTION: &str =
    "\"http://schemas.microsoft.com/exchange/services/2006/messages/GetItem\"";
/// Most entries listed (design).
pub const MAX_ENTRIES: usize = 100;
/// Longest body kept (design).
pub const MAX_BODY: usize = 256 * 1024;

/// One entry of the Conversation History folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: crate::ucs::ItemId,
    pub subject: String,
    /// When it was saved (Unix seconds).
    pub at: Option<u64>,
    /// The participants as Exchange shows them.
    pub display_to: String,
}

fn envelope(body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"{NS_SOAP}\" xmlns:t=\"{NS_TYPES}\" xmlns:m=\"{NS_MESSAGES}\">\
<soap:Header><t:RequestServerVersion Version=\"Exchange2013\"/></soap:Header><soap:Body>{body}</soap:Body></soap:Envelope>"
    )
}

/// The FindItem request: the newest [`MAX_ENTRIES`] entries of the Conversation History folder.
pub fn list_request() -> String {
    envelope(&format!(
        "<m:FindItem Traversal=\"Shallow\"><m:ItemShape><t:BaseShape>IdOnly</t:BaseShape><t:AdditionalProperties>\
<t:FieldURI FieldURI=\"item:Subject\"/><t:FieldURI FieldURI=\"item:DateTimeCreated\"/><t:FieldURI FieldURI=\"item:DisplayTo\"/>\
</t:AdditionalProperties></m:ItemShape>\
<m:IndexedPageItemView MaxEntriesReturned=\"{MAX_ENTRIES}\" Offset=\"0\" BasePoint=\"Beginning\"/>\
<m:SortOrder><t:FieldOrder Order=\"Descending\"><t:FieldURI FieldURI=\"item:DateTimeCreated\"/></t:FieldOrder></m:SortOrder>\
<m:ParentFolderIds><t:DistinguishedFolderId Id=\"conversationhistory\"/></m:ParentFolderIds></m:FindItem>"
    ))
}

/// The GetItem request for one entry's body as text.
pub fn body_request(id: &crate::ucs::ItemId) -> String {
    let item = match &id.change_key {
        Some(k) => format!(
            "<t:ItemId Id=\"{}\" ChangeKey=\"{}\"/>",
            escape(&id.id),
            escape(k)
        ),
        None => format!("<t:ItemId Id=\"{}\"/>", escape(&id.id)),
    };
    envelope(&format!(
        "<m:GetItem><m:ItemShape><t:BaseShape>IdOnly</t:BaseShape><t:BodyType>Text</t:BodyType>\
<t:AdditionalProperties><t:FieldURI FieldURI=\"item:Body\"/></t:AdditionalProperties></m:ItemShape>\
<m:ItemIds>{item}</m:ItemIds></m:GetItem>"
    ))
}

/// The response message of an operation after the usual checks (envelope, fault, class).
fn response_message(
    xml: &str,
    response: &'static str,
    message: &'static str,
) -> Result<commscraft_xml::Element, Error> {
    let root = commscraft_xml::parse(
        xml,
        &commscraft_xml::Limits {
            max_input: 2 * 1024 * 1024,
            ..commscraft_xml::Limits::default()
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
        return Err(Error::Fault(super::printable(&text)));
    }
    let m = body
        .child(response)
        .filter(|r| r.namespace.as_deref() == Some(NS_MESSAGES))
        .and_then(|r| r.child("ResponseMessages"))
        .and_then(|r| r.child(message))
        .ok_or(Error::WrongDocument(message))?;
    let code = m
        .child("ResponseCode")
        .map(|c| super::printable(&c.text()))
        .unwrap_or_default();
    if m.attr("ResponseClass") != Some("Success") || (code != "NoError" && !code.is_empty()) {
        return Err(Error::Server(if code.is_empty() {
            "Error".into()
        } else {
            code
        }));
    }
    Ok(m.clone())
}

/// Reads the FindItem answer: the entries as the server sorted them (newest first).
pub fn parse_list(xml: &str) -> Result<Vec<Entry>, Error> {
    let m = response_message(xml, "FindItemResponse", "FindItemResponseMessage")?;
    let Some(items) = m.child("RootFolder").and_then(|r| r.child("Items")) else {
        return Ok(Vec::new());
    };
    Ok(items
        .elements()
        .filter_map(|e| {
            Some(Entry {
                id: crate::ucs::ItemId::from_element(e.child("ItemId")?)?,
                subject: e
                    .child("Subject")
                    .map(|s| clean(&s.text(), MAX_SUBJECT))
                    .unwrap_or_default(),
                at: e
                    .child("DateTimeCreated")
                    .and_then(|d| commscraft_xml::parse_xs_datetime(&d.text())),
                display_to: e
                    .child("DisplayTo")
                    .map(|s| clean(&s.text(), 1024))
                    .unwrap_or_default(),
            })
        })
        .take(MAX_ENTRIES)
        .collect())
}

/// Reads the GetItem answer: the entry's body text (line breaks kept, other control
/// characters removed, bounded).
pub fn parse_body(xml: &str) -> Result<String, Error> {
    let m = response_message(xml, "GetItemResponse", "GetItemResponseMessage")?;
    let text = m
        .child("Items")
        .and_then(|i| i.elements().next())
        .and_then(|item| item.child("Body"))
        .map(|b| b.text())
        .ok_or(Error::WrongDocument("Body"))?;
    Ok(text
        .chars()
        .filter(|c| *c == '\n' || !c.is_control())
        .take(MAX_BODY)
        .collect())
}
