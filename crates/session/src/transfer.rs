//! Call transfer in the engine: as transferor (unattended and with consultation), as transferee
//! (following a REFER) and as transfer target (INVITE with Replaces).
//!
//! Evidence: file 24 (L-13, OB-25), file 06 (RE-64), file 03 (AE-14/AE-37). Sources: RFC 3515
//! §2.4 (REFER, implicit subscription, NOTIFY with message/sipfrag), RFC 3891 §3 (Replaces:
//! call-id, to-tag = the recipient's local tag, from-tag = its remote tag; 481 without a match),
//! RFC 3892 (Referred-By), RFC 5589 §6–§7 (unattended and attended transfer flows);
//! [MS-SIPRE] 20250520 §2.2.19, §3.14 (ms-safe-transfer, Ms-Sensitivity from the Refer-To URI,
//! pp. 31, 75–76); OB-25 (official UCMA reference): unattended transfer ends after 202 Accepted,
//! attended after a successful NOTIFY.
//!
//! Security by design: a REFER is honoured only inside an established call dialog from its peer
//! (the server-signed request), with exactly one checked `sip:` Refer-To; of the URI headers only
//! Replaces and Ms-Sensitivity are taken, both checked; Referred-By is checked or replaced by the
//! peer's URI; a Replaces header is honoured only if it names one of our established calls
//! exactly (otherwise 481); one transfer per call at a time.

use super::*;
use crate::call::CallState;
use commscraft_sdp as sdp;

/// Where to transfer a call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferTarget {
    /// Unattended: the peer calls this address.
    Address(String),
    /// Attended: the peer replaces our consultation call with this call's party.
    Call(CallId),
}

/// Our transfer of a call (transferor side).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Outgoing {
    /// The consultation call of an attended transfer.
    pub consultation: Option<CallId>,
}

/// A REFER we follow (transferee side).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Incoming {
    pub target: String,
    pub referred_by: String,
    pub replaces: Option<String>,
    pub sensitivity: Option<String>,
    /// The call placed to the target.
    pub new_call: Option<CallId>,
}

/// Ms-Sensitivity values (RE-53).
const SENSITIVITY: [&str; 4] = [
    "normal",
    "private",
    "normal-no-diversion",
    "private-no-diversion",
];

fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::new();
    let mut b = s.bytes();
    while let Some(c) = b.next() {
        if c == b'%' {
            let h = [b.next()?, b.next()?];
            out.push(u8::from_str_radix(std::str::from_utf8(&h).ok()?, 16).ok()?);
        } else {
            out.push(c);
        }
    }
    String::from_utf8(out).ok()
}

/// Percent-encodes a URI header value (RFC 3261 §25.1 hnv-unreserved and unreserved stay).
fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.!~*'()[]/?:+$".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// A Replaces value: `call-id;to-tag=…;from-tag=…` (RFC 3891 §6.1), checked.
fn replaces_parts(v: &str) -> Option<(String, String, String)> {
    if v.len() > 512
        || v.chars()
            .any(|c| c.is_control() || c.is_whitespace() || "<>\"".contains(c))
    {
        return None;
    }
    let mut it = v.split(';');
    let call_id = it.next()?.to_string();
    let (mut to, mut from) = (None, None);
    for p in it {
        match p.split_once('=') {
            Some((k, v)) if k.eq_ignore_ascii_case("to-tag") => to = Some(v.to_string()),
            Some((k, v)) if k.eq_ignore_ascii_case("from-tag") => from = Some(v.to_string()),
            _ => {}
        }
    }
    (!call_id.is_empty()).then_some((call_id, to?, from?))
}

/// The Refer-To of a REFER: the target and the accepted URI headers.
fn parse_refer_to(v: &str) -> Option<(String, Option<String>, Option<String>)> {
    let inner = v.split_once('<')?.1.split_once('>')?.0.trim();
    let (base, headers) = match inner.split_once('?') {
        Some((b, h)) => (b, Some(h)),
        None => (inner, None),
    };
    let target = pres::checked_sip_uri(base).ok()?;
    let (mut replaces, mut sensitivity) = (None, None);
    for h in headers
        .unwrap_or("")
        .split('&')
        .filter(|h| !h.is_empty())
        .take(8)
    {
        let (k, v) = h.split_once('=')?;
        let v = percent_decode(v)?;
        if k.eq_ignore_ascii_case("Replaces") {
            replaces_parts(&v)?;
            replaces = Some(v);
        } else if k.eq_ignore_ascii_case("Ms-Sensitivity") {
            let s = v.trim().to_ascii_lowercase();
            if SENSITIVITY.contains(&s.as_str()) {
                sensitivity = Some(s);
            }
        }
    }
    Some((target, replaces, sensitivity))
}

