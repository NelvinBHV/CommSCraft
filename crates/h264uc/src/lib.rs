//! H.264 UC RTP payload (function 28; [MS-H264PF] 20250520 – PL-01 … PL-09; [RFC6184] §5–§7;
//! [RFC6190] §4.9): one layer per RTP session (MST), packetization mode NI-TC with single NAL
//! unit packets, STAP-A and FU-A only; every layer of every access unit starts with a PACSI NAL
//! unit, and the PACSI of an IDR access unit (and the very first one) carries the full stream
//! layout SEI message. The depacketizer applies the discard rules of MS-H264PF §3.2.5.1.
//!
//! Sans-I/O: access units in Annex B byte stream format in, RTP payloads with their marker bit
//! out, and back. RTP headers, sequence numbers and timestamps (90 kHz) belong to the caller.
//!
//! Security by design: every length is checked before it is used; access units, NAL units per
//! access unit and fragment reassembly are bounded; malformed or out-of-order input discards the
//! access unit instead of passing partial data to a decoder.

/// RTP clock rate of H.264 UC (PL-01; §2.2.1, p. 9).
pub const CLOCK_RATE: u32 = 90_000;
/// NAL unit types used here ([RFC6184] §5.2, [RFC6190] §4.9).
pub const NAL_IDR: u8 = 5;
pub const NAL_SEI: u8 = 6;
/// Prefix NAL unit of SVC ([MSFT-H264UCConfig] §7.1: before the first slice of every frame).
pub const NAL_PREFIX: u8 = 14;
pub const NAL_STAP_A: u8 = 24;
pub const NAL_FU_A: u8 = 28;
pub const NAL_PACSI: u8 = 30;
/// UUID of the stream layout SEI message (PL-03; §2.2.5, p. 11).
pub const STREAM_LAYOUT_UUID: [u8; 16] = [
    0x13, 0x9f, 0xb1, 0xa9, 0x44, 0x6a, 0x4d, 0xec, 0x8c, 0xbf, 0x65, 0xb1, 0xe1, 0x2d, 0x2c, 0xfd,
];
/// Largest access unit accepted or produced (design).
pub const MAX_ACCESS_UNIT: usize = 2 * 1024 * 1024;
/// Most NAL units in one access unit (design).
pub const MAX_NALS: usize = 1024;
/// Smallest MTU the packetizer works with (design: room for a PACSI with a stream layout of a
/// few layers and FU-A fragments of useful size).
pub const MIN_MTU: usize = 200;
/// Most layer descriptions in a stream layout: the SEI payloadSize is one byte here
/// (26 + 16 × n < 255).
pub const MAX_LAYERS: usize = 14;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No NAL unit in the access unit.
    Empty,
    TooLarge,
    BadMtu,
    /// Input that does not follow the format (the reason names the rule).
    Malformed(&'static str),
}

/// The type of a NAL unit (low five bits of its header octet).
pub fn nal_type(nal: &[u8]) -> Option<u8> {
    nal.first().map(|b| b & 0x1f)
}

fn nri(nal: &[u8]) -> u8 {
    nal.first().map(|b| (b >> 5) & 0x03).unwrap_or(0)
}

/// The NAL units of an access unit in Annex B byte stream format (start codes `00 00 01` or
/// `00 00 00 01`); trailing zero bytes before a start code are dropped.
pub fn split_annex_b(au: &[u8]) -> Result<Vec<&[u8]>, Error> {
    if au.len() > MAX_ACCESS_UNIT {
        return Err(Error::TooLarge);
    }
    // Positions just after each start code.
    let mut starts = Vec::new();
    let mut i = 0;
    while i + 3 <= au.len() {
        if au.get(i..i + 3) == Some(&[0, 0, 1][..]) {
            starts.push(i + 3);
            i += 3;
        } else {
            i += 1;
        }
    }
    let mut nals = Vec::new();
    for (k, &s) in starts.iter().enumerate() {
        let end = match starts.get(k + 1) {
            Some(&next) => next - 3,
            None => au.len(),
        };
        let mut nal = au.get(s..end).ok_or(Error::Malformed("start code"))?;
        while let Some((&0, rest)) = nal.split_last() {
            nal = rest;
        }
        if !nal.is_empty() {
            if nals.len() >= MAX_NALS {
                return Err(Error::TooLarge);
            }
            nals.push(nal);
        }
    }
    if nals.is_empty() {
        return Err(Error::Empty);
    }
    Ok(nals)
}

