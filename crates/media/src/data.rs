//! The data stream of application sharing (function 43): an RDP byte stream as RTP payload
//! ([MS-RTASPF] 20250520, PL-40 … PL-42): payload type 127, marker 0, packets of at most 1500
//! octets on the wire, the receiver putting the payloads together in sequence number order
//! (§3.1.5, p. 9); protected by SRTP like every other stream (§1.6, p. 7). Sans-I/O like
//! [`Audio`]: the caller moves packets and the clock.
//!
//! Security by design: SRTP authentication comes before any parsing; only payload type 127
//! and the first SSRC seen are taken; reordering is bounded ([`MAX_PENDING`] packets) and a
//! hole that stays open ends the stream ([MS-RTASPF] §3.1.7: a loss stops the stream – the
//! byte stream cannot skip data); RTP sequence number 0 is never sent (MS-H264PF note <2>, the
//! same servers).
//!
//! [`Audio`]: crate::Audio

use commscraft_rtp as rtp;
use commscraft_srtp as srtp;
use std::collections::BTreeMap;

use crate::Error;

/// The payload type of RDP (§3.1.5, p. 9).
pub const RDP_PT: u8 = 127;
/// Largest RTP payload: 1500 octets less IPv4/TCP (40), RFC 4571 framing (2), RTP header (12)
/// with the send time extension (8), MKI (1) and the SRTP authentication tag (10), rounded down
/// (design; §2.2, p. 8).
pub const DATA_MTU: usize = 1400;
/// Out-of-order packets kept while a hole is open (design: over TCP nothing should be out of
/// order at all; a relay may still reorder a little).
pub const MAX_PENDING: usize = 256;
/// Interval of our RTCP reports (RFC 3550 §6.2: 5 s minimum).
pub const RTCP_INTERVAL_MS: u64 = 5_000;

/// The data stream of one application sharing session.
pub struct Data {
    tx_srtp: Option<srtp::Context>,
    rx_srtp: Option<srtp::Context>,
    sender: rtp::Sender,
    remote_ssrc: Option<u32>,
    next_seq: Option<u16>,
    pending: BTreeMap<u16, Vec<u8>>,
    broken: bool,
    /// When the next RTCP report is due (ms) and the CNAME it carries.
    next_rtcp: u64,
    cname: String,
    /// The ID of the `abs-send-time` header extension, when agreed.
    abs_send_time: Option<u8>,
    /// An authentic RTCP packet of the peer came in.
    peer_heard: bool,
    /// Received and sent payload octets.
    pub received: u64,
    pub sent: u64,
}

/// The 90 kHz timestamp of `now_ms` (RTP clock of `x-data/90000`).
fn ticks(now_ms: u64) -> u32 {
    (now_ms.wrapping_mul(90) & 0xffff_ffff) as u32
}

/// The 24-bit absolute send time of `now_ms` (Unix time): NTP seconds in 6.18 fixed point, the
/// low 6 bits of the seconds and 18 bits of fraction (abs-send-time, WebRTC; the URI the
/// Windows clients name).
fn abs_send_time(now_ms: u64) -> u32 {
    let secs = (now_ms / 1000).wrapping_add(2_208_988_800);
    let frac = ((now_ms % 1000) << 18) / 1000;
    (((secs & 0x3f) << 18) | frac) as u32 & 0x00ff_ffff
}

/// `packet` (a plain RTP packet without CSRCs or extension) with a one-byte header extension
/// ([RFC8285] §4.2) carrying element `id` = the absolute send time: X bit, `0xBEDE`, length 1
/// word, then ID/length octet and three value octets.
fn with_abs_send_time(packet: &[u8], id: u8, now_ms: u64) -> Option<Vec<u8>> {
    let (head, payload) = (packet.get(..12)?, packet.get(12..)?);
    if head.first()? & 0x1f != 0 || !(1..=14).contains(&id) {
        return None;
    }
    let mut out = Vec::with_capacity(packet.len() + 8);
    out.extend_from_slice(head);
    if let Some(b) = out.first_mut() {
        *b |= 0x10;
    }
    let t = abs_send_time(now_ms).to_be_bytes();
    out.extend_from_slice(&[0xbe, 0xde, 0x00, 0x01, (id << 4) | 2]);
    out.extend_from_slice(t.get(1..4)?);
    out.extend_from_slice(payload);
    Some(out)
}

