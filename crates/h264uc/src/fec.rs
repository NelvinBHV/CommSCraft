//! Forward error correction of H.264 UC (PL-06; [MS-H264PF] 20250520 §2.2.8, pp. 17–20;
//! §3.1.5.2, pp. 22–23; §3.2.5.2, pp. 25–26; example §4.4; [RFC5109] §7): one FEC packet protects
//! up to 48 media packets of an access unit with XOR. It has an SN offset instead of an SN base and
//! an FEC level extension header. Reed-Solomon (more than one FEC packet per protection operation)
//! is not specified and not used; such packets are ignored.
//!
//! Security by design: the media packets a FEC packet can name lie within the access unit being
//! received; a recovered packet is never longer than the protection length and goes through the
//! same checks as any other packet; memory per access unit is bounded.

use super::Error;
use std::collections::BTreeMap;

/// FEC header, level header (short mask) and level extension header.
const HEADER_SHORT: usize = 10 + 4 + 2;
/// Media packets of one protection operation: the bits of a long mask ([RFC5109] §7.4).
pub const MAX_PROTECTED: usize = 48;
const SHORT_MASK: usize = 16;
/// Media packets kept per access unit for a recovery.
const MAX_KEPT: usize = 256;
/// Largest protected payload (no RTP packet of ours is near it).
const MAX_PROTECTION: usize = 2048;

/// A media packet as far as FEC is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Media {
    pub seq: u16,
    pub timestamp: u32,
    pub marker: bool,
    pub payload_type: u8,
    pub payload: Vec<u8>,
}

fn xor_into(target: &mut [u8], source: &[u8]) {
    for (t, s) in target.iter_mut().zip(source) {
        *t ^= *s;
    }
}

/// The FEC payload for `packets`, for a FEC packet with sequence number `fec_seq`. `Marker` and
/// timestamp of the FEC packet itself are the caller's (§2.2.8.1.1).
pub fn protect(packets: &[Media], fec_seq: u16) -> Result<Vec<u8>, Error> {
    let first = packets.first().ok_or(Error::Malformed("no packets"))?;
    if packets.len() > MAX_PROTECTED {
        return Err(Error::Malformed("too many packets to protect"));
    }
    let long = packets.len() > SHORT_MASK;
    let mut mask = 0u64;
    let mut protection = 0usize;
    let mut marker_pt = 0u8;
    let mut length = 0u16;
    for p in packets {
        let bit = usize::from(p.seq.wrapping_sub(first.seq));
        if bit >= if long { MAX_PROTECTED } else { SHORT_MASK } {
            return Err(Error::Malformed("packets too far apart"));
        }
        mask |= 1u64 << (63 - bit);
        protection = protection.max(p.payload.len());
        marker_pt ^= (u8::from(p.marker) << 7) | (p.payload_type & 0x7f);
        length ^= u16::try_from(p.payload.len()).map_err(|_| Error::Malformed("payload size"))?;
    }
    if protection > MAX_PROTECTION {
        return Err(Error::Malformed("payload size"));
    }
    let mut fec = Vec::with_capacity(HEADER_SHORT + 4 + protection);
    // E = 1, L, P = X = 0, CC = 0: our packets have no padding, extension or CSRC list.
    fec.push(0x80 | if long { 0x40 } else { 0 });
    fec.push(marker_pt);
    fec.extend_from_slice(&fec_seq.wrapping_sub(first.seq).to_be_bytes());
    fec.extend_from_slice(&[0; 4]); // TS recovery: the protected string holds 32 zero bits
    fec.extend_from_slice(&length.to_be_bytes());
    fec.extend_from_slice(&(protection as u16).to_be_bytes());
    let mask_bytes = mask.to_be_bytes();
    fec.extend_from_slice(mask_bytes.get(..if long { 6 } else { 2 }).unwrap_or(&[]));
    fec.extend_from_slice(&[0x00, 0x10]); // V = C = HR1 = HR2 = 0, FEC count 1, index 0
    let start = fec.len();
    fec.resize(start + protection, 0);
    if let Some(body) = fec.get_mut(start..) {
        for p in packets {
            xor_into(body, &p.payload);
        }
    }
    Ok(fec)
}

/// What a FEC packet says.
#[derive(Debug, PartialEq, Eq)]
struct Parsed<'a> {
    marker_pt: u8,
    offset: u16,
    length: u16,
    mask: u64,
    payload: &'a [u8],
}

