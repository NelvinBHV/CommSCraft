//! MS-SIPCOMP: MPPC compression of the SIP stream with an 8 KiB history per direction.
//!
//! Evidence: `docs/protocol/evidence/10-sipcomp-compression.md` (CP-05 … CP-08, K-CP-1, O-CP-1)
//! and `docs/protocol/evidence/39-evidence-for-open-points.md` (OB-01). Sources: [MS-SIPCOMP]
//! 20250520 §3.2.5.1–3.2.5.2 (pp. 14–17); RFC 2118 §4; [MS-RDPBCGR] 20260309 §3.1.8.4.1
//! (pp. 267–268).
//!
//! The 6-byte packet header (CP-05): the flags in the high nibble of the first byte (the bit
//! diagram of §2.2.4, p. 10), type 0 in the low nibble, three reserved bytes and the uncompressed
//! size as a 16-bit number, most significant byte first. The text leaves the byte order open
//! (O-CP-1); it was settled by an interop run (`tested-in-lab`), which also showed that a packet
//! sent without compression carries PACKET_FLUSHED – as the expansion rule of §3.2.5.1.1 (p. 15)
//! does – and that the server refuses compressed client packets before it sent one itself (CP-04).
//!
//! Inference: bits are packed most significant bit first, as the bit strings of RFC 2118 are
//! written; the last byte is padded with zero bits. Every token is at least 8 bits long, so a
//! decoder stops when fewer than 8 bits remain.

/// History buffer size (CP-06).
pub const HISTORY_SIZE: usize = 8192;
/// Compression flags (CP-05; MS-SIPCOMP §2.2.4, p. 10).
pub const PACKET_FLUSHED: u8 = 0x8;
pub const PACKET_AT_FRONT: u8 = 0x4;
pub const PACKET_COMPRESSED: u8 = 0x2;

const MIN_MATCH: usize = 3;
const MAX_LENGTH: usize = 8191;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// PACKET_FLUSHED together with PACKET_COMPRESSED, or a bit outside the three flags. The
    /// receiver must close the TCP connection (CP-08).
    InconsistentFlags(u8),
    /// The bit stream ends inside a token.
    Truncated,
    /// A copy-tuple points before the start of the history.
    OffsetOutOfRange,
    /// A length prefix longer than 11 one-bits.
    BadLength,
    /// The data does not fit into the history buffer.
    HistoryOverflow,
    /// Offset or length outside the encodable range.
    NotEncodable,
    /// A packet header with a type other than 0 (CP-05).
    BadHeader,
}

/// A literal byte or a copy-tuple `<offset, length>` (RFC 2118 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Literal(u8),
    Copy { offset: u16, length: u16 },
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    bits: u32,
}

