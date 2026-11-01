//! The video core of a call (function 28): H.264 UC access units to SRTP packets and back
//! ([MS-H264PF] through `commscraft-h264uc`, PL-01 … PL-09), RTCP sender/receiver reports and
//! the video feedback of [MS-RTP] §2.2.12 (PLI, VSR; RT-25, RT-26). Sans-I/O like [`Audio`]:
//! the caller moves packets and the clock.
//!
//! Security by design: our SSRC comes from our declared range and the peer's packets are taken
//! only from its declared range (SD-64); SRTP authentication comes before any parsing; a lost or
//! malformed access unit is never decoded but answered with a picture loss indication; RTP
//! sequence number 0 is never sent (note <2>).
//!
//! [`Audio`]: crate::Audio

use commscraft_h264uc as uc;
use commscraft_rtp as rtp;
use commscraft_sdp as sdp;
use commscraft_srtp as srtp;
use rtp::feedback::{self, Pli, Retransmit, Vsr, VsrEntry};

use crate::{Error, RTCP_INTERVAL_MS};

/// Largest RTP payload of video (design: room for SRTP, TURN and IP headers in 1500 bytes).
pub const VIDEO_MTU: usize = 1200;

/// What the video core reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoEvent {
    /// A complete access unit for the decoder.
    AccessUnit { data: Vec<u8>, idr: bool },
    /// The peer asks for a key frame (PLI, or a VSR with the key-frame flag).
    KeyFrameRequested,
    /// The peer asks for video with these parameters (VSR; `SOURCE_NONE`: stop sending).
    SourceRequested { msi: u32, entries: Vec<VsrEntry> },
    /// The peer ended its stream (RTCP BYE).
    Bye,
}

/// The video stream of one call.
pub struct Video {
    pt: u8,
    tx_srtp: Option<srtp::Context>,
    rx_srtp: Option<srtp::Context>,
    sender: rtp::Sender,
    receiver: rtp::Receiver,
    remote_range: (u32, u32),
    remote_ssrc: Option<u32>,
    packetizer: uc::Packetizer,
    depacketizer: uc::Depacketizer,
    cname: String,
    next_rtcp: u64,
    last_sr: Option<(u32, u64)>,
    vsr: Option<(Vsr, Retransmit)>,
    pli: Option<(Pli, Retransmit)>,
    next_request_id: u16,
    /// Whether the peer accepts our feedback (its `recv:` capabilities, SD-63).
    peer_accepts: sdp::video::Feedback,
    pub sent_units: u64,
    pub received_units: u64,
    pub lost_units: u64,
}

impl std::fmt::Debug for Video {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Video")
            .field("sent_units", &self.sent_units)
            .field("received_units", &self.received_units)
            .field("lost_units", &self.lost_units)
            .finish_non_exhaustive()
    }
}

impl Video {
    /// `our_range`: the SSRC range we declared (our SSRC is its start); `layout`: the stream
    /// layout of what we send (one base layer, PRID 0); `random` from a CSPRNG.
    pub fn new(
        negotiated: &sdp::video::NegotiatedVideo,
        our_key: Option<&sdp::Key>,
        our_range: (u32, u32),
        layout: uc::StreamLayout,
        random: [u8; 10],
        cname: &str,
    ) -> Result<Video, Error> {
        let (tx_srtp, rx_srtp) = match (our_key, negotiated.remote_key.as_ref()) {
            (Some(o), Some(r)) => (
                Some(srtp::Context::new(&o.key, &o.salt, o.mki)),
                Some(srtp::Context::new(&r.key, &r.salt, r.mki)),
            ),
            (None, None) => (None, None),
            _ => return Err(Error::Video("only one side encrypts")),
        };
        let mut sender = rtp::Sender::new(random);
        sender.ssrc = our_range.0.max(1);
        sender.skip_sequence_zero();
        let packetizer =
            uc::Packetizer::new(VIDEO_MTU, 0, layout).map_err(|_| Error::Video("stream layout"))?;
        Ok(Video {
            pt: negotiated.payload_type,
            tx_srtp,
            rx_srtp,
            sender,
            receiver: rtp::Receiver::default(),
            remote_range: negotiated.ssrc_range,
            remote_ssrc: None,
            packetizer,
            depacketizer: uc::Depacketizer::new(),
            cname: cname
                .chars()
                .filter(|c| c.is_ascii_graphic())
                .take(64)
                .collect(),
            next_rtcp: 0,
            last_sr: None,
            vsr: None,
            pli: None,
            next_request_id: 1,
            peer_accepts: negotiated.feedback,
            sent_units: 0,
            received_units: 0,
            lost_units: 0,
        })
    }

