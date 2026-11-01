//! The video media description of H.264 UC calls (function 28; [MS-SDPEXT] 20250520 – SD-04,
//! SD-27, SD-60 … SD-66; example §4.1, p. 56): one `m=video` line with payload type 122
//! `X-H264UC/90000`, `a=fmtp:122 packetization-mode=1;mst-mode=NI-TC`, an SSRC range of 100,
//! RTCP feedback (`x-message app send:src,x-pli recv:src,x-pli` with `a=rtcp-rsize`) and
//! `a=label:main-video`. ICE and SRTP are those of the audio line, for their own ports.
//!
//! Answers name every `m=` line of the offer in its order; what we do not take is rejected with
//! port 0 ([RFC3264] §6).

use super::*;

/// Payload type of H.264 UC in the examples (SD-27).
pub const H264UC_PT: u8 = 122;
/// The only accepted format parameters of H.264 UC (SD-27, §3.1.5.19, pp. 23–24).
pub const H264UC_FMTP: &str = "packetization-mode=1;mst-mode=NI-TC";
/// Size of the SSRC range of a video send stream (SD-64, §3.1.5.31.2, p. 46).
pub const VIDEO_SSRC_RANGE: u32 = 100;
/// Largest value of an SSRC range (§3.1.5.31.1, p. 45).
pub const SSRC_RANGE_MAX: u32 = 4_294_967_040;

/// Which RTCP feedback messages ([MS-RTP] §2.2.12) a side sends and receives (SD-63).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Feedback {
    /// Video source requests.
    pub send_src: bool,
    pub recv_src: bool,
    /// Picture loss indications.
    pub send_pli: bool,
    pub recv_pli: bool,
}

/// Our video stream: its transport (ports, ICE, key, direction) and its SSRC range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoLocal {
    pub media: Local,
    pub ssrc_range: (u32, u32),
}

/// The negotiated video stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedVideo {
    pub payload_type: u8,
    pub remote_rtp: Endpoint,
    pub remote_rtcp: Endpoint,
    pub remote_key: Option<Key>,
    pub remote_ice: Option<Ice>,
    pub direction: Direction,
    /// The peer's SSRC range (its send stream uses SSRCs from it).
    pub ssrc_range: (u32, u32),
    /// What the peer declared (its `send` is what we receive and vice versa).
    pub feedback: Feedback,
}

fn valid_range((a, b): (u32, u32)) -> bool {
    a >= 1 && b >= a && b <= SSRC_RANGE_MAX
}

/// `a=x-ssrc-range:A-B` (§3.1.5.31.1).
fn ssrc_range_of(m: &Media) -> Option<(u32, u32)> {
    let (a, b) = m.attr("x-ssrc-range")?.split_once('-')?;
    let r = (a.trim().parse().ok()?, b.trim().parse().ok()?);
    valid_range(r).then_some(r)
}

/// `a=rtcp-fb:* x-message app send:… recv:…` (§3.1.5.30.2).
fn feedback_of(m: &Media) -> Feedback {
    let mut f = Feedback::default();
    for v in m.attrs("rtcp-fb") {
        let mut w = v.split_whitespace();
        if w.next() != Some("*") || w.next() != Some("x-message") || w.next() != Some("app") {
            continue;
        }
        for part in w {
            let (dir, caps) = match part.split_once(':') {
                Some(x) => x,
                None => continue,
            };
            for cap in caps.split(',') {
                match (dir, cap) {
                    ("send", "src") => f.send_src = true,
                    ("send", "x-pli") => f.send_pli = true,
                    ("recv", "src") => f.recv_src = true,
                    ("recv", "x-pli") => f.recv_pli = true,
                    _ => {}
                }
            }
        }
    }
    f
}

/// The payload type of H.264 UC in a video line, only with exactly our format parameters and
/// an SSRC range (SD-27: otherwise the format is ignored).
fn h264uc_pt(m: &Media) -> Option<u8> {
    ssrc_range_of(m)?;
    m.formats.iter().copied().find(|&pt| {
        let mapped = m.attrs("rtpmap").any(|v| {
            v.split_once(' ').is_some_and(|(p, name)| {
                p.trim().parse::<u8>().ok() == Some(pt)
                    && name.trim().eq_ignore_ascii_case("X-H264UC/90000")
            })
        });
        let fmtp = m.attrs("fmtp").any(|v| {
            v.split_once(' ').is_some_and(|(p, params)| {
                p.trim().parse::<u8>().ok() == Some(pt) && params.trim() == H264UC_FMTP
            })
        });
        mapped && fmtp
    })
}