/// One layer of the stream layout (PL-03; §2.2.5, pp. 11–12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerDescription {
    pub coded_width: u16,
    pub coded_height: u16,
    pub display_width: u16,
    pub display_height: u16,
    /// Target bit rate in bits per second.
    pub bitrate: u32,
    /// Index into the frame rate table: 0 = 7.5, 1 = 12.5, 2 = 15, 3 = 25, 4 = 30, 5 = 50,
    /// 6 = 60 fps.
    pub fps_idx: u8,
    /// 0 = base layer, 1 = temporal layer.
    pub layer_type: u8,
    /// Priority ID (0 … 63) of the RTP stream this describes.
    pub prid: u8,
    /// Coded in constrained baseline profile.
    pub constrained_baseline: bool,
}

/// The frame rate table index for a frame rate, if it is one of the defined values.
pub fn fps_index(fps_times_10: u32) -> Option<u8> {
    match fps_times_10 {
        75 => Some(0),
        125 => Some(1),
        150 => Some(2),
        250 => Some(3),
        300 => Some(4),
        500 => Some(5),
        600 => Some(6),
        _ => None,
    }
}

/// A stream layout: which priority IDs are present and, for a full layout, their descriptions
/// (§2.2.5.1, p. 13).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamLayout {
    /// Bit n set = the layer with PRID n is present.
    pub present: u64,
    /// Empty for an update layout.
    pub layers: Vec<LayerDescription>,
}

impl StreamLayout {
    /// A full layout of the given layers (each present).
    pub fn full(layers: Vec<LayerDescription>) -> Result<StreamLayout, Error> {
        if layers.is_empty() || layers.len() > MAX_LAYERS {
            return Err(Error::Malformed("layer count"));
        }
        let mut present = 0u64;
        for l in &layers {
            if l.prid > 63 || l.fps_idx > 6 || l.layer_type > 1 {
                return Err(Error::Malformed("layer description"));
            }
            if present & (1 << l.prid) != 0 {
                return Err(Error::Malformed("duplicate PRID"));
            }
            present |= 1 << l.prid;
        }
        Ok(StreamLayout { present, layers })
    }

    pub fn is_full(&self) -> bool {
        !self.layers.is_empty()
    }

    /// The description of a priority ID.
    pub fn layer(&self, prid: u8) -> Option<&LayerDescription> {
        self.layers.iter().find(|l| l.prid == prid)
    }

    /// The SEI NAL unit (§2.2.5, pp. 10–12; example §4.1, p. 27): user data unregistered,
    /// without emulation prevention and trailing bits.
    pub fn to_sei(&self) -> Result<Vec<u8>, Error> {
        if self.layers.len() > MAX_LAYERS {
            return Err(Error::Malformed("layer count"));
        }
        let size = 16
            + 8
            + 1
            + if self.is_full() {
                1 + 16 * self.layers.len()
            } else {
                0
            };
        let size = u8::try_from(size).map_err(|_| Error::Malformed("layer count"))?;
        let mut out = vec![NAL_SEI, 5, size];
        out.extend_from_slice(&STREAM_LAYOUT_UUID);
        out.extend_from_slice(&self.present.to_le_bytes());
        out.push(u8::from(self.is_full()));
        if self.is_full() {
            out.push(16);
            for l in &self.layers {
                out.extend_from_slice(&l.coded_width.to_be_bytes());
                out.extend_from_slice(&l.coded_height.to_be_bytes());
                out.extend_from_slice(&l.display_width.to_be_bytes());
                out.extend_from_slice(&l.display_height.to_be_bytes());
                out.extend_from_slice(&l.bitrate.to_be_bytes());
                out.push((l.fps_idx << 3) | (l.layer_type & 0x07));
                out.push((l.prid << 2) | (u8::from(l.constrained_baseline) << 1));
                out.extend_from_slice(&[0, 0]);
            }
        }
        Ok(out)
    }

