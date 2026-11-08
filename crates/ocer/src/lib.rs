//! Client error reporting (function 46; [MS-OCER] 20250520, evidence file 25): the server's
//! `ms-diagnostics` and `ms-diagnostics-public` headers (ER-01, ER-02), an explanation of the
//! error identifiers a client handles (ER-07: the meaning of §9.2.2 in our own words – the
//! product texts are not taken over), and the `reportError` document a client sends to the
//! server about a failed request (ER-04).
//!
//! Security by design: header values are hostile input – bounded in length and parameters,
//! quoted strings unescaped without control characters; the client never sends these headers
//! (§2.2.1.1); the report document is built with escaped values and the length limits of §2.2.2.

use commscraft_sip_msg::Message;
use commscraft_xml::escape;

pub const REPORT_ERROR_TYPE: &str = "application/msrtc-reporterror+xml";
const NS: &str = "http://schemas.microsoft.com/2006/09/sip/error-reporting";
/// Longest header value read (design).
pub const MAX_HEADER: usize = 4096;
/// Most generic parameters kept (design).
pub const MAX_PARAMS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No ErrorId, or the value is too long.
    Malformed,
    /// A value exceeds a limit of the report document (§2.2.2).
    TooLong(&'static str),
}

/// One diagnostics header (ER-01/ER-02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// ErrorId: what the client acts on.
    pub id: u32,
    /// The server's reason text (troubleshooting only, §2.2.1.1).
    pub reason: Option<String>,
    /// The server that added the header (absent in `ms-diagnostics-public`).
    pub source: Option<String>,
    /// Further parameters, e.g. `Setup_time`, `Peer`.
    pub params: Vec<(String, String)>,
    /// From `ms-diagnostics-public`.
    pub public: bool,
}

fn clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(512).collect()
}

/// Splits at `;` outside quoted strings.
fn split_params(v: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (i, c) in v.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            ';' if !quoted => {
                out.push(v.get(start..i).unwrap_or_default());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(v.get(start..).unwrap_or_default());
    out
}

/// A quoted-string's content (RFC 3261 §25.1: quoted-pair unescaped) or a token as it is.
fn unquote(v: &str) -> String {
    let v = v.trim();
    let Some(inner) = v.strip_prefix('"').and_then(|x| x.strip_suffix('"')) else {
        return clean(v);
    };
    let mut out = String::new();
    let mut escaped = false;
    for c in inner.chars() {
        if escaped {
            out.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else {
            out.push(c);
        }
    }
    clean(&out)
}

impl Diagnostic {
    /// Reads a header value: `ErrorId;reason="…";source="…"*(;param)` (§2.2.1.1/§2.2.1.2).
    pub fn parse(value: &str, public: bool) -> Result<Diagnostic, Error> {
        if value.len() > MAX_HEADER {
            return Err(Error::Malformed);
        }
        let mut parts = split_params(value).into_iter();
        let id = parts
            .next()
            .and_then(|p| p.trim().parse::<u32>().ok())
            .ok_or(Error::Malformed)?;
        let mut d = Diagnostic {
            id,
            reason: None,
            source: None,
            params: Vec::new(),
            public,
        };
        for p in parts {
            let (name, val) = p.split_once('=').unwrap_or((p, ""));
            let name = clean(name.trim());
            let val = unquote(val);
            match name.to_ascii_lowercase().as_str() {
                "reason" => d.reason = Some(val).filter(|v| !v.is_empty()),
                "source" if !public => d.source = Some(val).filter(|v| !v.is_empty()),
                "" => {}
                _ if d.params.len() < MAX_PARAMS => d.params.push((name, val)),
                _ => {}
            }
        }
        Ok(d)
    }

    /// The diagnostics of a message: `ms-diagnostics`, else `ms-diagnostics-public`; `None`
    /// without either or when unreadable. A second header of a kind (not allowed, §2.2.1.1) is
    /// ignored.
    pub fn from_message(m: &Message) -> Option<Diagnostic> {
        if let Some(v) = m.header_values("ms-diagnostics").next() {
            return Diagnostic::parse(v, false).ok();
        }
        m.header_values("ms-diagnostics-public")
            .next()
            .and_then(|v| Diagnostic::parse(v, true).ok())
    }

    /// Our explanation of the error identifier, if it is one a client handles.
    pub fn explanation(&self) -> Option<&'static str> {
        explain(self.id)
    }
}

/// What an error identifier means for the user (the "Diagnostic reason" of [MS-OCER] §9.2.2,
/// in our own words); `None` for an identifier without a handled meaning (§9.3: show it with
/// its number).
pub fn explain(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "the service had an internal error",
        1 => "the service is not available",
        4 => "the network has not enough bandwidth for this session",
        5 => "the network has not enough bandwidth; the call is tried over the phone network",
        1003 => "this user or meeting does not exist",
        1004 => "the route to the other side is no longer valid",
        1005 | 1007 => "the server cannot reach the other side right now",
        1008 => "the other side's domain cannot be found (DNS)",
        1009 => "the other side's domain is not served or not federated",
        1011 => "the request was refused by the other side",
        3026 | 3032 => "this meeting does not exist",
        3027 => "you are not allowed into this closed meeting",
        3028 | 3044 => "anonymous access to this meeting is not allowed",
        3033 => "the meeting command failed",
        3040 => "the meeting is not active",
        3041 => "this participant is not in the meeting",
        3043 => "the organizer's policy does not allow this",
        3097 | 3098 => "no conferencing server is available",
        3099 => "this kind of media is not available in the meeting",
        3100 => "the meeting is full",
        3108..=3110 => "the sharing server cannot be reached or refused the request",
        3113..=3116 => "the meeting has ended",
        3118 => "you were removed from the meeting",
        3119 => "you were not admitted to the meeting",
        3120 => "your meeting session has ended",
        3122 => "the meeting server did not answer the request",
        3138 => "the requested meeting function is not available",
        3181 => "joining with all features is not allowed here",
        4002 | 4009 => "more than one user matches this address",
        4004 => "the credentials were not accepted",
        5001 | 6001 => "the request timed out",
        5026 => "the request reached a server that cannot handle it",
        6000 => "the message could not be sent",
        6002 | 6005 | 6014 => "the conversation is no longer active",
        6004 => "this kind of message is not supported",
        6006 => "the chat server is shutting down",
        6007 | 6008 => "the conversation's meeting does not exist any more",
        6009 => "the message is too long",
        6010 => "you are not allowed to do this",
        6011 => "the user was not found",
        6012 => "this participant is not in the conversation",
        6013 => "the conversation is locked",
        6015 => "the session description was not accepted",
        6017 | 6024 => "the message format was not accepted",
        7012 => "the media of this call cannot be supported",
        7013 => "the other side does not accept encrypted media in this form",
        10006 | 10009 => "the gateway to the phone network is not available",
        10400 | 10503 => "the phone network gateway refused the call",
        12000 => "there is no route for this number",
        12001 => "your policy does not allow calls to this number",
        12013 => "calling out of the meeting is not allowed",
        13002 => "the routing rules did not find a destination",
        13010 => "the other side refused the call",
        14005 | 14010 => "the number could not be translated with the dial plan",
        16002 => "your policy does not allow this",
        21002 => "attendees cannot share here",
        21003 | 21012 => "the sharing media connection failed",
        21004 => "someone else started sharing",
        21005 => "you joined the sharing from somewhere else",
        21006 => "the sharing connection failed",
        21007 => "you were removed from the sharing",
        21008 => "the sharing session has ended",
        25001 => "the call is retried over the phone network",
        26014 => "you are not allowed to do this for this user",
        26019 => "the response group cannot take the call",
        35000 => "this parking orbit does not exist",
        35002 => "no parking orbit is free",
        35007 => "the call could not be parked",
        35015 => "the parked call's time ran out",
        _ => return None,
    })
}

