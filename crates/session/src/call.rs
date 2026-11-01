//! Audio calls in the engine: INVITE with the audio offer (multipart per MS-SIPRE §3.15),
//! ringing, answer, final offer / re-INVITE, hold, CANCEL, BYE; incoming calls with
//! ringing, accept, reject and timeout.
//!
//! Evidence: files 01 (OCS-UC12), 06 (RE-65), 14 (SD-02 … SD-13, SD-41, SD-51), 13 (IC-07).
//! Sources: [MS-OCSPROT] 20250520 §2.5.12 (call flow, pp. 38–41); [MS-SIPRE] 20250520 §3.15
//! (multipart SDP, pp. 76–78), example §4.12.1 (pp. 89–90); [MS-SDPEXT] §3.1.5.2.2/§3.1.5.14
//! (no renegotiation of keys); RFC 3261 §9 (CANCEL), §13–§15 (INVITE, re-INVITE, BYE);
//! draft-ietf-mmusic-ice-19 §9.1.2.2 (final offer with remote-candidates).
//!
//! Telephony (PS-01 … PS-09): [MS-OCPSTN] 20250520 §3.2 (phone-context, call back with the
//! P-Asserted-Identity, pp. 16–17), §3.3 (605 while ringing, p. 18), §3.5 (anonymous, p. 20),
//! §3.1 (`isGateway`, p. 16), §3.7 (Content-ID / ms-accepted-content-id, p. 21); RFC 3966 §5.1.4/
//! §5.1.5 (global and local numbers), RFC 3261 §19.1.1 (`user=phone`, escaping), RFC 3325 (PAI).
//!
//! Early media as the caller (SD-50, SD-51): [MS-SDPEXT] §3.1.5.12–3.1.5.13 (pp. 28–30),
//! RFC 3262 (100rel, PRACK with RAck, RSeq order), RFC 3261 §12.1.2 (early dialogs), [MS-SIPRE]
//! `Ms-Forking`, draft-ietf-sip-199-01 (199 ends an early dialog); `Supported: ms-early-media`
//! for gateways ([MS-OCPSTN] §2.2.4, PS-06).
//!
//! Scope: the media (sockets, TURN, ICE, SRTP, codec, devices) runs in the adapter; the engine
//! carries the SDP. Not offered: ICE-06 SDP (OCS 2007, RE-65 fallback); as the callee we answer
//! in the 200 only (no reliable 183 with an early answer, a SHOULD of SD-51); early media of a
//! forked call (`Ms-Forking`) is not started (SHOULD-level in §3.1.5.12.2) – the 200 decides.
//!
//! Security by design: every SDP goes through the bounded `sdp` parser and the encryption
//! policy (no silent plaintext); keys are never renegotiated in a re-INVITE (SD-13) – an update
//! must carry the key of the call; calls are bounded; incoming calls ring at most 60 s; nothing
//! is accepted without the user's command.

use super::*;
use commscraft_sdp as sdp;

pub type CallId = u32;
pub const MAX_CALLS: usize = 4;
/// An incoming call rings at most this long before 480 (design).
pub const RING_TIMEOUT: u64 = 60;
const BOUNDARY: &str = "CommSCraft_audio_offer";
const MIXED_BOUNDARY: &str = "CommSCraft_e911";

/// What an outgoing INVITE carries besides the offer: extra headers (transfer, emergency) and one
/// more MIME part.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Extras<'a> {
    pub headers: &'a [(String, String)],
    pub part: Option<&'a ExtraPart>,
    /// Our video stream: the INVITE offers audio and video (function 28).
    pub video: Option<&'a sdp::video::VideoLocal>,
    /// An emergency call: it may exceed [`MAX_CALLS`] by one (issue #67 – an emergency call is
    /// never refused for the number of calls in progress).
    pub emergency: bool,
}

/// One more MIME part of an outgoing INVITE (the PIDF-LO of an emergency call). The fields come
/// from checked builders; the Content-ID has no angle brackets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtraPart {
    pub content_type: String,
    pub content_id: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallState {
    /// Our INVITE is out; `cancelling` once the user hung up before the answer.
    Inviting {
        cancelling: bool,
    },
    /// Incoming INVITE, 180 sent, waiting for the user until `deadline`.
    Ringing {
        deadline: u64,
    },
    Active,
}

#[derive(Debug, Clone)]
pub(crate) struct Call {
    pub peer: String,
    pub state: CallState,
    /// Our INVITE (outgoing) or the peer's INVITE (incoming, until answered).
    pub invite: Message,
    pub dialog: Option<Dialog>,
    /// Our SDP parameters; the key stays the same for the whole call (SD-13).
    pub local: Option<sdp::Local>,
    pub policy: sdp::Encryption,
    /// The SDP of the incoming offer (the ICE-19 part, RE-65).
    pub offer: Vec<u8>,
    pub ringing_reported: bool,
    /// Early dialogs by To tag (for PRACK) with the last acknowledged RSeq; bounded.
    pub early: Vec<(Dialog, Option<u32>)>,
    /// The To tag whose SDP answer started early media.
    pub early_media: Option<String>,
    /// A provisional response carried `Ms-Forking` (MS-SIPRE).
    pub forked: bool,
    /// Content-ID of the offer part we answer (PS-09: copied to ms-accepted-content-id).
    pub content_id: Option<String>,
    /// Our transfer of this call (REFER sent).
    pub transfer_out: Option<transfer::Outgoing>,
    /// A REFER of the peer we follow.
    pub transfer_in: Option<transfer::Incoming>,
    /// This call was placed for a transfer of that call (its result is reported there).
    pub transferred_from: Option<CallId>,
    /// This incoming call replaces that call (RFC 3891).
    pub replaces: Option<CallId>,
    /// Our video stream, if the call carries video (function 28); its key stays for the whole
    /// call like the audio key (SD-13).
    pub video: Option<sdp::video::VideoLocal>,
}