    /// Reads a stream layout SEI NAL unit; `None` if the SEI is another message (other UUID or
    /// payload type).
    pub fn parse_sei(sei: &[u8]) -> Result<Option<StreamLayout>, Error> {
        let bad = Error::Malformed("stream layout");
        if nal_type(sei) != Some(NAL_SEI) {
            return Err(bad);
        }
        if sei.get(1) != Some(&5) || sei.get(3..19) != Some(&STREAM_LAYOUT_UUID[..]) {
            return Ok(None);
        }
        let size = usize::from(*sei.get(2).ok_or(bad.clone())?);
        let body = sei.get(3..3 + size).ok_or(bad.clone())?;
        let lpb: [u8; 8] = body
            .get(16..24)
            .and_then(|b| b.try_into().ok())
            .ok_or(bad.clone())?;
        let present = u64::from_le_bytes(lpb);
        let p = body.get(24).ok_or(bad.clone())? & 1 == 1;
        if !p {
            return Ok(Some(StreamLayout {
                present,
                layers: Vec::new(),
            }));
        }
        let ld_size = usize::from(*body.get(25).ok_or(bad.clone())?);
        if ld_size < 16 {
            return Err(bad);
        }
        let table = body.get(26..).ok_or(bad.clone())?;
        let mut layers = Vec::new();
        for d in table.chunks(ld_size) {
            let u16_at = |i: usize| {
                d.get(i..i + 2)
                    .and_then(|b| b.try_into().ok())
                    .map(u16::from_be_bytes)
            };
            let bitrate = d
                .get(8..12)
                .and_then(|b| b.try_into().ok())
                .map(u32::from_be_bytes);
            let (Some(cw), Some(ch), Some(dw), Some(dh), Some(bitrate), Some(&b12), Some(&b13)) = (
                u16_at(0),
                u16_at(2),
                u16_at(4),
                u16_at(6),
                bitrate,
                d.get(12),
                d.get(13),
            ) else {
                return Err(bad);
            };
            if layers.len() >= MAX_LAYERS * 4 {
                return Err(bad);
            }
            layers.push(LayerDescription {
                coded_width: cw,
                coded_height: ch,
                display_width: dw,
                display_height: dh,
                bitrate,
                fps_idx: b12 >> 3,
                layer_type: b12 & 0x07,
                prid: b13 >> 2,
                constrained_baseline: b13 & 0x02 != 0,
            });
        }
        // §2.2.5.1: every present bit has a description.
        if layers.is_empty()
            || (0..64u8).any(|p| present & (1 << p) != 0 && !layers.iter().any(|l| l.prid == p))
        {
            return Err(bad);
        }
        Ok(Some(StreamLayout { present, layers }))
    }
}

/// What a PACSI NAL unit says about the NAL units it precedes ([RFC6190] §4.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pacsi {
    pub nri: u8,
    /// I bit: an IDR picture follows.
    pub idr: bool,
    pub prid: u8,
    /// CS-DON of the first NAL unit that follows (T bit set).
    pub donc: Option<u16>,
    /// S and E bits: first / last VCL NAL unit of the layer representation follows.
    pub start: bool,
    pub end: bool,
    /// A stream layout SEI message in the PACSI, if any.
    pub layout: Option<StreamLayout>,
}

impl Pacsi {
    /// The PACSI NAL unit for an H.264 base layer (single layer, PL-08): R = 1, N = 1 (no
    /// inter-layer prediction in the base layer – inference), DID = QID = TID = 0, U = D = 0,
    /// O = 1, RR = 3; X = 1 with A = C = idr and P = 0, Y = 0, T = 1 (NI-TC carries the CS-DON in
    /// DONC, [RFC6190] §6.2).
    pub fn to_nal(&self) -> Result<Vec<u8>, Error> {
        if self.prid > 63 || self.nri > 3 {
            return Err(Error::Malformed("PACSI fields"));
        }
        let idr = u8::from(self.idr);
        let mut out = vec![
            (self.nri << 5) | NAL_PACSI,
            0x80 | (idr << 6) | self.prid,
            0x80,
            // TID 0, U 0, D 1 and O 1 as the prefix NAL unit that follows (UCConfig §7.1), RR 3.
            0x0f,
        ];
        let t = u8::from(self.donc.is_some());
        out.push(
            0x80 | (t << 5)
                | (idr << 4)
                | (idr << 2)
                | (u8::from(self.start) << 1)
                | u8::from(self.end),
        );
        if let Some(d) = self.donc {
            out.extend_from_slice(&d.to_be_bytes());
        }
        if let Some(l) = &self.layout {
            let sei = l.to_sei()?;
            let len = u16::try_from(sei.len()).map_err(|_| Error::TooLarge)?;
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(&sei);
        }
        Ok(out)
    }

