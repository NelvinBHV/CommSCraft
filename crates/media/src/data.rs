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
/// and the SRTP authentication tag (10), rounded down (design; §2.2, p. 8).
pub const DATA_MTU: usize = 1400;
/// Out-of-order packets kept while a hole is open (design: over TCP nothing should be out of
/// order at all; a relay may still reorder a little).
pub const MAX_PENDING: usize = 256;

/// The data stream of one application sharing session.
pub struct Data {
    tx_srtp: Option<srtp::Context>,
    rx_srtp: Option<srtp::Context>,
    sender: rtp::Sender,
    remote_ssrc: Option<u32>,
    next_seq: Option<u16>,
    pending: BTreeMap<u16, Vec<u8>>,
    broken: bool,
    /// Received and sent payload octets.
    pub received: u64,
    pub sent: u64,
}

/// The 90 kHz timestamp of `now_ms` (RTP clock of `x-data/90000`).
fn ticks(now_ms: u64) -> u32 {
    (now_ms.wrapping_mul(90) & 0xffff_ffff) as u32
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
            received: 0,
            sent: 0,
        })
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