fn parse(fec: &[u8]) -> Option<Parsed<'_>> {
    let b = |i: usize| fec.get(i).copied();
    let first = b(0)?;
    let long = first & 0x40 != 0;
    // E must be set; P, X and CC recovery must be 0: a recovered packet of ours has none of them.
    if first & 0x80 == 0 || first & 0x3f != 0 {
        return None;
    }
    if fec.get(4..8)? != [0; 4] {
        return None;
    }
    let mask_len = if long { 6 } else { 2 };
    let mut mask_bytes = [0u8; 8];
    mask_bytes
        .get_mut(..mask_len)?
        .copy_from_slice(fec.get(12..12 + mask_len)?);
    let ext = 12 + mask_len;
    // V = C = HR1 = HR2 = reserved = 0; one FEC packet per operation: XOR (§3.1.5.2.2).
    if b(ext)? != 0 || b(ext + 1)? != 0x10 {
        return None;
    }
    let protection = usize::from(u16::from_be_bytes([b(10)?, b(11)?]));
    let payload = fec.get(ext + 2..)?;
    if payload.len() != protection || protection > MAX_PROTECTION {
        return None;
    }
    Some(Parsed {
        marker_pt: b(1)?,
        offset: u16::from_be_bytes([b(2)?, b(3)?]),
        length: u16::from_be_bytes([b(8)?, b(9)?]),
        mask: u64::from_be_bytes(mask_bytes),
        payload,
    })
}

/// Receive side: hands media packets on in order as soon as that is possible, holds those after a
/// gap until the FEC packet of the access unit or the next access unit arrives.
#[derive(Debug, Default)]
pub struct Recovery {
    timestamp: Option<u32>,
    next_seq: Option<u16>,
    /// All packets of the access unit so far, for a recovery.
    kept: BTreeMap<u16, Media>,
    /// Those not handed on yet.
    held: BTreeMap<u16, Media>,
    /// The sequence number after the last packet seen, FEC packets included: a unit whose first
    /// packet is lost starts with a gap.
    expected: Option<u16>,
    /// Packets recovered, for statistics.
    pub recovered: u64,
}

impl Recovery {
    /// Order within the sequence number space, relative to the access unit's start.
    fn release(&mut self) -> Vec<Media> {
        let mut out = Vec::new();
        while let Some(next) = self.next_seq {
            let Some(p) = self.held.remove(&next) else {
                break;
            };
            self.next_seq = Some(next.wrapping_add(1));
            out.push(p);
        }
        out
    }

    /// Moves to another access unit. What is still held goes on, the gap makes the depacketizer
    /// give the old one up.
    fn start(&mut self, timestamp: u32, seq: u16) -> Vec<Media> {
        let mut rest: Vec<Media> = std::mem::take(&mut self.held).into_values().collect();
        // Sequence order across a wrap: start from the lowest distance to the expected number.
        if let Some(next) = self.next_seq {
            rest.sort_by_key(|p| p.seq.wrapping_sub(next));
        }
        self.kept.clear();
        self.timestamp = Some(timestamp);
        self.next_seq = Some(match self.expected {
            Some(e) if usize::from(seq.wrapping_sub(e)) < MAX_KEPT => e,
            _ => seq,
        });
        rest
    }

    fn note_seq(&mut self, seq: u16) {
        let after = seq.wrapping_add(1);
        if self.expected.is_none_or(|e| after.wrapping_sub(e) < 0x8000) {
            self.expected = Some(after);
        }
    }

    /// A media packet; returns the packets that can go on now.
    pub fn push(&mut self, packet: Media) -> Vec<Media> {
        let timestamp = packet.timestamp;
        let mut out = Vec::new();
        match self.timestamp {
            // Late, from an access unit already given up.
            Some(t) if timestamp.wrapping_sub(t) >= 0x8000_0000 => return out,
            Some(t) if t == timestamp => {}
            _ => out = self.start(timestamp, packet.seq),
        }
        self.note_seq(packet.seq);
        // Before the expected number: a duplicate, or an unusual reorder inside the unit.
        let next = self.next_seq.unwrap_or(packet.seq);
        if packet.seq.wrapping_sub(next) >= 0x8000 || self.kept.contains_key(&packet.seq) {
            return out;
        }
        if self.kept.len() < MAX_KEPT {
            self.kept.insert(packet.seq, packet.clone());
        }
        if self.held.len() < MAX_KEPT {
            self.held.insert(packet.seq, packet);
        }
        out.extend(self.release());
        out
    }

