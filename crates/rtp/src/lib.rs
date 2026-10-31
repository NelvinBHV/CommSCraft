//! RTP and RTCP for audio (sans-I/O): packet headers, the sending side (SSRC, sequence numbers,
//! timestamps), reception statistics, RTCP SR/RR/SDES/BYE and DTMF telephone events.
//!
//! Evidence: `docs/protocol/evidence/16-rtp-rtcp.md` (RT-01 … RT-10, RT-20, RT-21) and
//! `17-payload-formats-video-audio-appsharing.md` (PL-30). Sources: RFC 3550 §5.1 (header),
//! §6.4.1/6.4.2 (SR/RR), §6.5 (SDES), §6.6 (BYE), Appendix A.1/A.3/A.8 (validity, loss,
//! jitter); RFC 4733 §2.3, §2.5.1.2, §2.5.1.4 (telephone events); [MS-RTP] 20250520 §2.1–2.2.2
//! (pp. 14–17), §3.1.2 (p. 35); [MS-DTMF] 20250520 §2.2.1, §3.1–3.3 (pp. 8–12); [MS-SRTP] §5.1
//! (start sequence number, p. 15).
//!
//! Security by design: every length is checked before use (packet ≤ 1500 bytes, RT-01), CSRC
//! lists are bounded by the header field, RTCP walking is bounded (at most 16 packets per
//! compound, at most 10 report blocks, RT-21), SDES text is bounded and stripped of control
//! characters; nothing here allocates per received octet beyond the packet itself.

/// Largest RTP/RTCP packet (RT-01).
pub const MAX_PACKET: usize = 1500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Malformed,
    TooLarge,
    Version,
}

fn be16(p: &[u8], at: usize) -> Result<u16, Error> {
    let b = p.get(at..at + 2).ok_or(Error::Malformed)?;
    Ok(u16::from_be_bytes(
        b.try_into().map_err(|_| Error::Malformed)?,
    ))
}

fn be32(p: &[u8], at: usize) -> Result<u32, Error> {
    let b = p.get(at..at + 4).ok_or(Error::Malformed)?;
    Ok(u32::from_be_bytes(
        b.try_into().map_err(|_| Error::Malformed)?,
    ))
}

// ----- RTP header (RFC 3550 §5.1) ---------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub marker: bool,
    pub payload_type: u8,
    pub sequence: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    /// Contributing sources (from mixers; the first is the dominant speaker, RT-07).
    pub csrc: Vec<u32>,
}

/// A parsed RTP packet: header and payload (padding removed, header extension skipped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet<'a> {
    pub header: Header,
    pub payload: &'a [u8],
}

/// Parses and validates an RTP packet (RFC 3550 A.1: version 2, lengths, padding).
pub fn parse(p: &[u8]) -> Result<Packet<'_>, Error> {
    if p.len() > MAX_PACKET {
        return Err(Error::TooLarge);
    }
    let b0 = *p.first().ok_or(Error::Malformed)?;
    let b1 = *p.get(1).ok_or(Error::Malformed)?;
    if b0 >> 6 != 2 {
        return Err(Error::Version);
    }
    let cc = usize::from(b0 & 0x0f);
    let mut off = 12 + cc * 4;
    if p.len() < off {
        return Err(Error::Malformed);
    }
    let csrc = (0..cc)
        .map(|k| be32(p, 12 + 4 * k))
        .collect::<Result<Vec<u32>, Error>>()?;
    if b0 & 0x10 != 0 {
        off += 4 + usize::from(be16(p, off + 2)?) * 4;
    }
    let mut end = p.len();
    if b0 & 0x20 != 0 {
        let pad = usize::from(*p.last().ok_or(Error::Malformed)?);
        end = end
            .checked_sub(pad)
            .filter(|&e| pad > 0 && e >= off)
            .ok_or(Error::Malformed)?;
    }
    let payload = p.get(off..end).ok_or(Error::Malformed)?;
    Ok(Packet {
        header: Header {
            marker: b1 & 0x80 != 0,
            payload_type: b1 & 0x7f,
            sequence: be16(p, 2)?,
            timestamp: be32(p, 4)?,
            ssrc: be32(p, 8)?,
            csrc,
        },
        payload,
    })
}

