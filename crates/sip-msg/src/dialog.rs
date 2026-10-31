//! UAC dialogs (RFC 3261 §12): created from a 2xx (or, early, a 1xx with a To tag) to a
//! dialog-creating request (SUBSCRIBE, INVITE), used for in-dialog requests and to recognise requests of the peer in the dialog.
//!
//! Security by design: the route set and the remote target come from the network; every URI is
//! checked (`sip:`/`sips:`, no characters that could break a header) and the number of routes is
//! bounded. A dialog is only created from a response to our own request (same Call-ID, From tag
//! and CSeq) that carries a To tag.

use crate::{parse_name_addr, Error, Message};

const MAX_ROUTES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    pub call_id: String,
    pub local_uri: String,
    pub local_tag: String,
    pub remote_uri: String,
    pub remote_tag: String,
    /// Request-URI of in-dialog requests (the peer's Contact, §12.1.2).
    pub remote_target: String,
    /// Route set: the Record-Route URIs of the 2xx in reverse order (§12.1.2), as name-addrs.
    pub route_set: Vec<String>,
    pub local_cseq: u32,
}

fn safe_uri(u: &str) -> bool {
    let scheme_ok = u.get(..4).is_some_and(|s| s.eq_ignore_ascii_case("sip:"))
        || u.get(..5).is_some_and(|s| s.eq_ignore_ascii_case("sips:"));
    scheme_ok
        && u.len() <= 1024
        && !u
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "<>\",".contains(c))
}

impl Dialog {
    /// Creates the dialog from our request and the peer's 2xx (RFC 3261 §12.1.2).
    pub fn from_response(request: &Message, response: &Message) -> Result<Self, Error> {
        Self::from_response_in(request, response, 200..300)
    }

    /// An early dialog from a provisional response with a To tag (RFC 3261 §12.1.2, §13.2.2.1):
    /// used for PRACK (RFC 3262 §7.2); the confirmed dialog comes from the 2xx later.
    pub fn early_from_response(request: &Message, response: &Message) -> Result<Self, Error> {
        Self::from_response_in(request, response, 101..200)
    }

    fn from_response_in(
        request: &Message,
        response: &Message,
        codes: std::ops::Range<u16>,
    ) -> Result<Self, Error> {
        if !response.status_code().is_some_and(|c| codes.contains(&c)) {
            return Err(Error::InvalidValue);
        }
        let call_id = request.call_id()?.ok_or(Error::InvalidValue)?;
        if response.call_id()? != Some(call_id) {
            return Err(Error::InvalidValue);
        }
        let (req_cseq, resp_cseq) = (request.cseq()?, response.cseq()?);
        if req_cseq.map(|c| (c.number, c.method)) != resp_cseq.map(|c| (c.number, c.method)) {
            return Err(Error::InvalidValue);
        }
        let from = request.from()?.ok_or(Error::InvalidValue)?;
        let local_tag = from.param("tag").ok_or(Error::InvalidValue)?.to_string();
        if response
            .from()?
            .and_then(|f| f.param("tag").map(str::to_string))
            != Some(local_tag.clone())
        {
            return Err(Error::InvalidValue);
        }
        let to = response.to()?.ok_or(Error::InvalidValue)?;
        let remote_tag = to.param("tag").ok_or(Error::InvalidValue)?.to_string();
        let contact = response
            .name_addrs("Contact")?
            .into_iter()
            .next()
            .ok_or(Error::InvalidValue)?;
        if !safe_uri(&contact.uri) {
            return Err(Error::InvalidValue);
        }
        let mut route_set = Vec::new();
        for v in response.header_values("Record-Route") {
            for item in crate::split_list(v)? {
                let a = parse_name_addr(item)?;
                if !safe_uri(&a.uri) || route_set.len() >= MAX_ROUTES {
                    return Err(Error::InvalidValue);
                }
                route_set.push(item.trim().to_string());
            }
        }
        route_set.reverse();
        Ok(Self {
            call_id: call_id.to_string(),
            local_uri: from.uri,
            local_tag,
            remote_uri: to.uri,
            remote_tag,
            remote_target: contact.uri,
            route_set,
            local_cseq: req_cseq.map_or(1, |c| c.number),
        })
    }