impl BitWriter {
    fn push(&mut self, value: u32, count: u32) {
        for i in (0..count).rev() {
            if self.bits.is_multiple_of(8) {
                self.bytes.push(0);
            }
            if (value >> i) & 1 != 0 {
                if let Some(last) = self.bytes.last_mut() {
                    *last |= 0x80 >> (self.bits % 8);
                }
            }
            self.bits += 1;
        }
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl BitReader<'_> {
    fn remaining(&self) -> usize {
        self.data.len() * 8 - self.pos
    }

    fn bit(&mut self) -> Result<u32, Error> {
        let byte = self.data.get(self.pos / 8).ok_or(Error::Truncated)?;
        let bit = (byte >> (7 - self.pos % 8)) & 1;
        self.pos += 1;
        Ok(u32::from(bit))
    }

    fn bits(&mut self, count: u32) -> Result<u32, Error> {
        let mut v = 0;
        for _ in 0..count {
            v = (v << 1) | self.bit()?;
        }
        Ok(v)
    }
}

/// Literal encoding (RFC 2118 §4.1; OB-01).
fn put_literal(w: &mut BitWriter, b: u8) {
    if b < 0x80 {
        w.push(u32::from(b), 8);
    } else {
        w.push(0b10, 2);
        w.push(u32::from(b & 0x7f), 7);
    }
}

/// Offset encoding (RFC 2118 §4.2.1; MS-RDPBCGR §3.1.8.4.1.2.1). K-CP-1: MS-SIPCOMP §3.2.5.1
/// (p. 14) omits the subtraction of 64 and 320; RFC 2118, which MS-SIPCOMP cites as normative, and
/// MS-RDPBCGR include it, and their examples (128, 1024) only work with it.
fn put_offset(w: &mut BitWriter, offset: usize) -> Result<(), Error> {
    let o = u32::try_from(offset).map_err(|_| Error::NotEncodable)?;
    match offset {
        1..=63 => {
            w.push(0b1111, 4);
            w.push(o, 6);
        }
        64..=319 => {
            w.push(0b1110, 4);
            w.push(o - 64, 8);
        }
        320..=8191 => {
            w.push(0b110, 3);
            w.push(o - 320, 13);
        }
        _ => return Err(Error::NotEncodable),
    }
    Ok(())
}

/// Length-of-match encoding (RFC 2118 §4.2.2; MS-SIPCOMP p. 14–15).
fn put_length(w: &mut BitWriter, length: usize) -> Result<(), Error> {
    if !(MIN_MATCH..=MAX_LENGTH).contains(&length) {
        return Err(Error::NotEncodable);
    }
    let l = u32::try_from(length).map_err(|_| Error::NotEncodable)?;
    if l == 3 {
        w.push(0, 1);
        return Ok(());
    }
    let k = 31 - l.leading_zeros(); // 2 for 4..7, …, 12 for 4096..8191
    w.push((1 << (k - 1)) - 1, k - 1);
    w.push(0, 1);
    w.push(l & ((1 << k) - 1), k);
    Ok(())
}

fn put_token(w: &mut BitWriter, t: Token) -> Result<(), Error> {
    match t {
        Token::Literal(b) => put_literal(w, b),
        Token::Copy { offset, length } => {
            put_offset(w, usize::from(offset))?;
            put_length(w, usize::from(length))?;
        }
    }
    Ok(())
}

/// Encodes tokens into the MPPC bit stream.
pub fn encode_tokens(tokens: &[Token]) -> Result<Vec<u8>, Error> {
    let mut w = BitWriter::default();
    for t in tokens {
        put_token(&mut w, *t)?;
    }
    Ok(w.bytes)
}

fn get_token(r: &mut BitReader<'_>) -> Result<Token, Error> {
    if r.bit()? == 0 {
        return Ok(Token::Literal(r.bits(7)? as u8));
    }
    if r.bit()? == 0 {
        return Ok(Token::Literal(0x80 | r.bits(7)? as u8));
    }
    let offset = if r.bit()? == 0 {
        r.bits(13)? + 320
    } else if r.bit()? == 0 {
        r.bits(8)? + 64
    } else {
        r.bits(6)?
    };
    let mut ones = 0;
    while r.bit()? == 1 {
        ones += 1;
        if ones > 11 {
            return Err(Error::BadLength);
        }
    }
    let length = if ones == 0 {
        3
    } else {
        let k = ones + 1;
        (1 << k) | r.bits(k)?
    };
    Ok(Token::Copy {
        offset: offset as u16,
        length: length as u16,
    })
}

/// Decodes an MPPC bit stream into tokens.
pub fn decode_tokens(data: &[u8]) -> Result<Vec<Token>, Error> {
    let mut r = BitReader { data, pos: 0 };
    let mut tokens = Vec::new();
    while r.remaining() >= 8 {
        tokens.push(get_token(&mut r)?);
    }
    Ok(tokens)
}

/// Greedy tokenizer over `history[..end]` starting at `start`; prefers the longest match and,
/// among equally long ones, the nearest. The match search is not specified; this reproduces the
/// example of RFC 2118 §4 and MS-SIPCOMP §3.2.5.1.
pub fn tokenize(history: &[u8], start: usize, end: usize) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut pos = start;
    while pos < end {
        let max_len = (end - pos).min(MAX_LENGTH);
        let mut best = (0usize, 0usize);
        for offset in 1..=pos.min(HISTORY_SIZE - 1) {
            let mut len = 0;
            while len < max_len && history.get(pos - offset + len) == history.get(pos + len) {
                len += 1;
            }
            if len > best.1 {
                best = (offset, len);
                if len == max_len {
                    break;
                }
            }
        }
        if best.1 >= MIN_MATCH {
            tokens.push(Token::Copy {
                offset: best.0 as u16,
                length: best.1 as u16,
            });
            pos += best.1;
        } else {
            tokens.push(Token::Literal(history.get(pos).copied().unwrap_or(0)));
            pos += 1;
        }
    }
    tokens
}

