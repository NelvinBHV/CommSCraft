//! Viewing a shared screen (function 43): the sharer's offer for desktop or application sharing
//! carries an `m=applicationsharing` line (RDP over RTP, SD-80/SD-81) and – video-based screen
//! sharing, from the 2015 servers – an `m=video` line labelled `applicationsharing-video`
//! (CS-03, [MS-CONFAS] 20250520 §3.2.5.1.2, pp. 15–16): H.264 UC as in video calls, `sendonly`
//! from the sharer.
//!
//! The viewer takes the video line (`recvonly`, label `applicationsharing-video`, bandwidth
//! declared with `a=x-mediabw:applicationsharing-video`, SD-61) and – until this client has an
//! RDP viewer – rejects the RDP line with port 0 ([RFC3264] §6); every other line is rejected as
//! well, in the offer's order.

use super::video::{answer_labelled_video_line, NegotiatedVideo, VideoLocal};
use super::*;

/// The label of the video stream of video-based screen sharing (CS-03).
pub const SCREEN_VIDEO_LABEL: &str = "applicationsharing-video";
/// Our receive bandwidth for the shared screen in kbit/s, declared at session level (SD-61;
/// design: the Windows sharer declared 12000 in its offer, `tested-in-lab`).
pub const SCREEN_MEDIABW_KBPS: u32 = 12_000;

fn screen_line(s: &Session) -> Option<&Media> {
    s.media
        .iter()
        .find(|m| m.kind == "video" && m.port != 0 && m.attr("label") == Some(SCREEN_VIDEO_LABEL))
}

/// Whether an SDP offers video-based screen sharing (a video line with the screen label).
pub fn offered(offer: &[u8]) -> bool {
    parse(offer).ok().is_some_and(|s| screen_line(&s).is_some())
}

/// Whether an SDP offers RDP-based sharing (`m=applicationsharing`, SD-80).
pub fn offered_rdp(offer: &[u8]) -> bool {
    parse(offer).ok().is_some_and(|s| {
        s.media
            .iter()
            .any(|m| m.kind == "applicationsharing" && m.port != 0)
    })
}

/// The ICE candidates of the offer's screen video line.
pub fn offer_ice(offer: &[u8]) -> Option<Ice> {
    let s = parse(offer).ok()?;
    ice_of(screen_line(&s)?)
}

/// Our answer as viewer: the screen video line taken (receive only), everything else rejected.
/// Fails when the offer has no screen video line we can take (H.264 UC, key, SSRC range).
pub fn answer(
    offer: &[u8],
    video: &VideoLocal,
    fresh_key: Key,
    policy: Encryption,
) -> Result<(String, NegotiatedVideo), Error> {
    let s = parse(offer)?;
    let index = s
        .media
        .iter()
        .position(|m| {
            m.kind == "video" && m.port != 0 && m.attr("label") == Some(SCREEN_VIDEO_LABEL)
        })
        .ok_or(Error::Syntax("no screen video"))?;
    let m = s.media.get(index).ok_or(Error::Syntax("no screen video"))?;
    let mut ours = video.clone();
    ours.media.direction = Direction::RecvOnly;
    let (section, negotiated) =
        answer_labelled_video_line(&s, m, &ours, fresh_key, policy, SCREEN_VIDEO_LABEL)
            .ok_or(Error::Syntax("screen video not acceptable"))?;
    let mut out = header(&video.media);
    out.push_str(&format!(
        "a=x-mediabw:{SCREEN_VIDEO_LABEL} send={SCREEN_MEDIABW_KBPS};recv={SCREEN_MEDIABW_KBPS}\r\n"
    ));
    for (i, line) in s.media.iter().enumerate() {
        if i == index {
            out.push_str(&section);
        } else {
            let fmts: Vec<String> = line.formats.iter().map(u8::to_string).collect();
            let fmts = if fmts.is_empty() {
                "0".to_string()
            } else {
                fmts.join(" ")
            };
            out.push_str(&format!("m={} 0 {} {fmts}\r\n", line.kind, line.proto));
        }
    }
    Ok((out, negotiated))
}

