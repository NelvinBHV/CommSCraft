//! Endpoint identifiers for Skype for Business SIP.
//!
//! Evidence: `docs/protocol/evidence/06-sipre-routing.md` (RE-02, RE-06, RE-07, K-RE-1).

use sha1::{Digest, Sha1};

/// Namespace UUID for deriving `+sip.instance` from the EPID (RE-07, [MS-SIPRE] §3.3.3.1, p. 41).
const EPID_NAMESPACE: [u8; 16] = [
    0xfc, 0xac, 0xfb, 0x03, 0x8a, 0x73, 0x46, 0xef, 0x91, 0xb1, 0xe5, 0xeb, 0xee, 0xab, 0xa4, 0xfe,
];

/// Maximum EPID length in hex characters (RE-02, [MS-SIPRE] §2.2.7, p. 25).
pub const MAX_EPID_LEN: usize = 16;

/// Errors for endpoint identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    /// The EPID is empty, longer than 16 characters or not hexadecimal.
    InvalidEpid,
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEpid => f.write_str("EPID must be 1 to 16 hexadecimal characters"),
        }
    }
}

impl std::error::Error for IdentityError {}

/// Converts a big-endian UUID (RFC 4122 field order) to the Windows GUID byte order
/// (first three fields little endian).
fn to_guid_bytes(uuid: [u8; 16]) -> [u8; 16] {
    let [a0, a1, a2, a3, b0, b1, c0, c1, rest @ ..] = uuid;
    let [d0, d1, d2, d3, d4, d5, d6, d7] = rest;
    [
        a3, a2, a1, a0, b1, b0, c1, c0, d0, d1, d2, d3, d4, d5, d6, d7,
    ]
}

/// Formats GUID bytes (Windows byte order) as the usual lower-case string.
fn format_guid(g: [u8; 16]) -> String {
    let [b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15] = g;
    let data1 = u32::from_le_bytes([b0, b1, b2, b3]);
    let data2 = u16::from_le_bytes([b4, b5]);
    let data3 = u16::from_le_bytes([b6, b7]);
    format!(
        "{data1:08x}-{data2:04x}-{data3:04x}-{b8:02x}{b9:02x}-{b10:02x}{b11:02x}{b12:02x}{b13:02x}{b14:02x}{b15:02x}"
    )
}

/// Derives the `+sip.instance` UUID from the EPID (RE-07).
///
/// The specification text asks for SHA-256, but its own example only matches SHA-1 (conflict
/// K-RE-1); servers check the derivation (RE-08), so SHA-1 is used. The hash input is the
/// namespace in Windows GUID byte order followed by the ASCII EPID; the first 16 hash bytes are
/// read as a GUID in Windows byte order, with version 5 and the RFC 4122 variant set.
pub fn instance_uuid_from_epid(epid: &str) -> Result<String, IdentityError> {
    if epid.is_empty() || epid.len() > MAX_EPID_LEN || !epid.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(IdentityError::InvalidEpid);
    }
    let mut hasher = Sha1::new();
    hasher.update(to_guid_bytes(EPID_NAMESPACE));
    hasher.update(epid.as_bytes());
    let digest = hasher.finalize();
    let mut g = [0u8; 16];
    for (dst, src) in g.iter_mut().zip(digest.iter()) {
        *dst = *src;
    }
    // Version nibble lives in the high byte of Data3, which is byte 7 in Windows byte order.
    g[7] = (g[7] & 0x0f) | 0x50;
    g[8] = (g[8] & 0x3f) | 0x80;
    Ok(format_guid(g))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [MS-SIPRE] §4.2, p. 81: epid `01010101` → `4b1682a8-f968-5701-83fc-7c6741dc6697`.
    #[test]
    fn sipre_example_epid_to_instance() {
        assert_eq!(
            instance_uuid_from_epid("01010101").unwrap(),
            "4b1682a8-f968-5701-83fc-7c6741dc6697"
        );
    }

    #[test]
    fn rejects_invalid_epid() {
        assert_eq!(instance_uuid_from_epid(""), Err(IdentityError::InvalidEpid));
        assert_eq!(
            instance_uuid_from_epid("0123456789abcdef0"),
            Err(IdentityError::InvalidEpid)
        );
        assert_eq!(
            instance_uuid_from_epid("xyz"),
            Err(IdentityError::InvalidEpid)
        );
    }
}