/// The status code of a message/sipfrag body (RFC 3515 §2.4.5): its first line.
fn sipfrag_status(body: &[u8]) -> Option<u16> {
    let line = std::str::from_utf8(body).ok()?.lines().next()?;
    let mut f = line.split_whitespace();
    (f.next()? == "SIP/2.0").then_some(())?;
    f.next()?.parse().ok().filter(|c| (100..700).contains(c))
}

impl<R: Random> Engine<R> {
    /// `Transfer`: REFER in the call's dialog (RFC 3515, RFC 5589). Unattended: Refer-To the
    /// address; attended: Refer-To the consultation call's peer with Replaces for that call
    /// (to-tag = the peer's tag, from-tag = ours, RFC 3891 §3).
    pub(crate) fn transfer(
        &mut self,
        id: CallId,
        to: TransferTarget,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let refer_to = match &to {
            TransferTarget::Address(a) => match pres::checked_sip_uri(a) {
                Ok(u) => u,
                Err(_) => {
                    out.push(Output::Event(Event::Notice("not a usable address".into())));
                    return;
                }
            },
            TransferTarget::Call(other) => {
                let Some(d) = self
                    .calls
                    .get(other)
                    .filter(|c| c.state == CallState::Active && *other != id)
                    .and_then(|c| c.dialog.clone())
                else {
                    out.push(Output::Event(Event::Notice("no such active call".into())));
                    return;
                };
                let replaces = format!(
                    "{};to-tag={};from-tag={}",
                    d.call_id, d.remote_tag, d.local_tag
                );
                format!("{}?Replaces={}", d.remote_target, percent_encode(&replaces))
            }
        };
        let aor = self.cfg.aor.clone();
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        if c.state != CallState::Active || c.transfer_out.is_some() {
            out.push(Output::Event(Event::Notice(
                "the call cannot be transferred now".into(),
            )));
            return;
        }
        let Some(mut d) = c.dialog.take() else { return };
        let mut m = d.request("REFER");
        c.dialog = Some(d);
        c.transfer_out = Some(Outgoing {
            consultation: match to {
                TransferTarget::Call(o) => Some(o),
                TransferTarget::Address(_) => None,
            },
        });
        self.complete(&mut m);
        m.push_header("Refer-To", &format!("<{refer_to}>"));
        // AE-37: the call reached us under another address (forwarded, team, delegate) – the
        // dialog's own URI is not ours; name the real recipient in ms-referee-uri.
        let own_uri = m
            .single("From")
            .ok()
            .flatten()
            .and_then(|v| commscraft_sip_msg::parse_name_addr(v).ok())
            .map(|n| n.uri);
        let retargeted = own_uri.is_some_and(|u| !u.eq_ignore_ascii_case(&aor));
        if retargeted {
            m.push_header("Referred-By", &format!("<{aor}>;ms-referee-uri=\"{aor}\""));
        } else {
            m.push_header("Referred-By", &format!("<{aor}>"));
        }
        self.send_request(m, Purpose::Refer(id), now, out);
    }

    /// The answer to our REFER: 202 starts the transfer; unattended ends our call here (OB-25).
    pub(crate) fn on_refer_answered(
        &mut self,
        id: CallId,
        code: u16,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(t) = self.calls.get(&id).and_then(|c| c.transfer_out.clone()) else {
            return;
        };
        if (200..300).contains(&code) {
            if t.consultation.is_none() {
                Self::call_event(
                    out,
                    Event::TransferResult {
                        call: id,
                        status: code,
                    },
                );
                self.hangup(id, now, out);
            }
        } else {
            if let Some(c) = self.calls.get_mut(&id) {
                c.transfer_out = None;
            }
            Self::call_event(
                out,
                Event::TransferResult {
                    call: id,
                    status: code,
                },
            );
        }
    }