/// Our `m=video` section (example §4.1, p. 56).
pub fn video_section(v: &VideoLocal, proto: &str, pt: u8) -> Result<String, Error> {
    if !valid_range(v.ssrc_range) {
        return Err(Error::Syntax("SSRC range"));
    }
    let l = &v.media;
    let mut s = format!("m=video {} {proto} {pt}\r\n", l.rtp_port);
    s.push_str(&format!(
        "a=x-ssrc-range:{}-{}\r\n",
        v.ssrc_range.0, v.ssrc_range.1
    ));
    // SD-63 / §3.1.5.30.3: feedback needs the reduced-size attribute too.
    s.push_str("a=rtcp-fb:* x-message app send:src,x-pli recv:src,x-pli\r\na=rtcp-rsize\r\n");
    s.push_str("a=label:main-video\r\n");
    transport_attributes(l, &mut s);
    if let Some(k) = &l.key {
        s.push_str(&format!("a=crypto:{}\r\n", k.attribute()));
    }
    s.push_str(&format!("a=rtcp:{}\r\n", l.rtcp_port));
    s.push_str(&format!("a={}\r\n", l.direction.as_str()));
    s.push_str(&format!(
        "a=rtpmap:{pt} X-H264UC/90000\r\na=fmtp:{pt} {H264UC_FMTP}\r\n"
    ));
    Ok(s)
}

/// Our audio and video offer (audio first, as in the examples).
pub fn offer_with_video(
    audio: &Local,
    video: &VideoLocal,
    policy: Encryption,
) -> Result<String, Error> {
    let mut s = offer(audio, policy);
    let proto = match policy {
        Encryption::Required => "RTP/SAVP",
        _ => "RTP/AVP",
    };
    let mut v = video.clone();
    if policy == Encryption::Off {
        v.media.key = None;
    }
    s.push_str(&video_section(&v, proto, H264UC_PT)?);
    Ok(s)
}

/// Whether an offer has a video line we could take (H.264 UC with the NI-TC parameters and an
/// SSRC range, SD-27).
pub fn offered_video(offer: &[u8]) -> bool {
    parse(offer)
        .ok()
        .is_some_and(|s| video_line(&s).is_some_and(|m| h264uc_pt(m).is_some()))
}

fn video_line(s: &Session) -> Option<&Media> {
    s.media.iter().find(|m| m.kind == "video" && m.port != 0)
}

/// The peer's answer to our video line: `None` if it rejected video (port 0, no line, or no
/// acceptable H.264 UC format); an error if the line is there but unusable (key, encryption).
pub fn video_answer_to_offer(
    answer: &[u8],
    our_key: Option<&Key>,
    policy: Encryption,
) -> Result<Option<NegotiatedVideo>, Error> {
    let s = parse(answer)?;
    let Some(m) = video_line(&s) else {
        return Ok(None);
    };
    let (Some(pt), Some(range)) = (h264uc_pt(m), ssrc_range_of(m)) else {
        return Ok(None);
    };
    let secure = m.proto.eq_ignore_ascii_case("RTP/SAVP");
    let remote_key = if secure {
        let ours = our_key.ok_or(Error::Crypto("SAVP answer without our offer of a key"))?;
        let k = keys_of(m)
            .into_iter()
            .find(|k| k.tag == ours.tag)
            .ok_or(Error::Crypto("no key with our tag"))?;
        if k.key == ours.key {
            return Err(Error::Crypto("peer reused our key"));
        }
        Some(k)
    } else {
        match policy {
            Encryption::Off
            | Encryption::Optional {
                allow_plaintext: true,
            } => None,
            _ => return Err(Error::Unencrypted),
        }
    };
    let (rtp, rtcp) = endpoints(&s, m)?;
    Ok(Some(NegotiatedVideo {
        payload_type: pt,
        remote_rtp: rtp,
        remote_rtcp: rtcp,
        remote_key,
        remote_ice: ice_of(m),
        direction: Direction::of(m),
        ssrc_range: range,
        feedback: feedback_of(m),
    }))
}

