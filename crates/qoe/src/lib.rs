//! Quality of Experience (function 45, M4): the two report documents a client sends to the QoE
//! URI from provisioning in a SIP SERVICE – the `VQReportEvent` after every call with media
//! (`application/vq-rtcpxr+xml`) and the user's call rating (`application/ms-cqf+xml`).
//! Sans-I/O; the SIP side lives in `commscraft-session`.
//!
//! Evidence: `docs/protocol/evidence/26-qoe-quality-reports.md` (QE-01 … QE-06).
//! Sources: [MS-QoE] 20250520 §2.2.1 (VQReportEvent, element order and required items,
//! pp. 13–75), §2.2.2 (CallQualityFeedbackReport, pp. 75–78), §3.2 (401 when From ≠ LocalPAI,
//! p. 79), examples §4.1–§4.2 (pp. 82–85). Only network figures that RTP/RTCP measures are
//! reported; MOS estimates are optional (OB-47) and left out.
//!
//! Security by design: every text is escaped, made printable and cut to the documented length
//! (QE-05; identifiers that must not be cut are refused instead); numbers are written
//! culture-invariant; the rating is checked (1–5) and the feedback text is bounded (4000).

use std::net::SocketAddr;

use commscraft_xml::{escape, xs_datetime};

pub const REPORT_TYPE: &str = "application/vq-rtcpxr+xml";
pub const RATING_TYPE: &str = "application/ms-cqf+xml";
pub const NS_REPORT: &str = "ms-rtcp-metrics";
pub const NS_RATING: &str = "ms-cqf";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    BadValue(&'static str),
}

/// The dialog of the call the report belongs to (DialogInfo, §2.2.1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogInfo {
    pub call_id: String,
    pub from_tag: String,
    pub to_tag: String,
    /// Unix times of the start and end of the media session.
    pub start: u64,
    pub end: u64,
    pub from_uri: String,
    pub to_uri: String,
    /// True if we placed the call.
    pub caller: bool,
    pub local_contact: String,
    pub remote_contact: String,
    pub local_user_agent: String,
    pub remote_user_agent: String,
    /// Our own SIP address: the server compares it with the From header (QE-04).
    pub local_pai: String,
}

/// One direction's network figures (Network, §2.2.1.13).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stream {
    pub ssrc: u32,
    pub packets: u64,
    pub jitter_avg_ms: Option<u32>,
    pub jitter_max_ms: Option<u32>,
    /// Loss rate in parts per million (written as a fraction with six decimals).
    pub loss_avg_ppm: Option<u32>,
    pub loss_max_ppm: Option<u32>,
    pub rtt_avg_ms: Option<u32>,
    pub rtt_max_ms: Option<u32>,
}

/// The audio media line (MediaLine, §2.2.1.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioLine {
    pub encrypted: bool,
    pub offerer: bool,
    /// TCP instead of UDP (the TCP relay).
    pub tcp: bool,
    pub local: SocketAddr,
    pub remote: SocketAddr,
    pub inbound: Option<Stream>,
    pub outbound: Option<Stream>,
    /// Payload description (codec name) and sample rate.
    pub codec: String,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Endpoint Name (≤ 256, must not be cut).
    pub endpoint: String,
    pub dialog: DialogInfo,
    pub audio: AudioLine,
}

/// Printable and cut to `max` characters (QE-05: "strings over the length limit are truncated").
fn text(s: &str, max: usize) -> String {
    escape(
        &s.chars()
            .filter(|c| !c.is_control())
            .take(max)
            .collect::<String>(),
    )
}

/// A value that must not be cut (QE-05: SessionId, Name, CallId ⇒ rejection instead).
fn whole(s: &str, max: usize, what: &'static str) -> Result<String, Error> {
    if s.is_empty() || s.chars().count() > max || s.chars().any(char::is_control) {
        return Err(Error::BadValue(what));
    }
    Ok(escape(s))
}

fn ms(name: &str, v: Option<u32>) -> String {
    v.map(|v| format!("<{name}>{v}</{name}>"))
        .unwrap_or_default()
}

fn rate(name: &str, ppm: Option<u32>) -> String {
    // xs:float between 0 and 1, culture-invariant with six decimals (integer arithmetic).
    ppm.map(|p| {
        let p = p.min(1_000_000);
        format!("<{name}>{}.{:06}</{name}>", p / 1_000_000, p % 1_000_000)
    })
    .unwrap_or_default()
}

fn stream(tag: &str, s: &Stream, codec: &str, sample_rate: u32) -> String {
    // A block only with content (an empty element would break the schema's required items).
    let block = |name: &str, inner: String| {
        if inner.is_empty() {
            String::new()
        } else {
            format!("<{name}>{inner}</{name}>")
        }
    };
    let jitter = block(
        "Jitter",
        ms("InterArrival", s.jitter_avg_ms) + &ms("InterArrivalMax", s.jitter_max_ms),
    );
    let loss = block(
        "PacketLoss",
        rate("LossRate", s.loss_avg_ppm) + &rate("LossRateMax", s.loss_max_ppm),
    );
    let delay = block(
        "Delay",
        ms("RoundTrip", s.rtt_avg_ms) + &ms("RoundTripMax", s.rtt_max_ms),
    );
    let packets = i32::try_from(s.packets).unwrap_or(i32::MAX);
    // Element order of NetworkMetricsType (§2.2.1.13): Jitter, PacketLoss, BurstGapLoss, Delay,
    // Utilization; Payload/Audio is required (§2.2.1.10).
    format!(
        "<{tag} Id=\"{}\"><Network>{jitter}{loss}{delay}<Utilization><Packets>{packets}</Packets></Utilization></Network><Payload><Audio><PayloadDescription>{}</PayloadDescription><SampleRate>{sample_rate}</SampleRate></Audio></Payload></{tag}>",
        s.ssrc,
        text(codec, 64)
    )
}

