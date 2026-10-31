//! MS-TURNBWM (sans-I/O): the bandwidth admission control attributes of the Check, Commit and
//! Update requests and the evaluation of their responses.
//!
//! Evidence: `docs/protocol/evidence/23-bandwidth-management-ice2bwm-turnbwm.md` (BW-10 … BW-16).
//! Source: [MS-TURNBWM] 20250520 §2.2 (pp. 13–24), §2.2.4 (pp. 15–16), §3.2.4–3.2.5 (pp. 26–29).
//!
//! Security by design: every response attribute has a fixed length or a bound (dialog and call
//! IDs ≤ 256 octets); a response missing a required attribute is invalid and changes nothing
//! (§3.2.5.1/§3.2.5.2); an all-zero reservation ID means "not managed" and ends the exchange.

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
    pub const REMOTE_SITE_ADDRESS_RESPONSE: u16 = 0x805d;
    pub const REMOTE_RELAY_SITE_ADDRESS_RESPONSE: u16 = 0x805e;
    pub const LOCAL_SITE_ADDRESS_RESPONSE: u16 = 0x805f;
    pub const LOCAL_RELAY_SITE_ADDRESS_RESPONSE: u16 = 0x8060;
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

/// Bandwidth admission control message types (§2.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Check = 0,
    Commit = 1,
    Update = 2,
}

/// Bandwidth Reservation Amount in kbit/s (§2.2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Amount {
    pub min_send: u32,
    pub max_send: u32,
    pub min_receive: u32,
    pub max_receive: u32,
}

impl Amount {
    fn encode(&self) -> Vec<u8> {
        [
            self.min_send,
            self.max_send,
            self.min_receive,
            self.max_receive,
        ]
        .iter()
        .flat_map(|v| v.to_be_bytes())
        .collect()
    }

    fn decode(v: &[u8]) -> Option<Self> {
        let w = |i: usize| -> Option<u32> {
            Some(u32::from_be_bytes(v.get(i..i + 4)?.try_into().ok()?))
        };
        (v.len() == 16).then_some(())?;
        Some(Amount {
            min_send: w(0)?,
            max_send: w(4)?,
            min_receive: w(8)?,
            max_receive: w(12)?,
        })
    }
}

/// Location of the peer and of ourselves (§2.2.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    Unknown = 0,
    Internet = 1,
    Intranet = 2,
}

/// Federation status of the peer (§2.2.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Federation {
    None = 0,
    Enterprise = 1,
    PublicCloud = 2,
}

/// Location Profile (§2.2.14), MUST in Check and Commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocationProfile {
    pub peer: Location,
    pub own: Location,
    pub federation: Federation,
}

/// The longest SIP dialog or call ID carried (§2.2.12, §2.2.13).
pub const MAX_SIP_ID: usize = 256;

/// The site addresses of a Check or Commit (§3.2.4.1, §3.2.4.2; ICE2BWM §3.1.4.8.1.1/§3.1.4.8.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sites {
    pub local: SocketAddr,
    pub local_relay: Option<SocketAddr>,
    pub remote: SocketAddr,
    pub remote_relay: Option<SocketAddr>,
}

fn sip_id(v: &str) -> Option<Vec<u8>> {
    (!v.is_empty() && v.len() <= MAX_SIP_ID && !v.chars().any(char::is_control))
        .then(|| v.as_bytes().to_vec())
}

fn bac(action: Action) -> Vec<u8> {
    vec![0, 0, 0, action as u8]
}

/// Attributes of a Reservation Check (§3.2.4.1). MS-Service-Quality is added by the TURN client.
pub fn check_attributes(
    sites: &Sites,
    amount: Amount,
    call_id: &str,
    profile: LocationProfile,
    transaction: &[u8; 16],
) -> Vec<(u16, Vec<u8>)> {
    let mut v = vec![
        (attr::BAC_MESSAGE, bac(Action::Check)),
        (
            attr::REMOTE_SITE_ADDRESS,
            encode_site_address(sites.remote, transaction),
        ),
    ];
    if let Some(r) = sites.remote_relay {
        v.push((
            attr::REMOTE_RELAY_SITE_ADDRESS,
            encode_site_address(r, transaction),
        ));
    }
    if let Some(r) = sites.local_relay {
        v.push((
            attr::LOCAL_RELAY_SITE_ADDRESS,
            encode_site_address(r, transaction),
        ));
    }
    v.push((
        attr::LOCAL_SITE_ADDRESS,
        encode_site_address(sites.local, transaction),
    ));
    v.push((attr::RESERVATION_AMOUNT, amount.encode()));
    if let Some(id) = sip_id(call_id) {
        v.push((attr::SIP_CALL_ID, id));
    }
    v.push((attr::LOCATION_PROFILE, location_profile(profile)));
    v
}