/// The data of a `reportError` document (§2.2.2): the failed request, the final response's
/// diagnostics and those of its progress responses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub from_uri: Option<String>,
    pub to_uri: Option<String>,
    pub call_id: String,
    pub from_tag: Option<String>,
    pub to_tag: Option<String>,
    pub request_type: String,
    pub content_type: Option<String>,
    pub response_code: u16,
    pub diag_header: Option<String>,
    pub progress: Vec<String>,
}

/// Most progress reports put into one document (design).
pub const MAX_PROGRESS: usize = 8;

/// The `reportError` document (ER-04; §2.2.2, example §4.1) with its length limits.
pub fn report_error_body(r: &Report) -> Result<String, Error> {
    if r.request_type.is_empty() || r.request_type.len() > 33 {
        return Err(Error::TooLong("requestType"));
    }
    if r.content_type.as_ref().is_some_and(|c| c.len() > 257) {
        return Err(Error::TooLong("contentType"));
    }
    let ids = [&r.from_uri, &r.to_uri, &r.from_tag, &r.to_tag]
        .iter()
        .map(|o| o.as_ref().map_or(0, String::len))
        .sum::<usize>()
        + r.call_id.len();
    if r.call_id.is_empty() || ids > 669 {
        return Err(Error::TooLong("URIs, tags and Call-ID"));
    }
    let too_long = |d: &String| d.len() > 65_535;
    if r.diag_header.as_ref().is_some_and(too_long) || r.progress.iter().any(too_long) {
        return Err(Error::TooLong("diagHeader"));
    }
    let attr = |name: &str, v: &Option<String>| {
        v.as_ref()
            .map(|v| format!(" {name}=\"{}\"", escape(v)))
            .unwrap_or_default()
    };
    let mut b = format!("<reportError xmlns=\"{NS}\"><error");
    b.push_str(&attr("fromUri", &r.from_uri));
    b.push_str(&attr("toUri", &r.to_uri));
    b.push_str(&format!(" callId=\"{}\"", escape(&r.call_id)));
    b.push_str(&attr("fromTag", &r.from_tag));
    b.push_str(&attr("toTag", &r.to_tag));
    b.push_str(&attr("contentType", &r.content_type));
    b.push_str(&format!(
        " responseCode=\"{}\" requestType=\"{}\">",
        r.response_code,
        escape(&r.request_type)
    ));
    if let Some(d) = &r.diag_header {
        b.push_str(&format!("<diagHeader>{}</diagHeader>", escape(d)));
    }
    b.push_str("<progressReports>");
    for p in r.progress.iter().take(MAX_PROGRESS) {
        b.push_str(&format!(
            "<progressReport><diagHeader>{}</diagHeader></progressReport>",
            escape(p)
        ));
    }
    b.push_str("</progressReports></error></reportError>");
    Ok(b)
}

/// Whether further reports may be sent in this session after an answer to a `reportError`
/// (ER-05; §3.1.5, p. 17): not after a 503 without diagnostics or with ErrorId 2019 (no error
/// reporting service). A 503 with Retry-After means busy (later reports are fine; the report
/// itself is not repeated – design: it is optional), a 413 refuses only this report.
pub fn keep_reporting(status: u16, retry_after: bool, diag: Option<&Diagnostic>) -> bool {
    !(status == 503 && !retry_after && diag.is_none_or(|d| d.id == 2019))
}

#[cfg(test)]
mod tests;
