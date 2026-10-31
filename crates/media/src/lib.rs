//! The audio media core of one call (sans-I/O): codec, RTP, SRTP, jitter buffer, DTMF and RTCP,
//! plus the classification of received packets (STUN, RTP, RTCP).
//!
//! Evidence: files 15 (SR-*), 16 (RT-01 … RT-10, RT-20), 17 (G7-*, G2-*, PL-30), 14 (SD-10,
//! SD-25). Sources: RFC 3550 §6.2 (RTCP interval ≥ 5 s), §6.4; RFC 5761 §4 (RTP/RTCP
//! demultiplexing by the second octet 192–223); draft-ietf-behave-rfc3489bis-02 §10.1 (STUN: the
//! two most significant bits are 0); RFC 4733 §2.5.1.2/§2.5.1.4; [MS-RTP] §2.1 (one packet per
//! datagram, ≤ 1500 octets); [MS-DTMF] §3.1–3.3.
//!
//! Security by design: nothing reaches the decoder without SRTP authentication (when keys are
//! negotiated); only the negotiated payload types and the peer's first SSRC are accepted; the
//! jitter buffer is bounded (50 packets) and drops late or far-ahead packets; packets > 1500
//! octets are dropped; the playout never blocks or panics.

use std::collections::BTreeMap;

use commscraft_g711 as g711;
use commscraft_g722 as g722;
use commscraft_rtp as rtp;
use commscraft_sdp as sdp;
use commscraft_srtp as srtp;

/// One frame is 20 ms ([MS-RTP] §2.2.1: 20 ms usual; `a=ptime:20` in our SDP).
pub const FRAME_MS: u32 = 20;
/// Target jitter-buffer delay in frames (design: 60 ms) and its bound.
pub const JITTER_FRAMES: u32 = 3;
pub const MAX_BUFFERED: usize = 50;
/// RTCP report interval (RFC 3550 §6.2: minimum 5 s).
pub const RTCP_INTERVAL_MS: u64 = 5000;
/// Duration of a DTMF digit and update step, in 8 kHz units (100 ms, 20 ms).
const DTMF_DURATION: u16 = 800;
const DTMF_STEP: u16 = 160;

/// What a received datagram is (RFC 5761 §4, STUN-02 §10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Stun,
    Rtp,
    Rtcp,
    Other,
}

pub fn classify(p: &[u8]) -> Kind {
    match (p.first(), p.get(1)) {
        (Some(b0), _) if b0 >> 6 == 0 && p.len() >= 20 => Kind::Stun,
        (Some(b0), Some(b1)) if b0 >> 6 == 2 && (192..=223).contains(b1) => Kind::Rtcp,
        (Some(b0), Some(_)) if b0 >> 6 == 2 && p.len() >= 12 => Kind::Rtp,
        _ => Kind::Other,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The negotiated codec is not one we implement.
    Codec,
}

enum Encoder {
    G722(Box<g722::Encoder>),
    G711(g711::Law),
}

enum Decoder {
    G722(Box<g722::Decoder>),
    G711(g711::Law),
}

/// Statistics for the user interface and diagnostics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub sent: u64,
    pub received: u64,
    pub dropped: u64,
    pub concealed: u64,
}

/// The media of one call. Sample rate 16 kHz for G.722, 8 kHz for G.711.
pub struct Audio {
    codec: sdp::Codec,
    pt: u8,
    dtmf_pt: Option<u8>,
    encoder: Encoder,
    decoder: Decoder,
    tx_srtp: Option<srtp::Context>,
    rx_srtp: Option<srtp::Context>,
    sender: rtp::Sender,
    receiver: rtp::Receiver,
    remote_ssrc: Option<u32>,
    /// Jitter buffer: extended sequence → (timestamp, payload).
    buffer: BTreeMap<u64, Vec<u8>>,
    next_play: Option<u64>,
    highest: Option<u64>,
    last_frame: Vec<i16>,
    muted: bool,
    /// False while the call is on hold (ours or the peer's): no RTP is sent, RTCP continues.
    sending: bool,
    dtmf_queue: Vec<(bool, [u8; 4])>,
    dtmf_timestamp: u32,
    last_dtmf_end: Option<u32>,
    next_rtcp: u64,
    cname: String,
    pub stats: Stats,
}