/// Attributes of a Reservation Commit (§3.2.4.2): the relay sites only if the media path uses
/// them.
pub fn commit_attributes(
    sites: &Sites,
    amount: Amount,
    dialog_id: &str,
    profile: LocationProfile,
    transaction: &[u8; 16],
) -> Vec<(u16, Vec<u8>)> {
    let mut v = vec![
        (attr::BAC_MESSAGE, bac(Action::Commit)),
        (
            attr::REMOTE_SITE_ADDRESS,
            encode_site_address(sites.remote, transaction),
        ),
    ];
    if let Some(r) = sites.remote_relay {
        v.push((
            attr::REMOTE_RELAY_SITE_ADDRESS,
            encode_site_address(r, transaction),
        ));
    }
    v.push((
        attr::LOCAL_SITE_ADDRESS,
        encode_site_address(sites.local, transaction),
    ));
    if let Some(r) = sites.local_relay {
        v.push((
            attr::LOCAL_RELAY_SITE_ADDRESS,
            encode_site_address(r, transaction),
        ));
    }
    v.push((attr::RESERVATION_AMOUNT, amount.encode()));
    if let Some(id) = sip_id(dialog_id) {
        v.push((attr::SIP_DIALOG_ID, id));
    }
    v.push((attr::LOCATION_PROFILE, location_profile(profile)));
    v
}

/// Attributes of a Reservation Update (§3.2.4.3); an all-zero amount cancels the reservation.
pub fn update_attributes(
    id: &[u8; 16],
    amount: Option<Amount>,
    dialog_id: &str,
) -> Vec<(u16, Vec<u8>)> {
    let mut v = vec![
        (attr::BAC_MESSAGE, bac(Action::Update)),
        (attr::RESERVATION_ID, id.to_vec()),
    ];
    if let Some(a) = amount {
        v.push((attr::RESERVATION_AMOUNT, a.encode()));
    }
    if let Some(d) = sip_id(dialog_id) {
        v.push((attr::SIP_DIALOG_ID, d));
    }
    v
}

fn location_profile(p: LocationProfile) -> Vec<u8> {
    vec![p.peer as u8, p.own as u8, p.federation as u8, 0]
}

/// One site address response (§2.2.8–2.2.11): V and F flags, maximum send/receive (kbit/s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiteResponse {
    pub valid: bool,
    /// PSTN failover allowed (remote and local site responses only).
    pub pstn: bool,
    pub max_send: u32,
    pub max_receive: u32,
}

impl SiteResponse {
    fn decode(v: &[u8]) -> Option<Self> {
        let w = |i: usize| -> Option<u32> {
            Some(u32::from_be_bytes(v.get(i..i + 4)?.try_into().ok()?))
        };
        (v.len() == 12).then_some(())?;
        let flags = w(0)?;
        Some(SiteResponse {
            valid: flags & 0x8000_0000 != 0,
            pstn: flags & 0x4000_0000 != 0,
            max_send: w(4)?,
            max_receive: w(8)?,
        })
    }
}

/// The policy from a Check response (§3.2.5.1; ICE2BWM §3.1.5.1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub local: SiteResponse,
    pub local_relay: Option<SiteResponse>,
    pub remote: SiteResponse,
    pub remote_relay: Option<SiteResponse>,
}