/// One compressed (or flushed) packet payload with its flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub flags: u8,
    pub payload: Vec<u8>,
}

/// Sender side (MS-SIPCOMP §3.2.5.1–3.2.5.1.1, pp. 14–15).
pub struct Compressor {
    history: Vec<u8>,
    offset: usize,
    started: bool,
}

impl Default for Compressor {
    fn default() -> Self {
        Self {
            history: vec![0; HISTORY_SIZE],
            offset: 0,
            started: false,
        }
    }
}

impl Compressor {
    pub fn compress(&mut self, data: &[u8]) -> Result<Packet, Error> {
        if data.len() > HISTORY_SIZE {
            return Err(Error::HistoryOverflow);
        }
        let mut flags = PACKET_COMPRESSED;
        if !self.started || self.offset + data.len() > HISTORY_SIZE {
            self.offset = 0;
            self.started = true;
            flags |= PACKET_AT_FRONT;
        }
        let start = self.offset;
        let end = start + data.len();
        self.history
            .get_mut(start..end)
            .ok_or(Error::HistoryOverflow)?
            .copy_from_slice(data);
        let payload = encode_tokens(&tokenize(&self.history, start, end))?;
        if payload.len() > data.len() {
            // Expansion: send uncompressed with PACKET_FLUSHED alone and reset the history.
            self.history.fill(0);
            self.offset = 0;
            self.started = false;
            return Ok(Packet {
                flags: PACKET_FLUSHED,
                payload: data.to_vec(),
            });
        }
        self.offset = end;
        Ok(Packet { flags, payload })
    }
}

/// Receiver side (MS-SIPCOMP §3.2.5.2, pp. 16–17).
pub struct Decompressor {
    history: Vec<u8>,
    offset: usize,
    /// How much of the history holds data since the last flush: a copy may only read there.
    filled: usize,
}

impl Default for Decompressor {
    fn default() -> Self {
        Self {
            history: vec![0; HISTORY_SIZE],
            offset: 0,
            filled: 0,
        }
    }
}

impl Decompressor {
    fn flush(&mut self) {
        self.history.fill(0);
        self.offset = 0;
        self.filled = 0;
    }

    fn put(&mut self, pos: usize, b: u8, end: usize) -> Result<usize, Error> {
        if pos >= end {
            return Err(Error::HistoryOverflow);
        }
        *self.history.get_mut(pos).ok_or(Error::HistoryOverflow)? = b;
        self.filled = self.filled.max(pos + 1);
        Ok(pos + 1)
    }

    /// A copy-tuple. The history is addressed as a ring: after PACKET_AT_FRONT a copy at the
    /// front may reach back into data left at the end of the buffer by earlier packets. Neither
    /// MS-SIPCOMP nor RFC 2118 says so; the server's packets were only decodable this way
    /// (interop, `tested-in-lab`). A copy never reads where no data was written since the last
    /// flush.
    fn copy(
        &mut self,
        mut pos: usize,
        offset: u16,
        length: u16,
        end: usize,
    ) -> Result<usize, Error> {
        let offset = usize::from(offset);
        if offset == 0 || offset >= HISTORY_SIZE {
            return Err(Error::OffsetOutOfRange);
        }
        for _ in 0..length {
            let from = (pos + HISTORY_SIZE - offset) % HISTORY_SIZE;
            if from >= self.filled {
                return Err(Error::OffsetOutOfRange);
            }
            let b = *self.history.get(from).ok_or(Error::OffsetOutOfRange)?;
            pos = self.put(pos, b, end)?;
        }
        Ok(pos)
    }