/// The caller of an incoming call as far as the signalling tells (PS-02, PS-04, PS-07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    /// The asserted SIP identity, otherwise the From URI.
    pub uri: String,
    pub number: Option<String>,
    pub anonymous: bool,
    /// The call comes from a PSTN gateway (`isGateway`).
    pub gateway: bool,
    /// Whom the call was meant for when it reaches us through team or delegate ringing or a
    /// forwarding: the checked `Referred-By` address (RFC 3892 §3; RE-72, inference).
    pub on_behalf_of: Option<String>,
    /// The caller is a delegate calling for this person (`p-session-on-behalf-of`, AE-38,
    /// [MS-SIPAE] §3.2.5.4), checked.
    pub acting_for: Option<String>,
}

/// Most early dialogs (forks) tracked per call.
const MAX_EARLY: usize = 8;

/// RSeq of a reliable provisional response (RFC 3262 §7.1: 1 … 2^31 − 1), or `None` when the
/// response is not sent reliably (no `Require: 100rel`).
fn reliable_rseq(resp: &Message) -> Option<u32> {
    let reliable = resp
        .header_values("Require")
        .flat_map(|v| v.split(','))
        .any(|t| t.trim().eq_ignore_ascii_case("100rel"));
    let rseq: u32 = resp.single("RSeq").ok().flatten()?.trim().parse().ok()?;
    (reliable && (1..=0x7fff_ffff).contains(&rseq)).then_some(rseq)
}

fn to_tag(m: &Message) -> Option<String> {
    m.to()
        .ok()
        .flatten()
        .and_then(|t| t.param("tag").map(str::to_string))
}

/// The audio SDP of a message: plain `application/sdp`, or the last `application/sdp` part of a
/// `multipart/alternative` / `multipart/mixed` body (MS-SIPRE §3.15.5.1: ICE-19 is last). Only
/// SDPs with an `m=audio` line count.
pub(crate) fn audio_sdp(m: &Message) -> Option<Vec<u8>> {
    audio_sdp_part(m).map(|(b, _)| b)
}

/// Like [`audio_sdp`], with the part's Content-ID (PS-09) if it is a safe header value.
pub(crate) fn audio_sdp_part(m: &Message) -> Option<(Vec<u8>, Option<String>)> {
    // The header value is the Content-ID in angle brackets (MS-OCPSTN §2.2.7 example; RFC 2045
    // §7); the ID itself must be printable without brackets or white space.
    let safe = |id: &Option<String>| {
        id.as_ref()
            .map(|s| {
                s.trim()
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string()
            })
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 256
                    && s.chars()
                        .all(|c| c.is_ascii_graphic() && !"<>\"".contains(c))
            })
            .map(|s| format!("<{s}>"))
    };
    let ct = m.single("Content-Type").ok().flatten()?.to_string();
    let lower = ct.to_ascii_lowercase();
    let has_audio = |b: &[u8]| {
        sdp::parse(b)
            .ok()
            .is_some_and(|s| s.media.iter().any(|x| x.kind == "audio"))
    };
    if lower.starts_with("application/sdp") {
        return has_audio(&m.body).then(|| (m.body.clone(), None));
    }
    if !lower.starts_with("multipart/") {
        return None;
    }
    let body = std::str::from_utf8(&m.body).ok()?;
    let parts = pres::split_multipart(body, &pres::boundary(&ct)?).ok()?;
    let mut found = None;
    for p in parts.iter().take(8) {
        let pct = p.content_type.to_ascii_lowercase();
        if pct.starts_with("application/sdp") && has_audio(p.body.as_bytes()) {
            found = Some((p.body.as_bytes().to_vec(), safe(&p.content_id)));
        } else if pct.starts_with("multipart/alternative") {
            // 3-level structure: multipart/mixed ⊃ multipart/alternative ⊃ SDPs.
            let inner = pres::boundary(&p.content_type)
                .and_then(|b| pres::split_multipart(&p.body, &b).ok())
                .unwrap_or_default();
            for q in inner.iter().take(8) {
                if q.content_type
                    .to_ascii_lowercase()
                    .starts_with("application/sdp")
                    && has_audio(q.body.as_bytes())
                {
                    found = Some((q.body.as_bytes().to_vec(), safe(&q.content_id)));
                }
            }
        }
    }
    found
}

impl<R: Random> Engine<R> {
    pub(crate) fn call_event(out: &mut Vec<Output>, e: Event) {
        out.push(Output::Event(e));
    }

    /// `Call`: INVITE with our offer as a 2-level multipart/alternative whose last (and here
    /// only) part is the ICE-19 SDP (MS-SIPRE §3.15.4.1).
    pub(crate) fn start_call(
        &mut self,
        to: &str,
        local: sdp::Local,
        policy: sdp::Encryption,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        self.start_call_with(to, local, policy, Extras::default(), now, out);
    }

