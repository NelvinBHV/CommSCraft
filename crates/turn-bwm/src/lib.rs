//! MS-TURNBWM: attribute types and the XOR-coded site address value.
//!
//! Evidence: `docs/protocol/evidence/23-bandwidth-management-ice2bwm-turnbwm.md` (BW-11, BW-12).
//! Source: [MS-TURNBWM] 20250520 §2.2 (pp. 13–24), §2.2.4 (pp. 15–16).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// Attribute types (BW-11; §2.2.1–2.2.14).
pub mod attr {
    pub const BAC_MESSAGE: u16 = 0x8056;
    pub const RESERVATION_ID: u16 = 0x8057;
    pub const RESERVATION_AMOUNT: u16 = 0x8058;
    pub const REMOTE_SITE_ADDRESS: u16 = 0x8059;
    pub const REMOTE_RELAY_SITE_ADDRESS: u16 = 0x805a;
    pub const LOCAL_SITE_ADDRESS: u16 = 0x805b;
    pub const LOCAL_RELAY_SITE_ADDRESS: u16 = 0x805c;
    pub const SIP_DIALOG_ID: u16 = 0x8061;
    pub const SIP_CALL_ID: u16 = 0x8062;
    pub const LOCATION_PROFILE: u16 = 0x8068;
}

/// Address family values (§2.2.4, p. 16).
pub const FAMILY_IPV4: u8 = 0x01;
pub const FAMILY_IPV6: u8 = 0x02;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Attribute length other than 8 (IPv4) or 20 (IPv6), or inconsistent with the family.
    BadLength(usize),
}

/// Value of a (Remote/Local) (Relay) Site Address attribute: Reserved (1 byte), Family (1 byte),
/// X-Port (2 bytes), X-IP Address (4 or 16 bytes). X-Port = port ⊕ the top 16 bits of the
/// 128-bit MS-TURN transaction ID; X-IP = IPv4 ⊕ the top 32 bits, or IPv6 ⊕ all 128 bits.
pub fn encode_site_address(addr: SocketAddr, transaction_id: &[u8; 16]) -> Vec<u8> {
    let [t0, t1, ..] = *transaction_id;
    let port = addr.port() ^ u16::from_be_bytes([t0, t1]);
    let mut out = vec![0];
    match addr.ip() {
        IpAddr::V4(ip) => {
            out.push(FAMILY_IPV4);
            out.extend_from_slice(&port.to_be_bytes());
            out.extend(ip.octets().iter().zip(transaction_id).map(|(a, t)| a ^ t));
        }
        IpAddr::V6(ip) => {
            out.push(FAMILY_IPV6);
            out.extend_from_slice(&port.to_be_bytes());
            out.extend(ip.octets().iter().zip(transaction_id).map(|(a, t)| a ^ t));
        }
    }
    out
}

/// Decodes a site address value. An unknown family yields `Ok(None)`: the attribute "MUST be
/// silently ignored" (§2.2.4, p. 16).
pub fn decode_site_address(
    value: &[u8],
    transaction_id: &[u8; 16],
) -> Result<Option<SocketAddr>, Error> {
    let [_reserved, family, p0, p1, ip @ ..] = value else {
        return Err(Error::BadLength(value.len()));
    };
    let [t0, t1, ..] = *transaction_id;
    let port = u16::from_be_bytes([*p0, *p1]) ^ u16::from_be_bytes([t0, t1]);
    let mut x = [0u8; 16];
    for (dst, (a, t)) in x.iter_mut().zip(ip.iter().zip(transaction_id)) {
        *dst = a ^ t;
    }
    let ip = match (*family, ip.len()) {
        (FAMILY_IPV4, 4) => {
            let [a, b, c, d, ..] = x;
            IpAddr::V4(Ipv4Addr::new(a, b, c, d))
        }
        (FAMILY_IPV6, 16) => IpAddr::V6(Ipv6Addr::from(x)),
        (FAMILY_IPV4 | FAMILY_IPV6, _) => return Err(Error::BadLength(value.len())),
        _ => return Ok(None),
    };
    Ok(Some(SocketAddr::new(ip, port)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tid(prefix: &[u8]) -> [u8; 16] {
        let mut t = [0u8; 16];
        t[..prefix.len()].copy_from_slice(prefix);
        t
    }

    /// §2.2.4 (p. 16): port 0x1122 ⊕ 0x4455 = 0x5577.
    #[test]
    fn x_port_example() {
        let t = tid(&[0x44, 0x55]);
        let addr: SocketAddr = "0.0.0.0:4386".parse().unwrap(); // 0x1122
        let v = encode_site_address(addr, &t);
        assert_eq!(&v[2..4], &[0x55, 0x77]);
        assert_eq!(decode_site_address(&v, &t).unwrap(), Some(addr));
    }

    /// §2.2.4 (p. 16): 0x11223344 ⊕ 0xaabbccdd = 0xbb99ff99; attribute length 8.
    #[test]
    fn x_ipv4_example() {
        let t = tid(&[0xaa, 0xbb, 0xcc, 0xdd]);
        let addr = SocketAddr::new(Ipv4Addr::new(0x11, 0x22, 0x33, 0x44).into(), 0);
        let v = encode_site_address(addr, &t);
        assert_eq!(v.len(), 8);
        assert_eq!(v, [0, FAMILY_IPV4, 0xaa, 0xbb, 0xbb, 0x99, 0xff, 0x99]);
        assert_eq!(decode_site_address(&v, &t).unwrap(), Some(addr));
    }

    /// §2.2.4 (p. 16): IPv6 example; attribute length 20.
    #[test]
    fn x_ipv6_example() {
        let t: [u8; 16] = 0x1122_3344_5566_7788_99aa_bbcc_ddee_ff00u128.to_be_bytes();
        let ip = Ipv6Addr::from(0x2001_0db8_1122_3344_5566_7788_99aa_bbccu128);
        let addr = SocketAddr::new(ip.into(), 0);
        let v = encode_site_address(addr, &t);
        assert_eq!(v.len(), 20);
        assert_eq!(
            &v[4..],
            &0x3123_3efc_4444_44cc_cccc_cc44_4444_44ccu128.to_be_bytes()
        );
        assert_eq!(decode_site_address(&v, &t).unwrap(), Some(addr));
    }

    #[test]
    fn malformed_and_unknown_family() {
        let t = [0u8; 16];
        assert_eq!(
            decode_site_address(&[0, 1, 0], &t),
            Err(Error::BadLength(3))
        );
        assert_eq!(
            decode_site_address(&[0, FAMILY_IPV4, 0, 0, 1, 2, 3], &t),
            Err(Error::BadLength(7))
        );
        assert_eq!(
            decode_site_address(&[0, FAMILY_IPV6, 0, 0, 1, 2, 3, 4], &t),
            Err(Error::BadLength(8))
        );
        assert_eq!(decode_site_address(&[0, 3, 0, 0, 1, 2, 3, 4], &t), Ok(None));
    }
}