impl Policy {
    /// Local host/server-reflexive/peer-derived candidates may be used.
    pub fn local_site_ok(&self) -> bool {
        self.local.valid
    }
    /// Local relayed candidates may be used (absent response = not available).
    pub fn local_relay_ok(&self) -> bool {
        self.local_relay.is_some_and(|r| r.valid)
    }
    pub fn remote_site_ok(&self) -> bool {
        self.remote.valid
    }
    pub fn remote_relay_ok(&self) -> bool {
        self.remote_relay.is_some_and(|r| r.valid)
    }
    /// Full check failure: no valid path out of the local or the remote site.
    pub fn blocked(&self) -> bool {
        !(self.local_site_ok() || self.local_relay_ok())
            || !(self.remote_site_ok() || self.remote_relay_ok())
    }
    /// The policy allows rerouting over the PSTN for a failed site (F flag, §2.2.8).
    pub fn pstn_failover(&self) -> bool {
        (!self.local.valid && self.local.pstn) || (!self.remote.valid && self.remote.pstn)
    }
}

fn get(attrs: &[(u16, Vec<u8>)], kind: u16) -> Option<&[u8]> {
    attrs
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, v)| v.as_slice())
}

/// Evaluates a Check response; `None` if a required site response is missing or malformed
/// (§3.2.5.1: remote and local always, the relay ones when they were asked for).
pub fn check_response(
    attrs: &[(u16, Vec<u8>)],
    asked_remote_relay: bool,
    asked_local_relay: bool,
) -> Option<Policy> {
    let site = |k| get(attrs, k).and_then(SiteResponse::decode);
    let remote = site(attr::REMOTE_SITE_ADDRESS_RESPONSE)?;
    let local = site(attr::LOCAL_SITE_ADDRESS_RESPONSE)?;
    // The relay responses have no F flag: that bit is reserved and ignored (§2.2.9, §2.2.11).
    let no_pstn = |r: SiteResponse| SiteResponse { pstn: false, ..r };
    let remote_relay = site(attr::REMOTE_RELAY_SITE_ADDRESS_RESPONSE).map(no_pstn);
    let local_relay = site(attr::LOCAL_RELAY_SITE_ADDRESS_RESPONSE).map(no_pstn);
    if (asked_remote_relay && remote_relay.is_none())
        || (asked_local_relay && local_relay.is_none())
    {
        return None;
    }
    Some(Policy {
        local,
        local_relay,
        remote,
        remote_relay,
    })
}

/// A Commit or Update response: the reservation ID (all zero = not managed) and the amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reservation {
    pub id: [u8; 16],
    pub amount: Option<Amount>,
}

impl Reservation {
    pub fn managed(&self) -> bool {
        self.id != [0; 16]
    }
}

