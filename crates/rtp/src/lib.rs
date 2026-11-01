//! RTP and RTCP for audio (sans-I/O): packet headers, the sending side (SSRC, sequence numbers,
//! timestamps), reception statistics, RTCP SR/RR/SDES/BYE and DTMF telephone events.
//!
//! Evidence: `docs/protocol/evidence/16-rtp-rtcp.md` (RT-01 … RT-10, RT-20, RT-21) and
//! `17-payload-formats-video-audio-appsharing.md` (PL-30). Sources: RFC 3550 §5.1 (header),
//! §6.4.1/6.4.2 (SR/RR), §6.5 (SDES), §6.6 (BYE), Appendix A.1/A.3/A.8 (validity, loss,
//! jitter); RFC 4733 §2.3, §2.5.1.2, §2.5.1.4 (telephone events); [MS-RTP] 20250520 §2.1–2.2.2
//! (pp. 14–17), §2.2.3–2.2.11 (probe, packet pair and train, SDES PRIV "MS-EVT", profile
//! extensions, pp. 16–27, RT-22 … RT-24), §3.1.2 (p. 35); [MS-DTMF] 20250520 §2.2.1, §3.1–3.3 (pp. 8–12); [MS-SRTP] §5.1
//! (start sequence number, p. 15).
//!
//! Security by design: every length is checked before use (packet ≤ 1500 bytes, RT-01), CSRC
//! lists are bounded by the header field, RTCP walking is bounded (at most 16 packets per
//! compound, at most 10 report blocks, RT-21), SDES text is bounded and stripped of control
//! characters; profile extensions are walked by their length field (at least 4, a multiple of 4,
//! inside the report) and at most 20 per report (§2.2.11), unknown types are skipped; nothing
//! here allocates per received octet beyond the packet itself.

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

    /// The sequence number of the next packet.
    pub fn sequence(&self) -> u16 {
        self.sequence
    }

    /// Lync Server 2013 and SfB Server 2015/2019 do not support sequence number 0 on video
    /// ([MS-H264PF] endnote <2>, PL-09): the next packet takes 1 instead.
    pub fn skip_sequence_zero(&mut self) {
        if self.sequence == 0 {
            self.sequence = 1;
        }
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

/// [MS-RTP] §2.2.11 profile-specific extensions after the report blocks of an SR or RR
/// (RT-23). Bandwidths in bits per second unless noted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extension {
    /// Type 1 (§2.2.11.1): the receiver's estimate; −3/−5/−6 are the special values.
    Bandwidth {
        ssrc: u32,
        bps: u32,
        confidence: Option<u8>,
    },
    /// Type 4 (§2.2.11.2): a lost video packet.
    PacketLoss { sequence: u16 },
    /// Type 5 (§2.2.11.3).
    VideoPreference { width: u16, height: u16 },
    /// Type 6 (§2.2.11.4): `n` padding words of 4 octets.
    Padding { words: u16 },
    /// Type 7 (§2.2.11.5).
    PolicyBandwidth(u32),
    /// Type 8 (§2.2.11.6).
    TurnBandwidth(u32),
    /// Type 9 (§2.2.11.7): frames of 10 ms; quality 0 unknown, 1 good, 2 poor, 3 bad.
    AudioHealer {
        ssrc: u32,
        concealed: u32,
        stretched: u32,
        compressed: u32,
        total: u32,
        quality: u8,
        fec_distance: u8,
    },
    /// Type 10 (§2.2.11.8).
    ReceiverLimit(u32),
    /// Type 11 (§2.2.11.9).
    PacketTrain {
        ssrc: u32,
        last: bool,
        index: u8,
        count: u8,
        bytes: u16,
    },
    /// Type 12 (§2.2.11.10).
    PeerInfo {
        ssrc: u32,
        inbound: u32,
        outbound: u32,
        no_cache: bool,
    },
    /// Type 13 (§2.2.11.11): NTP timestamp and the congestion bit mask.
    Congestion { ntp: u64, info: u8 },
    /// Type 14 (§2.2.11.12): modality 2 = video.
    ModalityLimit { modality: u8, bps: u32 },
}

/// Bandwidth estimate special values (§2.2.11.1).
pub const BANDWIDTH_PACKET_PAIR: u32 = 0xffff_fffd;
pub const BANDWIDTH_PACKET_TRAIN: u32 = 0xffff_fffb;
pub const BANDWIDTH_SEND_TRAIN: u32 = 0xffff_fffa;
/// Most profile extensions per report (§2.2.11).
pub const MAX_EXTENSIONS: usize = 20;