    /// The UAS side (RFC 3261 §12.1.1): a dialog from a peer's dialog-creating request (INVITE)
    /// that we answer with a 2xx carrying `local_tag` in the To header. Route set = the
    /// Record-Route values in order; remote target = the peer's Contact.
    pub fn from_request(request: &Message, local_tag: &str) -> Result<Self, Error> {
        if !request
            .method()
            .is_some_and(|m| m.eq_ignore_ascii_case("INVITE"))
        {
            return Err(Error::InvalidValue);
        }
        let call_id = request.call_id()?.ok_or(Error::InvalidValue)?;
        let from = request.from()?.ok_or(Error::InvalidValue)?;
        let remote_tag = from.param("tag").ok_or(Error::InvalidValue)?.to_string();
        let to = request.to()?.ok_or(Error::InvalidValue)?;
        if to.param("tag").is_some()
            || local_tag.is_empty()
            || !local_tag.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Err(Error::InvalidValue);
        }
        let contact = request
            .name_addrs("Contact")?
            .into_iter()
            .next()
            .ok_or(Error::InvalidValue)?;
        if !safe_uri(&contact.uri) || !safe_uri(&from.uri) {
            return Err(Error::InvalidValue);
        }
        let mut route_set = Vec::new();
        for v in request.header_values("Record-Route") {
            for item in crate::split_list(v)? {
                let a = parse_name_addr(item)?;
                if !safe_uri(&a.uri) || route_set.len() >= MAX_ROUTES {
                    return Err(Error::InvalidValue);
                }
                route_set.push(item.trim().to_string());
            }
        }
        Ok(Self {
            call_id: call_id.to_string(),
            local_uri: to.uri,
            local_tag: local_tag.to_string(),
            remote_uri: from.uri,
            remote_tag,
            remote_target: contact.uri,
            route_set,
            local_cseq: 0,
        })
    }

    /// A new in-dialog request (RFC 3261 §12.2.1.1). The caller adds Via, Contact and the
    /// method-specific headers, and signs it.
    pub fn request(&mut self, method: &str) -> Message {
        self.local_cseq = self.local_cseq.saturating_add(1);
        let mut m = Message::request(method, &self.remote_target);
        for r in &self.route_set {
            m.push_header("Route", r);
        }
        m.push_header(
            "From",
            &format!("<{}>;tag={}", self.local_uri, self.local_tag),
        );
        m.push_header(
            "To",
            &format!("<{}>;tag={}", self.remote_uri, self.remote_tag),
        );
        m.push_header("Call-ID", &self.call_id);
        m.push_header("CSeq", &format!("{} {method}", self.local_cseq));
        m
    }

    /// True if `msg` is a request of the peer inside this dialog (Call-ID, our tag in To, the
    /// peer's tag in From).
    pub fn is_peer_request(&self, msg: &Message) -> bool {
        let check = || -> Result<bool, Error> {
            Ok(msg.method().is_some()
                && msg.call_id()? == Some(self.call_id.as_str())
                && msg
                    .to()?
                    .is_some_and(|t| t.param("tag") == Some(self.local_tag.as_str()))
                && msg
                    .from()?
                    .is_some_and(|f| f.param("tag") == Some(self.remote_tag.as_str())))
        };
        check().unwrap_or(false)
    }
}