fn addr(tag: &str, a: &SocketAddr) -> String {
    format!(
        "<{tag}><IPAddr>{}</IPAddr><Port>{}</Port></{tag}>",
        a.ip(),
        a.port()
    )
}

/// The VQReportEvent (QE-02; §2.2.1, example §4.1): one VQSessionReport with Endpoint,
/// DialogInfo (incl. LocalPAI, QE-04) and the audio MediaLine with both streams.
pub fn report(r: &Report) -> Result<String, Error> {
    let d = &r.dialog;
    let call_id = whole(&d.call_id, 256, "CallId")?;
    let from_tag = whole(&d.from_tag, 256, "FromTag")?;
    let to_tag = whole(&d.to_tag, 256, "ToTag")?;
    let endpoint = whole(&r.endpoint, 256, "Name")?;
    let session_id = format!("{call_id};from-tag={from_tag};to-tag={to_tag}");
    if session_id.chars().count() > 755 {
        return Err(Error::BadValue("SessionId"));
    }
    let start = xs_datetime(d.start).ok_or(Error::BadValue("Start"))?;
    let end = xs_datetime(d.end.max(d.start)).ok_or(Error::BadValue("End"))?;
    let a = &r.audio;
    let streams = [
        a.inbound
            .as_ref()
            .map(|s| stream("InboundStream", s, &a.codec, a.sample_rate)),
        a.outbound
            .as_ref()
            .map(|s| stream("OutboundStream", s, &a.codec, a.sample_rate)),
    ]
    .into_iter()
    .flatten()
    .collect::<String>();
    Ok(format!(
        "<?xml version=\"1.0\"?><VQReportEvent xmlns=\"{NS_REPORT}\"><VQSessionReport SessionId=\"{session_id}\"><Endpoint Name=\"{endpoint}\"/><DialogInfo CallId=\"{call_id}\" FromTag=\"{from_tag}\" ToTag=\"{to_tag}\" Start=\"{start}\" End=\"{end}\"><FromURI>{}</FromURI><ToURI>{}</ToURI><Caller>{}</Caller><LocalContactURI>{}</LocalContactURI><RemoteContactURI>{}</RemoteContactURI><LocalUserAgent>{}</LocalUserAgent><RemoteUserAgent>{}</RemoteUserAgent><LocalPAI>{}</LocalPAI></DialogInfo><MediaLine Label=\"main-audio\"><Description>{}<Offerer>{}</Offerer><Transport>{}</Transport>{}{}</Description>{streams}</MediaLine></VQSessionReport></VQReportEvent>",
        text(&d.from_uri, 256),
        text(&d.to_uri, 256),
        d.caller,
        text(&d.local_contact, 256),
        text(&d.remote_contact, 256),
        text(&d.local_user_agent, 256),
        text(&d.remote_user_agent, 256),
        text(&d.local_pai, 256),
        if a.encrypted { "<Security>SRTP</Security>" } else { "" },
        a.offerer,
        if a.tcp { "TCP" } else { "UDP" },
        addr("LocalAddr", &a.local),
        addr("RemoteAddr", &a.remote),
    ))
}

/// The user's call rating (QE-03; §2.2.2, example §4.2): rating 1–5, optional feedback text
/// (≤ 4000 characters) with its language tag.
pub fn rating(
    call_id: &str,
    from_tag: &str,
    to_tag: &str,
    reporting_user: &str,
    stars: u8,
    feedback: Option<&str>,
) -> Result<String, Error> {
    if !(1..=5).contains(&stars) {
        return Err(Error::BadValue("Rating"));
    }
    let call_id = whole(call_id, 256, "CallId")?;
    let from_tag = whole(from_tag, 256, "FromTag")?;
    let to_tag = whole(to_tag, 256, "ToTag")?;
    let feedback = feedback
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(|f| {
            format!(
                "<Feedback LanguageTag=\"en-US\"><Text>{}</Text></Feedback>",
                text(f, 4000)
            )
        })
        .unwrap_or_default();
    Ok(format!(
        "<?xml version=\"1.0\"?><CallQualityFeedbackReport xmlns=\"{NS_RATING}\" CallId=\"{call_id}\" FromTag=\"{from_tag}\" ToTag=\"{to_tag}\"><ReportingUserURI>{}</ReportingUserURI><Rating>{stars}</Rating>{feedback}</CallQualityFeedbackReport>",
        text(reporting_user, 256)
    ))
}

#[cfg(test)]
mod tests;
