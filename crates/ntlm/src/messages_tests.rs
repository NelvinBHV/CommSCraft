//! CHALLENGE/AUTHENTICATE test vectors from [MS-NLMP] 20260330 §4.2.4.3 (p. 82) and hostile input.

use crate::messages::*;
use crate::{hmac_md5, nt_proof_str, ntowf_v2, utf16le};

fn hex(s: &str) -> Vec<u8> {
    let s: String = s.split_whitespace().collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

const CHALLENGE: &str = "4e 54 4c 4d 53 53 50 00 02 00 00 00 0c 00 0c 00
    38 00 00 00 33 82 8a e2 01 23 45 67 89 ab cd ef
    00 00 00 00 00 00 00 00 24 00 24 00 44 00 00 00
    06 00 70 17 00 00 00 0f 53 00 65 00 72 00 76 00
    65 00 72 00 02 00 0c 00 44 00 6f 00 6d 00 61 00
    69 00 6e 00 01 00 0c 00 53 00 65 00 72 00 76 00
    65 00 72 00 00 00 00 00";

const AUTHENTICATE: &str = "4e 54 4c 4d 53 53 50 00 03 00 00 00 18 00 18 00
    6c 00 00 00 54 00 54 00 84 00 00 00 0c 00 0c 00
    48 00 00 00 08 00 08 00 54 00 00 00 10 00 10 00
    5c 00 00 00 10 00 10 00 d8 00 00 00 35 82 88 e2
    05 01 28 0a 00 00 00 0f 44 00 6f 00 6d 00 61 00
    69 00 6e 00 55 00 73 00 65 00 72 00 43 00 4f 00
    4d 00 50 00 55 00 54 00 45 00 52 00 86 c3 50 97
    ac 9c ec 10 25 54 76 4a 57 cc cc 19 aa aa aa aa
    aa aa aa aa 68 cd 0a b8 51 e5 1c 96 aa bc 92 7b
    eb ef 6a 1c 01 01 00 00 00 00 00 00 00 00 00 00
    00 00 00 00 aa aa aa aa aa aa aa aa 00 00 00 00
    02 00 0c 00 44 00 6f 00 6d 00 61 00 69 00 6e 00
    01 00 0c 00 53 00 65 00 72 00 76 00 65 00 72 00
    00 00 00 00 00 00 00 00 c5 da d2 54 4f c9 79 90
    94 ce 1c e9 0b c9 d0 3e";

fn input(flags: u32, modern: bool) -> AuthenticateInput<'static> {
    AuthenticateInput {
        user: "User",
        domain: "Domain",
        password: "Password",
        workstation: "COMPUTER",
        flags,
        version: [0x05, 0x01, 0x28, 0x0a, 0x00, 0x00, 0x00, 0x0f],
        client_challenge: [0xaa; 8],
        random_session_key: [0x55; 16],
        now_filetime: 0,
        modern,
        negotiate: None,
        channel_bindings: None,
    }
}

#[test]
fn parses_the_example_challenge() {
    let ch = Challenge::parse(&hex(CHALLENGE)).unwrap();
    assert_eq!(ch.flags, 0xe28a_8233);
    assert_eq!(
        ch.server_challenge,
        [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]
    );
    assert_eq!(ch.target_name, utf16le("Server"));
    assert_eq!(ch.av(2), Some(utf16le("Domain").as_slice()));
    assert_eq!(ch.av(1), Some(utf16le("Server").as_slice()));
    assert_eq!(ch.target_info.len(), 2);
}

/// The complete AUTHENTICATE_MESSAGE of §4.2.4.3, byte for byte.
#[test]
fn builds_the_example_authenticate() {
    let ch = Challenge::parse(&hex(CHALLENGE)).unwrap();
    let (msg, key) = authenticate(&ch, &input(0xe288_8235, false)).unwrap();
    assert_eq!(msg, hex(AUTHENTICATE));
    assert_eq!(key.as_bytes(), &[0x55; 16]);
    assert_eq!(format!("{key:?}"), "SessionKey(<redacted>)");
}

