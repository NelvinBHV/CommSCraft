//! 1:1 instant messaging (M2): the IM session (INVITE with a message SDP, ACK, MESSAGE, INFO for
//! the typing indicator, BYE) as building blocks for a sans-I/O session.
//!
//! Evidence: `docs/protocol/evidence/01-ocsprot-system-overview.md` (OCS-UC09),
//! `21-confim-im-conference.md` (CI-02), file 39 OB-16, OB-39 (typing), `06-sipre-routing.md`.
//! Sources: [MS-OCSPROT] 20250520 §2.5.9 (pp. 36–37) – flow INVITE, 200, ACK, MESSAGE, 200;
//! [MS-CONFIM] 20250520 §2.2.5 (p. 14), §3.1.1.1 (pp. 16–17) – `m=message 5060 sip null`,
//! `a=accept-types`, text/plain always allowed, never offer `*`; RFC 3428 (MESSAGE), RFC 2976
//! (INFO), RFC 3261 §13, §17.1.1.3 (INVITE, ACK).
//! Secondary: [SIPE] `sipe-im.c` l. 385–411 (the same SDP for 1:1 sessions), `sipe-user.c`
//! l. 120–150 (typing as INFO `application/xml` `<KeyboardActivity><status status="type"/>`).
//!
//! Security by design (`docs/security.md`):
//! - only `text/plain` is offered and accepted; HTML or RTF from others is refused with 415, never
//!   rendered, so no markup or scripts can be injected through messages;
//! - incoming text is UTF-8 checked, bounded and stripped of control characters before display;
//! - the SDP of the peer is parsed with limits and must describe a message session;
//! - the typing indicator is recognised only in its exact, small XML form.

use commscraft_sip_msg::dialog::{response_to, Dialog};
use commscraft_sip_msg::Message;

/// Largest message text we send or show (inference/design: far above what a chat line needs).
pub const MAX_TEXT: usize = 16 * 1024;
const MAX_SDP: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    EmptyText,
    TextTooLong,
    /// The SDP does not describe a message session we can join.
    BadSdp,
    /// The address is not usable.
    BadAddress,
}

/// The SDP offer/answer for a message session (CI-02). `ip` is the local address of the SIP
/// connection (the port is meaningless for `sip` transport, §2.2.5). `conference`: a session with
/// an IM MCU, which also announces `application/ms-imdn+xml` (CI-14): the client waits for the
/// MCU's IMDN after a 202 (CI-05), and the MCU stores the announced formats for what it sends to
/// the client ([MS-CONFIM] §3.1.3.5.1, p. 19; the type in the accept-types example of §2.2.5,
/// p. 14). Plain text stays the only message format either way.
pub fn sdp(ip: std::net::IpAddr, conference: bool) -> String {
    let (family, addr) = match ip {
        std::net::IpAddr::V4(v4) => ("IP4", v4.to_string()),
        std::net::IpAddr::V6(v6) => ("IP6", v6.to_string()),
    };
    format!(
        "v=0\r\no=- 0 0 IN {family} {addr}\r\ns=session\r\nc=IN {family} {addr}\r\nt=0 0\r\nm=message 5060 sip null\r\na=accept-types:text/plain{}\r\n",
        if conference {
            " application/ms-imdn+xml"
        } else {
            ""
        }
    )
}

/// Checks the peer's SDP: exactly one `m=message … sip null` line; if it lists accept-types,
/// `text/plain` or `*` must be among them (without the attribute, text/plain is implied, CI-02).
pub fn check_sdp(body: &[u8]) -> Result<(), Error> {
    if body.len() > MAX_SDP {
        return Err(Error::BadSdp);
    }
    let text = std::str::from_utf8(body).map_err(|_| Error::BadSdp)?;
    let mut message_lines = 0;
    let mut types: Option<Vec<String>> = None;
    for line in text.lines() {
        if let Some(m) = line.strip_prefix("m=") {
            let f: Vec<&str> = m.split_whitespace().collect();
            if f.first() == Some(&"message") && f.get(2) == Some(&"sip") {
                message_lines += 1;
            }
        } else if let Some(t) = line.strip_prefix("a=accept-types:") {
            types = Some(
                t.split_whitespace()
                    .map(|s| s.to_ascii_lowercase())
                    .collect(),
            );
        }
    }
    if message_lines != 1 {
        return Err(Error::BadSdp);
    }
    match types {
        Some(t) if !t.iter().any(|x| x == "text/plain" || x == "*") => Err(Error::BadSdp),
        _ => Ok(()),
    }
}

/// Removes control characters (except newline and tab) and bidirectional overrides from text that
/// came from the network, and bounds it.
pub fn printable(s: &str) -> String {
    s.chars()
        .filter(|c| {
            (!c.is_control() || *c == '\n' || *c == '\t')
                && !('\u{202a}'..='\u{202e}').contains(c)
                && !('\u{2066}'..='\u{2069}').contains(c)
        })
        .take(MAX_TEXT)
        .collect()
}