    /// Reads a PACSI NAL unit; SEI NAL units other than the stream layout are skipped.
    pub fn parse(nal: &[u8]) -> Result<Pacsi, Error> {
        let bad = Error::Malformed("PACSI");
        if nal_type(nal) != Some(NAL_PACSI) {
            return Err(bad);
        }
        let b1 = *nal.get(1).ok_or(bad.clone())?;
        let flags = *nal.get(4).ok_or(bad.clone())?;
        let (y, t) = (flags & 0x40 != 0, flags & 0x20 != 0);
        let mut at = 5;
        if y {
            at += 3; // TL0PICIDX, IDRPICID
        }
        let donc = if t {
            let d = nal
                .get(at..at + 2)
                .and_then(|b| b.try_into().ok())
                .map(u16::from_be_bytes)
                .ok_or(bad.clone())?;
            at += 2;
            Some(d)
        } else {
            None
        };
        let mut layout = None;
        let mut rest = nal.get(at..).ok_or(bad.clone())?;
        while !rest.is_empty() {
            let len = rest
                .get(..2)
                .and_then(|b| b.try_into().ok())
                .map(u16::from_be_bytes)
                .ok_or(bad.clone())?;
            let len = usize::from(len);
            let sei = rest.get(2..2 + len).ok_or(bad.clone())?;
            if nal_type(sei) == Some(NAL_SEI) {
                if let Some(l) = StreamLayout::parse_sei(sei)? {
                    layout = Some(l);
                }
            }
            rest = rest.get(2 + len..).ok_or(bad.clone())?;
        }
        Ok(Pacsi {
            nri: nri(nal),
            idr: b1 & 0x40 != 0,
            prid: b1 & 0x3f,
            donc,
            start: flags & 0x02 != 0,
            end: flags & 0x01 != 0,
            layout,
        })
    }
}

/// One RTP payload of an access unit and its marker bit (PL-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    pub data: Vec<u8>,
    pub marker: bool,
}

/// Packetizer of one layer (one RTP session, PL-02, PL-08).
#[derive(Debug, Clone)]
pub struct Packetizer {
    mtu: usize,
    prid: u8,
    layout: StreamLayout,
    layout_sent: bool,
    /// CS-DON of the next NAL unit in decoding order.
    don: u16,
}

fn is_vcl(t: u8) -> bool {
    (1..=5).contains(&t)
}

/// The prefix NAL unit before the first slice of a frame (PL-13; [MSFT-H264UCConfig] V1.1 §7.1,
/// p. 11: PRID of the stream, DID = QID = TID = 0, no_inter_layer_pred = discardable = output = 1,
/// use_ref_base_pic = 0; the MCU tells the streams apart by it). Its body (H.264 Annex G
/// prefix_nal_unit_svc – inference, the standard is not in the package): with nal_ref_idc ≠ 0
/// store_ref_base_pic_flag 0, additional_prefix_nal_unit_extension_flag 0 and the trailing bits
/// (0x20); otherwise empty.
pub fn prefix_nal(slice: &[u8], prid: u8) -> Vec<u8> {
    let nri = nri(slice);
    let idr = u8::from(nal_type(slice) == Some(NAL_IDR));
    let mut p = vec![
        (nri << 5) | NAL_PREFIX,
        0x80 | (idr << 6) | (prid & 0x3f),
        0x80,
        0x0f,
    ];
    if nri != 0 {
        p.push(0x20);
    }
    p
}

impl Packetizer {
    /// `mtu` is the largest RTP payload; `layout` must be a full layout describing `prid`.
    pub fn new(mtu: usize, prid: u8, layout: StreamLayout) -> Result<Packetizer, Error> {
        if !(MIN_MTU..=65_535).contains(&mtu) {
            return Err(Error::BadMtu);
        }
        if !layout.is_full() || layout.layer(prid).is_none() {
            return Err(Error::Malformed("layout without this layer"));
        }
        Ok(Packetizer {
            mtu,
            prid,
            layout,
            layout_sent: false,
            don: 0,
        })
    }

    /// Another stream layout (e.g. another size after a video source request); it goes out in
    /// full with the next access unit. The decoding order numbers continue.
    pub fn set_layout(&mut self, layout: StreamLayout) -> Result<(), Error> {
        if !layout.is_full() || layout.layer(self.prid).is_none() {
            return Err(Error::Malformed("layout without this layer"));
        }
        self.layout = layout;
        self.layout_sent = false;
        Ok(())
    }

