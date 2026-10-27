//! MS-ABS address book files: compressed blocks, CRC-32 and file-name dates.
//!
//! Evidence: `docs/protocol/evidence/27-abs-address-book-files.md` (AB-03, AB-04, K-AB-1,
//! K-AB-4). The bit order of the token-type word (bit 31 = first token, K-AB-1) is confirmed by
//! the compressed example of [MS-ABS] §3.2: only this order consumes exactly its
//! CompressedSize bytes and yields its DecompressedSize bytes.

/// Size of the COMPRESSED_BLOCK_HEADER (CRC, CompressedSize, DecompressedSize; little endian).
pub const BLOCK_HEADER_LEN: usize = 12;
/// Upper bound of CompressedSize (§2.2.2, p. 18) and the buffer size of the pseudo code (§5.5.1).
pub const MAX_BLOCK_LEN: usize = 0x10000;

const MAX_OFFSET: usize = 1 << 13;
const MIN_RUN: usize = 3;
const MAX_RUN: usize = 0xffff + MIN_RUN;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input ends inside a block header, a token-type word or a token.
    Truncated,
    /// A run points before the start of the output.
    OffsetOutOfRange,
    /// The tokens produce more bytes than DecompressedSize.
    OutputOverflow,
    /// CompressedSize > 0x10000, DecompressedSize < CompressedSize, or DecompressedSize too large.
    BadBlockSize,
    /// The CRC field does not match the decompressed data.
    CrcMismatch { stored: u32, computed: u32 },
}

const CRC_TABLE: [u32; 256] = crc32_table();

/// Reflected CRC-32 table (polynomial 0xEDB88320). The table printed in [MS-ABS] §5.1
/// (pp. 63–64) is identical to it (all 256 entries checked with our own script).
#[allow(clippy::indexing_slicing)] // n < 256 by the loop condition
const fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
}

/// CalculateCRC32 of [MS-ABS] §5.1: start 0xFFFFFFFF, final XOR 0xFFFFFFFF.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for b in data {
        let idx = usize::from(((crc ^ u32::from(*b)) & 0xff) as u8);
        crc = CRC_TABLE.get(idx).copied().unwrap_or(0) ^ (crc >> 8);
    }
    !crc
}

struct Input<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Input<'_> {
    fn byte(&mut self) -> Result<u8, Error> {
        let b = self.data.get(self.pos).copied().ok_or(Error::Truncated)?;
        self.pos += 1;
        Ok(b)
    }

    fn u16_le(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes([self.byte()?, self.byte()?]))
    }

    fn u32_le(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes([
            self.byte()?,
            self.byte()?,
            self.byte()?,
            self.byte()?,
        ]))
    }
}

/// DecompressBuffer of [MS-ABS] §5.5.2 (pp. 67–68): decompresses the Data field of one
/// COMPRESSED_BLOCK into `output_size` bytes. Returns the bytes and the number of input bytes
/// consumed.
pub fn decompress_block(input: &[u8], output_size: usize) -> Result<(Vec<u8>, usize), Error> {
    // Inference: the pseudo code decompresses into a 0x10000-byte buffer (§5.5.1).
    if output_size > MAX_BLOCK_LEN {
        return Err(Error::BadBlockSize);
    }
    let mut inp = Input {
        data: input,
        pos: 0,
    };
    let mut out: Vec<u8> = Vec::with_capacity(output_size);
    let mut token_type = 0u32;
    let mut mask = 0u32;
    let mut nibble_index: Option<usize> = None;

    while out.len() < output_size {
        if mask < 2 {
            token_type = inp.u32_le()?;
            mask = 0x8000_0000;
        } else {
            mask /= 2;
        }

        if token_type & mask == 0 {
            out.push(inp.byte()?);
            continue;
        }

        let word = usize::from(inp.u16_le()?);
        let mut length = word & 0x7;
        let offset = word >> 3;
        if length == 7 {
            length = match nibble_index.take() {
                None => {
                    nibble_index = Some(inp.pos);
                    usize::from(inp.byte()? & 0x0f)
                }
                Some(at) => usize::from(input.get(at).copied().ok_or(Error::Truncated)? >> 4),
            };
            if length == 15 {
                length = usize::from(inp.byte()?);
                if length == 255 {
                    length = usize::from(inp.u16_le()?);
                    // The pseudo code subtracts 15 + 7 here and adds them back below.
                    length = length.checked_sub(15 + 7).ok_or(Error::BadBlockSize)?;
                }
                length += 15;
            }
            length += 7;
        }
        length += MIN_RUN;

        let start = out
            .len()
            .checked_sub(offset + 1)
            .ok_or(Error::OffsetOutOfRange)?;
        if out.len() + length > output_size {
            return Err(Error::OutputOverflow);
        }
        for i in 0..length {
            let b = out.get(start + i).copied().ok_or(Error::OffsetOutOfRange)?;
            out.push(b);
        }
    }
    Ok((out, inp.pos))
}

