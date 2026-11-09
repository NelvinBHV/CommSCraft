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