    /// `start_call` with extra headers (transfer: Referred-By, Replaces, Ms-Sensitivity); returns
    /// the new call's ID.
    pub(crate) fn start_call_with(
        &mut self,
        to: &str,
        local: sdp::Local,
        policy: sdp::Encryption,
        extras: Extras<'_>,
        now: u64,
        out: &mut Vec<Output>,
    ) -> Option<CallId> {
        let (extra, part) = (extras.headers, extras.part);
        let limit = if extras.emergency {
            MAX_CALLS + 1
        } else {
            MAX_CALLS
        };
        let Ok(peer) = pres::checked_sip_uri(to) else {
            out.push(Output::Event(Event::Notice(
                "not a usable SIP address".into(),
            )));
            return None;
        };
        if self.calls.len() >= limit {
            out.push(Output::Event(Event::Notice("too many calls".into())));
            return None;
        }
        if policy != sdp::Encryption::Off && local.key.is_none() {
            out.push(Output::Event(Event::Notice(
                "no media key for an encrypted call".into(),
            )));
            return None;
        }
        let offer = match extras.video {
            Some(v) => match sdp::video::offer_with_video(&local, v, policy) {
                Ok(o) => o,
                Err(_) => {
                    out.push(Output::Event(Event::Notice("unusable video stream".into())));
                    return None;
                }
            },
            None => sdp::offer(&local, policy),
        };
        let mut m = self.request("INVITE", &peer, &peer);
        // SD-51: 100rel is supported (never required in a request); PS-06: gateways may send
        // the answer in an unreliable 183.
        m.push_header("Supported", "100rel");
        m.push_header("Supported", "ms-early-media");
        // RE-64: we take Ms-Sensitivity from a Refer-To into the triggered INVITE.
        m.push_header("Supported", "ms-safe-transfer");
        for (k, v) in extra {
            if !v.chars().any(|c| c.is_control()) {
                m.push_header(k, v);
            }
        }
        // PS-09: every SDP part of an offer has a Content-ID.
        let alternative = format!(
            "--{BOUNDARY}\r\nContent-Type: application/sdp\r\nContent-Transfer-Encoding: 7bit\r\nContent-ID: <audio-ice19@commscraft>\r\nContent-Disposition: session; handling=optional\r\n\r\n{offer}\r\n--{BOUNDARY}--\r\n"
        );
        match part {
            // RE-67 (MS-SIPRE §2.2.23, §4.14): multipart/mixed with the offer and one more part
            // (the PIDF-LO of an emergency call; the server takes the last PIDF-LO part).
            Some(p) => {
                m.push_header(
                    "Content-Type",
                    &format!("multipart/mixed;boundary=\"{MIXED_BOUNDARY}\""),
                );
                m.body = format!(
                    "--{MIXED_BOUNDARY}\r\nContent-Type: multipart/alternative;boundary=\"{BOUNDARY}\"\r\n\r\n{alternative}\r\n--{MIXED_BOUNDARY}\r\nContent-Type: {}\r\nContent-ID: <{}>\r\n\r\n{}\r\n--{MIXED_BOUNDARY}--\r\n",
                    p.content_type, p.content_id, p.body
                )
                .into_bytes();
            }
            None => {
                m.push_header(
                    "Content-Type",
                    &format!("multipart/alternative;boundary=\"{BOUNDARY}\""),
                );
                m.body = alternative.into_bytes();
            }
        }
        let id = self.next_call;
        self.next_call = self.next_call.wrapping_add(1).max(1);
        self.calls.insert(
            id,
            Call {
                peer: peer.clone(),
                state: CallState::Inviting { cancelling: false },
                invite: m.clone(),
                dialog: None,
                local: Some(local),
                policy,
                offer: Vec::new(),
                ringing_reported: false,
                early: Vec::new(),
                early_media: None,
                forked: false,
                content_id: None,
                transfer_out: None,
                transfer_in: None,
                transferred_from: None,
                replaces: None,
                video: extras.video.cloned(),
            },
        );
        self.send_request(m, Purpose::CallInvite(id), now, out);
        // The callee may ring for minutes (RFC 3261 §13.3.1.1, timer C ≥ 3 min).
        if let Some(p) = self
            .pending
            .values_mut()
            .find(|p| p.purpose == Purpose::CallInvite(id))
        {
            p.deadline = now + 180;
            // The INVITE as signed (for CANCEL with the same Via).
            if let Some(c) = self.calls.get_mut(&id) {
                c.invite = p.request.clone();
            }
        }
        Some(id)
    }

    /// A dial string as SIP URI (PS-01): visual separators removed; `+` and 1–15 digits is a
    /// global number (`sip:+…@domain;user=phone`); anything else is a local number with the
    /// location profile – or `dialstring` without one – as phone-context, which the server
    /// normalises with its dial plan (no client-side rules needed). `#` is escaped (RFC 3261
    /// §19.1.2). `None` for anything that is not a dial string.
    pub fn phone_uri(&self, dial: &str) -> Option<String> {
        let d = dial.trim();
        let d = d.strip_prefix("tel:").unwrap_or(d);
        let cleaned: String = d.chars().filter(|c| !" -.()/".contains(*c)).collect();
        let (plus, digits) = match cleaned.strip_prefix('+') {
            Some(r) => (true, r),
            None => (false, cleaned.as_str()),
        };
        if digits.is_empty()
            || digits.len() > 32
            || !digits
                .chars()
                .all(|c| c.is_ascii_digit() || c == '*' || c == '#')
        {
            return None;
        }
        let domain = self.user_domain();
        if plus && digits.len() <= 15 && digits.chars().all(|c| c.is_ascii_digit()) {
            return Some(format!("sip:+{digits}@{domain};user=phone"));
        }
        if plus {
            return None;
        }
        let context = self.location_profile.as_deref().unwrap_or("dialstring");
        let number = digits.replace('#', "%23");
        Some(format!(
            "sip:{number};phone-context={context}@{domain};user=phone"
        ))
    }

    /// Who is calling (PS-02, PS-04, PS-07): the asserted identity of a ringing call if the server
    /// gave one, its phone number, whether the caller is anonymous and whether a gateway calls.
    pub fn caller(&self, id: CallId) -> Option<Caller> {
        let c = self.calls.get(&id)?;
        let asserted = c
            .invite
            .header_values("P-Asserted-Identity")
            .flat_map(|v| v.split(','))
            .filter_map(|v| {
                let v = v.trim();
                let inner = v
                    .split_once('<')
                    .and_then(|(_, r)| r.split_once('>'))
                    .map(|(u, _)| u)
                    .unwrap_or(v);
                let inner = inner.trim();
                if inner.starts_with("tel:") {
                    let n = inner.trim_start_matches("tel:").split(';').next()?;
                    (n.len() <= 32 && n.chars().all(|c| c.is_ascii_digit() || "+*#".contains(c)))
                        .then(|| format!("tel:{n}"))
                } else {
                    pres::checked_sip_uri(inner).ok()
                }
            })
            .take(2)
            .collect::<Vec<_>>();
        let sip = asserted.iter().find(|u| u.starts_with("sip:")).cloned();
        let number = asserted
            .iter()
            .find_map(|u| u.strip_prefix("tel:").map(str::to_string))
            .or_else(|| {
                let u = sip.as_deref().unwrap_or(&c.peer);
                u.contains(";user=phone")
                    .then(|| {
                        u.trim_start_matches("sip:")
                            .split(['@', ';'])
                            .next()
                            .unwrap_or("")
                            .to_string()
                    })
                    .filter(|n| !n.is_empty())
            });
        let from = c.peer.to_ascii_lowercase();
        let anonymous = from.starts_with("sip:anonymous@") || from.contains("anonymous.invalid");
        let gateway = c
            .invite
            .header_values("Contact")
            .any(|v| v.to_ascii_lowercase().contains(";isgateway"));
        // Issue #67: only a call that was addressed to that person and retargeted to us (team,
        // delegate, forwarding: To = Referred-By ≠ us). A transferred call also carries
        // Referred-By (the transferor) but is addressed to us – it is nobody's call but ours.
        let addressed_to = c
            .invite
            .single("To")
            .ok()
            .flatten()
            .and_then(|v| commscraft_sip_msg::parse_name_addr(v).ok())
            .map(|n| n.uri);
        let on_behalf_of = c
            .invite
            .single("Referred-By")
            .ok()
            .flatten()
            .and_then(|v| commscraft_sip_msg::parse_name_addr(v).ok())
            .and_then(|n| pres::checked_sip_uri(&n.uri).ok())
            .filter(|u| !u.eq_ignore_ascii_case(&self.cfg.aor))
            .filter(|u| {
                addressed_to
                    .as_deref()
                    .is_some_and(|t| t.eq_ignore_ascii_case(u))
            });
        let acting_for = c
            .invite
            .single("P-Session-On-Behalf-Of")
            .ok()
            .flatten()
            .and_then(|v| commscraft_sip_msg::parse_name_addr(v).ok())
            .and_then(|n| pres::checked_sip_uri(&n.uri).ok());
        Some(Caller {
            uri: sip.unwrap_or_else(|| c.peer.clone()),
            number,
            anonymous,
            gateway,
            on_behalf_of,
            acting_for,
        })
    }