/// Builds an RTP packet (no padding, no extension).
pub fn build(h: &Header, payload: &[u8]) -> Result<Vec<u8>, Error> {
    let cc = h.csrc.len().min(15);
    let len = 12 + 4 * cc + payload.len();
    if len > MAX_PACKET {
        return Err(Error::TooLarge);
    }
    let mut p = Vec::with_capacity(len);
    p.push(0x80 | cc as u8);
    p.push((u8::from(h.marker) << 7) | (h.payload_type & 0x7f));
    p.extend_from_slice(&h.sequence.to_be_bytes());
    p.extend_from_slice(&h.timestamp.to_be_bytes());
    p.extend_from_slice(&h.ssrc.to_be_bytes());
    for c in h.csrc.iter().take(cc) {
        p.extend_from_slice(&c.to_be_bytes());
    }
    p.extend_from_slice(payload);
    Ok(p)
}

// ----- sending side -----------------------------------------------------------------------------

/// The sending side of one RTP stream.
#[derive(Debug, Clone)]
pub struct Sender {
    pub ssrc: u32,
    sequence: u16,
    timestamp: u32,
    /// Counters for the sender report.
    pub packets: u32,
    pub octets: u32,
}

impl Sender {
    /// `random` from a CSPRNG: SSRC ≠ 0 (RT-06), start sequence 0 … 32767 (MS-SRTP §5.1: not close
    /// to 65535), random start timestamp (RFC 3550 §5.1).
    pub fn new(random: [u8; 10]) -> Self {
        let [a, b, c, d, e, f, g, h, i, j] = random;
        let ssrc = u32::from_be_bytes([a, b, c, d]).max(1);
        Sender {
            ssrc,
            sequence: u16::from_be_bytes([e, f]) & 0x7fff,
            timestamp: u32::from_be_bytes([g, h, i, j]),
            packets: 0,
            octets: 0,
        }
    }

    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    /// One packet; afterwards the timestamp advances by `samples` (clock units of the payload).
    pub fn packet(
        &mut self,
        payload_type: u8,
        marker: bool,
        payload: &[u8],
        samples: u32,
    ) -> Result<Vec<u8>, Error> {
        let p = self.packet_at(payload_type, marker, payload, self.timestamp)?;
        self.timestamp = self.timestamp.wrapping_add(samples);
        Ok(p)
    }

    /// One packet with an explicit timestamp (telephone events keep the start timestamp,
    /// RFC 4733 §2.5.1.2); sequence numbers continue.
    pub fn packet_at(
        &mut self,
        payload_type: u8,
        marker: bool,
        payload: &[u8],
        timestamp: u32,
    ) -> Result<Vec<u8>, Error> {
        let p = build(
            &Header {
                marker,
                payload_type,
                sequence: self.sequence,
                timestamp,
                ssrc: self.ssrc,
                csrc: Vec::new(),
            },
            payload,
        )?;
        self.sequence = self.sequence.wrapping_add(1);
        self.packets = self.packets.wrapping_add(1);
        self.octets = self.octets.wrapping_add(payload.len() as u32);
        Ok(p)
    }

    /// Advances the timestamp without sending (silence, or audio replaced by events).
    pub fn skip(&mut self, samples: u32) {
        self.timestamp = self.timestamp.wrapping_add(samples);
    }
}

// ----- receiving side (RFC 3550 A.1, A.3, A.8) --------------------------------------------------

/// Reception statistics of one source.
#[derive(Debug, Clone, Default)]
pub struct Receiver {
    started: bool,
    base_seq: u32,
    max_seq: u16,
    cycles: u32,
    received: u32,
    expected_prior: u32,
    received_prior: u32,
    transit: Option<i64>,
    /// Jitter estimate scaled by 16 (A.8 integer form).
    jitter16: u32,
    /// Timestamp of the last packet (for the SR delay fields we keep only what RR needs).
    pub last_received: Option<u64>,
}

/// Sequence numbers further ahead than this are treated as a restart (A.1 MAX_DROPOUT).
const MAX_DROPOUT: u16 = 3000;
const MAX_MISORDER: u16 = 100;