    pub fn decompress(&mut self, flags: u8, payload: &[u8]) -> Result<Vec<u8>, Error> {
        check_flags(flags)?;
        if flags & PACKET_FLUSHED != 0 {
            self.flush();
        }
        if flags & PACKET_AT_FRONT != 0 {
            self.offset = 0;
        }
        if flags & PACKET_COMPRESSED == 0 {
            return Ok(payload.to_vec());
        }
        let start = self.offset;
        let mut pos = start;
        for token in decode_tokens(payload)? {
            match token {
                Token::Literal(b) => {
                    pos = self.put(pos, b, HISTORY_SIZE)?;
                }
                Token::Copy { offset, length } => {
                    pos = self.copy(pos, offset, length, HISTORY_SIZE)?;
                }
            }
        }
        self.offset = pos;
        Ok(self.history.get(start..pos).unwrap_or(&[]).to_vec())
    }
}

/// Length of the packet header (CP-05).
pub const HEADER_LEN: usize = 6;

/// Bound on undecoded input held by [`Inbound`]: one packet of at most 65535 uncompressed bytes
/// plus its header; a compressed payload is never longer than its uncompressed data, since a
/// sender that would expand sends the data uncompressed instead (§3.2.5.1.1, p. 15).
const MAX_PENDING: usize = HEADER_LEN + u16::MAX as usize;

/// The packet header: flags (low four bits of `flags`) and the uncompressed size.
pub fn header(flags: u8, size: u16) -> [u8; HEADER_LEN] {
    let [hi, lo] = size.to_be_bytes();
    [(flags & 0x0f) << 4, 0, 0, 0, hi, lo]
}

/// Reads a packet header: `(flags, uncompressed size)`. Type 0 is the only type (CP-05).
pub fn parse_header(h: &[u8; HEADER_LEN]) -> Result<(u8, u16), Error> {
    let [b0, _, _, _, hi, lo] = *h;
    if b0 & 0x0f != 0 {
        return Err(Error::BadHeader);
    }
    Ok((b0 >> 4, u16::from_be_bytes([hi, lo])))
}

fn check_flags(flags: u8) -> Result<(), Error> {
    let known = PACKET_FLUSHED | PACKET_AT_FRONT | PACKET_COMPRESSED;
    if flags & !known != 0 || (flags & PACKET_FLUSHED != 0 && flags & PACKET_COMPRESSED != 0) {
        return Err(Error::InconsistentFlags(flags));
    }
    Ok(())
}

impl Compressor {
    /// Starts over: the next packet goes to the front of an empty history. Used after a packet
    /// sent uncompressed with PACKET_FLUSHED, which clears the receiver's history (CP-08).
    pub fn reset(&mut self) {
        self.history.fill(0);
        self.offset = 0;
        self.started = false;
    }
}