#[test]
fn flag_choice_follows_the_policy() {
    let ch = Challenge::parse(&hex(CHALLENGE)).unwrap();
    let chosen = choose_flags(ch.flags | flags::DATAGRAM).unwrap();
    for f in [
        flags::DATAGRAM,
        flags::IDENTIFY,
        flags::SIGN,
        flags::EXTENDED_SESSIONSECURITY,
        flags::KEY_EXCH,
        flags::N128,
    ] {
        assert_ne!(chosen & f, 0, "{f:#x}");
    }
    for f in [
        flags::OEM,
        flags::VERSION,
        flags::TARGET_TYPE_SERVER,
        flags::LM_KEY,
    ] {
        assert_eq!(chosen & f, 0, "{f:#x}");
    }
    assert_eq!(
        choose_flags(ch.flags & !flags::EXTENDED_SESSIONSECURITY),
        Err(Error::MissingFlag(flags::EXTENDED_SESSIONSECURITY))
    );
    assert_eq!(
        choose_flags(ch.flags & !(flags::KEY_EXCH | flags::N128)),
        Err(Error::MissingFlag(flags::KEY_EXCH | flags::N128))
    );
}

/// A challenge with MsvAvTimestamp: LM response empty, MsvAvFlags/channel bindings/target name
/// added, MIC over CHALLENGE ‖ AUTHENTICATE (§3.1.5.2.1).
#[test]
fn modern_authenticate_with_mic() {
    let mut raw = hex(CHALLENGE);
    // Replace the AV list: Domain, Server, Timestamp, EOL; fix TargetInfo length.
    raw.truncate(0x44);
    let mut avs = vec![0x02, 0x00, 0x0c, 0x00];
    avs.extend(utf16le("Domain"));
    avs.extend([0x01, 0x00, 0x0c, 0x00]);
    avs.extend(utf16le("Server"));
    avs.extend([0x07, 0x00, 0x08, 0x00, 1, 2, 3, 4, 5, 6, 7, 8]);
    avs.extend([0, 0, 0, 0]);
    let len = u16::try_from(avs.len()).unwrap().to_le_bytes();
    raw[0x28..0x2a].copy_from_slice(&len);
    raw[0x2a..0x2c].copy_from_slice(&len);
    raw.extend(&avs);
    let ch = Challenge::parse(&raw).unwrap();
    let flags = choose_flags(ch.flags | flags::DATAGRAM).unwrap();
    let (msg, key) = authenticate(&ch, &input(flags, true)).unwrap();

    let field = |at: usize| {
        let len = usize::from(u16::from_le_bytes([msg[at], msg[at + 1]]));
        let off = u32::from_le_bytes(msg[at + 4..at + 8].try_into().unwrap()) as usize;
        msg[off..off + len].to_vec()
    };
    assert!(field(12).is_empty(), "no LM response");
    assert_eq!(&msg[64..72], &[0; 8], "no Windows version claimed");
    let nt = field(20);
    let temp = &nt[16..];
    assert_eq!(
        &temp[8..16],
        &[1, 2, 3, 4, 5, 6, 7, 8],
        "time from MsvAvTimestamp"
    );
    let k = ntowf_v2("Password", "User", "Domain");
    assert_eq!(
        nt_proof_str(&k, ch.server_challenge, temp).to_vec(),
        nt[..16].to_vec()
    );
    let avs_sent = &temp[28..];
    let find = |id: u16| {
        let mut b = avs_sent;
        loop {
            let i = u16::from_le_bytes([b[0], b[1]]);
            let l = usize::from(u16::from_le_bytes([b[2], b[3]]));
            if i == id {
                return Some(b[4..4 + l].to_vec());
            }
            if i == 0 {
                return None;
            }
            b = &b[4 + l..];
        }
    };
    assert_eq!(find(av::FLAGS), Some(2u32.to_le_bytes().to_vec()));
    assert_eq!(find(av::CHANNEL_BINDINGS), Some(vec![0; 16]));
    assert_eq!(find(av::TARGET_NAME), Some(vec![]));

    let mut zeroed = msg.clone();
    zeroed[72..88].fill(0);
    let mic = hmac_md5(key.as_bytes(), &[&raw, &zeroed]);
    assert_eq!(&msg[72..88], &mic);
}