impl Receiver {
    /// Accounts one packet. `arrival` is the arrival time in clock units of the payload;
    /// `count_jitter` is false for telephone events (RT-10). Returns false for packets outside
    /// the plausible sequence range (they are not counted).
    pub fn on_packet(&mut self, h: &Header, arrival: u32, count_jitter: bool) -> bool {
        let seq = h.sequence;
        if !self.started {
            self.started = true;
            self.base_seq = u32::from(seq);
            self.max_seq = seq;
        } else {
            let delta = seq.wrapping_sub(self.max_seq);
            if delta < MAX_DROPOUT {
                if seq < self.max_seq {
                    self.cycles = self.cycles.wrapping_add(1 << 16);
                }
                self.max_seq = seq;
            } else if delta <= u16::MAX - MAX_MISORDER {
                // A very large jump: not accepted (A.1 would wait for a second packet).
                return false;
            }
        }
        self.received = self.received.wrapping_add(1);
        if count_jitter {
            let transit = i64::from(arrival) - i64::from(h.timestamp);
            if let Some(prev) = self.transit {
                let d = (transit - prev)
                    .unsigned_abs()
                    .min(u64::from(u32::MAX >> 4)) as u32;
                // A.8: jitter += d − ((jitter + 8) >> 4), scaled by 16.
                self.jitter16 = self
                    .jitter16
                    .wrapping_add(d)
                    .wrapping_sub((self.jitter16 + 8) >> 4);
            }
            self.transit = Some(transit);
        }
        true
    }

    pub fn extended_max(&self) -> u32 {
        self.cycles.wrapping_add(u32::from(self.max_seq))
    }

    pub fn jitter(&self) -> u32 {
        self.jitter16 >> 4
    }

    /// A report block (RFC 3550 §6.4.1; A.3 for loss); updates the interval counters.
    pub fn report(&mut self, ssrc: u32, lsr: u32, dlsr: u32) -> ReportBlock {
        let expected = self
            .extended_max()
            .wrapping_sub(self.base_seq)
            .wrapping_add(1);
        let lost = i64::from(expected) - i64::from(self.received);
        let expected_interval = expected.wrapping_sub(self.expected_prior);
        let received_interval = self.received.wrapping_sub(self.received_prior);
        self.expected_prior = expected;
        self.received_prior = self.received;
        let lost_interval = i64::from(expected_interval) - i64::from(received_interval);
        let fraction = if expected_interval == 0 || lost_interval <= 0 {
            0
        } else {
            ((lost_interval << 8) / i64::from(expected_interval)).min(255) as u8
        };
        ReportBlock {
            ssrc,
            fraction_lost: fraction,
            // 24-bit signed, clamped.
            cumulative_lost: lost.clamp(-0x80_0000, 0x7f_ffff) as i32,
            highest_sequence: self.extended_max(),
            jitter: self.jitter(),
            lsr,
            dlsr,
        }
    }
}

// ----- RTCP (RFC 3550 §6) -----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportBlock {
    pub ssrc: u32,
    pub fraction_lost: u8,
    pub cumulative_lost: i32,
    pub highest_sequence: u32,
    pub jitter: u32,
    pub lsr: u32,
    pub dlsr: u32,
}

impl ReportBlock {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.ssrc.to_be_bytes());
        out.push(self.fraction_lost);
        let lost = (self.cumulative_lost as u32) & 0x00ff_ffff;
        out.extend_from_slice(&lost.to_be_bytes()[1..]);
        out.extend_from_slice(&self.highest_sequence.to_be_bytes());
        out.extend_from_slice(&self.jitter.to_be_bytes());
        out.extend_from_slice(&self.lsr.to_be_bytes());
        out.extend_from_slice(&self.dlsr.to_be_bytes());
    }

    fn read(p: &[u8]) -> Result<Self, Error> {
        let lost = be32(p, 4)? & 0x00ff_ffff;
        let lost = if lost & 0x80_0000 != 0 {
            lost as i32 - 0x100_0000
        } else {
            lost as i32
        };
        Ok(ReportBlock {
            ssrc: be32(p, 0)?,
            fraction_lost: *p.get(4).ok_or(Error::Malformed)?,
            cumulative_lost: lost,
            highest_sequence: be32(p, 8)?,
            jitter: be32(p, 12)?,
            lsr: be32(p, 16)?,
            dlsr: be32(p, 20)?,
        })
    }
}

/// Sender information of an SR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SenderInfo {
    pub ntp: u64,
    pub rtp_timestamp: u32,
    pub packets: u32,
    pub octets: u32,
}