/// EncodeOffsetLength of [MS-ABS] §5.4 (pp. 65–66). `nibble_index` is the shared length-nibble
/// state of the block.
fn encode_run(offset: usize, length: usize, out: &mut Vec<u8>, nibble_index: &mut Option<usize>) {
    let offset = (offset - 1) * 8;
    let mut length = length - MIN_RUN;
    if length < 7 {
        let word = offset | length;
        out.extend_from_slice(&[(word & 0xff) as u8, ((word >> 8) & 0xff) as u8]);
        return;
    }
    let word = offset | 0x7;
    length -= 7;
    out.extend_from_slice(&[(word & 0xff) as u8, ((word >> 8) & 0xff) as u8]);

    let nibble = length.min(15);
    match nibble_index.take() {
        None => {
            *nibble_index = Some(out.len());
            out.push(nibble as u8);
        }
        Some(at) => {
            if let Some(b) = out.get_mut(at) {
                *b |= (nibble as u8) << 4;
            }
        }
    }
    if nibble == 15 {
        length -= 15;
        if length >= 255 {
            let total = length + 7 + 15;
            out.extend_from_slice(&[255, (total & 0xff) as u8, ((total >> 8) & 0xff) as u8]);
        } else {
            out.push(length as u8);
        }
    }
}

/// Compresses one block with a simple greedy longest-match search. The search itself is not
/// specified ("beyond the scope of this specification", §5.4); only the token format is.
pub fn compress_block(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 8 + 4);
    let mut nibble_index = None;
    let mut pos = 0;
    while pos < data.len() {
        let group_at = out.len();
        out.extend_from_slice(&[0; 4]);
        let mut token_type = 0u32;
        let mut tokens = 0;
        while tokens < 32 && pos < data.len() {
            let (offset, length) = longest_match(data, pos);
            if length >= MIN_RUN {
                token_type |= 0x8000_0000 >> tokens;
                encode_run(offset, length, &mut out, &mut nibble_index);
                pos += length;
            } else {
                out.push(data.get(pos).copied().unwrap_or(0));
                pos += 1;
            }
            tokens += 1;
        }
        if let Some(word) = out.get_mut(group_at..group_at + 4) {
            word.copy_from_slice(&token_type.to_le_bytes());
        }
    }
    out
}

fn longest_match(data: &[u8], pos: usize) -> (usize, usize) {
    let mut best = (0, 0);
    let max_len = (data.len() - pos).min(MAX_RUN);
    for offset in 1..=pos.min(MAX_OFFSET) {
        let start = pos - offset;
        let mut len = 0;
        while len < max_len && data.get(start + len) == data.get(pos + len) {
            len += 1;
        }
        if len > best.1 {
            best = (offset, len);
            if len == max_len {
                break;
            }
        }
    }
    best
}

/// Decompresses a whole file (sequence of COMPRESSED_BLOCK, §5.5.1) and checks every CRC.
pub fn decompress_file(file: &[u8]) -> Result<Vec<u8>, Error> {
    let mut inp = Input { data: file, pos: 0 };
    let mut out = Vec::new();
    while inp.pos < file.len() {
        let stored = inp.u32_le()?;
        let compressed = usize::try_from(inp.u32_le()?).map_err(|_| Error::BadBlockSize)?;
        let decompressed = usize::try_from(inp.u32_le()?).map_err(|_| Error::BadBlockSize)?;
        if compressed > MAX_BLOCK_LEN || decompressed > MAX_BLOCK_LEN || decompressed < compressed {
            return Err(Error::BadBlockSize);
        }
        let data = file
            .get(inp.pos..inp.pos + compressed)
            .ok_or(Error::Truncated)?;
        inp.pos += compressed;
        let block = if compressed == decompressed {
            data.to_vec()
        } else {
            decompress_block(data, decompressed)?.0
        };
        let computed = crc32(&block);
        if computed != stored {
            return Err(Error::CrcMismatch { stored, computed });
        }
        out.extend_from_slice(&block);
    }
    Ok(out)
}

/// Compresses a file into blocks of at most 0x10000 input bytes; a block that does not shrink is
/// stored uncompressed (CompressedSize = DecompressedSize, §2.2.2).
pub fn compress_file(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in data.chunks(MAX_BLOCK_LEN) {
        let packed = compress_block(chunk);
        let body: &[u8] = if packed.len() < chunk.len() {
            &packed
        } else {
            chunk
        };
        out.extend_from_slice(&crc32(chunk).to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
    }
    out
}

/// A calendar date (proleptic Gregorian, UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

/// Days from 1970-01-01 to 2001-01-01.
const EPOCH_2001: i64 = 11_323;

/// File-name date `XXXX`: 0-based days after 2001-01-01 00:00:00 UTC (§1.3, p. 10).
pub fn date_from_day_number(n: u16) -> Date {
    civil_from_days(EPOCH_2001 + i64::from(n))
}

/// Inverse of [`date_from_day_number`]; `None` outside 2001-01-01 … 2180-06-06.
pub fn day_number_from_date(date: Date) -> Option<u16> {
    u16::try_from(days_from_civil(date) - EPOCH_2001).ok()
}

/// Weekday of a day number, 0 = Monday … 6 = Sunday (2001-01-01 was a Monday).
pub fn weekday(n: u16) -> u8 {
    (n % 7) as u8
}

// Howard Hinnant's days_from_civil / civil_from_days.
fn days_from_civil(d: Date) -> i64 {
    let y = i64::from(d.year) - i64::from(d.month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(d.month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d.day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> Date {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u8;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    Date { year, month, day }
}

#[cfg(test)]
mod tests;