    /// Our SSRC (the start of our range).
    pub fn ssrc(&self) -> u32 {
        self.sender.ssrc
    }

    /// One access unit (Annex B) captured at `timestamp` (90 kHz) → the protected RTP packets.
    pub fn on_access_unit(&mut self, au: &[u8], timestamp: u32) -> Vec<Vec<u8>> {
        let Ok(payloads) = self.packetizer.packetize(au) else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(payloads.len());
        for p in payloads {
            self.sender.skip_sequence_zero();
            let Ok(packet) = self.sender.packet_at(self.pt, p.marker, &p.data, timestamp) else {
                return Vec::new();
            };
            let protected = match self.tx_srtp.as_mut() {
                Some(c) => match c.protect_rtp(&packet) {
                    Ok(p) => p,
                    Err(_) => return Vec::new(),
                },
                None => packet,
            };
            out.push(protected);
        }
        self.sent_units += 1;
        out
    }

    /// A received RTP/SRTP packet; `arrival` in 90 kHz ticks. A lost access unit asks the
    /// peer for a key frame at once (PLI).
    pub fn on_rtp(&mut self, packet: &[u8], arrival: u32, now_ms: u64) -> Vec<VideoEvent> {
        if packet.len() > rtp::MAX_PACKET {
            return Vec::new();
        }
        let plain = match self.rx_srtp.as_mut() {
            Some(c) => match c.unprotect_rtp(packet) {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            },
            None => packet.to_vec(),
        };
        let Ok(parsed) = rtp::parse(&plain) else {
            return Vec::new();
        };
        let h = parsed.header;
        // SD-64: only SSRCs from the peer's range; then the first one is the stream.
        if h.ssrc < self.remote_range.0 || h.ssrc > self.remote_range.1 || h.payload_type != self.pt
        {
            return Vec::new();
        }
        match self.remote_ssrc {
            None => self.remote_ssrc = Some(h.ssrc),
            Some(s) if s != h.ssrc => return Vec::new(),
            _ => {}
        }
        if !self.receiver.on_packet(&h, arrival, true) {
            return Vec::new();
        }
        let mut events = Vec::new();
        for out in self.depacketizer.push(uc::RtpIn {
            seq: h.sequence,
            timestamp: h.timestamp,
            marker: h.marker,
            payload: parsed.payload,
        }) {
            match out {
                uc::Output::AccessUnit { data, idr, .. } => {
                    self.received_units += 1;
                    events.push(VideoEvent::AccessUnit { data, idr });
                }
                uc::Output::Lost { .. } => {
                    self.lost_units += 1;
                    self.request_key_frame(now_ms);
                }
            }
        }
        events
    }