/// The rejection of an offered media line we do not take ([RFC3264] §6: port 0, the offer's
/// formats).
fn rejected(m: &Media) -> String {
    let fmts: Vec<String> = m.formats.iter().map(u8::to_string).collect();
    let fmts = if fmts.is_empty() {
        "0".to_string()
    } else {
        fmts.join(" ")
    };
    format!("m={} 0 {} {fmts}\r\n", m.kind, m.proto)
}

/// Our answer to an offer with audio and possibly video. `video` is our video stream and a fresh
/// key, if we want to take video; every other `m=` line – and video without an acceptable H.264
/// UC format or key – is rejected with port 0, in the offer's order.
pub fn answer_with_video(
    offer: &[u8],
    local: &Local,
    fresh_key: Key,
    video: Option<(&VideoLocal, Key)>,
    policy: Encryption,
) -> Result<(String, Negotiated, Option<NegotiatedVideo>), Error> {
    let s = parse(offer)?;
    let (audio_sdp, negotiated) = answer_audio(offer, local, fresh_key, policy)?;
    let audio_part = audio_sdp
        .find("m=audio")
        .and_then(|i| audio_sdp.get(i..))
        .ok_or(Error::NoAudio)?;
    let audio_index = s
        .media
        .iter()
        .position(|m| m.kind == "audio" && m.port != 0)
        .ok_or(Error::NoAudio)?;
    let video_index = s
        .media
        .iter()
        .position(|m| m.kind == "video" && m.port != 0);
    let mut taken: Option<(String, NegotiatedVideo)> = None;
    if let (Some(vi), Some((ours, fresh))) = (video_index, video) {
        if let Some(m) = s.media.get(vi) {
            taken = answer_video_line(&s, m, ours, fresh, policy);
        }
    }
    let mut out = header(local);
    for (i, m) in s.media.iter().enumerate() {
        if i == audio_index {
            out.push_str(audio_part);
        } else if Some(i) == video_index && taken.is_some() {
            if let Some((section, _)) = &taken {
                out.push_str(section);
            }
        } else {
            out.push_str(&rejected(m));
        }
    }
    Ok((out, negotiated, taken.map(|(_, n)| n)))
}

/// Our answer section for an offered video line, or `None` to reject it.
fn answer_video_line(
    s: &Session,
    m: &Media,
    ours: &VideoLocal,
    fresh: Key,
    policy: Encryption,
) -> Option<(String, NegotiatedVideo)> {
    let pt = h264uc_pt(m)?;
    let range = ssrc_range_of(m)?;
    let required = m.proto.eq_ignore_ascii_case("RTP/SAVP");
    let offered = keys_of(m).into_iter().next();
    let (proto, my_key, remote_key) = match (offered, policy) {
        (Some(k), Encryption::Required | Encryption::Optional { .. }) => {
            let mut mine = fresh;
            mine.tag = k.tag;
            if mine.key == k.key {
                return None;
            }
            ("RTP/SAVP", Some(mine), Some(k))
        }
        (Some(_), Encryption::Off) if !required => ("RTP/AVP", None, None),
        (
            None,
            Encryption::Off
            | Encryption::Optional {
                allow_plaintext: true,
            },
        ) if !required => ("RTP/AVP", None, None),
        _ => return None,
    };
    // SD-64: our range must not overlap the peer's ranges in the same SDP (audio's included).
    let peer_ranges: Vec<(u32, u32)> = s.media.iter().filter_map(ssrc_range_of).collect();
    if peer_ranges
        .iter()
        .any(|(a, b)| *a <= ours.ssrc_range.1 && ours.ssrc_range.0 <= *b)
    {
        return None;
    }
    let (rtp, rtcp) = endpoints(s, m).ok()?;
    let remote_direction = Direction::of(m);
    let mut v = ours.clone();
    v.media.key = my_key;
    v.media.ssrtp = false;
    v.media.direction = match remote_direction {
        Direction::SendOnly => Direction::RecvOnly,
        Direction::RecvOnly => Direction::SendOnly,
        Direction::Inactive => Direction::Inactive,
        Direction::SendRecv => ours.media.direction,
    };
    let section = video_section(&v, proto, pt).ok()?;
    Some((
        section,
        NegotiatedVideo {
            payload_type: pt,
            remote_rtp: rtp,
            remote_rtcp: rtcp,
            remote_key,
            remote_ice: ice_of(m),
            direction: remote_direction,
            ssrc_range: range,
            feedback: feedback_of(m),
        },
    ))
}