/// RTCP packet types handled here (others are skipped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rtcp {
    SenderReport {
        ssrc: u32,
        info: SenderInfo,
        reports: Vec<ReportBlock>,
    },
    ReceiverReport {
        ssrc: u32,
        reports: Vec<ReportBlock>,
    },
    /// SDES CNAME items (other items are skipped).
    Cname {
        ssrc: u32,
        cname: String,
    },
    Bye {
        sources: Vec<u32>,
    },
}

/// Most report blocks per packet ([MS-RTP] RT-21).
pub const MAX_REPORTS: usize = 10;
const MAX_COMPOUND: usize = 16;

fn header(out: &mut Vec<u8>, count: usize, pt: u8, body_len: usize) {
    out.push(0x80 | (count as u8 & 0x1f));
    out.push(pt);
    out.extend_from_slice(&(((body_len + 4) / 4 - 1) as u16).to_be_bytes());
}

/// A compound RTCP packet: SR (if `info`) or RR, then SDES with CNAME (RFC 3550 §6.1).
pub fn compound(
    ssrc: u32,
    info: Option<SenderInfo>,
    reports: &[ReportBlock],
    cname: &str,
) -> Vec<u8> {
    let reports = reports.get(..reports.len().min(MAX_REPORTS)).unwrap_or(&[]);
    let mut out = Vec::new();
    let mut body = ssrc.to_be_bytes().to_vec();
    let pt = match info {
        Some(i) => {
            body.extend_from_slice(&i.ntp.to_be_bytes());
            body.extend_from_slice(&i.rtp_timestamp.to_be_bytes());
            body.extend_from_slice(&i.packets.to_be_bytes());
            body.extend_from_slice(&i.octets.to_be_bytes());
            200
        }
        None => 201,
    };
    for r in reports {
        r.write(&mut body);
    }
    header(&mut out, reports.len(), pt, body.len());
    out.extend_from_slice(&body);
    // SDES: SSRC, CNAME item, END item, padding to 32 bits.
    let cname: Vec<u8> = cname
        .bytes()
        .filter(|b| b.is_ascii_graphic())
        .take(255)
        .collect();
    let mut sdes = ssrc.to_be_bytes().to_vec();
    sdes.push(1);
    sdes.push(cname.len() as u8);
    sdes.extend_from_slice(&cname);
    sdes.push(0);
    while !sdes.len().is_multiple_of(4) {
        sdes.push(0);
    }
    header(&mut out, 1, 202, sdes.len());
    out.extend_from_slice(&sdes);
    out
}

/// BYE for our source (RFC 3550 §6.6), as its own compound with an empty RR first.
pub fn bye(ssrc: u32, cname: &str) -> Vec<u8> {
    let mut out = compound(ssrc, None, &[], cname);
    header(&mut out, 1, 203, 4);
    out.extend_from_slice(&ssrc.to_be_bytes());
    out
}