impl Data {
    /// `our_key`/`remote_key`: both or neither (no one-sided encryption).
    pub fn new(
        our_key: Option<&commscraft_sdp::Key>,
        remote_key: Option<&commscraft_sdp::Key>,
        random: [u8; 10],
    ) -> Result<Data, Error> {
        let (tx_srtp, rx_srtp) = match (our_key, remote_key) {
            (Some(o), Some(r)) => (
                Some(srtp::Context::new(&o.key, &o.salt, o.mki)),
                Some(srtp::Context::new(&r.key, &r.salt, r.mki)),
            ),
            (None, None) => (None, None),
            _ => return Err(Error::Video("only one side encrypts")),
        };
        let mut sender = rtp::Sender::new(random);
        sender.skip_sequence_zero();
        Ok(Data {
            tx_srtp,
            rx_srtp,
            sender,
            remote_ssrc: None,
            next_seq: None,
            pending: BTreeMap::new(),
            broken: false,
            next_rtcp: 0,
            cname: "commscraft@linux".into(),
            abs_send_time: None,
            peer_heard: false,
            received: 0,
            sent: 0,
        })
    }

    /// Sends every RTP packet with the `abs-send-time` header extension of this ID (agreed in
    /// the SDP).
    pub fn set_abs_send_time(&mut self, id: Option<u8>) {
        self.abs_send_time = id.filter(|i| (1..=14).contains(i));
    }

    /// The packet with the agreed header extensions.
    fn extend(&self, packet: Vec<u8>, now_ms: u64) -> Vec<u8> {
        match self.abs_send_time {
            Some(id) => with_abs_send_time(&packet, id, now_ms).unwrap_or(packet),
            None => packet,
        }
    }

    /// The CNAME of our reports (RFC 3550 §6.5.1).
    pub fn set_cname(&mut self, cname: &str) {
        self.cname = cname
            .chars()
            .filter(|c| !c.is_control())
            .take(255)
            .collect();
    }

    /// Our RTCP sender report with SDES CNAME when one is due (every [`RTCP_INTERVAL_MS`], the
    /// first at once; RFC 3550 §6.4.1). The peer's own reports to us carry a CNAME too.
    pub fn rtcp_due(&mut self, now_ms: u64, ntp: u64) -> Option<Vec<u8>> {
        if now_ms < self.next_rtcp {
            return None;
        }
        self.next_rtcp = now_ms + RTCP_INTERVAL_MS;
        let info = rtp::SenderInfo {
            ntp,
            rtp_timestamp: ticks(now_ms),
            packets: self.sender.packets,
            octets: self.sender.octets,
        };
        let plain = rtp::compound(self.sender.ssrc, Some(info), &[], &self.cname);
        match self.tx_srtp.as_mut() {
            Some(c) => c.protect_rtcp(&plain).ok(),
            None => Some(plain),
        }
    }

    /// Whether a hole in the received stream ended it.
    pub fn broken(&self) -> bool {
        self.broken
    }

    /// RTP/SRTP packets carrying `bytes` (split at [`DATA_MTU`]).
    pub fn send(&mut self, bytes: &[u8], now_ms: u64) -> Vec<Vec<u8>> {
        let ts = ticks(now_ms);
        let mut out = Vec::new();
        for chunk in bytes.chunks(DATA_MTU) {
            let Ok(p) = self.sender.packet_at(RDP_PT, false, chunk, ts) else {
                continue;
            };
            let p = self.extend(p, now_ms);
            let wire = match self.tx_srtp.as_mut() {
                Some(c) => match c.protect_rtp(&p) {
                    Ok(w) => w,
                    Err(_) => continue,
                },
                None => p,
            };
            self.sent += chunk.len() as u64;
            out.push(wire);
        }
        out
    }

    /// A received RTCP/SRTCP packet: only noted when it is authentic (its content is not used).
    pub fn on_rtcp(&mut self, packet: &[u8]) {
        let ok = match self.rx_srtp.as_mut() {
            Some(c) => c.unprotect_rtcp(packet).is_ok(),
            None => rtp::parse_rtcp(packet).is_ok(),
        };
        self.peer_heard |= ok;
    }

    /// Whether the peer's RTCP came in: the Windows viewer starts RDP only after the first
    /// reports went both ways (`tested-in-lab`).
    pub fn peer_heard(&self) -> bool {
        self.peer_heard
    }