    /// A FEC packet of the given sequence number and timestamp; returns packets that can go on
    /// thanks to it. Exactly one missing packet can be recovered.
    pub fn push_fec(&mut self, seq: u16, timestamp: u32, fec: &[u8]) -> Vec<Media> {
        self.note_seq(seq);
        if self.timestamp != Some(timestamp) {
            return Vec::new();
        }
        let Some(f) = parse(fec) else {
            return Vec::new();
        };
        let base = seq.wrapping_sub(f.offset);
        let mut missing = None;
        let mut marker_pt = f.marker_pt;
        let mut length = f.length;
        let mut body = f.payload.to_vec();
        for bit in 0..MAX_PROTECTED as u16 {
            if f.mask & (1u64 << (63 - bit)) == 0 {
                continue;
            }
            let protected = base.wrapping_add(bit);
            match self.kept.get(&protected) {
                Some(p) => {
                    if p.payload.len() > body.len() {
                        return Vec::new(); // only whole packets can be recovered (§3.2.5.2)
                    }
                    xor_into(&mut body, &p.payload);
                    marker_pt ^= (u8::from(p.marker) << 7) | (p.payload_type & 0x7f);
                    length ^= p.payload.len() as u16;
                }
                None if missing.is_none() => missing = Some(protected),
                None => return Vec::new(), // two gone: one FEC packet cannot help
            }
        }
        let Some(lost) = missing else {
            return Vec::new();
        };
        let length = usize::from(length);
        // A name for a packet before the access unit began would not fit in it.
        let next = self.next_seq.unwrap_or(lost);
        if length > body.len() || lost.wrapping_sub(next) >= 0x8000 {
            return Vec::new();
        }
        body.truncate(length);
        let packet = Media {
            seq: lost,
            timestamp,
            marker: marker_pt & 0x80 != 0,
            payload_type: marker_pt & 0x7f,
            payload: body,
        };
        self.recovered += 1;
        self.kept.insert(lost, packet.clone());
        self.held.insert(lost, packet);
        self.release()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media(seq: u16, marker: bool, payload: &[u8]) -> Media {
        Media {
            seq,
            timestamp: 90,
            marker,
            payload_type: 122,
            payload: payload.to_vec(),
        }
    }

    fn seqs(v: &[Media]) -> Vec<u16> {
        v.iter().map(|m| m.seq).collect()
    }

    /// §4.4: the fields of the example, as far as they depend on the packets.
    #[test]
    fn layout_of_the_example() {
        let packets: Vec<Media> = (10..16u16)
            .map(|s| Media {
                seq: s,
                timestamp: 90,
                marker: false,
                payload_type: 0,
                payload: vec![1; 872],
            })
            .collect();
        let f = protect(&packets, 17).unwrap();
        assert_eq!(f.get(..2).unwrap(), &[0x80, 0x00]);
        assert_eq!(f.get(2..4).unwrap(), &[0x00, 0x07], "SN offset");
        assert_eq!(f.get(4..8).unwrap(), &[0; 4], "TS recovery");
        assert_eq!(
            f.get(10..14).unwrap(),
            &[0x03, 0x68, 0xfc, 0x00],
            "length, mask"
        );
        assert_eq!(f.get(14..16).unwrap(), &[0x00, 0x10], "count 1, index 0");
        assert_eq!(f.len(), 16 + 872);
    }

    #[test]
    fn recovers_any_one_packet() {
        let packets = vec![
            media(65534, false, b"first packet"),
            media(65535, false, b"second, a longer packet"),
            media(0, false, b"third"),
            media(1, true, b"the last one"),
        ];
        let fec = protect(&packets, 2).unwrap();
        for lost in 0..packets.len() {
            let mut r = Recovery::default();
            // The unit before tells where this one starts.
            r.push(Media {
                timestamp: 0,
                ..media(65533, true, b"before")
            });
            let mut out = Vec::new();
            for (i, p) in packets.iter().enumerate() {
                if i != lost {
                    out.extend(r.push(p.clone()));
                }
            }
            out.extend(r.push_fec(2, 90, &fec));
            assert_eq!(out, packets, "lost {lost}");
            assert_eq!(r.recovered, 1);
        }
        // Nothing lost: straight through, and the FEC packet changes nothing.
        let mut r = Recovery::default();
        for p in &packets {
            assert_eq!(r.push(p.clone()), vec![p.clone()]);
        }
        assert!(r.push_fec(2, 90, &fec).is_empty());
    }

    #[test]
    fn two_lost_are_not_recovered_and_the_next_unit_moves_on() {
        let packets = vec![
            media(1, false, b"aa"),
            media(2, false, b"bb"),
            media(3, false, b"cc"),
            media(4, true, b"dd"),
        ];
        let fec = protect(&packets, 5).unwrap();
        let mut r = Recovery::default();
        assert_eq!(seqs(&r.push(packets[0].clone())), vec![1]);
        assert!(r.push(packets[3].clone()).is_empty(), "held after the gap");
        assert!(r.push_fec(5, 90, &fec).is_empty());
        let next = Media {
            timestamp: 180,
            ..media(6, false, b"new")
        };
        let out = r.push(next);
        assert_eq!(seqs(&out), vec![4, 6]);
        assert_eq!((out[0].timestamp, out[1].timestamp), (90, 180));
        // Late packets of the old unit do not come back.
        assert!(r.push(packets[1].clone()).is_empty());
    }

    #[test]
    fn long_mask() {
        let packets: Vec<Media> = (0..48u16)
            .map(|s| media(s, s == 47, &[s as u8; 50]))
            .collect();
        let fec = protect(&packets, 48).unwrap();
        assert_eq!(fec[0], 0xc0);
        let mut r = Recovery::default();
        let mut out = Vec::new();
        for p in packets.iter().filter(|p| p.seq != 40) {
            out.extend(r.push(p.clone()));
        }
        out.extend(r.push_fec(48, 90, &fec));
        assert_eq!(out, packets);
        assert!(protect(&vec![media(0, false, b"x"); 49], 1).is_err());
        assert!(protect(&[], 1).is_err());
    }

    #[test]
    fn hostile_fec_packets() {
        let packets = vec![media(1, false, b"aaaa"), media(2, true, b"bbbb")];
        let good = protect(&packets, 3).unwrap();
        let attempt = |fec: &[u8], ts: u32, seq: u16| {
            let mut r = Recovery::default();
            r.push(packets[0].clone());
            r.push_fec(seq, ts, fec)
        };
        assert_eq!(seqs(&attempt(&good, 90, 3)), vec![2]);
        // Another access unit, truncated, no E bit, padding or CSRC recovery, Reed-Solomon.
        assert!(attempt(&good, 91, 3).is_empty());
        for n in 0..good.len() {
            assert!(attempt(&good[..n], 90, 3).is_empty(), "{n}");
        }
        for (i, value) in [
            (0, 0x00),
            (0, 0xa0),
            (0, 0x81),
            (15, 0x20),
            (14, 0x10),
            (4, 1),
        ] {
            let mut bad = good.clone();
            bad[i] = value;
            assert!(attempt(&bad, 90, 3).is_empty(), "{i} {value}");
        }
        // A recovered length beyond the protection length.
        let mut long = good.clone();
        long[8] = 0x7f;
        assert!(attempt(&long, 90, 3).is_empty());
        // A protection length that does not fit the packet.
        let mut lying = good.clone();
        lying[10] = 0xff;
        assert!(attempt(&lying, 90, 3).is_empty());
        // Naming a packet before the access unit.
        let mut r = Recovery::default();
        r.push(media(100, false, b"aaaa"));
        r.push(media(101, true, b"bbbb"));
        assert!(r.push_fec(50, 90, &good).is_empty());
        // Garbage never panics.
        for len in 0..64usize {
            let junk: Vec<u8> = (0..len).map(|i| (i * 37) as u8 | 0x80).collect();
            let mut r = Recovery::default();
            r.push(media(1, false, b"x"));
            let _ = r.push_fec(9, 90, &junk);
        }
    }

    #[test]
    fn memory_is_bounded() {
        let mut r = Recovery::default();
        r.push(media(0, false, b"first"));
        for s in 2..2000u16 {
            r.push(media(s, false, b"after a gap"));
        }
        assert!(r.kept.len() <= MAX_KEPT && r.held.len() <= MAX_KEPT);
    }
}