    /// The RTP payloads of one access unit (Annex B), in order; the last one has the marker.
    pub fn packetize(&mut self, access_unit: &[u8]) -> Result<Vec<Payload>, Error> {
        let input = split_annex_b(access_unit)?;
        if input
            .iter()
            .any(|n| matches!(nal_type(n), Some(0 | 14 | 20 | 24..=31)))
        {
            return Err(Error::Malformed("NAL unit type not allowed in the input"));
        }
        // PL-13: a prefix NAL unit before the first slice of the frame.
        let first_slice = input.iter().position(|n| nal_type(n).is_some_and(is_vcl));
        let prefix = first_slice
            .and_then(|i| input.get(i))
            .map(|s| prefix_nal(s, self.prid));
        let mut nals: Vec<&[u8]> = Vec::with_capacity(input.len() + 1);
        for (i, n) in input.iter().enumerate() {
            if Some(i) == first_slice {
                if let Some(p) = &prefix {
                    nals.push(p);
                }
            }
            nals.push(n);
        }
        let idr = nals.iter().any(|n| nal_type(n) == Some(NAL_IDR));
        // §2.2.5.1: the first PACSI and the one of every IDR carry the full layout.
        let layout = (idr || !self.layout_sent).then(|| self.layout.clone());
        let last_vcl = nals.iter().rposition(|n| nal_type(n).is_some_and(is_vcl));
        // [RFC6190] §4.9: S = the first NAL unit after the PACSI is the first VCL NAL unit of
        // the layer representation (not so when SPS/PPS lead), E = the last one after it is the
        // last VCL NAL unit.
        let first_is_vcl = nals.first().and_then(|n| nal_type(n)).is_some_and(is_vcl);
        let mut pacsi = Pacsi {
            nri: 0,
            idr,
            prid: self.prid,
            donc: Some(self.don),
            start: first_is_vcl,
            end: false,
            layout,
        };
        let pacsi_len = pacsi.to_nal()?.len();
        if 1 + 2 + pacsi_len > self.mtu {
            return Err(Error::BadMtu);
        }
        // How many leading NAL units fit into one STAP-A behind the PACSI.
        let mut size = 1 + 2 + pacsi_len;
        let mut k = 0;
        for n in &nals {
            if size + 2 + n.len() > self.mtu {
                break;
            }
            size += 2 + n.len();
            k += 1;
        }
        let mut out = Vec::new();
        if k == 0 {
            // PACSI on its own, associated with the next NAL unit ([RFC6190] §4.9).
            let first = nals.first().ok_or(Error::Empty)?;
            pacsi.nri = nri(first);
            pacsi.end = last_vcl == Some(0);
            out.push(pacsi.to_nal()?);
        } else {
            let group = nals.get(..k).ok_or(Error::Empty)?;
            pacsi.nri = group.iter().map(|n| nri(n)).max().unwrap_or(0);
            pacsi.end = last_vcl == Some(k - 1);
            let mut stap = vec![pacsi.nri << 5 | NAL_STAP_A];
            push_aggregated(&mut stap, &pacsi.to_nal()?)?;
            for n in group {
                push_aggregated(&mut stap, n)?;
            }
            out.push(stap);
        }
        let mut rest = nals.get(k..).unwrap_or(&[]);
        while let Some((first, tail)) = rest.split_first() {
            if first.len() > self.mtu {
                out.extend(fu_a(first, self.mtu)?);
                rest = tail;
                continue;
            }
            // Aggregate following small NAL units into a STAP-A while they fit.
            let mut size = 1 + 2 + first.len();
            let mut m = 1;
            for n in tail {
                if size + 2 + n.len() > self.mtu {
                    break;
                }
                size += 2 + n.len();
                m += 1;
            }
            if m == 1 {
                out.push(first.to_vec());
            } else {
                let group = rest.get(..m).unwrap_or(&[]);
                let mut stap =
                    vec![group.iter().map(|n| nri(n)).max().unwrap_or(0) << 5 | NAL_STAP_A];
                for n in group {
                    push_aggregated(&mut stap, n)?;
                }
                out.push(stap);
            }
            rest = rest.get(m..).unwrap_or(&[]);
        }
        if idr || !self.layout_sent {
            self.layout_sent = true;
        }
        let count = u16::try_from(nals.len()).map_err(|_| Error::TooLarge)?;
        self.don = self.don.wrapping_add(count);
        let n = out.len();
        Ok(out
            .into_iter()
            .enumerate()
            .map(|(i, data)| Payload {
                data,
                marker: i + 1 == n,
            })
            .collect())
    }
}