    /// A received RTP/SRTP packet: the bytes that are now in order (possibly none).
    pub fn on_rtp(&mut self, packet: &[u8]) -> Vec<u8> {
        if self.broken || packet.len() > rtp::MAX_PACKET {
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
        if h.payload_type != RDP_PT {
            return Vec::new();
        }
        match self.remote_ssrc {
            None => self.remote_ssrc = Some(h.ssrc),
            Some(s) if s != h.ssrc => return Vec::new(),
            Some(_) => {}
        }
        let next = *self.next_seq.get_or_insert(h.sequence);
        // Older than what was delivered: a duplicate.
        if h.sequence.wrapping_sub(next) >= 0x8000 {
            return Vec::new();
        }
        self.pending.insert(h.sequence, parsed.payload.to_vec());
        if self.pending.len() > MAX_PENDING {
            self.broken = true;
            self.pending.clear();
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut seq = next;
        loop {
            // A sender for these servers skips sequence number 0 (note <2>): no hole there.
            if seq == 0 && !self.pending.contains_key(&0) && self.pending.contains_key(&1) {
                seq = 1;
            }
            let Some(p) = self.pending.remove(&seq) else {
                break;
            };
            out.extend_from_slice(&p);
            seq = seq.wrapping_add(1);
        }
        self.next_seq = Some(seq);
        self.received += out.len() as u64;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commscraft_sdp::Key;

    fn pair() -> (Data, Data) {
        let a = Key::new(1, [3; 30]);
        let b = Key::new(1, [4; 30]);
        (
            Data::new(Some(&a), Some(&b), [1; 10]).unwrap(),
            Data::new(Some(&b), Some(&a), [2; 10]).unwrap(),
        )
    }

    /// §2.2, §3.1.5: a long RDP message is split, sent with PT 127 and marker 0, and comes back
    /// in order.
    #[test]
    fn round_trip() {
        let (mut tx, mut rx) = pair();
        let msg: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
        let packets = tx.send(&msg, 1000);
        assert_eq!(packets.len(), 4);
        // §2.2: with IPv4/TCP (40) and RFC 4571 framing (2) within 1500 octets.
        assert!(packets.iter().all(|p| p.len() + 42 <= 1500));
        let mut got = Vec::new();
        for p in &packets {
            got.extend(rx.on_rtp(p));
        }
        assert_eq!(got, msg);
        assert_eq!((tx.sent, rx.received), (5000, 5000));
    }

    /// RFC 8285 one-byte extension: X bit, 0xBEDE, one word with the absolute send time; the
    /// receiver skips it and still gets the bytes; the packet stays within the size limit.
    #[test]
    fn abs_send_time_extension() {
        let (mut tx, mut rx) = pair();
        tx.set_abs_send_time(Some(1));
        let packets = tx.send(&[7u8; 1400], 1_000_000);
        let plain =
            super::with_abs_send_time(&[0x80, 0x7f, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 9], 1, 500)
                .unwrap();
        assert_eq!(plain.get(..1), Some(&[0x90u8][..]));
        assert_eq!(plain.get(12..17), Some(&[0xbe, 0xde, 0x00, 0x01, 0x12][..]));
        // 2208988800 s + 0.5 s: seconds mod 64 = 0, fraction 0.5 = 1 << 17.
        assert_eq!(plain.get(17..20), Some(&[0x02, 0x00, 0x00][..]));
        assert_eq!(plain.last(), Some(&9));
        assert!(packets.iter().all(|p| p.len() + 42 <= 1500));
        let got: Vec<u8> = packets.iter().flat_map(|p| rx.on_rtp(p)).collect();
        assert_eq!(got, vec![7u8; 1400]);
    }

    /// Our report is an SRTCP-protected sender report, due every 5 s.
    #[test]
    fn report_interval() {
        let (mut tx, _) = pair();
        let first = tx.rtcp_due(10_000, 1).unwrap();
        assert_eq!(first.get(1), Some(&200), "a sender report");
        assert!(tx.rtcp_due(14_999, 1).is_none());
        assert!(tx.rtcp_due(15_000, 1).is_some());
    }

    /// Reordered packets wait for the hole; a duplicate is dropped; a forged packet is not taken.
    #[test]
    fn order_and_authentication() {
        let (mut tx, mut rx) = pair();
        let p: Vec<Vec<u8>> = (0..3u8).flat_map(|i| tx.send(&[i; 10], 0)).collect();
        assert!(rx.on_rtp(&p[0]).len() == 10);
        assert!(rx.on_rtp(&p[2]).is_empty());
        assert_eq!(rx.on_rtp(&p[1]), [[1u8; 10], [2u8; 10]].concat());
        assert!(rx.on_rtp(&p[1]).is_empty());
        let mut forged = tx.send(&[9; 10], 0).remove(0);
        if let Some(b) = forged.last_mut() {
            *b ^= 1;
        }
        assert!(rx.on_rtp(&forged).is_empty());
    }

    /// A hole that stays open ends the stream (§3.1.7).
    #[test]
    fn lasting_hole_breaks() {
        let (mut tx, mut rx) = pair();
        let p: Vec<Vec<u8>> = (0..(MAX_PENDING + 3))
            .flat_map(|_| tx.send(&[7; 4], 0))
            .collect();
        rx.on_rtp(&p[0]);
        for q in p.iter().skip(2) {
            rx.on_rtp(q);
        }
        assert!(rx.broken());
        assert!(rx.on_rtp(&p[1]).is_empty());
    }
}