impl std::fmt::Debug for Audio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Audio")
            .field("codec", &self.codec)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl Audio {
    /// `our_key`: the key we offered/answered (we encrypt with it); the peer's key is in
    /// `negotiated`. `random`: 10 bytes from a CSPRNG for SSRC/sequence/timestamp.
    pub fn new(
        negotiated: &sdp::Negotiated,
        our_key: Option<&sdp::Key>,
        random: [u8; 10],
        cname: &str,
    ) -> Result<Self, Error> {
        let (encoder, decoder) = match negotiated.codec {
            sdp::Codec::G722 => (
                Encoder::G722(Box::new(g722::Encoder::new())),
                Decoder::G722(Box::new(g722::Decoder::new())),
            ),
            sdp::Codec::Pcmu => (
                Encoder::G711(g711::Law::Ulaw),
                Decoder::G711(g711::Law::Ulaw),
            ),
            sdp::Codec::Pcma => (
                Encoder::G711(g711::Law::Alaw),
                Decoder::G711(g711::Law::Alaw),
            ),
        };
        // Both sides encrypt or neither (the SDP layer enforces the policy).
        let (tx_srtp, rx_srtp) = match (our_key, negotiated.remote_key.as_ref()) {
            (Some(o), Some(r)) => (
                Some(srtp::Context::new(&o.key, &o.salt, o.mki)),
                Some(srtp::Context::new(&r.key, &r.salt, r.mki)),
            ),
            _ => (None, None),
        };
        Ok(Audio {
            codec: negotiated.codec,
            pt: negotiated.payload_type,
            dtmf_pt: negotiated.dtmf_payload_type,
            encoder,
            decoder,
            tx_srtp,
            rx_srtp,
            sender: rtp::Sender::new(random),
            receiver: rtp::Receiver::default(),
            remote_ssrc: None,
            buffer: BTreeMap::new(),
            next_play: None,
            highest: None,
            last_frame: Vec::new(),
            muted: false,
            sending: true,
            dtmf_queue: Vec::new(),
            dtmf_timestamp: 0,
            last_dtmf_end: None,
            next_rtcp: 0,
            cname: cname
                .chars()
                .filter(|c| c.is_ascii_graphic())
                .take(64)
                .collect(),
            stats: Stats::default(),
        })
    }

    /// Samples per second of the device side.
    pub fn sample_rate(&self) -> u32 {
        match self.codec {
            sdp::Codec::G722 => 16000,
            _ => 8000,
        }
    }

    /// Samples per 20 ms frame.
    pub fn frame_samples(&self) -> usize {
        (self.sample_rate() / 1000 * FRAME_MS) as usize
    }

    /// RTP timestamp units per frame: 8000 Hz clock for all three codecs (RFC 3551 §4.5.2).
    fn frame_ticks(&self) -> u32 {
        8 * FRAME_MS
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// Hold: no RTP while `false` (RFC 3264 §8.4: inactive/recvonly streams carry no media).
    pub fn set_sending(&mut self, sending: bool) {
        self.sending = sending;
        if !sending {
            self.dtmf_queue.clear();
        }
    }

    /// Queues a DTMF digit (RFC 4733, MS-DTMF); refused if no telephone-event was negotiated.
    pub fn send_dtmf(&mut self, digit: char) -> bool {
        match (self.dtmf_pt, rtp::dtmf_event(digit)) {
            (Some(_), Some(ev)) if self.dtmf_queue.is_empty() => {
                self.dtmf_queue = rtp::dtmf_packets(ev, DTMF_DURATION, DTMF_STEP, 10);
                self.dtmf_timestamp = self.sender.timestamp();
                true
            }
            _ => false,
        }
    }

    fn protect(&mut self, packet: Vec<u8>) -> Option<Vec<u8>> {
        match self.tx_srtp.as_mut() {
            Some(c) => c.protect_rtp(&packet).ok(),
            None => Some(packet),
        }
    }

    /// One captured frame (20 ms) → the RTP (or SRTP) packet to send. During a DTMF digit the
    /// event packets replace audio (RFC 4733 §2.5.1.2: same timestamp, cumulative duration).
    pub fn on_capture(&mut self, pcm: &[i16]) -> Option<Vec<u8>> {
        let ticks = self.frame_ticks();
        if !self.sending {
            self.sender.skip(ticks);
            return None;
        }
        if !self.dtmf_queue.is_empty() {
            let (marker, payload) = self.dtmf_queue.remove(0);
            let pt = self.dtmf_pt?;
            let p = self
                .sender
                .packet_at(pt, marker, &payload, self.dtmf_timestamp)
                .ok()?;
            self.sender.skip(ticks);
            self.stats.sent += 1;
            return self.protect(p);
        }
        let mut payload = Vec::with_capacity(self.frame_samples());
        let silence;
        let input = if self.muted {
            silence = vec![0i16; self.frame_samples()];
            &silence[..]
        } else {
            pcm.get(..self.frame_samples().min(pcm.len()))?
        };
        match &mut self.encoder {
            Encoder::G722(e) => e.encode(input, &mut payload),
            Encoder::G711(l) => l.encode(input, &mut payload),
        }
        let p = self.sender.packet(self.pt, false, &payload, ticks).ok()?;
        self.stats.sent += 1;
        self.protect(p)
    }

    /// A received RTP/SRTP packet. Returns a DTMF digit when an event ended.
    pub fn on_rtp(&mut self, packet: &[u8], arrival_ticks: u32) -> Option<char> {
        if packet.len() > rtp::MAX_PACKET {
            self.stats.dropped += 1;
            return None;
        }
        let plain = match self.rx_srtp.as_mut() {
            Some(c) => match c.unprotect_rtp(packet) {
                Ok(p) => p,
                Err(_) => {
                    self.stats.dropped += 1;
                    return None;
                }
            },
            None => packet.to_vec(),
        };
        let Ok(parsed) = rtp::parse(&plain) else {
            self.stats.dropped += 1;
            return None;
        };
        let h = parsed.header;
        // The first authenticated SSRC is the peer; others are dropped (RT-02).
        match self.remote_ssrc {
            None => self.remote_ssrc = Some(h.ssrc),
            Some(s) if s != h.ssrc => {
                self.stats.dropped += 1;
                return None;
            }
            _ => {}
        }
        let is_dtmf = Some(h.payload_type) == self.dtmf_pt;
        if !self.receiver.on_packet(&h, arrival_ticks, !is_dtmf) {
            self.stats.dropped += 1;
            return None;
        }
        self.stats.received += 1;
        if is_dtmf {
            let ev = rtp::TelephoneEvent::decode(parsed.payload)?;
            // Report each digit once: on the first end packet (it is sent three times).
            if ev.end && self.last_dtmf_end != Some(h.timestamp) {
                self.last_dtmf_end = Some(h.timestamp);
                return rtp::dtmf_char(ev.event);
            }
            return None;
        }
        if h.payload_type != self.pt {
            self.stats.dropped += 1;
            return None;
        }
        let ext = self.extend(h.sequence);
        let window_ok = match self.next_play {
            Some(n) => ext >= n && ext < n + MAX_BUFFERED as u64,
            None => true,
        };
        if !window_ok || self.buffer.len() >= MAX_BUFFERED {
            self.stats.dropped += 1;
            return None;
        }
        self.buffer.insert(ext, parsed.payload.to_vec());
        None
    }

    /// Extends a 16-bit sequence number relative to the highest seen.
    fn extend(&mut self, seq: u16) -> u64 {
        let ext = match self.highest {
            None => u64::from(seq) + (1 << 16),
            Some(h) => {
                let base = h & !0xffff;
                let candidates = [base.wrapping_sub(1 << 16), base, base + (1 << 16)];
                candidates
                    .iter()
                    .map(|b| b + u64::from(seq))
                    .min_by_key(|c| c.abs_diff(h))
                    .unwrap_or(h)
            }
        };
        if self.highest.is_none_or(|h| ext > h) {
            self.highest = Some(ext);
        }
        ext
    }

    /// One 20 ms frame for the speaker, called every 20 ms. Silence until the buffer holds the
    /// target delay; a missing packet is concealed by the previous frame at half level.
    pub fn playout(&mut self) -> Vec<i16> {
        let n = self.frame_samples();
        let start = match self.next_play {
            Some(s) => s,
            None => {
                let (Some(&first), true) = (
                    self.buffer.keys().next(),
                    self.buffer.len() >= JITTER_FRAMES as usize,
                ) else {
                    return vec![0; n];
                };
                self.next_play = Some(first);
                first
            }
        };
        self.next_play = Some(start + 1);
        let mut out = Vec::with_capacity(n);
        match self.buffer.remove(&start) {
            Some(payload) => {
                match &mut self.decoder {
                    Decoder::G722(d) => d.decode(&payload, &mut out),
                    Decoder::G711(l) => l.decode(&payload, &mut out),
                }
                out.resize(n, 0);
                self.last_frame.clone_from(&out);
            }
            None => {
                self.stats.concealed += 1;
                out = self.last_frame.iter().map(|s| s / 2).collect();
                out.resize(n, 0);
                self.last_frame.clone_from(&out);
            }
        }
        // Drop anything that can no longer be played.
        self.buffer.retain(|k, _| *k > start);
        out
    }

    /// An RTCP compound packet when the report interval has passed (RFC 3550 §6.2), protected.
    pub fn rtcp_due(&mut self, now_ms: u64, ntp: u64) -> Option<Vec<u8>> {
        if now_ms < self.next_rtcp {
            return None;
        }
        self.next_rtcp = now_ms + RTCP_INTERVAL_MS;
        let info = rtp::SenderInfo {
            ntp,
            rtp_timestamp: self.sender.timestamp(),
            packets: self.sender.packets,
            octets: self.sender.octets,
        };
        let reports: Vec<rtp::ReportBlock> = self
            .remote_ssrc
            .map(|s| self.receiver.report(s, 0, 0))
            .into_iter()
            .collect();
        let p = rtp::compound(self.sender.ssrc, Some(info), &reports, &self.cname);
        match self.tx_srtp.as_mut() {
            Some(c) => c.protect_rtcp(&p).ok(),
            None => Some(p),
        }
    }

    /// RTCP BYE when the call ends (RFC 3550 §6.6).
    pub fn bye(&mut self) -> Option<Vec<u8>> {
        let p = rtp::bye(self.sender.ssrc, &self.cname);
        match self.tx_srtp.as_mut() {
            Some(c) => c.protect_rtcp(&p).ok(),
            None => Some(p),
        }
    }

    /// A received RTCP packet (authenticated and decrypted when keys are set). Returns true for
    /// a BYE of the peer.
    pub fn on_rtcp(&mut self, packet: &[u8]) -> bool {
        let plain = match self.rx_srtp.as_mut() {
            Some(c) => match c.unprotect_rtcp(packet) {
                Ok(p) => p,
                Err(_) => return false,
            },
            None => packet.to_vec(),
        };
        rtp::parse_rtcp(&plain).is_ok_and(|items| {
            items.iter().any(|i| match i {
                rtp::Rtcp::Bye { sources } => {
                    self.remote_ssrc.is_some_and(|s| sources.contains(&s))
                }
                _ => false,
            })
        })
    }
}

#[cfg(test)]
mod tests;
