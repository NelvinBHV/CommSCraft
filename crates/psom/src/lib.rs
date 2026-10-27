//! MS-PSOM primitive encodings and the RPC message frame.
//!
//! Evidence: `docs/protocol/evidence/33-psom-web-conference-content.md` (PO-04, PO-05, K-PO-2,
//! K-PO-3). Sources: [MS-PSOM] 20250520 §2.2.2.2 (pp. 24–25), §3.1.1.1 (pp. 26–27), appendix A §6
//! (pp. 102–104), examples §4.1.4–4.1.5 (pp. 93–96).

/// Record type of an RPC message (§2.2.2, p. 21).
pub const RPC_MESSAGE: u8 = 0x16;
/// Encoding of a null distributed object reference (§3.1.1.1.4, p. 27).
pub const NULL_REFERENCE: u8 = 0x8c;
/// Upper bound of the encoded String length (§6.2, p. 104).
pub const MAX_STRING_LEN: usize = 0xffff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Truncated,
    /// Lead byte 0x84, 0x86, 0x8C or 0x8E: data lengths 5 and 7 are never used by GenericInt;
    /// 0x84, 0x86 and 0x8C have other meanings (§2.2.2.2, p. 25; §3.1.1.1.4, p. 27).
    InvalidGenericInt(u8),
    /// The value does not fit the 64-bit range.
    Overflow,
    /// Negative zero other than the two documented irregular forms (K-PO-2).
    IrregularGenericInt,
    /// String longer than 0xFFFF bytes.
    StringTooLong,
    /// Characters outside U+0001…U+FFFF: §6.2 describes the 1-, 2- and 3-byte UTF-8 forms only.
    UnsupportedCharacter(char),
    InvalidUtf8,
    /// Frame type other than 0x16, or MessageLength inconsistent with the frame.
    BadFrame,
}

/// Encodes a GenericInt (§6.1, pseudo code §6.1.1).
pub fn encode_generic_int(x: i64, out: &mut Vec<u8>) {
    if (-112..=127).contains(&x) {
        out.push(x as u8);
        return;
    }
    let abs = x.unsigned_abs();
    let size: u8 = if abs <= 0xff {
        0
    } else if abs <= 0xffff {
        1
    } else if abs <= 0xff_ffff {
        2
    } else if abs <= 0xffff_ffff {
        3
    } else if abs <= 0xffff_ffff_ffff {
        5
    } else {
        7
    };
    out.push(if x >= 0 { 0x80 } else { 0x88 } + size);
    let bytes = abs.to_be_bytes();
    out.extend_from_slice(bytes.get(7 - usize::from(size)..).unwrap_or(&[]));
}

/// Decodes a GenericInt; returns the value and the number of bytes read.
///
/// The two irregular forms of the table in §6.1 (p. 102), `88 00` = −2^31 and `8d 00 00 00 00 00
/// 00` = −2^63, are accepted as documented (K-PO-2); the encoder never produces them.
pub fn decode_generic_int(input: &[u8]) -> Result<(i64, usize), Error> {
    let lead = *input.first().ok_or(Error::Truncated)?;
    if lead < 0x80 {
        return Ok((i64::from(lead), 1));
    }
    if lead >= 0x90 {
        return Ok((i64::from(lead as i8), 1));
    }
    let negative = lead & 0x08 != 0;
    let n = usize::from(lead & 0x07) + 1;
    if n == 5 || n == 7 {
        return Err(Error::InvalidGenericInt(lead));
    }
    let data = input.get(1..1 + n).ok_or(Error::Truncated)?;
    let magnitude = data.iter().fold(0u64, |acc, b| (acc << 8) | u64::from(*b));
    let value = match (negative, magnitude) {
        (true, 0) if n == 1 => i64::from(i32::MIN),
        (true, 0) if n == 6 => i64::MIN,
        (true, 0) => return Err(Error::IrregularGenericInt),
        (true, m) if m == i64::MIN.unsigned_abs() => i64::MIN,
        (true, m) => -i64::try_from(m).map_err(|_| Error::Overflow)?,
        (false, m) => i64::try_from(m).map_err(|_| Error::Overflow)?,
    };
    Ok((value, 1 + n))
}

