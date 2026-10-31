//! MS-ICE2 helpers.
//!
//! Evidence: `docs/protocol/evidence/13-ice-connectivity.md` (IC-12, IC-14),
//! `docs/protocol/evidence/38-ietf-foundations-and-archive-sources.md` (IE-03, IE-04, IE-05) and
//! `docs/protocol/evidence/39-evidence-for-open-points.md` (OB-09, secondary).

/// XOR value of the STUN FINGERPRINT attribute (IE-03).
pub const FINGERPRINT_XOR: u32 = 0x5354_554e;

/// Index and value of the misprinted entry in the CRC table printed in [MS-ICE2] §3.1.4.8.2
/// (pp. 25–26): entry 90 reads `8BBE8EA` instead of `8BBEB8EA` (IC-14).
pub const MS_ICE2_MISPRINT_INDEX: usize = 90;
pub const MS_ICE2_MISPRINT_VALUE: u32 = 0x08bb_e8ea;

/// Standard reflected CRC-32 table (polynomial 0xEDB88320, ITU V.42).
#[allow(clippy::indexing_slicing)] // n < 256 by the loop condition
pub const fn crc32_table() -> [u32; 256] {
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

/// The table as printed in [MS-ICE2] (standard table with the misprint at index 90).
pub const fn ms_ice2_table() -> [u32; 256] {
    let mut t = crc32_table();
    t[MS_ICE2_MISPRINT_INDEX] = MS_ICE2_MISPRINT_VALUE;
    t
}

const STANDARD: [u32; 256] = crc32_table();
const MS_ICE2: [u32; 256] = ms_ice2_table();

fn crc32_with(table: &[u32; 256], data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for b in data {
        let idx = ((crc ^ u32::from(*b)) & 0xff) as u8;
        let entry = table.get(usize::from(idx)).copied().unwrap_or(0);
        crc = entry ^ (crc >> 8);
    }
    !crc
}

/// Standard CRC-32.
pub fn crc32(data: &[u8]) -> u32 {
    crc32_with(&STANDARD, data)
}

/// FINGERPRINT value per RFC 5389 / ice-19: CRC-32 of the message up to the attribute, XORed
/// with 0x5354554E (IE-03).
pub fn fingerprint(message_before_attribute: &[u8]) -> u32 {
    crc32(message_before_attribute) ^ FINGERPRINT_XOR
}

/// FINGERPRINT of the MS-ICE2 duplicate connectivity check: computed with the misprinted table
/// and XORed with 0x5354554E (IC-14; the XOR is confirmed only by a secondary source, OB-09).
pub fn fingerprint_ms_ice2_duplicate(message_before_attribute: &[u8]) -> u32 {
    crc32_with(&MS_ICE2, message_before_attribute) ^ FINGERPRINT_XOR
}

/// Candidate priority (IE-04): 2^24·type preference + 2^8·local preference + (256 − component).
pub fn candidate_priority(type_preference: u8, local_preference: u16, component: u8) -> u32 {
    (u32::from(type_preference) << 24)
        + (u32::from(local_preference) << 8)
        + (256 - u32::from(component))
}

/// Local preference for TCP candidates (ice-tcp-07 §3.2, IE-04):
/// 2^12·transport + 2^9·direction + other, each field masked to its width.
pub fn tcp_local_preference(transport: u8, direction: u8, other: u16) -> u16 {
    (u16::from(transport & 0x0f) << 12) | (u16::from(direction & 0x07) << 9) | (other & 0x01ff)
}

/// Whether media or connectivity checks may go to `remote` (issue #42, design): never to
/// unspecified, multicast or broadcast addresses or port 0; to loopback or link-local addresses
/// only when one of `ours` is of the same kind (a local test), so a peer's SDP cannot point our
/// STUN and RTP traffic at services on this host or the local link. IPv4-mapped IPv6 addresses
/// are judged as IPv4.
pub fn usable_peer_address(remote: std::net::SocketAddr, ours: &[std::net::IpAddr]) -> bool {
    use std::net::IpAddr;
    let canon = |a: IpAddr| match a {
        IpAddr::V6(v) => v.to_ipv4_mapped().map_or(a, IpAddr::V4),
        v4 => v4,
    };
    let loopback = |a: IpAddr| canon(a).is_loopback();
    let link_local = |a: IpAddr| match canon(a) {
        IpAddr::V4(v) => v.is_link_local(),
        IpAddr::V6(v) => v.segments()[0] & 0xffc0 == 0xfe80,
    };
    let ip = canon(remote.ip());
    let never = match ip {
        IpAddr::V4(v) => v.is_unspecified() || v.is_broadcast() || v.is_multicast(),
        IpAddr::V6(v) => v.is_unspecified() || v.is_multicast(),
    };
    if never || remote.port() == 0 {
        return false;
    }
    if loopback(ip) {
        return ours.iter().any(|&o| loopback(o));
    }
    if link_local(ip) {
        return ours.iter().any(|&o| link_local(o));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_check_value() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn misprint_differs_only_at_index_90() {
        let differing: Vec<usize> = (0..256).filter(|&i| STANDARD[i] != MS_ICE2[i]).collect();
        assert_eq!(differing, vec![90]);
        assert_eq!(STANDARD[90], 0x8bbe_b8ea);
    }

    #[test]
    fn duplicate_fingerprint_differs_when_entry_90_is_used() {
        // A single byte whose table lookup hits index 90 on the first step: 0xff ^ b == 90.
        let data = [0xffu8 ^ 90];
        assert_ne!(fingerprint(&data), fingerprint_ms_ice2_duplicate(&data));
        // Data that never touches index 90 gives identical values.
        assert_eq!(fingerprint(b""), fingerprint_ms_ice2_duplicate(b""));
    }

    /// Priorities from SfB example SDPs, decomposed in IE-05.
    #[test]
    fn priorities_from_sfb_examples() {
        assert_eq!(candidate_priority(126, 0xffff, 1), 2_130_706_431);
        assert_eq!(
            candidate_priority(0, tcp_local_preference(15, 7, 9), 1),
            16_648_703
        );
        assert_eq!(
            candidate_priority(0, tcp_local_preference(6, 5, 505), 1),
            7_076_351
        );
        assert_eq!(
            candidate_priority(100, tcp_local_preference(6, 5, 505), 1),
            1_684_797_951
        );
        assert_eq!(
            candidate_priority(0, tcp_local_preference(6, 2, 9), 1),
            6_556_159
        );
    }

    /// Issue #42.
    #[test]
    fn peer_address_rules() {
        let public: std::net::IpAddr = "192.0.2.5".parse().unwrap();
        let lo: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        let ok = |a: &str, ours: &[std::net::IpAddr]| usable_peer_address(a.parse().unwrap(), ours);
        assert!(ok("198.51.100.7:5000", &[public]));
        assert!(ok("[2001:db8::1]:5000", &[public]));
        for bad in [
            "127.0.0.1:5000",
            "127.9.9.9:5000",
            "[::1]:5000",
            "[::ffff:127.0.0.1]:5000",
            "169.254.1.1:5000",
            "[fe80::1]:5000",
            "0.0.0.0:5000",
            "255.255.255.255:5000",
            "224.0.0.1:5000",
            "[ff02::1]:5000",
            "198.51.100.7:0",
        ] {
            assert!(!ok(bad, &[public]), "{bad}");
        }
        assert!(ok("127.0.0.1:5000", &[lo]), "local test on loopback");
        assert!(!ok("0.0.0.0:5000", &[lo]));
    }
}