/// Our offer as viewer to the application sharing MCU of a meeting (CS-02, CS-03; [MS-CONFAS]
/// §3.2.5.1.2, pp. 15–16): the screen video line receive-only with the screen label and, as
/// the MCU expects an application sharing line, `m=applicationsharing` in the viewer role,
/// disabled with port 0 (no RDP viewer yet; inference – whether the MCU takes it is shown by
/// `tested-in-lab`).
pub fn viewer_offer(video: &VideoLocal, policy: Encryption) -> Result<String, Error> {
    let proto = match policy {
        Encryption::Required => "RTP/SAVP",
        _ => "RTP/AVP",
    };
    let mut v = video.clone();
    v.media.direction = Direction::RecvOnly;
    if policy == Encryption::Off {
        v.media.key = None;
    }
    let mut out = header(&v.media);
    out.push_str(&format!(
        "a=x-mediabw:{SCREEN_VIDEO_LABEL} send={SCREEN_MEDIABW_KBPS};recv={SCREEN_MEDIABW_KBPS}\r\n"
    ));
    out.push_str("m=applicationsharing 0 TCP/RTP/AVP 127\r\na=x-applicationsharing-role:viewer\r\na=x-applicationsharing-media-type:rdp\r\na=x-applicationsharing-contentflow:inactive\r\na=rtpmap:127 x-data/90000\r\n");
    out.push_str(&super::video::labelled_video_section(
        &v,
        proto,
        super::video::H264UC_PT,
        Some(super::video::ULPFEC_PT),
        SCREEN_VIDEO_LABEL,
    )?);
    Ok(out)
}

/// What the viewer needs of a sharer's RDP line (SD-80, SD-81).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedRdp {
    pub remote_key: Option<Key>,
    pub remote_ice: Option<Ice>,
    pub remote_rtp: Endpoint,
    pub session_id: Option<String>,
    /// The ID of the RTP header extension `abs-send-time` both sides use ([RFC8285]): the
    /// Windows clients send every RDP packet with it (`tested-in-lab`).
    pub abs_send_time: Option<u8>,
}

/// The URI of the absolute send time extension as the Windows clients write it (with
/// backslashes, `tested-in-lab`); compared without regard to the slashes' direction.
const ABS_SEND_TIME: &str = "http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time";

/// The offered `a=extmap` of `abs-send-time`: its ID (1 … 14, the one-byte form) and the line.
fn abs_send_time_of(m: &Media) -> Option<(u8, String)> {
    m.attrs("extmap").find_map(|v| {
        let (id, uri) = v.trim().split_once(' ')?;
        let id: u8 = id.trim().parse().ok().filter(|i| (1..=14).contains(i))?;
        let uri = uri.trim();
        (uri.replace('\\', "/").eq_ignore_ascii_case(ABS_SEND_TIME)
            && uri.len() <= 128
            && uri.chars().all(|c| c.is_ascii_graphic()))
        .then(|| (id, format!("a=extmap:{id} {uri}\r\n")))
    })
}

fn rdp_line(s: &Session) -> Option<&Media> {
    s.media.iter().find(|m| {
        m.kind == "applicationsharing"
            && m.port != 0
            && m.attr("x-applicationsharing-role") == Some("sharer")
    })
}