    /// The address to call back (PS-02): the asserted SIP identity as it is; an asserted `tel:`
    /// number as phone URI; otherwise the From URI. Anonymous callers cannot be called back.
    pub fn callback_uri(&self, id: CallId) -> Option<String> {
        let who = self.caller(id)?;
        if who.anonymous {
            return None;
        }
        let c = self.calls.get(&id)?;
        if who.uri != c.peer {
            return Some(who.uri);
        }
        match who.number {
            Some(n) if !c.peer.contains(";user=phone") => self.phone_uri(&n).or(Some(who.uri)),
            _ => Some(who.uri),
        }
    }

    /// A provisional response to our INVITE: ringing (reported once), PRACK for reliable ones
    /// (RFC 3262), early media from an SDP answer (MS-SDPEXT §3.1.5.12), 199 ends a fork's early
    /// media.
    pub(crate) fn on_call_provisional(
        &mut self,
        purpose: &Purpose,
        request: &Message,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Purpose::CallInvite(id) = *purpose else {
            return;
        };
        let Some(code) = resp.status_code() else {
            return;
        };
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        if !matches!(c.state, CallState::Inviting { .. }) {
            return;
        }
        if (180..=183).contains(&code) && !c.ringing_reported {
            c.ringing_reported = true;
            Self::call_event(out, Event::CallRinging { call: id });
        }
        if resp.single("Ms-Forking").ok().flatten().is_some() {
            c.forked = true;
        }
        let Some(tag) = to_tag(resp) else {
            return;
        };
        // The early dialog of this fork.
        let pos = match c.early.iter().position(|(d, _)| d.remote_tag == tag) {
            Some(i) => i,
            None => {
                let Ok(d) = Dialog::early_from_response(request, resp) else {
                    return;
                };
                if c.early.len() >= MAX_EARLY {
                    return;
                }
                c.early.push((d, None));
                c.early.len() - 1
            }
        };
        // RFC 3262 §4: a reliable response is acknowledged once, in RSeq order; a retransmission
        // or one out of order is discarded.
        if let Some(rseq) = reliable_rseq(resp) {
            let Some((d, last)) = c.early.get_mut(pos) else {
                return;
            };
            match *last {
                Some(l) if rseq != l.wrapping_add(1) => return,
                _ => *last = Some(rseq),
            }
            let invite_cseq = request.cseq().ok().flatten().map_or(1, |x| x.number);
            // §7.2: PRACK in the early dialog, no SDP (SD-51).
            let mut prack = d.request("PRACK");
            prack.push_header("RAck", &format!("{rseq} {invite_cseq} INVITE"));
            self.complete(&mut prack);
            self.send_request(prack, Purpose::Ignore, now, out);
        }
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        if code == 199 {
            if c.early_media.as_deref() == Some(tag.as_str()) {
                c.early_media = None;
                Self::call_event(out, Event::CallEarlyMediaEnded { call: id });
            }
            return;
        }
        // SD-50: one early answer per call (later or other answers are ignored), only with
        // ICE, not for a forked call; an answer we cannot use is ignored – the 200 decides.
        if c.early_media.is_some()
            || c.forked
            || c.state != (CallState::Inviting { cancelling: false })
        {
            return;
        }
        let Some(answer) = audio_sdp(resp) else {
            return;
        };
        let key = c.local.as_ref().and_then(|l| l.key.clone());
        if let Ok(n) = sdp::answer_to_offer(&answer, key.as_ref(), c.policy) {
            if n.remote_ice.is_some() {
                c.early_media = Some(tag);
                Self::call_event(
                    out,
                    Event::CallEarlyMedia {
                        call: id,
                        media: Box::new(n),
                    },
                );
            }
        }
    }