    /// NOTIFY of the implicit refer subscription (RFC 3515 §2.4.4): the target's answer as
    /// message/sipfrag. Attended: a final 2xx completes the transfer – our call ends.
    pub(crate) fn on_refer_notify(
        &mut self,
        id: CallId,
        req: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let r = response_to(req, 200, "OK");
        self.send_plain(r, now, out);
        let Some(code) = sipfrag_status(&req.body) else {
            return;
        };
        if code < 200
            || self
                .calls
                .get(&id)
                .and_then(|c| c.transfer_out.as_ref())
                .is_none()
        {
            return;
        }
        if let Some(c) = self.calls.get_mut(&id) {
            c.transfer_out = None;
        }
        Self::call_event(
            out,
            Event::TransferResult {
                call: id,
                status: code,
            },
        );
        if (200..300).contains(&code) {
            self.hangup(id, now, out);
        }
    }

    /// A REFER of our call's peer (transferee side): 202, NOTIFY 100 Trying, and the user's
    /// application is told to place the new call (`FollowTransfer`).
    pub(crate) fn on_refer(&mut self, id: CallId, req: &Message, now: u64, out: &mut Vec<Output>) {
        let parsed = (req.header_values("Refer-To").count() == 1)
            .then(|| {
                req.single("Refer-To")
                    .ok()
                    .flatten()
                    .and_then(parse_refer_to)
            })
            .flatten();
        let busy = self.calls.get(&id).is_none_or(|c| c.transfer_in.is_some());
        let Some((target, replaces, sensitivity)) = parsed.filter(|_| !busy) else {
            let r = response_to(
                req,
                if busy { 491 } else { 400 },
                if busy {
                    "Request Pending"
                } else {
                    "Bad Request"
                },
            );
            self.send_plain(r, now, out);
            return;
        };
        let peer = self
            .calls
            .get(&id)
            .map(|c| c.peer.clone())
            .unwrap_or_default();
        let referred_by = req
            .single("Referred-By")
            .ok()
            .flatten()
            .and_then(|v| {
                v.split_once('<')
                    .and_then(|(_, r)| r.split_once('>'))
                    .map(|(u, _)| u.to_string())
            })
            .and_then(|u| pres::checked_sip_uri(&u).ok())
            .unwrap_or(peer);
        let mut ok = response_to(req, 202, "Accepted");
        ok.push_header("Contact", &format!("<{}>", self.gruu));
        self.send_plain(ok, now, out);
        if let Some(c) = self.calls.get_mut(&id) {
            c.transfer_in = Some(Incoming {
                target: target.clone(),
                referred_by,
                replaces,
                sensitivity,
                new_call: None,
            });
        }
        self.notify_refer(id, "SIP/2.0 100 Trying", false, now, out);
        Self::call_event(
            out,
            Event::TransferRequested {
                call: id,
                to: target,
            },
        );
    }

    /// NOTIFY in the transferred call's dialog (RFC 3515 §2.4.4–2.4.5).
    fn notify_refer(
        &mut self,
        id: CallId,
        frag: &str,
        last: bool,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        let Some(mut d) = c.dialog.take() else { return };
        let mut m = d.request("NOTIFY");
        c.dialog = Some(d);
        self.complete(&mut m);
        m.push_header("Event", "refer");
        m.push_header(
            "Subscription-State",
            if last {
                "terminated;reason=noresource"
            } else {
                "active;expires=60"
            },
        );
        m.push_header("Content-Type", "message/sipfrag;version=2.0");
        m.body = format!("{frag}\r\n").into_bytes();
        self.send_request(m, Purpose::Ignore, now, out);
    }

    /// `DeclineTransfer`: the user does not want to follow the REFER (issue #67) – the final
    /// NOTIFY reports 603 (RFC 3515 §2.4.5) and the transfer is forgotten; our call stays.
    pub(crate) fn decline_transfer(&mut self, id: CallId, now: u64, out: &mut Vec<Output>) {
        let pending = self
            .calls
            .get_mut(&id)
            .and_then(|c| c.transfer_in.take_if(|t| t.new_call.is_none()));
        if pending.is_none() {
            out.push(Output::Event(Event::Notice(
                "no transfer to decline".into(),
            )));
            return;
        }
        self.notify_refer(id, "SIP/2.0 603 Declined", true, now, out);
    }