fn push_aggregated(stap: &mut Vec<u8>, nal: &[u8]) -> Result<(), Error> {
    let len = u16::try_from(nal.len()).map_err(|_| Error::TooLarge)?;
    stap.extend_from_slice(&len.to_be_bytes());
    stap.extend_from_slice(nal);
    Ok(())
}

/// FU-A fragments of a NAL unit larger than the MTU ([RFC6184] §5.8).
fn fu_a(nal: &[u8], mtu: usize) -> Result<Vec<Vec<u8>>, Error> {
    let (&header, body) = nal.split_first().ok_or(Error::Empty)?;
    let indicator = (header & 0xe0) | NAL_FU_A;
    let kind = header & 0x1f;
    let chunk = mtu - 2;
    let pieces: Vec<&[u8]> = body.chunks(chunk).collect();
    let n = pieces.len();
    Ok(pieces
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let s = if i == 0 { 0x80 } else { 0 };
            let e = if i + 1 == n { 0x40 } else { 0 };
            let mut f = Vec::with_capacity(p.len() + 2);
            f.push(indicator);
            f.push(s | e | kind);
            f.extend_from_slice(p);
            f
        })
        .collect())
}

/// One received RTP packet of an H.264 UC stream (one SSRC).
#[derive(Debug, Clone, Copy)]
pub struct RtpIn<'a> {
    pub seq: u16,
    pub timestamp: u32,
    pub marker: bool,
    pub payload: &'a [u8],
}

/// What the depacketizer hands on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// A complete access unit in Annex B format for the decoder.
    AccessUnit {
        timestamp: u32,
        data: Vec<u8>,
        idr: bool,
    },
    /// An access unit was lost or discarded (the caller may ask for a key frame).
    Lost { timestamp: u32 },
}

#[derive(Debug)]
struct Pending {
    timestamp: u32,
    next_seq: u16,
    nals: Vec<Vec<u8>>,
    size: usize,
    fu: Option<Vec<u8>>,
    ok: bool,
    idr: bool,
}

/// Depacketizer of one RTP stream (PL-04, PL-09; §3.2.5.1, p. 24; [RFC6184] §7.1).
#[derive(Debug, Default)]
pub struct Depacketizer {
    layout: Option<StreamLayout>,
    cur: Option<Pending>,
}

impl Depacketizer {
    pub fn new() -> Depacketizer {
        Depacketizer::default()
    }

    /// The last full stream layout received, with updates applied.
    pub fn layout(&self) -> Option<&StreamLayout> {
        self.layout.as_ref()
    }

    /// Feeds one packet; returns what became complete or was given up.
    pub fn push(&mut self, p: RtpIn<'_>) -> Vec<Output> {
        let mut out = Vec::new();
        if self
            .cur
            .as_ref()
            .is_some_and(|c| c.timestamp != p.timestamp)
        {
            // A new access unit before the marker: the previous one is incomplete.
            if let Some(c) = self.cur.take() {
                out.push(Output::Lost {
                    timestamp: c.timestamp,
                });
            }
        }
        let fresh = self.cur.is_none();
        let cur = self.cur.get_or_insert_with(|| Pending {
            timestamp: p.timestamp,
            next_seq: p.seq,
            nals: Vec::new(),
            size: 0,
            fu: None,
            ok: true,
            idr: false,
        });
        if cur.next_seq != p.seq {
            cur.ok = false; // a packet of this access unit is missing
        }
        cur.next_seq = p.seq.wrapping_add(1);
        if cur.ok {
            let first_is_pacsi = match nal_type(p.payload) {
                Some(NAL_PACSI) => true,
                Some(NAL_STAP_A) => p.payload.get(3).is_some_and(|b| b & 0x1f == NAL_PACSI),
                _ => false,
            };
            // §3.2.5.1: the first packet of a timestamp starts with a PACSI.
            if fresh && !first_is_pacsi {
                cur.ok = false;
            } else if let Err(()) = self.take_payload(p.payload) {
                if let Some(c) = self.cur.as_mut() {
                    c.ok = false;
                }
            }
        }
        if p.marker {
            if let Some(c) = self.cur.take() {
                let complete = c.ok && c.fu.is_none() && !c.nals.is_empty();
                if complete {
                    let mut data = Vec::with_capacity(c.size + 4 * c.nals.len());
                    // Prefix NAL units are for the MCU; an AVC decoder discards them anyway.
                    for n in c.nals.iter().filter(|n| nal_type(n) != Some(NAL_PREFIX)) {
                        data.extend_from_slice(&[0, 0, 0, 1]);
                        data.extend_from_slice(n);
                    }
                    out.push(Output::AccessUnit {
                        timestamp: c.timestamp,
                        data,
                        idr: c.idr,
                    });
                } else {
                    out.push(Output::Lost {
                        timestamp: c.timestamp,
                    });
                }
            }
        }
        out
    }