    /// 2xx to our INVITE: dialog, ACK, answer evaluated with our key and policy.
    pub(crate) fn on_call_answered(
        &mut self,
        id: CallId,
        request: &Message,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Ok(mut d) = Dialog::from_response(request, resp) else {
            self.calls.remove(&id);
            Self::call_event(
                out,
                Event::CallEnded {
                    call: id,
                    status: 500,
                },
            );
            return;
        };
        let cseq = request.cseq().ok().flatten().map_or(1, |x| x.number);
        // CSeq keeps rising across the PRACKs of the early dialog (RFC 3261 §12.2.1.1).
        if let Some((e, _)) = self
            .calls
            .get(&id)
            .and_then(|c| c.early.iter().find(|(e, _)| e.remote_tag == d.remote_tag))
        {
            d.local_cseq = d.local_cseq.max(e.local_cseq);
        }
        let mut ack = im::ack_for_2xx(&d, cseq);
        self.complete(&mut ack);
        self.send_plain(ack, now, out);
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        if c.state == (CallState::Inviting { cancelling: true }) {
            // The answer crossed our CANCEL: end with BYE.
            let mut d2 = d;
            let mut bye = d2.request("BYE");
            self.complete(&mut bye);
            self.send_request(bye, Purpose::Ignore, now, out);
            self.calls.remove(&id);
            Self::call_event(
                out,
                Event::CallEnded {
                    call: id,
                    status: 487,
                },
            );
            return;
        }
        let key = c.local.as_ref().and_then(|l| l.key.clone());
        let answer = audio_sdp(resp);
        let negotiated = answer
            .as_deref()
            .ok_or(sdp::Error::NoAudio)
            .and_then(|s| sdp::answer_to_offer(s, key.as_ref(), c.policy));
        // Function 28: the video line of the answer, if we offered video.
        let video = match (&c.video, answer.as_deref()) {
            (Some(v), Some(s)) => Some(sdp::video::video_answer_to_offer(
                s,
                v.media.key.as_ref(),
                c.policy,
            )),
            _ => None,
        };
        match negotiated {
            Ok(n) => {
                let transferred = c.transferred_from.is_some();
                let continues_early = c.early_media.as_deref() == Some(d.remote_tag.as_str());
                c.dialog = Some(d);
                c.state = CallState::Active;
                c.early.clear();
                self.keep_for_quality(id, resp, now);
                Self::call_event(
                    out,
                    Event::CallAnswered {
                        call: id,
                        media: Box::new(n),
                        continues_early,
                    },
                );
                self.report_video(id, video, out);
                if transferred {
                    self.transferred_call_result(id, 200, now, out);
                }
            }
            Err(e) => {
                // An answer we cannot use (e.g. plaintext against our policy): hang up.
                let mut d2 = d;
                let mut bye = d2.request("BYE");
                self.complete(&mut bye);
                self.send_request(bye, Purpose::Ignore, now, out);
                self.calls.remove(&id);
                out.push(Output::Event(Event::Notice(format!(
                    "call {id}: unusable answer ({e:?})"
                ))));
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 488,
                    },
                );
            }
        }
    }

    /// The video outcome of an answer: `CallVideo`, or a notice when the peer took no video
    /// (the call goes on with audio).
    fn report_video(
        &mut self,
        id: CallId,
        video: Option<Result<Option<sdp::video::NegotiatedVideo>, sdp::Error>>,
        out: &mut Vec<Output>,
    ) {
        match video {
            Some(Ok(Some(v))) => Self::call_event(
                out,
                Event::CallVideo {
                    call: id,
                    video: Box::new(v),
                },
            ),
            Some(Ok(None)) => {
                if let Some(c) = self.calls.get_mut(&id) {
                    c.video = None;
                }
                out.push(Output::Event(Event::Notice(format!(
                    "call {id}: the other side takes no video"
                ))));
            }
            Some(Err(e)) => {
                if let Some(c) = self.calls.get_mut(&id) {
                    c.video = None;
                }
                out.push(Output::Event(Event::Notice(format!(
                    "call {id}: unusable video answer ({e:?}), audio only"
                ))));
            }
            None => {}
        }
    }

    /// `SetCallVideo`: a newer description of our video stream (the final offer after ICE);
    /// the key stays (SD-13).
    pub(crate) fn set_call_video(&mut self, id: CallId, mut video: sdp::video::VideoLocal) {
        if let Some(c) = self.calls.get_mut(&id) {
            if let Some(old) = &c.video {
                video.media.key = old.media.key.clone();
                video.ssrc_range = old.ssrc_range;
                c.video = Some(video);
            }
        }
    }

    /// `RequestPictureUpdate`: an INFO with `picture_fast_update` in the call's dialog (PL-50).
    pub(crate) fn request_picture_update(&mut self, id: CallId, now: u64, out: &mut Vec<Output>) {
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        if c.state != CallState::Active {
            return;
        }
        let Some(d) = c.dialog.as_mut() else {
            return;
        };
        let mut m = d.request("INFO");
        self.complete(&mut m);
        m.push_header("Content-Type", media_control::CONTENT_TYPE);
        m.body = media_control::fast_update().into_bytes();
        self.send_request(m, Purpose::Ignore, now, out);
    }

    /// Whether an incoming call offers video we could take (SD-27).
    pub fn incoming_video(&self, id: CallId) -> bool {
        self.calls
            .get(&id)
            .is_some_and(|c| sdp::video::offered_video(&c.offer))
    }

    pub(crate) fn on_call_failed(&mut self, id: CallId, code: u16, out: &mut Vec<Output>) {
        if self.calls.remove(&id).is_some() {
            Self::call_event(
                out,
                Event::CallEnded {
                    call: id,
                    status: code,
                },
            );
        }
    }

    /// `UpdateCall`: re-INVITE with a new local description (final offer with the selected
    /// candidates, IC-07; hold/resume, SD-06). The key of the call is kept (SD-13).
    pub(crate) fn update_call(
        &mut self,
        id: CallId,
        mut local: sdp::Local,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        let (Some(mut d), CallState::Active) = (c.dialog.take(), c.state.clone()) else {
            out.push(Output::Event(Event::Notice(
                "the call is not active".into(),
            )));
            return;
        };
        // SD-13 ([MS-SDPEXT] §3.1.5.2.2, p. 22): a re-INVITE never renegotiates encryption or any
        // parameter of the crypto/cryptoscale lines – the same key and the same lines as the first
        // offer (towards an MCU the client-SSRTP line stays).
        local.key = c.local.as_ref().and_then(|l| l.key.clone());
        local.ssrtp = c.local.as_ref().is_some_and(|l| l.ssrtp);
        let hold = local.direction != sdp::Direction::SendRecv;
        // The video line stays in every re-INVITE, with the same direction (hold) and key; with
        // music on hold it is inactive – a=feature:MoH belongs to the audio line only (SD-07).
        let body = match &c.video {
            Some(v) => {
                let mut v = v.clone();
                v.media.direction = if local.music_on_hold {
                    sdp::Direction::Inactive
                } else {
                    local.direction
                };
                sdp::video::offer_with_video(&local, &v, c.policy)
                    .unwrap_or_else(|_| sdp::offer(&local, c.policy))
            }
            None => sdp::offer(&local, c.policy),
        };
        c.local = Some(local);
        let mut m = d.request("INVITE");
        c.dialog = Some(d);
        self.complete(&mut m);
        if hold {
            // MS-SDPEXT §3.1.5.23.1 (SD-06): the sip.rendering feature tag with value "no" on the
            // Contact, encoded `+sip.rendering="no"` (RFC 4235 §5.2 example; RFC 3840 §9: tags
            // outside the base set carry a "+").
            for h in m
                .headers
                .iter_mut()
                .filter(|h| h.name.eq_ignore_ascii_case("Contact"))
            {
                h.value = format!("{};+sip.rendering=\"no\"", h.value);
            }
        }
        m.push_header("Content-Type", "application/sdp");
        m.body = body.into_bytes();
        self.send_request(m, Purpose::CallReinvite(id), now, out);
    }

    pub(crate) fn on_call_reinvite_answered(
        &mut self,
        id: CallId,
        request: &Message,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(c) = self.calls.get(&id) else { return };
        let Some(d) = c.dialog.clone() else { return };
        let key = c.local.as_ref().and_then(|l| l.key.clone());
        let policy = c.policy;
        let cseq = request.cseq().ok().flatten().map_or(1, |x| x.number);
        let mut ack = im::ack_for_2xx(&d, cseq);
        self.complete(&mut ack);
        self.send_plain(ack, now, out);
        match audio_sdp(resp)
            .ok_or(sdp::Error::NoAudio)
            .and_then(|s| sdp::answer_to_offer(&s, key.as_ref(), policy))
        {
            Ok(n) => Self::call_event(
                out,
                Event::CallUpdated {
                    call: id,
                    media: Box::new(n),
                },
            ),
            Err(e) => out.push(Output::Event(Event::Notice(format!(
                "call {id}: unusable re-INVITE answer ({e:?})"
            )))),
        }
    }

    /// `Hangup`: BYE, CANCEL while inviting, 603 while ringing (RFC 3261 §9.1, §15.1).
    pub(crate) fn hangup(&mut self, id: CallId, now: u64, out: &mut Vec<Output>) {
        let Some(c) = self.calls.get_mut(&id) else {
            return;
        };
        match c.state.clone() {
            CallState::Active => {
                if let Some(mut d) = c.dialog.take() {
                    let mut bye = d.request("BYE");
                    self.complete(&mut bye);
                    self.send_request(bye, Purpose::Ignore, now, out);
                }
                self.calls.remove(&id);
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 0,
                    },
                );
            }
            CallState::Inviting { cancelling: false } => {
                c.state = CallState::Inviting { cancelling: true };
                let cancel = cancel_for(&c.invite);
                // CANCEL has its own transaction; its answer is not tracked (the 487 to the
                // INVITE ends the call).
                self.send_plain(cancel, now, out);
            }
            CallState::Inviting { cancelling: true } => {}
            CallState::Ringing { .. } => {
                let invite = c.invite.clone();
                self.calls.remove(&id);
                self.answer_call_request(&invite, 603, "Decline", now, out);
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 603,
                    },
                );
            }
        }
    }

    fn answer_call_request(
        &mut self,
        invite: &Message,
        code: u16,
        reason: &str,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let r = response_to(invite, code, reason);
        self.send_plain(r, now, out);
    }

    /// An incoming INVITE with an audio offer: 180 Ringing, report it, wait for the user.
    pub(crate) fn on_incoming_call(
        &mut self,
        req: &Message,
        offer: Vec<u8>,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        if self.calls.len() >= MAX_CALLS {
            let r = response_to(req, 486, "Busy Here");
            self.send_plain(r, now, out);
            return;
        }
        // PS-05: a loop call (Ms-Call-Source) while another call rings here is refused.
        if req.single("Ms-Call-Source").ok().flatten().is_some()
            && self
                .calls
                .values()
                .any(|c| matches!(c.state, CallState::Ringing { .. }))
        {
            let r = response_to(req, 605, "Rejected");
            self.send_plain(r, now, out);
            return;
        }
        let content_id = audio_sdp_part(req).and_then(|(_, id)| id);
        // RFC 3891 §3: Replaces must name one of our established calls, else 481.
        let Ok(replaces) = self.replaced_call(req) else {
            let r = response_to(req, 481, "Call/Transaction Does Not Exist");
            self.send_plain(r, now, out);
            return;
        };
        let tag = self.hex(5);
        let Ok(dialog) = Dialog::from_request(req, &tag) else {
            let r = response_to(req, 400, "Bad Request");
            self.send_plain(r, now, out);
            return;
        };
        let mut invite = req.clone();
        // Our To tag goes into every response of this transaction.
        for h in invite
            .headers
            .iter_mut()
            .filter(|h| h.name.eq_ignore_ascii_case("To"))
        {
            h.value = format!("{};tag={tag}", h.value);
        }
        let mut ringing = response_to(&invite, 180, "Ringing");
        ringing.push_header("Contact", &format!("<{}>", self.gruu));
        self.send_plain(ringing, now, out);
        let id = self.next_call;
        self.next_call = self.next_call.wrapping_add(1).max(1);
        let from = dialog.remote_uri.clone();
        self.calls.insert(
            id,
            Call {
                peer: from.clone(),
                state: CallState::Ringing {
                    deadline: now + RING_TIMEOUT,
                },
                invite,
                dialog: Some(dialog),
                local: None,
                policy: sdp::Encryption::Required,
                offer,
                ringing_reported: true,
                early: Vec::new(),
                early_media: None,
                forked: false,
                content_id,
                transfer_out: None,
                transfer_in: None,
                transferred_from: None,
                replaces,
                video: None,
            },
        );
        Self::call_event(out, Event::IncomingCall { call: id, from });
    }

    /// `AcceptCall`: answer the offer with our parameters (200 OK with SDP).
    pub(crate) fn accept_call(
        &mut self,
        id: CallId,
        local: sdp::Local,
        policy: sdp::Encryption,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        self.accept_call_with_video(id, local, None, policy, now, out);
    }

    /// `AcceptCall` / `AcceptVideoCall`: with `video`, an offered H.264 UC line is taken.
    pub(crate) fn accept_call_with_video(
        &mut self,
        id: CallId,
        local: sdp::Local,
        video: Option<sdp::video::VideoLocal>,
        policy: sdp::Encryption,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        // AE-38: a call meant for someone else (Referred-By, RE-72) is answered on their behalf.
        let behalf = self.caller(id).and_then(|c| c.on_behalf_of);
        let Some(c) = self.calls.get(&id) else { return };
        if !matches!(c.state, CallState::Ringing { .. }) {
            return;
        }
        let invite = c.invite.clone();
        let fresh = local
            .key
            .clone()
            .unwrap_or_else(|| sdp::Key::new(0, [0; 30]));
        let video_key = video
            .as_ref()
            .and_then(|v| v.media.key.clone())
            .unwrap_or_else(|| sdp::Key::new(0, [0; 30]));
        let answered = if policy != sdp::Encryption::Off && local.key.is_none() {
            Err(sdp::Error::Crypto("no media key"))
        } else {
            sdp::video::answer_with_video(
                &c.offer,
                &local,
                fresh,
                video.as_ref().map(|v| (v, video_key)),
                policy,
            )
        };
        match answered {
            Ok((body, n, nv)) => {
                let mut ok = response_to(&invite, 200, "OK");
                for rr in invite.header_values("Record-Route") {
                    ok.push_header("Record-Route", rr);
                }
                ok.push_header("Contact", &format!("<{}>", self.gruu));
                ok.push_header("Supported", "ms-safe-transfer");
                if let Some(b) = &behalf {
                    ok.push_header("P-Session-On-Behalf-Of", &format!("<{b}>"));
                }
                // PS-09: name the offer part we answer.
                if let Some(cid) = c.content_id.clone() {
                    ok.push_header("ms-accepted-content-id", &cid);
                }
                ok.push_header("Content-Type", "application/sdp");
                ok.body = body.into_bytes();
                self.send_plain(ok, now, out);
                let mut kept = local;
                // Keep the key under the tag the peer offered (it was answered with it).
                if let (Some(k), Some(tag)) =
                    (kept.key.as_mut(), n.remote_key.as_ref().map(|r| r.tag))
                {
                    k.tag = tag;
                }
                // Our video key under the tag the peer offered for video.
                let kept_video = match (video, &nv) {
                    (Some(mut v), Some(n)) => {
                        if let (Some(k), Some(tag)) =
                            (v.media.key.as_mut(), n.remote_key.as_ref().map(|r| r.tag))
                        {
                            k.tag = tag;
                        }
                        Some(v)
                    }
                    _ => None,
                };
                if let Some(c) = self.calls.get_mut(&id) {
                    c.state = CallState::Active;
                    c.local = Some(kept);
                    c.policy = policy;
                    c.video = kept_video;
                }
                if let Some(invite) = self.calls.get(&id).map(|c| c.invite.clone()) {
                    self.keep_for_quality(id, &invite, now);
                }
                // RFC 3891 §3: the replaced call ends once the new one is accepted.
                if let Some(old) = self.calls.get(&id).and_then(|c| c.replaces) {
                    self.hangup(old, now, out);
                }
                Self::call_event(
                    out,
                    Event::CallAnswered {
                        call: id,
                        media: Box::new(n),
                        continues_early: false,
                    },
                );
                if let Some(v) = nv {
                    Self::call_event(
                        out,
                        Event::CallVideo {
                            call: id,
                            video: Box::new(v),
                        },
                    );
                }
            }
            Err(e) => {
                self.calls.remove(&id);
                self.answer_call_request(&invite, 488, "Not Acceptable Here", now, out);
                out.push(Output::Event(Event::Notice(format!(
                    "call {id}: offer not acceptable ({e:?})"
                ))));
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 488,
                    },
                );
            }
        }
    }

    /// Requests of the peer in a call dialog: BYE, re-INVITE (final offer, hold), ACK.
    pub(crate) fn on_call_request(
        &mut self,
        id: CallId,
        req: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let method = req.method().unwrap_or("").to_ascii_uppercase();
        match method.as_str() {
            "ACK" => {}
            "BYE" => {
                let r = response_to(req, 200, "OK");
                self.send_plain(r, now, out);
                self.calls.remove(&id);
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 0,
                    },
                );
            }
            "INVITE" => {
                let Some(c) = self.calls.get(&id) else { return };
                let (Some(local), Some(offer)) = (c.local.clone(), audio_sdp(req)) else {
                    let r = response_to(req, 488, "Not Acceptable Here");
                    self.send_plain(r, now, out);
                    return;
                };
                let key = local
                    .key
                    .clone()
                    .unwrap_or_else(|| sdp::Key::new(0, [0; 30]));
                let video = c.video.as_ref().map(|v| {
                    (
                        v,
                        v.media
                            .key
                            .clone()
                            .unwrap_or_else(|| sdp::Key::new(0, [0; 30])),
                    )
                });
                match sdp::video::answer_with_video(&offer, &local, key, video, c.policy) {
                    // SD-13: the peer must keep its key as well.
                    Ok((body, n, nv))
                        if c.policy == sdp::Encryption::Off || n.remote_key.is_some() =>
                    {
                        let mut ok = response_to(req, 200, "OK");
                        ok.push_header("Contact", &format!("<{}>", self.gruu));
                        ok.push_header("Content-Type", "application/sdp");
                        ok.body = body.into_bytes();
                        self.send_plain(ok, now, out);
                        Self::call_event(
                            out,
                            Event::CallUpdated {
                                call: id,
                                media: Box::new(n),
                            },
                        );
                        if let Some(v) = nv {
                            Self::call_event(
                                out,
                                Event::CallVideo {
                                    call: id,
                                    video: Box::new(v),
                                },
                            );
                        }
                    }
                    _ => {
                        let r = response_to(req, 488, "Not Acceptable Here");
                        self.send_plain(r, now, out);
                    }
                }
            }
            "REFER" => self.on_refer(id, req, now, out),
            "NOTIFY"
                if req
                    .single("Event")
                    .ok()
                    .flatten()
                    .is_some_and(|e| e.trim().eq_ignore_ascii_case("refer")) =>
            {
                self.on_refer_notify(id, req, now, out)
            }
            "INFO" => {
                // PL-50: video control commands go to the application; the INFO itself is always
                // answered with 200 OK (XMLSMC §6, XMLMC §3.1.5.2).
                let is_control = req
                    .single("Content-Type")
                    .ok()
                    .flatten()
                    .is_some_and(media_control::is_media_control);
                if is_control {
                    let commands = std::str::from_utf8(&req.body)
                        .ok()
                        .and_then(|b| media_control::parse(b).ok())
                        .unwrap_or_default();
                    for control in commands {
                        Self::call_event(out, Event::CallVideoControl { call: id, control });
                    }
                }
                let r = response_to(req, 200, "OK");
                self.send_plain(r, now, out);
            }
            "UPDATE" | "OPTIONS" => {
                let r = response_to(req, 200, "OK");
                self.send_plain(r, now, out);
            }
            _ => {
                let r = response_to(req, 405, "Method Not Allowed");
                self.send_plain(r, now, out);
            }
        }
    }

    /// CANCEL of a ringing incoming call (RFC 3261 §9.2): 200 to the CANCEL, 487 to the INVITE.
    pub(crate) fn on_cancel(&mut self, req: &Message, now: u64, out: &mut Vec<Output>) {
        let call_id = req.call_id().ok().flatten().map(str::to_string);
        let from_tag = req
            .from()
            .ok()
            .flatten()
            .and_then(|f| f.param("tag").map(str::to_string));
        let found = self.calls.iter().find(|(_, c)| {
            matches!(c.state, CallState::Ringing { .. })
                && c.invite.call_id().ok().flatten().map(str::to_string) == call_id
                && c.invite
                    .from()
                    .ok()
                    .flatten()
                    .and_then(|f| f.param("tag").map(str::to_string))
                    == from_tag
        });
        let Some((&id, c)) = found else {
            let r = response_to(req, 481, "Call/Transaction Does Not Exist");
            self.send_plain(r, now, out);
            return;
        };
        let invite = c.invite.clone();
        let ok = response_to(req, 200, "OK");
        self.send_plain(ok, now, out);
        self.calls.remove(&id);
        self.answer_call_request(&invite, 487, "Request Terminated", now, out);
        Self::call_event(
            out,
            Event::CallEnded {
                call: id,
                status: 487,
            },
        );
    }

    /// Ringing timeout of incoming calls.
    pub(crate) fn call_tick(&mut self, now: u64, out: &mut Vec<Output>) {
        let expired: Vec<CallId> = self
            .calls
            .iter()
            .filter(|(_, c)| matches!(c.state, CallState::Ringing { deadline } if now >= deadline))
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            if let Some(c) = self.calls.remove(&id) {
                self.answer_call_request(&c.invite, 480, "Temporarily Unavailable", now, out);
                Self::call_event(
                    out,
                    Event::CallEnded {
                        call: id,
                        status: 480,
                    },
                );
            }
        }
    }

    pub(crate) fn call_of(&self, req: &Message) -> Option<CallId> {
        self.calls
            .iter()
            .find(|(_, c)| {
                matches!(c.state, CallState::Active)
                    && c.dialog.as_ref().is_some_and(|d| d.is_peer_request(req))
            })
            .map(|(id, _)| *id)
    }

    /// `Hold` / `Resume` (MS-SDPEXT §3.1.5.23): a re-INVITE with the current description and the
    /// direction inactive (hold, sip.rendering="no") or sendrecv (resume); holding with music on
    /// hold offers sendonly with `a=feature:MoH` (§3.1.5.27.2, SD-07).
    pub(crate) fn hold_call(
        &mut self,
        id: CallId,
        hold: bool,
        music: bool,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(mut local) = self.calls.get(&id).and_then(|c| c.local.clone()) else {
            out.push(Output::Event(Event::Notice("no such call".into())));
            return;
        };
        local.direction = match (hold, music) {
            (true, true) => sdp::Direction::SendOnly,
            (true, false) => sdp::Direction::Inactive,
            (false, _) => sdp::Direction::SendRecv,
        };
        local.music_on_hold = hold && music;
        self.update_call(id, local, now, out);
    }

    /// The ID the next `Command::Call` will get (to map the media of the adapter).
    pub fn next_call_id(&self) -> CallId {
        self.next_call
    }

    /// The peer of a call (for the user interface).
    /// Call-ID and offer (audio SDP) of an incoming call that is still ringing.
    pub fn incoming_offer(&self, id: CallId) -> Option<(&str, &[u8])> {
        let c = self.calls.get(&id)?;
        if !matches!(c.state, CallState::Ringing { .. }) {
            return None;
        }
        Some((c.invite.call_id().ok().flatten()?, c.offer.as_slice()))
    }

    /// Our own address of record.
    pub fn own_aor(&self) -> &str {
        &self.cfg.aor
    }

    pub fn call_peer(&self, id: CallId) -> Option<&str> {
        self.calls.get(&id).map(|c| c.peer.as_str())
    }

    /// Ends all calls (sign-out).
    pub(crate) fn end_all_calls(&mut self, now: u64, out: &mut Vec<Output>) {
        let ids: Vec<CallId> = self.calls.keys().copied().collect();
        for id in ids {
            self.hangup(id, now, out);
        }
    }
}

/// CANCEL for an INVITE (RFC 3261 §9.1): same Request-URI, Call-ID, To, From, CSeq number, top
/// Via and Route set.
fn cancel_for(invite: &Message) -> Message {
    let uri = match &invite.start {
        StartLine::Request { uri, .. } => uri.clone(),
        StartLine::Response { .. } => String::new(),
    };
    let mut m = Message::request("CANCEL", &uri);
    if let Some(v) = invite.header_values("Via").next() {
        m.push_header("Via", v);
    }
    for r in invite.header_values("Route") {
        m.push_header("Route", r);
    }
    m.push_header("Max-Forwards", "70");
    for name in ["From", "To", "Call-ID"] {
        if let Some(v) = invite.header_values(name).next() {
            m.push_header(name, v);
        }
    }
    let n = invite.cseq().ok().flatten().map_or(1, |c| c.number);
    m.push_header("CSeq", &format!("{n} CANCEL"));
    m
}