impl Extension {
    fn write(&self, out: &mut Vec<u8>) {
        let mut body: Vec<u8> = Vec::new();
        let kind: u16 = match *self {
            Extension::Bandwidth {
                ssrc,
                bps,
                confidence,
            } => {
                body.extend_from_slice(&ssrc.to_be_bytes());
                body.extend_from_slice(&bps.to_be_bytes());
                if let Some(c) = confidence {
                    body.extend_from_slice(&[(c & 0x0f) << 4, 0, 0, 0]);
                }
                1
            }
            Extension::PacketLoss { sequence } => {
                body.extend_from_slice(&[0, 0]);
                body.extend_from_slice(&sequence.to_be_bytes());
                4
            }
            Extension::VideoPreference { width, height } => {
                body.extend_from_slice(&[0; 4]);
                body.extend_from_slice(&width.to_be_bytes());
                body.extend_from_slice(&height.to_be_bytes());
                body.extend_from_slice(&[0; 8]);
                5
            }
            Extension::Padding { words } => {
                body.resize(usize::from(words.min(16382)) * 4, 0);
                6
            }
            Extension::PolicyBandwidth(b)
            | Extension::TurnBandwidth(b)
            | Extension::ReceiverLimit(b) => {
                body.extend_from_slice(&[0; 4]);
                body.extend_from_slice(&b.to_be_bytes());
                match self {
                    Extension::PolicyBandwidth(_) => 7,
                    Extension::TurnBandwidth(_) => 8,
                    _ => 10,
                }
            }
            Extension::AudioHealer {
                ssrc,
                concealed,
                stretched,
                compressed,
                total,
                quality,
                fec_distance,
            } => {
                for v in [ssrc, concealed, stretched, compressed, total] {
                    body.extend_from_slice(&v.to_be_bytes());
                }
                body.extend_from_slice(&[0, 0, quality.min(3), fec_distance.min(3)]);
                9
            }
            Extension::PacketTrain {
                ssrc,
                last,
                index,
                count,
                bytes,
            } => {
                body.extend_from_slice(&ssrc.to_be_bytes());
                body.push((u8::from(last) << 7) | (index & 0x7f));
                body.push(count & 0x7f);
                body.extend_from_slice(&bytes.to_be_bytes());
                11
            }
            Extension::PeerInfo {
                ssrc,
                inbound,
                outbound,
                no_cache,
            } => {
                for v in [ssrc, inbound, outbound] {
                    body.extend_from_slice(&v.to_be_bytes());
                }
                body.extend_from_slice(&[u8::from(no_cache) << 7, 0, 0, 0]);
                12
            }
            Extension::Congestion { ntp, info } => {
                body.extend_from_slice(&ntp.to_be_bytes());
                body.extend_from_slice(&[info & 0x0f, 0, 0, 0]);
                13
            }
            Extension::ModalityLimit { modality, bps } => {
                body.extend_from_slice(&[modality, 0, 0, 0]);
                body.extend_from_slice(&bps.to_be_bytes());
                14
            }
        };
        out.extend_from_slice(&kind.to_be_bytes());
        out.extend_from_slice(&((body.len() + 4) as u16).to_be_bytes());
        out.extend_from_slice(&body);
    }

    /// One extension (`p` = type, length and info); `None` for unknown types or wrong lengths.
    fn read(kind: u16, p: &[u8]) -> Option<Self> {
        let w = |at: usize| be32(p, at).ok();
        let h = |at: usize| be16(p, at).ok();
        let b = |at: usize| p.get(at).copied();
        Some(match (kind, p.len()) {
            (1, 12) => Extension::Bandwidth {
                ssrc: w(4)?,
                bps: w(8)?,
                confidence: None,
            },
            (1, 16) => Extension::Bandwidth {
                ssrc: w(4)?,
                bps: w(8)?,
                confidence: Some(b(12)? >> 4),
            },
            (4, 8) => Extension::PacketLoss { sequence: h(6)? },
            (5, 20) => Extension::VideoPreference {
                width: h(8)?,
                height: h(10)?,
            },
            (6, n) => Extension::Padding {
                words: ((n - 4) / 4) as u16,
            },
            (7, 12) => Extension::PolicyBandwidth(w(8)?),
            (8, 12) => Extension::TurnBandwidth(w(8)?),
            (9, 28) => Extension::AudioHealer {
                ssrc: w(4)?,
                concealed: w(8)?,
                stretched: w(12)?,
                compressed: w(16)?,
                total: w(20)?,
                // Other values mean "unknown" (§2.2.11.7).
                quality: Some(b(26)?).filter(|q| *q <= 3).unwrap_or(0),
                fec_distance: Some(b(27)?).filter(|f| *f <= 3).unwrap_or(0),
            },
            (10, 12) => Extension::ReceiverLimit(w(8)?),
            (11, 12) => Extension::PacketTrain {
                ssrc: w(4)?,
                last: b(8)? & 0x80 != 0,
                index: b(8)? & 0x7f,
                count: b(9)? & 0x7f,
                bytes: h(10)?,
            },
            (12, 20) => Extension::PeerInfo {
                ssrc: w(4)?,
                inbound: w(8)?,
                outbound: w(12)?,
                no_cache: b(16)? & 0x80 != 0,
            },
            (13, 16) => Extension::Congestion {
                ntp: (u64::from(w(4)?) << 32) | u64::from(w(8)?),
                info: b(12)? & 0x0f,
            },
            (14, 12) => Extension::ModalityLimit {
                modality: b(4)?,
                bps: w(8)?,
            },
            _ => return None,
        })
    }
}