/// Our answer as RDP viewer to a sharer's offer without screen video (window sharing sends only
/// RDP, `tested-in-lab`): `m=applicationsharing` over TCP with our ICE candidates (an active TCP
/// host candidate) and key, role viewer and content flow recvonly (SD-81, CS-02), RTCP on the
/// same port (`a=rtcp-mux`, as offered). No `a=setup`: with the offer's fingerprint it would
/// choose DTLS-SRTP ([RFC5763] §5), we take the SDES key; the TCP connection comes from ICE.
/// As the Windows viewer answers (`tested-in-lab`): `TCP/RTP/SAVP` when encrypted, the offered
/// `a=extmap` lines and `a=x-bwealgorithm:packetpair` taken over. Every other line is rejected
/// with port 0.
pub fn answer_rdp(
    offer: &[u8],
    local: &Local,
    fresh_key: Key,
    policy: Encryption,
) -> Result<(String, NegotiatedRdp), Error> {
    let s = parse(offer)?;
    let index = s
        .media
        .iter()
        .position(|m| {
            m.kind == "applicationsharing"
                && m.port != 0
                && m.attr("x-applicationsharing-role") == Some("sharer")
        })
        .ok_or(Error::Syntax("no RDP line"))?;
    let m = rdp_line(&s).ok_or(Error::Syntax("no RDP line"))?;
    if !m.formats.contains(&127) {
        return Err(Error::Syntax("no x-data payload type"));
    }
    let offered = keys_of(m).into_iter().next();
    let (my_key, remote_key) = match (offered, policy) {
        (Some(k), Encryption::Required | Encryption::Optional { .. }) => {
            let mut mine = fresh_key;
            mine.tag = k.tag;
            if mine.key == k.key {
                return Err(Error::Crypto("peer reused our key"));
            }
            (Some(mine), Some(k))
        }
        (
            None,
            Encryption::Off
            | Encryption::Optional {
                allow_plaintext: true,
            },
        ) => (None, None),
        (Some(_), Encryption::Off) => (None, None),
        _ => return Err(Error::Unencrypted),
    };
    let (rtp, _) = endpoints(&s, m)?;
    let session_id = m
        .attr("x-applicationsharing-session-id")
        .map(str::trim)
        .filter(|v| !v.is_empty() && v.len() <= 32 && v.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(str::to_string);
    let mid = m
        .attr("mid")
        .map(str::trim)
        .filter(|v| !v.is_empty() && v.len() <= 16 && v.chars().all(|c| c.is_ascii_alphanumeric()));
    let proto = if my_key.is_some() {
        "TCP/RTP/SAVP"
    } else {
        m.proto.as_str()
    };
    let mut section = format!("m=applicationsharing {} {proto} 127\r\n", local.rtp_port);
    transport_attributes(local, &mut section);
    if let Some(k) = &my_key {
        section.push_str(&format!("a=crypto:{}\r\n", k.attribute()));
    }
    section.push_str(&format!("a=rtcp:{}\r\n", local.rtp_port));
    if let Some(mid) = mid {
        section.push_str(&format!("a=mid:{mid}\r\n"));
    }
    section.push_str("a=rtpmap:127 x-data/90000\r\na=rtcp-mux\r\n");
    let abs_send_time = abs_send_time_of(m);
    for v in m.attrs("extmap").take(8) {
        let v = v.trim();
        if v.len() <= 160 && v.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
            section.push_str(&format!("a=extmap:{v}\r\n"));
        }
    }
    if m.attr("x-bwealgorithm").map(str::trim) == Some("packetpair") {
        section.push_str("a=x-bwealgorithm:packetpair\r\n");
    }
    if let Some(id) = &session_id {
        section.push_str(&format!("a=x-applicationsharing-session-id:{id}\r\n"));
    }
    section.push_str("a=x-applicationsharing-role:viewer\r\na=x-applicationsharing-media-type:rdp\r\na=x-applicationsharing-contentflow:recvonly\r\n");
    let mut out = header(local);
    for (i, line) in s.media.iter().enumerate() {
        if i == index {
            out.push_str(&section);
        } else {
            let fmts: Vec<String> = line.formats.iter().map(u8::to_string).collect();
            let fmts = if fmts.is_empty() {
                "0".to_string()
            } else {
                fmts.join(" ")
            };
            out.push_str(&format!("m={} 0 {} {fmts}\r\n", line.kind, line.proto));
        }
    }
    Ok((
        out,
        NegotiatedRdp {
            remote_key,
            remote_ice: ice_of(m),
            remote_rtp: rtp,
            session_id,
            abs_send_time: abs_send_time.map(|(id, _)| id),
        },
    ))
}

/// Whether an SDP offers RDP-based sharing from a sharer.
pub fn offered_rdp_sharer(offer: &[u8]) -> bool {
    parse(offer).ok().is_some_and(|s| rdp_line(&s).is_some())
}
