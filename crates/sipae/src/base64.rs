//! Base64 for the `gssapi-data` parameter ([MS-SIPAE] §3.2.5.1, RFC 3548 §3 / RFC 4648 §4).
//!
//! Strict decoding (security by design): standard alphabet, padding required, no whitespace, no
//! non-zero bits in the padding, and a length limit, so one value has exactly one encoding.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Upper bound for decoded tokens (inference: NTLM and Kerberos tokens in SIP stay far below).
pub const MAX_DECODED_LEN: usize = 48 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Base64Error;

pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                let idx = ((n >> (18 - 6 * i)) & 0x3f) as usize;
                out.push(char::from(ALPHABET.get(idx).copied().unwrap_or(b'A')));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn value(c: u8) -> Option<u32> {
    ALPHABET
        .iter()
        .position(|a| *a == c)
        .and_then(|p| u32::try_from(p).ok())
}

pub fn decode(s: &str) -> Result<Vec<u8>, Base64Error> {
    let bytes = s.as_bytes();
    if !bytes.len().is_multiple_of(4) || bytes.len() / 4 * 3 > MAX_DECODED_LEN + 2 {
        return Err(Base64Error);
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let quads = bytes.len() / 4;
    for (q, chunk) in bytes.chunks(4).enumerate() {
        let last = q + 1 == quads;
        let pad = chunk.iter().rev().take_while(|c| **c == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return Err(Base64Error);
        }
        let mut n: u32 = 0;
        for (i, c) in chunk.iter().enumerate() {
            let v = if i >= 4 - pad {
                0
            } else {
                value(*c).ok_or(Base64Error)?
            };
            n = (n << 6) | v;
        }
        // Canonical form: the bits dropped by the padding must be zero.
        if (pad == 1 && n & 0xff != 0) || (pad == 2 && n & 0xffff != 0) {
            return Err(Base64Error);
        }
        let b = n.to_be_bytes();
        out.extend(b.iter().skip(1).take(3 - pad));
    }
    if out.len() > MAX_DECODED_LEN {
        return Err(Base64Error);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 4648 §10 test vectors.
    #[test]
    fn rfc4648_vectors() {
        for (plain, enc) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(plain.as_bytes()), enc);
            assert_eq!(decode(enc).unwrap(), plain.as_bytes());
        }
    }

    #[test]
    fn strict_decoding() {
        for bad in [
            "Zg", "Zg=", "Zh==", "Zm9=", "Z===", "Zg==Zg==", "Zm 9v", "Zm9v\r\n", "Zm-v", "====",
        ] {
            assert_eq!(decode(bad), Err(Base64Error), "{bad:?}");
        }
        assert_eq!(decode(&"A".repeat(70_000)), Err(Base64Error));
    }
}