/// The response to a request of the peer (RFC 3261 §8.2.6): Via, From, To, Call-ID and CSeq
/// copied. The caller signs it.
pub fn response_to(request: &Message, code: u16, reason: &str) -> Message {
    let mut r = Message::response(code, reason);
    for name in ["Via", "From", "To", "Call-ID", "CSeq"] {
        for v in request.header_values(name) {
            r.push_header(name, v);
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse, Limits};

    fn msg(lines: &[&str]) -> Message {
        let mut s = lines.join("\r\n");
        s.push_str("\r\n\r\n");
        parse(s.as_bytes(), &Limits::default()).unwrap()
    }

    /// [MS-PRES] 20250520 §4.14 (contact list SUBSCRIBE, p. 197) with Record-Route as SfB sends
    /// it (two hops).
    fn request() -> Message {
        msg(&[
            "SUBSCRIBE sip:user5@contoso.com SIP/2.0",
            "Via: SIP/2.0/TLS 192.168.0.244:51649;branch=z9hG4bK1",
            "From: <sip:user5@contoso.com>;tag=435520b0a8;epid=a8716485d4",
            "To: <sip:user5@contoso.com>",
            "Call-ID: f9971f8004b34e7497573f3dbefe3caf",
            "CSeq: 1 SUBSCRIBE",
            "Event: vnd-microsoft-roaming-contacts",
            "Content-Length: 0",
        ])
    }

    fn ok() -> Message {
        msg(&[
            "SIP/2.0 200 OK",
            "Contact: <sip:user5.contoso.com:5061;transport=tls;ms-fe=SERVER.contoso.com>",
            "From: \"User 5\"<sip:user5@contoso.com>;tag=435520b0a8;epid=a8716485d4",
            "To: <sip:user5@contoso.com>;tag=C0B5AE6F",
            "Call-ID: f9971f8004b34e7497573f3dbefe3caf",
            "CSeq: 1 SUBSCRIBE",
            "Record-Route: <sip:pool.contoso.com:5061;transport=tls;lr>",
            "Record-Route: <sip:edge.contoso.com:443;transport=tls;opaque=state:Ci.R1;lr>",
            "Expires: 30384",
            "Content-Length: 0",
        ])
    }

    #[test]
    fn dialog_from_2xx_and_in_dialog_requests() {
        let mut d = Dialog::from_response(&request(), &ok()).unwrap();
        assert_eq!(d.remote_tag, "C0B5AE6F");
        assert_eq!(
            d.remote_target,
            "sip:user5.contoso.com:5061;transport=tls;ms-fe=SERVER.contoso.com"
        );
        assert_eq!(
            d.route_set,
            [
                "<sip:edge.contoso.com:443;transport=tls;opaque=state:Ci.R1;lr>",
                "<sip:pool.contoso.com:5061;transport=tls;lr>"
            ]
        );
        let m = d.request("SUBSCRIBE");
        assert_eq!(m.cseq().unwrap().unwrap().number, 2);
        assert_eq!(m.to().unwrap().unwrap().param("tag"), Some("C0B5AE6F"));
        assert_eq!(m.header_values("Route").count(), 2);
        assert!(m.to_bytes().is_ok());
        let mut notify = Message::request("NOTIFY", "sip:192.168.0.244:51649;transport=tls");
        notify.push_header("From", "<sip:user5@contoso.com>;tag=C0B5AE6F");
        notify.push_header("To", "<sip:user5@contoso.com>;tag=435520b0a8");
        notify.push_header("Call-ID", "f9971f8004b34e7497573f3dbefe3caf");
        notify.push_header("CSeq", "7 NOTIFY");
        assert!(d.is_peer_request(&notify));
        let answer = response_to(&notify, 200, "OK");
        assert_eq!(answer.cseq().unwrap().unwrap().method, "NOTIFY");
    }

    #[test]
    fn uas_dialog_from_an_invite() {
        let invite = msg(&[
            "INVITE sip:alice@contoso.com;opaque=user:epid:x;gruu SIP/2.0",
            "Via: SIP/2.0/TLS 10.0.0.1;branch=z9hG4bKi",
            "From: <sip:bob@contoso.com>;tag=b1;epid=0b85fd1bd6",
            "To: <sip:alice@contoso.com>",
            "Call-ID: c9",
            "CSeq: 1 INVITE",
            "Contact: <sip:bob@contoso.com;opaque=user:epid:y;gruu>",
            "Record-Route: <sip:pool.contoso.com:5061;transport=tls;lr>",
            "Content-Length: 0",
        ]);
        let mut d = Dialog::from_request(&invite, "a1").unwrap();
        assert_eq!((d.remote_tag.as_str(), d.local_tag.as_str()), ("b1", "a1"));
        assert_eq!(
            d.route_set,
            ["<sip:pool.contoso.com:5061;transport=tls;lr>"]
        );
        let bye = d.request("BYE");
        assert_eq!(
            bye.single("To").unwrap(),
            Some("<sip:bob@contoso.com>;tag=b1")
        );
        assert_eq!(bye.cseq().unwrap().unwrap().number, 1);
        assert!(Dialog::from_request(&invite, "a\"b").is_err());
        let mut with_tag = invite.clone();
        for h in with_tag.headers.iter_mut().filter(|h| h.name == "To") {
            h.value.push_str(";tag=x");
        }
        assert!(
            Dialog::from_request(&with_tag, "a1").is_err(),
            "re-INVITE is not a new dialog"
        );
    }

    #[test]
    fn foreign_or_hostile_responses_create_no_dialog() {
        let req = request();
        let mut other = ok();
        for h in other.headers.iter_mut().filter(|h| h.name == "Call-ID") {
            h.value = "other".into();
        }
        assert!(Dialog::from_response(&req, &other).is_err());
        let mut no_tag = ok();
        for h in no_tag.headers.iter_mut().filter(|h| h.name == "To") {
            h.value = "<sip:user5@contoso.com>".into();
        }
        assert!(Dialog::from_response(&req, &no_tag).is_err());
        let mut tel = ok();
        for h in tel.headers.iter_mut().filter(|h| h.name == "Contact") {
            h.value = "<tel:+4930123>".into();
        }
        assert!(Dialog::from_response(&req, &tel).is_err());
        let mut many = ok();
        for i in 0..20 {
            many.push_header("Record-Route", &format!("<sip:r{i}.contoso.com;lr>"));
        }
        assert!(Dialog::from_response(&req, &many).is_err());
        let mut err = ok();
        err.start = crate::StartLine::Response {
            code: 403,
            reason: "Forbidden".into(),
        };
        assert!(Dialog::from_response(&req, &err).is_err());
    }
}