    /// `FollowTransfer`: the call to the REFER's target with Referred-By, Replaces and
    /// Ms-Sensitivity (RFC 3892, RFC 3891, RE-64).
    pub(crate) fn follow_transfer(
        &mut self,
        id: CallId,
        local: sdp::Local,
        policy: sdp::Encryption,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(t) = self.calls.get(&id).and_then(|c| c.transfer_in.clone()) else {
            out.push(Output::Event(Event::Notice("no transfer to follow".into())));
            return;
        };
        if t.new_call.is_some() {
            return;
        }
        let mut extra = vec![("Referred-By".to_string(), format!("<{}>", t.referred_by))];
        if let Some(r) = &t.replaces {
            extra.push(("Replaces".to_string(), r.clone()));
        }
        if let Some(s) = &t.sensitivity {
            extra.push(("Ms-Sensitivity".to_string(), s.clone()));
        }
        let Some(new_call) = self.start_call_with(
            &t.target,
            local,
            policy,
            crate::call::Extras {
                headers: &extra,
                part: None,
                video: None,
                emergency: false,
            },
            now,
            out,
        ) else {
            self.notify_refer(id, "SIP/2.0 500 Server Internal Error", true, now, out);
            return;
        };
        if let Some(c) = self.calls.get_mut(&new_call) {
            c.transferred_from = Some(id);
        }
        if let Some(c) = self.calls.get_mut(&id) {
            if let Some(t) = c.transfer_in.as_mut() {
                t.new_call = Some(new_call);
            }
        }
    }

    /// The outcome of a call placed for a transfer, reported to the transferor.
    pub(crate) fn transferred_call_result(
        &mut self,
        new_call: CallId,
        code: u16,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(old) = self.calls.get(&new_call).and_then(|c| c.transferred_from) else {
            return;
        };
        let frag = if (200..300).contains(&code) {
            "SIP/2.0 200 OK".to_string()
        } else {
            format!("SIP/2.0 {code} Failed")
        };
        self.notify_refer(old, &frag, true, now, out);
        if let Some(c) = self.calls.get_mut(&old) {
            c.transfer_in = None;
        }
    }

    /// An incoming INVITE with Replaces (RFC 3891 §3): the call it replaces, if it names one of
    /// our established dialogs exactly; `Err(())` if it names none (answer 481).
    pub(crate) fn replaced_call(&self, req: &Message) -> Result<Option<CallId>, ()> {
        let Some(v) = req.single("Replaces").ok().flatten() else {
            return Ok(None);
        };
        let (call_id, to_tag, from_tag) = replaces_parts(v.trim()).ok_or(())?;
        self.calls
            .iter()
            .find(|(_, c)| {
                c.state == CallState::Active
                    && c.dialog.as_ref().is_some_and(|d| {
                        d.call_id == call_id && d.local_tag == to_tag && d.remote_tag == from_tag
                    })
            })
            .map(|(id, _)| Some(*id))
            .ok_or(())
    }

    /// The call an incoming call replaces (the application answers it without asking).
    pub fn replaces(&self, id: CallId) -> Option<CallId> {
        self.calls.get(&id).and_then(|c| c.replaces)
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn refer_to_parsing() {
        let (t, r, s) = parse_refer_to(
            "<sip:carol@contoso.com;opaque=user:epid:x;gruu?Replaces=abc%40host%3Bto-tag%3D7%3Bfrom-tag%3D9&Ms-Sensitivity=private&X-Evil=1>",
        )
        .unwrap();
        assert_eq!(t, "sip:carol@contoso.com;opaque=user:epid:x;gruu");
        assert_eq!(r.as_deref(), Some("abc@host;to-tag=7;from-tag=9"));
        assert_eq!(s.as_deref(), Some("private"));
        assert!(parse_refer_to("<tel:+491234>").is_none());
        assert!(
            parse_refer_to("<sip:a@b?Replaces=x%3Bto-tag%3D1>").is_none(),
            "Replaces without from-tag"
        );
        assert!(
            parse_refer_to("<sip:a@b?Replaces=%0D%0AVia:x%3Bto-tag%3D1%3Bfrom-tag%3D2>").is_none()
        );
        assert_eq!(
            parse_refer_to("<sip:a@b?Ms-Sensitivity=secret>").unwrap().2,
            None
        );
        assert_eq!(percent_encode("a@b;to-tag=1"), "a%40b%3Bto-tag%3D1");
        assert_eq!(sipfrag_status(b"SIP/2.0 200 OK\r\n"), Some(200));
        assert_eq!(sipfrag_status(b"HTTP/1.1 200 OK"), None);
    }
}