    fn take_payload(&mut self, payload: &[u8]) -> Result<(), ()> {
        match nal_type(payload) {
            Some(NAL_STAP_A) => {
                let mut rest = payload.get(1..).ok_or(())?;
                if rest.is_empty() {
                    return Err(());
                }
                while !rest.is_empty() {
                    let len = rest
                        .get(..2)
                        .and_then(|b| b.try_into().ok())
                        .map(u16::from_be_bytes)
                        .ok_or(())?;
                    let len = usize::from(len);
                    let nal = rest.get(2..2 + len).ok_or(())?;
                    if len == 0 || nal_type(nal).is_some_and(|t| matches!(t, 24..=29 | 31)) {
                        return Err(());
                    }
                    self.take_nal(nal)?;
                    rest = rest.get(2 + len..).ok_or(())?;
                }
                Ok(())
            }
            Some(NAL_FU_A) => self.take_fragment(payload),
            Some(1..=23) | Some(NAL_PACSI) => self.take_nal(payload),
            _ => Err(()), // STAP-B, MTAP, FU-B and reserved types are not allowed (PL-02)
        }
    }

    fn take_nal(&mut self, nal: &[u8]) -> Result<(), ()> {
        if nal_type(nal) == Some(NAL_PACSI) {
            let pacsi = Pacsi::parse(nal).map_err(|_| ())?;
            if let Some(l) = pacsi.layout {
                if l.is_full() {
                    self.layout = Some(l);
                } else if let Some(full) = &mut self.layout {
                    // §2.2.5.1: an update may only clear layers or set known ones.
                    if l.present & !full.layers.iter().fold(0u64, |a, d| a | (1 << d.prid)) != 0 {
                        return Err(());
                    }
                    full.present = l.present;
                }
            }
            // §3.2.5.1: a full layout must be known, and this layer must be in it.
            let known = self.layout.as_ref().is_some_and(|l| {
                l.present & (1u64 << pacsi.prid) != 0 && l.layer(pacsi.prid).is_some()
            });
            if !known {
                return Err(());
            }
            if let Some(c) = self.cur.as_mut() {
                c.idr |= pacsi.idr;
            }
            return Ok(());
        }
        let c = self.cur.as_mut().ok_or(())?;
        if c.fu.is_some() || c.nals.len() >= MAX_NALS || c.size + nal.len() > MAX_ACCESS_UNIT {
            return Err(());
        }
        c.idr |= nal_type(nal) == Some(NAL_IDR);
        c.size += nal.len();
        c.nals.push(nal.to_vec());
        Ok(())
    }

    fn take_fragment(&mut self, payload: &[u8]) -> Result<(), ()> {
        let (&indicator, rest) = payload.split_first().ok_or(())?;
        let (&header, body) = rest.split_first().ok_or(())?;
        let (start, end) = (header & 0x80 != 0, header & 0x40 != 0);
        let kind = header & 0x1f;
        if start && end || !(1..=23).contains(&kind) {
            return Err(());
        }
        let c = self.cur.as_mut().ok_or(())?;
        if c.size + body.len() + 1 > MAX_ACCESS_UNIT {
            return Err(());
        }
        match (&mut c.fu, start) {
            (None, true) => {
                let mut nal = Vec::with_capacity(body.len() + 1);
                nal.push((indicator & 0xe0) | kind);
                nal.extend_from_slice(body);
                c.size += nal.len();
                c.fu = Some(nal);
            }
            (Some(nal), false) => {
                nal.extend_from_slice(body);
                c.size += body.len();
            }
            _ => return Err(()),
        }
        if end {
            let nal = c.fu.take().ok_or(())?;
            if c.nals.len() >= MAX_NALS {
                return Err(());
            }
            c.idr |= kind == NAL_IDR;
            c.nals.push(nal);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