/// Walks a (compound or simple, RT-20) RTCP packet; unknown types are skipped.
pub fn parse_rtcp(p: &[u8]) -> Result<Vec<Rtcp>, Error> {
    if p.len() > MAX_PACKET {
        return Err(Error::TooLarge);
    }
    let mut out = Vec::new();
    let mut off = 0;
    for _ in 0..MAX_COMPOUND {
        if off >= p.len() {
            break;
        }
        let b0 = *p.get(off).ok_or(Error::Malformed)?;
        if b0 >> 6 != 2 {
            return Err(Error::Version);
        }
        let count = usize::from(b0 & 0x1f);
        let pt = *p.get(off + 1).ok_or(Error::Malformed)?;
        let len = (usize::from(be16(p, off + 2)?) + 1) * 4;
        let pkt = p.get(off..off + len).ok_or(Error::Malformed)?;
        match pt {
            200 | 201 => {
                let ssrc = be32(pkt, 4)?;
                let (info, first) = if pt == 200 {
                    let info = SenderInfo {
                        ntp: (u64::from(be32(pkt, 8)?) << 32) | u64::from(be32(pkt, 12)?),
                        rtp_timestamp: be32(pkt, 16)?,
                        packets: be32(pkt, 20)?,
                        octets: be32(pkt, 24)?,
                    };
                    (Some(info), 28)
                } else {
                    (None, 8)
                };
                let reports = (0..count.min(MAX_REPORTS))
                    .map(|k| {
                        let at = first + 24 * k;
                        pkt.get(at..at + 24)
                            .ok_or(Error::Malformed)
                            .and_then(ReportBlock::read)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                out.push(match info {
                    Some(info) => Rtcp::SenderReport {
                        ssrc,
                        info,
                        reports,
                    },
                    None => Rtcp::ReceiverReport { ssrc, reports },
                });
            }
            202 => {
                // First chunk only; the CNAME item if present.
                if count > 0 {
                    let ssrc = be32(pkt, 4)?;
                    let mut at = 8;
                    while let (Some(&kind), Some(&l)) = (pkt.get(at), pkt.get(at + 1)) {
                        if kind == 0 {
                            break;
                        }
                        let text = pkt
                            .get(at + 2..at + 2 + usize::from(l))
                            .ok_or(Error::Malformed)?;
                        if kind == 1 {
                            let cname: String = String::from_utf8_lossy(text)
                                .chars()
                                .filter(|c| !c.is_control())
                                .collect();
                            out.push(Rtcp::Cname { ssrc, cname });
                            break;
                        }
                        at += 2 + usize::from(l);
                    }
                }
            }
            203 => {
                let sources = (0..count)
                    .map(|k| be32(pkt, 4 + 4 * k))
                    .collect::<Result<Vec<_>, _>>()?;
                out.push(Rtcp::Bye { sources });
            }
            _ => {}
        }
        off += len;
    }
    Ok(out)
}

// ----- telephone events (RFC 4733, MS-DTMF) ----------------------------------------------------

/// DTMF events 0 … 9, * (10), # (11), A … D (12 … 15), flash (16) (PL-30, RFC 4733 §3.2).
pub fn dtmf_event(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        '*' => Some(10),
        '#' => Some(11),
        'A'..='D' => Some(c as u8 - b'A' + 12),
        'a'..='d' => Some(c as u8 - b'a' + 12),
        _ => None,
    }
}

pub fn dtmf_char(event: u8) -> Option<char> {
    "0123456789*#ABCD!".chars().nth(usize::from(event))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelephoneEvent {
    pub event: u8,
    pub end: bool,
    /// Power level in −dBm0 (0 … 63).
    pub volume: u8,
    /// Duration in 8000 Hz units (MS-DTMF: clock fixed at 8000 Hz).
    pub duration: u16,
}

impl TelephoneEvent {
    pub fn encode(&self) -> [u8; 4] {
        let d = self.duration.to_be_bytes();
        [
            self.event,
            (u8::from(self.end) << 7) | (self.volume & 0x3f),
            d[0],
            d[1],
        ]
    }

    /// One event per packet (MS-DTMF §3.2); events above 16 and zero durations are refused
    /// (MS-DTMF §3.3, RFC 4733 §2.3.5).
    pub fn decode(p: &[u8]) -> Option<Self> {
        let [event, flags, d0, d1] = <[u8; 4]>::try_from(p).ok()?;
        let duration = u16::from_be_bytes([d0, d1]);
        (event <= 16 && duration != 0).then_some(TelephoneEvent {
            event,
            end: flags & 0x80 != 0,
            volume: flags & 0x3f,
            duration,
        })
    }
}

/// The packets of one DTMF digit (RFC 4733 §2.5.1.2/§2.5.1.4): payloads with marker on the
/// first, cumulative durations every `step` units, the final packet with E set three times. All
/// share the start timestamp; the caller sends them at the packet interval.
pub fn dtmf_packets(event: u8, total: u16, step: u16, volume: u8) -> Vec<(bool, [u8; 4])> {
    let step = step.max(1);
    let total = total.max(step);
    let mut out = Vec::new();
    let mut d = step;
    while d < total {
        let e = TelephoneEvent {
            event,
            end: false,
            volume,
            duration: d,
        };
        out.push((out.is_empty(), e.encode()));
        d = d.saturating_add(step);
    }
    let last = TelephoneEvent {
        event,
        end: true,
        volume,
        duration: total,
    };
    for _ in 0..3 {
        out.push((out.is_empty(), last.encode()));
    }
    out
}

#[cfg(test)]
mod tests;