/// Headers and body of the INVITE that opens an IM session (OCS-UC09). The caller adds Via,
/// From/To/Call-ID/CSeq/Contact (an out-of-dialog request with the GRUU as Contact, RE-09) and
/// signs it.
pub fn invite_body(m: &mut Message, ip: std::net::IpAddr, conference: bool) {
    m.push_header("Content-Type", "application/sdp");
    m.body = sdp(ip, conference).into_bytes();
}

/// ACK for a 2xx to our INVITE: a new request in the dialog with the INVITE's CSeq number
/// (RFC 3261 §13.2.2.4). The caller adds Via and signs it.
pub fn ack_for_2xx(dialog: &Dialog, invite_cseq: u32) -> Message {
    let mut d = dialog.clone();
    d.local_cseq = invite_cseq.saturating_sub(1);
    d.request("ACK")
}

/// ACK for a non-2xx final response: same Request-URI, Via (branch), From, Call-ID and CSeq number
/// as the INVITE, To from the response (RFC 3261 §17.1.1.3).
pub fn ack_for_failure(invite: &Message, response: &Message) -> Message {
    let uri = match &invite.start {
        commscraft_sip_msg::StartLine::Request { uri, .. } => uri.clone(),
        commscraft_sip_msg::StartLine::Response { .. } => String::new(),
    };
    let mut ack = Message::request("ACK", &uri);
    if let Some(v) = invite.header_values("Via").next() {
        ack.push_header("Via", v);
    }
    for (name, src) in [("From", invite), ("To", response), ("Call-ID", invite)] {
        if let Some(v) = src.header_values(name).next() {
            ack.push_header(name, v);
        }
    }
    let n = invite.cseq().ok().flatten().map_or(1, |c| c.number);
    ack.push_header("CSeq", &format!("{n} ACK"));
    ack.push_header("Max-Forwards", "70");
    ack
}

/// A MESSAGE with plain text in the dialog (RFC 3428; OCS-UC09 step 5).
pub fn message(dialog: &mut Dialog, text: &str) -> Result<Message, Error> {
    if text.trim().is_empty() {
        return Err(Error::EmptyText);
    }
    if text.len() > MAX_TEXT {
        return Err(Error::TextTooLong);
    }
    let mut m = dialog.request("MESSAGE");
    m.push_header("Content-Type", "text/plain; charset=UTF-8");
    m.body = printable(text).into_bytes();
    Ok(m)
}

/// The typing indicator as INFO (OB-16, secondary: `application/xml`, `<KeyboardActivity>`).
pub fn typing(dialog: &mut Dialog, typing: bool) -> Message {
    let mut m = dialog.request("INFO");
    m.push_header("Content-Type", "application/xml");
    let status = if typing { "type" } else { "idle" };
    m.body = format!(
        "<?xml version=\"1.0\"?><KeyboardActivity><status status=\"{status}\"/></KeyboardActivity>"
    )
    .into_bytes();
    m
}

/// What a request of the peer in the IM dialog means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    Text(String),
    Typing(bool),
    Bye,
    /// Not supported; answered with the given status (415 for other content types, 488/501 …).
    Refused(u16),
}

fn content_type(m: &Message) -> String {
    m.single("Content-Type")
        .ok()
        .flatten()
        .and_then(|v| v.split(';').next())
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

/// Interprets a request of the peer (already checked to belong to the dialog and to be signed by
/// the server) and builds the answer (to be signed by the caller).
pub fn handle_request(req: &Message) -> (Incoming, Message) {
    let method = req.method().unwrap_or("").to_ascii_uppercase();
    let incoming = match method.as_str() {
        "BYE" => Incoming::Bye,
        "MESSAGE" => {
            let ct = content_type(req);
            let charset_ok = req.single("Content-Type").ok().flatten().is_none_or(|v| {
                !v.to_ascii_lowercase().contains("charset=")
                    || v.to_ascii_lowercase().contains("charset=utf-8")
            });
            match std::str::from_utf8(&req.body) {
                Ok(t) if ct == "text/plain" && charset_ok && req.body.len() <= MAX_TEXT => {
                    Incoming::Text(printable(t))
                }
                _ => Incoming::Refused(415),
            }
        }
        "INFO" => match std::str::from_utf8(&req.body)
            .ok()
            .filter(|_| content_type(req) == "application/xml" && req.body.len() < 512)
        {
            Some(b) => match parse_typing(b) {
                Some(t) => Incoming::Typing(t),
                None => Incoming::Refused(415),
            },
            None => Incoming::Refused(415),
        },
        _ => Incoming::Refused(501),
    };
    let answer = match &incoming {
        Incoming::Refused(415) => {
            let mut r = response_to(req, 415, "Unsupported Media Type");
            r.push_header("Accept", "text/plain");
            r
        }
        Incoming::Refused(code) => response_to(req, *code, "Not Implemented"),
        _ => response_to(req, 200, "OK"),
    };
    (incoming, answer)
}

fn parse_typing(xml: &str) -> Option<bool> {
    let root = commscraft_xml::parse(
        xml,
        &commscraft_xml::Limits {
            max_depth: 4,
            max_nodes: 16,
            ..commscraft_xml::Limits::default()
        },
    )
    .ok()?;
    if root.name != "KeyboardActivity" {
        return None;
    }
    match root.child("status")?.attr("status")? {
        "type" => Some(true),
        "idle" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