/// The extensions of one report (`p` = the octets after the report blocks); bounded and
/// fail-closed: a broken length ends the walk, what came before stays.
fn read_extensions(p: &[u8]) -> Vec<Extension> {
    let mut out = Vec::new();
    let mut at = 0;
    for _ in 0..MAX_EXTENSIONS {
        let (Ok(kind), Ok(len)) = (be16(p, at), be16(p, at + 2)) else {
            break;
        };
        let len = usize::from(len);
        if len < 4 || !len.is_multiple_of(4) {
            break;
        }
        let Some(ext) = p.get(at..at + len) else {
            break;
        };
        if let Some(e) = Extension::read(kind, ext) {
            out.push(e);
        }
        at += len;
    }
    out
}

/// SDES PRIV "MS-EVT" media quality (§2.2.10.1, RT-22): `known` = the m mask, `bad` = the q mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Quality {
    pub known: u32,
    pub bad: u32,
}

impl Quality {
    fn value(&self) -> String {
        format!("v=1 m={:08x} q={:08x}", self.known, self.bad & self.known)
    }

    /// Parses `v=1 m=… q=…`; only the last 8 hex digits count, other fields are ignored.
    fn parse(v: &[u8]) -> Option<Self> {
        let s = std::str::from_utf8(v).ok()?;
        let mut version = None;
        let (mut known, mut bad) = (None, None);
        for field in s.split(' ') {
            let (k, val) = field.split_once('=')?;
            let low = |x: &str| u32::from_str_radix(x.get(x.len().saturating_sub(8)..)?, 16).ok();
            match k {
                "v" => version = Some(val),
                "m" => known = low(val),
                "q" => bad = low(val),
                _ => {}
            }
        }
        if version != Some("1") {
            return None;
        }
        let known = known?;
        Some(Quality {
            known,
            bad: if known == 0 { 0 } else { bad? & known },
        })
    }
}

/// RTCP packet types handled here (others are skipped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rtcp {
    SenderReport {
        ssrc: u32,
        info: SenderInfo,
        reports: Vec<ReportBlock>,
        extensions: Vec<Extension>,
    },
    ReceiverReport {
        ssrc: u32,
        reports: Vec<ReportBlock>,
        extensions: Vec<Extension>,
    },
    /// SDES PRIV "MS-EVT" (RT-22).
    MediaQuality {
        ssrc: u32,
        quality: Quality,
    },
    /// SDES CNAME items (other items are skipped).
    Cname {
        ssrc: u32,
        cname: String,
    },
    Bye {
        sources: Vec<u32>,
    },
    /// Picture loss indication or video source request (RT-25).
    Feedback(feedback::Feedback),
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
    Report {
        ssrc,
        info,
        reports,
        extensions: &[],
        cname,
        quality: None,
    }
    .build()
}

/// A report with [MS-RTP] additions: profile extensions after the report blocks (at most 20)
/// and the SDES PRIV "MS-EVT" item after the CNAME.
#[derive(Debug, Clone, Copy)]
pub struct Report<'a> {
    pub ssrc: u32,
    pub info: Option<SenderInfo>,
    pub reports: &'a [ReportBlock],
    pub extensions: &'a [Extension],
    pub cname: &'a str,
    pub quality: Option<Quality>,
}

/// The SR or RR alone (a simple packet, RT-20), with its extensions.
fn report_packet(
    ssrc: u32,
    info: Option<SenderInfo>,
    reports: &[ReportBlock],
    extensions: &[Extension],
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
    for e in extensions.iter().take(MAX_EXTENSIONS) {
        e.write(&mut body);
    }
    header(&mut out, reports.len(), pt, body.len());
    out.extend_from_slice(&body);
    out
}

impl Report<'_> {
    pub fn build(&self) -> Vec<u8> {
        let mut out = report_packet(self.ssrc, self.info, self.reports, self.extensions);
        sdes(&mut out, self.ssrc, self.cname, self.quality);
        out
    }
}

/// RTCP probe packet (§2.2.3): a simple SR without report blocks, first of a packet pair.
pub fn probe(ssrc: u32, info: SenderInfo) -> Vec<u8> {
    report_packet(ssrc, Some(info), &[], &[])
}