/// Evaluates a Commit (`amount_required`) or Update response (§3.2.5.2, §3.2.5.3).
pub fn reservation_response(
    attrs: &[(u16, Vec<u8>)],
    amount_required: bool,
) -> Option<Reservation> {
    let id: [u8; 16] = get(attrs, attr::RESERVATION_ID)?.try_into().ok()?;
    let amount = get(attrs, attr::RESERVATION_AMOUNT).and_then(Amount::decode);
    if amount_required && amount.is_none() {
        return None;
    }
    Some(Reservation { id, amount })
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

    fn site(valid: bool, pstn: bool) -> Vec<u8> {
        let flags: u32 = (u32::from(valid) << 31) | (u32::from(pstn) << 30);
        [flags, 100, 200]
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .collect()
    }

    /// §3.2.4.1: Check carries BAC=Check, the sites (XOR with the transaction), the amount, the
    /// call ID and the location profile, in this order; lengths per §2.2.
    #[test]
    fn check_request_attributes() {
        let t = [7u8; 16];
        let sites = Sites {
            local: "10.0.0.1:5000".parse().unwrap(),
            local_relay: Some("192.0.2.1:6000".parse().unwrap()),
            remote: "10.1.0.1:7000".parse().unwrap(),
            remote_relay: None,
        };
        let amount = Amount {
            min_send: 85,
            max_send: 85,
            min_receive: 85,
            max_receive: 85,
        };
        let profile = LocationProfile {
            peer: Location::Unknown,
            own: Location::Intranet,
            federation: Federation::None,
        };
        let a = check_attributes(&sites, amount, "call-1", profile, &t);
        let kinds: Vec<u16> = a.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                attr::BAC_MESSAGE,
                attr::REMOTE_SITE_ADDRESS,
                attr::LOCAL_RELAY_SITE_ADDRESS,
                attr::LOCAL_SITE_ADDRESS,
                attr::RESERVATION_AMOUNT,
                attr::SIP_CALL_ID,
                attr::LOCATION_PROFILE
            ]
        );
        assert_eq!(a[0].1, [0, 0, 0, 0]);
        assert_eq!(decode_site_address(&a[3].1, &t).unwrap(), Some(sites.local));
        assert_eq!(a[4].1.len(), 16);
        assert_eq!(a[6].1, [0, 2, 0, 0]);
        let long = "x".repeat(MAX_SIP_ID + 1);
        assert!(check_attributes(&sites, amount, &long, profile, &t)
            .iter()
            .all(|(k, _)| *k != attr::SIP_CALL_ID));
        let u = update_attributes(&[1; 16], Some(Amount::default()), "d");
        assert_eq!(u[0].1, [0, 0, 0, 2]);
        assert_eq!(u[1].1, [1; 16]);
    }

    /// §3.2.5.1: path rules; missing required responses make the answer invalid.
    #[test]
    fn check_response_rules() {
        let attrs = |l, lr: Option<bool>, r, rr: Option<bool>| {
            let mut v = vec![
                (attr::LOCAL_SITE_ADDRESS_RESPONSE, site(l, false)),
                (attr::REMOTE_SITE_ADDRESS_RESPONSE, site(r, true)),
            ];
            if let Some(x) = lr {
                v.push((attr::LOCAL_RELAY_SITE_ADDRESS_RESPONSE, site(x, false)));
            }
            if let Some(x) = rr {
                v.push((attr::REMOTE_RELAY_SITE_ADDRESS_RESPONSE, site(x, false)));
            }
            v
        };
        let p = check_response(&attrs(true, Some(false), true, None), false, true).unwrap();
        assert!(
            p.local_site_ok() && !p.local_relay_ok() && p.remote_site_ok() && !p.remote_relay_ok()
        );
        assert!(!p.blocked());
        assert_eq!((p.local.max_send, p.local.max_receive), (100, 200));
        let p = check_response(&attrs(false, Some(false), true, None), false, true).unwrap();
        assert!(p.blocked(), "nothing out of the local site");
        let p = check_response(&attrs(true, None, false, Some(true)), true, false).unwrap();
        assert!(!p.blocked(), "the remote relay is a way out");
        let p = check_response(&attrs(true, None, false, None), false, false).unwrap();
        assert!(p.blocked() && p.pstn_failover());
        assert!(
            check_response(&attrs(true, None, true, None), true, false).is_none(),
            "remote relay response missing"
        );
        let mut v = attrs(true, None, true, None);
        v.push((attr::LOCAL_RELAY_SITE_ADDRESS_RESPONSE, site(true, true)));
        assert!(
            !check_response(&v, false, false)
                .unwrap()
                .local_relay
                .unwrap()
                .pstn,
            "reserved bit ignored"
        );
        assert!(check_response(
            &[(attr::LOCAL_SITE_ADDRESS_RESPONSE, vec![0; 11])],
            false,
            false
        )
        .is_none());
    }

    #[test]
    fn reservation_responses() {
        let amount = Amount {
            min_send: 1,
            max_send: 2,
            min_receive: 3,
            max_receive: 4,
        };
        let a = vec![
            (attr::RESERVATION_ID, vec![9; 16]),
            (attr::RESERVATION_AMOUNT, amount.encode()),
        ];
        let r = reservation_response(&a, true).unwrap();
        assert!(r.managed());
        assert_eq!(r.amount, Some(amount));
        assert!(
            !reservation_response(&[(attr::RESERVATION_ID, vec![0; 16])], false)
                .unwrap()
                .managed()
        );
        assert!(
            reservation_response(&[(attr::RESERVATION_ID, vec![0; 16])], true).is_none(),
            "commit needs the amount"
        );
        assert!(reservation_response(&[(attr::RESERVATION_ID, vec![0; 15])], false).is_none());
    }
}