    fn next_id(&mut self) -> u16 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        id
    }

    /// The SSRC the feedback is about: the peer's stream, or before it sends the start of its
    /// range (inference, O-RT-1).
    fn media_ssrc(&self) -> u32 {
        self.remote_ssrc.unwrap_or(self.remote_range.0)
    }

    /// Ask the peer for a key frame (extended PLI with a new request ID for priority ID 0),
    /// unless one is still being repeated or the peer does not accept PLI.
    pub fn request_key_frame(&mut self, now_ms: u64) {
        if !self.peer_accepts.recv_pli || self.pli.as_ref().is_some_and(|(_, r)| !r.finished()) {
            return;
        }
        let id = self.next_id();
        self.pli = Some((
            Pli {
                sender_ssrc: self.sender.ssrc,
                media_ssrc: self.media_ssrc(),
                extended: Some((id, 1)),
            },
            Retransmit::new(now_ms, false),
        ));
    }

    /// Ask the peer for video (VSR for the source `msi`, e.g. `SOURCE_ANY` in a 1:1 call) with
    /// the given parameters and a key frame; a new request replaces the old one.
    pub fn request_source(&mut self, msi: u32, entry: VsrEntry, now_ms: u64) {
        if !self.peer_accepts.recv_src {
            return;
        }
        let id = self.next_id();
        self.vsr = Some((
            Vsr {
                sender_ssrc: self.sender.ssrc,
                media_ssrc: self.media_ssrc(),
                msi,
                request_id: id,
                key_frame: true,
                entries: vec![entry],
            },
            Retransmit::new(now_ms, true),
        ));
    }

    fn protect_rtcp(&mut self, p: &[u8]) -> Option<Vec<u8>> {
        match self.tx_srtp.as_mut() {
            Some(c) => c.protect_rtcp(p).ok(),
            None => Some(p.to_vec()),
        }
    }

    /// The RTCP packets due now: feedback being repeated (each on its own, reduced size) and
    /// every 5 s a sender report with our reception report.
    pub fn rtcp_due(&mut self, now_ms: u64, ntp: u64) -> Vec<Vec<u8>> {
        let mut plain = Vec::new();
        if let Some((p, r)) = self.pli.as_mut() {
            if r.due(now_ms) {
                if let Ok(b) = p.to_bytes() {
                    plain.push(b);
                }
            }
        }
        if let Some((v, r)) = self.vsr.as_mut() {
            if r.due(now_ms) {
                if let Ok(b) = v.to_bytes() {
                    plain.push(b);
                }
            }
        }
        if now_ms >= self.next_rtcp {
            self.next_rtcp = now_ms + RTCP_INTERVAL_MS;
            let (lsr, dlsr) = self.last_sr.map_or((0, 0), |(mid, at)| {
                (
                    mid,
                    (now_ms.saturating_sub(at).saturating_mul(65536) / 1000)
                        .min(u64::from(u32::MAX)) as u32,
                )
            });
            let reports: Vec<rtp::ReportBlock> = self
                .remote_ssrc
                .map(|s| vec![self.receiver.report(s, lsr, dlsr)])
                .unwrap_or_default();
            let info = rtp::SenderInfo {
                ntp,
                rtp_timestamp: self.sender.timestamp(),
                packets: self.sender.packets,
                octets: self.sender.octets,
            };
            plain.push(
                rtp::Report {
                    ssrc: self.sender.ssrc,
                    info: Some(info),
                    reports: &reports,
                    extensions: &[],
                    cname: &self.cname,
                    quality: None,
                }
                .build(),
            );
        }
        plain.iter().filter_map(|p| self.protect_rtcp(p)).collect()
    }

    /// A received RTCP packet: reports, BYE and the peer's feedback about our stream.
    pub fn on_rtcp(&mut self, packet: &[u8], now_ms: u64) -> Vec<VideoEvent> {
        let plain = match self.rx_srtp.as_mut() {
            Some(c) => match c.unprotect_rtcp(packet) {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            },
            None => packet.to_vec(),
        };
        let Ok(items) = rtp::parse_rtcp(&plain) else {
            return Vec::new();
        };
        let in_range = |s: u32| s >= self.remote_range.0 && s <= self.remote_range.1;
        let mut events = Vec::new();
        for i in items {
            match i {
                rtp::Rtcp::SenderReport { ssrc, info, .. } if in_range(ssrc) => {
                    self.last_sr = Some(((info.ntp >> 16) as u32, now_ms));
                }
                rtp::Rtcp::Bye { sources } if sources.iter().any(|s| in_range(*s)) => {
                    events.push(VideoEvent::Bye)
                }
                rtp::Rtcp::Feedback(feedback::Feedback::Pli(p))
                    if in_range(p.sender_ssrc) && p.media_ssrc == self.sender.ssrc =>
                {
                    events.push(VideoEvent::KeyFrameRequested)
                }
                rtp::Rtcp::Feedback(feedback::Feedback::Vsr(v)) if in_range(v.sender_ssrc) => {
                    if v.key_frame {
                        events.push(VideoEvent::KeyFrameRequested);
                    }
                    events.push(VideoEvent::SourceRequested {
                        msi: v.msi,
                        entries: v.entries,
                    });
                }
                _ => {}
            }
        }
        events
    }

    /// RTCP BYE when the call ends.
    pub fn bye(&mut self) -> Option<Vec<u8>> {
        let p = rtp::bye(self.sender.ssrc, &self.cname);
        self.protect_rtcp(&p)
    }
}