impl Decompressor {
    /// Decodes one packet from the front of `data` (its payload, after the header) that expands
    /// to `size` bytes. A compressed payload has no length field: it ends with the byte that
    /// holds the last bit of the token that completes `size` bytes. `Ok(None)` while `data` does
    /// not yet hold the whole packet; the state changes only once a packet is complete.
    pub fn decompress_packet(
        &mut self,
        flags: u8,
        data: &[u8],
        size: usize,
    ) -> Result<Option<(Vec<u8>, usize)>, Error> {
        check_flags(flags)?;
        if flags & PACKET_COMPRESSED == 0 {
            let Some(plain) = data.get(..size) else {
                return Ok(None);
            };
            if flags & PACKET_FLUSHED != 0 {
                self.flush();
            }
            return Ok(Some((plain.to_vec(), size)));
        }
        let start = if flags & PACKET_AT_FRONT != 0 {
            0
        } else {
            self.offset
        };
        let end = start.checked_add(size).ok_or(Error::HistoryOverflow)?;
        if end > HISTORY_SIZE {
            return Err(Error::HistoryOverflow);
        }
        // A packet not yet complete must leave no trace: a copy may read ahead of the write
        // position (ring), so partly written data would change a later attempt.
        let saved = (self.history.clone(), self.filled);
        let mut r = BitReader { data, pos: 0 };
        let mut pos = start;
        while pos < end {
            let token = match get_token(&mut r) {
                Ok(t) => t,
                Err(Error::Truncated) => {
                    (self.history, self.filled) = saved;
                    return Ok(None);
                }
                Err(e) => return Err(e),
            };
            match token {
                Token::Literal(b) => {
                    pos = self.put(pos, b, end)?;
                }
                Token::Copy { offset, length } => {
                    pos = self.copy(pos, offset, length, end)?;
                }
            }
        }
        self.offset = end;
        let used = r.pos.div_ceil(8);
        Ok(Some((
            self.history.get(start..end).unwrap_or(&[]).to_vec(),
            used,
        )))
    }
}

/// The receiving direction of a compressed connection: packets in, SIP bytes out.
#[derive(Default)]
pub struct Inbound {
    pending: Vec<u8>,
    decompressor: Decompressor,
    /// The peer has sent a compressed packet: from now on we may compress too (CP-04).
    pub peer_compressed: bool,
    /// Packets received, and how many of them were compressed (diagnostics).
    pub packets: u64,
    pub compressed: u64,
}

impl Inbound {
    /// Takes bytes read from the connection and returns the SIP bytes of the packets they
    /// complete. Any error means the connection must be closed (CP-08).
    pub fn push(&mut self, data: &[u8]) -> Result<Vec<u8>, Error> {
        self.pending.extend_from_slice(data);
        let mut out = Vec::new();
        let mut used = 0;
        while let Some(h) = self.pending.get(used..used + HEADER_LEN) {
            let mut head = [0u8; HEADER_LEN];
            head.copy_from_slice(h);
            let (flags, size) = parse_header(&head)?;
            let body = self.pending.get(used + HEADER_LEN..).unwrap_or(&[]);
            let Some((plain, n)) =
                self.decompressor
                    .decompress_packet(flags, body, usize::from(size))?
            else {
                break;
            };
            self.packets += 1;
            if flags & PACKET_COMPRESSED != 0 {
                self.compressed += 1;
                self.peer_compressed = true;
            }
            out.extend_from_slice(&plain);
            used += HEADER_LEN + n;
        }
        self.pending.drain(..used);
        if self.pending.len() > MAX_PENDING {
            return Err(Error::HistoryOverflow);
        }
        Ok(out)
    }
}

/// The sending direction: SIP bytes in, packets out.
#[derive(Default)]
pub struct Outbound {
    compressor: Compressor,
    /// Packets sent, and how many of them were compressed (diagnostics).
    pub packets: u64,
    pub compressed: u64,
}

impl Outbound {
    /// Frames `data` as packets of at most one history buffer each; compressed only when
    /// `compress` is set (after the peer's first compressed packet, CP-04), else uncompressed
    /// with PACKET_FLUSHED.
    pub fn frame(&mut self, data: &[u8], compress: bool) -> Result<Vec<u8>, Error> {
        let mut out = Vec::with_capacity(data.len() + HEADER_LEN);
        for chunk in data.chunks(HISTORY_SIZE) {
            let packet = if compress {
                self.compressor.compress(chunk)?
            } else {
                self.compressor.reset();
                Packet {
                    flags: PACKET_FLUSHED,
                    payload: chunk.to_vec(),
                }
            };
            let size = u16::try_from(chunk.len()).map_err(|_| Error::HistoryOverflow)?;
            out.extend_from_slice(&header(packet.flags, size));
            out.extend_from_slice(&packet.payload);
            self.packets += 1;
            if packet.flags & PACKET_COMPRESSED != 0 {
                self.compressed += 1;
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