/// Obfuscation of §6.2: from the last byte towards the first, XOR with a running value that
/// decreases by 17 per byte. It is its own inverse.
fn xor_backwards(bytes: &mut [u8]) {
    let mut value = 0u8;
    for b in bytes.iter_mut().rev() {
        value = value.wrapping_sub(17);
        *b ^= value;
    }
}

/// Encodes a String: 16-bit big-endian length, then the obfuscated UTF-8 bytes (§6.2).
///
/// Inference: the length counts UTF-8 bytes. The only example ("pptdemo2.pptx", p. 104) is
/// ASCII, where bytes and characters coincide.
pub fn encode_string(s: &str, out: &mut Vec<u8>) -> Result<(), Error> {
    if let Some(c) = s
        .chars()
        .find(|c| u32::from(*c) == 0 || u32::from(*c) > 0xffff)
    {
        return Err(Error::UnsupportedCharacter(c));
    }
    let len = u16::try_from(s.len()).map_err(|_| Error::StringTooLong)?;
    out.extend_from_slice(&len.to_be_bytes());
    let mut bytes = s.as_bytes().to_vec();
    xor_backwards(&mut bytes);
    out.extend_from_slice(&bytes);
    Ok(())
}

/// Decodes a String; returns it and the number of bytes read.
pub fn decode_string(input: &[u8]) -> Result<(String, usize), Error> {
    let head = input.get(..2).ok_or(Error::Truncated)?;
    let len = usize::from(u16::from_be_bytes([
        head.first().copied().unwrap_or(0),
        head.get(1).copied().unwrap_or(0),
    ]));
    let mut bytes = input.get(2..2 + len).ok_or(Error::Truncated)?.to_vec();
    xor_backwards(&mut bytes);
    let s = String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
    if let Some(c) = s
        .chars()
        .find(|c| u32::from(*c) == 0 || u32::from(*c) > 0xffff)
    {
        return Err(Error::UnsupportedCharacter(c));
    }
    Ok((s, 2 + len))
}

/// An RPC (call method) message: ProxyId, MethodIndex and the encoded parameters (§2.2.2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcCall {
    pub proxy_id: i64,
    pub method_index: u8,
    pub parameters: Vec<u8>,
}

impl RpcCall {
    /// Frame: 0x16, MessageLength (4 bytes, big endian, counts the bytes after it), ProxyId
    /// (GenericInt), MethodIndex (1 byte), parameters (§4.1.4.1, p. 94).
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut body = Vec::with_capacity(self.parameters.len() + 10);
        encode_generic_int(self.proxy_id, &mut body);
        body.push(self.method_index);
        body.extend_from_slice(&self.parameters);
        let len = u32::try_from(body.len()).map_err(|_| Error::BadFrame)?;
        let mut out = Vec::with_capacity(body.len() + 5);
        out.push(RPC_MESSAGE);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&body);
        Ok(out)
    }

    /// Parses one frame; returns the call and the number of bytes read.
    pub fn decode(input: &[u8]) -> Result<(Self, usize), Error> {
        if *input.first().ok_or(Error::Truncated)? != RPC_MESSAGE {
            return Err(Error::BadFrame);
        }
        let len_bytes: [u8; 4] = input
            .get(1..5)
            .ok_or(Error::Truncated)?
            .try_into()
            .map_err(|_| Error::Truncated)?;
        let len = usize::try_from(u32::from_be_bytes(len_bytes)).map_err(|_| Error::BadFrame)?;
        let body = input.get(5..5 + len).ok_or(Error::Truncated)?;
        let (proxy_id, used) = decode_generic_int(body)?;
        let method_index = *body.get(used).ok_or(Error::BadFrame)?;
        let parameters = body.get(used + 1..).ok_or(Error::BadFrame)?.to_vec();
        Ok((
            Self {
                proxy_id,
                method_index,
                parameters,
            },
            5 + len,
        ))
    }
}

#[cfg(test)]
mod tests;