#[test]
fn hostile_challenges_are_rejected() {
    let good = hex(CHALLENGE);
    assert_eq!(Challenge::parse(&good[..20]), Err(Error::BadField));
    let mut bad = good.clone();
    bad[0] = b'X';
    assert_eq!(Challenge::parse(&bad), Err(Error::BadSignature));
    let mut bad = good.clone();
    bad[8] = 3;
    assert_eq!(Challenge::parse(&bad), Err(Error::WrongMessageType));
    let mut bad = good.clone();
    bad[0x2c] = 0xf0; // TargetInfo offset past the end
    assert_eq!(Challenge::parse(&bad), Err(Error::BadField));
    let mut bad = good.clone();
    bad[0x10..0x14].fill(0xff); // TargetName offset at the end of the address space
    assert_eq!(Challenge::parse(&bad), Err(Error::BadField));
    let mut bad = good.clone();
    let n = bad.len();
    bad[n - 4] = 0x01; // EOL replaced: list never ends inside the field
    assert_eq!(Challenge::parse(&bad), Err(Error::BadTargetInfo));
    let mut bad = good.clone();
    bad[0x54] = 0x02; // second pair (at 0x44 + 4 + 12) also MsvAvNbDomainName
    assert_eq!(Challenge::parse(&bad), Err(Error::BadTargetInfo));
    assert_eq!(
        Challenge::parse(&vec![0; MAX_CHALLENGE_LEN + 1]),
        Err(Error::TooLong)
    );
}

#[test]
fn mutated_challenges_never_panic() {
    let good = hex(CHALLENGE);
    let mut s: u64 = 0x2545_f491_4f6c_dd1d;
    let mut rnd = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    for _ in 0..20_000 {
        let mut m = good.clone();
        for _ in 0..(rnd() % 6 + 1) {
            let pos = (rnd() as usize) % m.len();
            match rnd() % 3 {
                0 => m[pos] = rnd() as u8,
                1 => m.truncate(pos.max(1)),
                _ => m.insert(pos, rnd() as u8),
            }
        }
        if let Ok(ch) = Challenge::parse(&m) {
            let _ = authenticate(&ch, &input(ch.flags, true));
        }
    }
}

/// Connection-oriented NTLM (HTTP): NEGOTIATE message layout and the MIC over all three messages.
#[test]
fn connection_oriented_negotiate_and_mic() {
    let neg = negotiate_message(HTTP_NEGOTIATE);
    assert_eq!(&neg[..12], b"NTLMSSP\0\x01\0\0\0");
    assert_eq!(
        u32::from_le_bytes(neg[12..16].try_into().unwrap()),
        HTTP_NEGOTIATE
    );
    assert_eq!(neg.len(), 32);
    assert_eq!(HTTP_NEGOTIATE & flags::DATAGRAM, 0);
    // A challenge with a timestamp (as built in modern_authenticate_with_mic).
    let mut raw = hex(CHALLENGE);
    raw.truncate(0x44);
    let mut avs = vec![0x02, 0x00, 0x0c, 0x00];
    avs.extend(utf16le("Domain"));
    avs.extend([0x07, 0x00, 0x08, 0x00, 1, 2, 3, 4, 5, 6, 7, 8]);
    avs.extend([0, 0, 0, 0]);
    let len = u16::try_from(avs.len()).unwrap().to_le_bytes();
    raw[0x28..0x2a].copy_from_slice(&len);
    raw[0x2a..0x2c].copy_from_slice(&len);
    raw.extend(&avs);
    let ch = Challenge::parse(&raw).unwrap();
    let flags = choose_flags_http(ch.flags).unwrap();
    assert_eq!(flags & flags::DATAGRAM, 0);
    let mut inp = input(flags, true);
    inp.negotiate = Some(&neg);
    let (msg, key) = authenticate(&ch, &inp).unwrap();
    let mut zeroed = msg.clone();
    zeroed[72..88].fill(0);
    assert_eq!(
        &msg[72..88],
        &hmac_md5(key.as_bytes(), &[&neg, &raw, &zeroed])
    );
    assert_ne!(&msg[72..88], &hmac_md5(key.as_bytes(), &[&raw, &zeroed]));
}