/// A packet train packet (§2.2.6, §2.2.11.9): a simple RR with the train extension, padded with
/// the padding extension to `size` octets (at least 20). `bytes` = the train's octets so far,
/// including this packet.
pub fn train_packet(ssrc: u32, index: u8, count: u8, bytes: u16, size: usize) -> Vec<u8> {
    let train = Extension::PacketTrain {
        ssrc,
        last: index + 1 == count,
        index,
        count,
        bytes,
    };
    // RR header (8) + train extension (12) + padding extension (4 + 4n).
    let words = size.saturating_sub(8 + 12 + 4) / 4;
    let pad = Extension::Padding {
        words: words.min(16382) as u16,
    };
    report_packet(ssrc, None, &[], &[train, pad])
}

/// Pads a report so the packet reaches `size` octets: a padding extension appended to the first
/// SR/RR, whose length field grows accordingly (§2.2.4). Unchanged if it is already larger.
pub fn pad_report(packet: &[u8], size: usize) -> Vec<u8> {
    let missing = size.saturating_sub(packet.len());
    let (Some(&b0), Ok(len)) = (packet.first(), be16(packet, 2)) else {
        return packet.to_vec();
    };
    if missing < 4 || b0 & 0x20 != 0 {
        return packet.to_vec();
    }
    let first = (usize::from(len) + 1) * 4;
    let words = (missing - 4) / 4;
    let mut ext = Vec::new();
    Extension::Padding {
        words: words.min(16382) as u16,
    }
    .write(&mut ext);
    let (Some(head), Some(rest)) = (packet.get(..first), packet.get(first..)) else {
        return packet.to_vec();
    };
    let mut out = head.to_vec();
    out.extend_from_slice(&ext);
    let new_len = ((out.len() / 4 - 1) as u16).to_be_bytes();
    if let Some(l) = out.get_mut(2..4) {
        l.copy_from_slice(&new_len);
    }
    out.extend_from_slice(rest);
    out
}

/// SDES with CNAME, optionally the "MS-EVT" PRIV item, END item, padding to 32 bits.
fn sdes(out: &mut Vec<u8>, ssrc: u32, cname: &str, quality: Option<Quality>) {
    let cname: Vec<u8> = cname
        .bytes()
        .filter(|b| b.is_ascii_graphic())
        .take(254)
        .collect();
    let mut sdes = ssrc.to_be_bytes().to_vec();
    // CNAME text NUL-terminated, the NUL counted in the length ([MS-RTP] §2.2.10, RT-20).
    sdes.push(1);
    sdes.push((cname.len() + 1) as u8);
    sdes.extend_from_slice(&cname);
    sdes.push(0);
    if let Some(q) = quality {
        // PRIV (8): prefix length, prefix "MS-EVT", value; neither NUL-terminated (§2.2.10.1).
        let value = q.value();
        sdes.push(8);
        sdes.push((1 + 6 + value.len()) as u8);
        sdes.push(6);
        sdes.extend_from_slice(b"MS-EVT");
        sdes.extend_from_slice(value.as_bytes());
    }
    sdes.push(0);
    while !sdes.len().is_multiple_of(4) {
        sdes.push(0);
    }
    header(out, 1, 202, sdes.len());
    out.extend_from_slice(&sdes);
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
                // Profile extensions follow the report blocks (§2.2.11); RTCP padding is removed.
                let pad = if b0 & 0x20 != 0 {
                    usize::from(*pkt.last().ok_or(Error::Malformed)?)
                } else {
                    0
                };
                let end = pkt.len().checked_sub(pad).ok_or(Error::Malformed)?;
                let extensions = pkt
                    .get(first + 24 * count..end)
                    .map(read_extensions)
                    .unwrap_or_default();
                out.push(match info {
                    Some(info) => Rtcp::SenderReport {
                        ssrc,
                        info,
                        reports,
                        extensions,
                    },
                    None => Rtcp::ReceiverReport {
                        ssrc,
                        reports,
                        extensions,
                    },
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
                        } else if kind == 8 {
                            // PRIV: prefix length, prefix, value (RT-22).
                            if let Some((&pl, rest)) = text.split_first() {
                                let pl = usize::from(pl);
                                if rest.get(..pl) == Some(&b"MS-EVT"[..]) {
                                    if let Some(quality) = rest.get(pl..).and_then(Quality::parse) {
                                        out.push(Rtcp::MediaQuality { ssrc, quality });
                                    }
                                }
                            }
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
            feedback::PT_PSFB => {
                if let Some(f) = feedback::parse(pkt)? {
                    out.push(Rtcp::Feedback(f));
                }
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

pub mod feedback;

#[cfg(test)]
mod tests;
