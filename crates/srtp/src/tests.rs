use super::*;

fn hex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn arr<const N: usize>(s: &str) -> [u8; N] {
    hex(s).try_into().unwrap()
}

/// RFC 3711 appendix B.3 (IE-09).
#[test]
fn rfc3711_b3_key_derivation() {
    let key: [u8; 16] = arr("E1F97A0D3E018BE0D64FA32C06DE4139");
    let salt: [u8; 14] = arr("0EC675AD498AFEEBB6960B3AABE6");
    let k = SessionKeys::derive(&key, &salt, false);
    assert_eq!(
        k.encryption.to_vec(),
        hex("C61E7A93744F39EE10734AFE3FF7A087")
    );
    assert_eq!(k.salt.to_vec(), hex("30CBBC08863D8C85D49DB34A9AE1"));
    assert_eq!(
        k.authentication.to_vec(),
        hex("CEBE321F6FF7716B6FD4AB49AF256A156D38BAA4")
    );
}

const MASTER_KEY: &str = "CB4A3C93F3D587ABA1AB0BDF8C6AA0FB";
const MASTER_SALT: &str = "53EF4F4594296D0EB286D9CC96E4";

/// [MS-SSRTP] §4.1, p. 18.
#[test]
fn ssrtp_key_derivation() {
    let rtp = SessionKeys::derive(&arr(MASTER_KEY), &arr(MASTER_SALT), false);
    assert_eq!(
        rtp.encryption.to_vec(),
        hex("C3FCC67BFBF17CFA2DC69F4B4CFC59CD")
    );
    assert_eq!(
        rtp.authentication.to_vec(),
        hex("23B8B2D911CF8C6416F4AAB94083E0CC32615694")
    );
    assert_eq!(rtp.salt.to_vec(), hex("929B3AD0FDB565FDBEAA50412C8D"));
    let rtcp = SessionKeys::derive(&arr(MASTER_KEY), &arr(MASTER_SALT), true);
    assert_eq!(
        rtcp.encryption.to_vec(),
        hex("122E3C94A0D945242AF0B79C6EDCE0BB")
    );
    assert_eq!(
        rtcp.authentication.to_vec(),
        hex("999BDAC078DBC12E7677AD05B9B2B54CBFDCBAA6")
    );
    assert_eq!(rtcp.salt.to_vec(), hex("839D270762975E43F6351493434E"));
}

const PAYLOAD: &str = "3F68B92587D38C18D22AFA3FCF30B63098BDB1213F30F91054911E0521EE3A8EE386794C5B5F
D4B9A6477719F27937B6A0C7E8221250A57C5A42E8A99565F7559F21998F2555003F4677DB4AFCD3
59738B51D538B4BE1780CC618E686E9862343F0C65D5A86C334B1915B48D99FCAD8E39E9C8F9BD6
915FD7CBBFFD94A73F373615C5CC8C827B2E4C33EEB492D38";

const ENCRYPTED: &str =
    "C1D49FFD5B845AAC755FCE604A2B9225D672DDB5A3C4664447F3D39D841B6C84373437FAED0
11C30AD1D91FB9CC7CF1796A97D99886EBB694E6C050ED100073D2526C9FC56AB08555B3A1A2589
D1491D0402EB79C1C1C6E439C815B4AB83421F57293008B70AB296DAFFD7E6E2E67E6A93FF89FE8C
DE14C49FBAB13E233793B1934AA8A5BDBC3BD6B0A91D520EC9";

/// [MS-SSRTP] §4.2, pp. 18–19.
#[test]
fn ssrtp_packet_transform() {
    let keys = SessionKeys::derive(&arr(MASTER_KEY), &arr(MASTER_SALT), false);
    let header = hex("80728001AE773346DE1A3236");
    let payload = hex(PAYLOAD);
    assert_eq!(payload.len(), 142);
    let out = ssrtp_protect(&keys, &header, &payload, 0x5E1A_3236_8001, 2).unwrap();
    assert_eq!(out.encrypted_payload, hex(ENCRYPTED));
    let mut expected_block = hex(ENCRYPTED);
    expected_block.extend(hex("5E1A32368001"));
    expected_block.resize(192, 0);
    expected_block.extend(hex("80728001AE773346DE1A3236"));
    expected_block.extend(hex("00000002"));
    assert_eq!(out.authenticated_block.len(), 208);
    assert_eq!(out.authenticated_block, expected_block);
    assert_eq!(out.tag.to_vec(), hex("2FA5BAC13AC58423BE4A"));
}

#[test]
fn rejects_malformed_input() {
    let keys = SessionKeys::derive(&[0; 16], &[0; 14], false);
    assert_eq!(
        ssrtp_protect(&keys, &[0x80], &[], 0, 0),
        Err(SrtpError::MalformedHeader)
    );
    let header_with_missing_csrc = hex("81728001AE773346DE1A3236");
    assert_eq!(
        ssrtp_protect(&keys, &header_with_missing_csrc, &[], 0, 0),
        Err(SrtpError::MalformedHeader)
    );
    assert_eq!(
        ssrtp_protect(&keys, &hex("80728001AE773346DE1A3236"), &[], MAX_ESN + 1, 0),
        Err(SrtpError::EsnOutOfRange)
    );
}

/// RFC 3711 appendix B.2: AES-CM key stream at the start and around the 2^16 block boundary.
#[test]
fn rfc3711_b2_aes_cm_keystream() {
    let key: [u8; 16] = arr("2B7E151628AED2A6ABF7158809CF4F3C");
    let iv = u128::from_be_bytes(arr("F0F1F2F3F4F5F6F7F8F9FAFBFCFD0000"));
    let mut ks = vec![0u8; 65282 * 16];
    aes_cm(&key, iv, &mut ks);
    assert_eq!(&ks[..48], hex("E03EAD0935C95E80E166B16DD92B4EB4 D23513162B02D0F72A43A2FE4A5F97AB 41E95B3BB0A2E8DD477901E4FCA894C0"));
    let block = |n: usize| ks[n * 16..n * 16 + 16].to_vec();
    assert_eq!(block(0xfeff), hex("EC8CDF7398607CB0F2D21675EA9EA1E4"));
    assert_eq!(block(0xff00), hex("362B7C3C6773516318A077D7FC5073AE"));
    assert_eq!(block(0xff01), hex("6A2CC3787889374FBEB4C81B17BA6C44"));
}

/// Secondary reference (libsrtp v2.6.0 `test/srtp_driver.c`, `srtp_validate()`, data only):
/// RFC 3711 B.3 master key/salt, SSRC 0xCAFEBABE, no MKI, first SRTCP index 1.
#[test]
fn libsrtp_reference_packets() {
    let key: [u8; 16] = arr("E1F97A0D3E018BE0D64FA32C06DE4139");
    let salt: [u8; 14] = arr("0EC675AD498AFEEBB6960B3AABE6");
    let rtp = hex("800F1234DECAFBADCAFEBABE ABABABABABABABABABABABABABABABAB");
    let srtp =
        hex("800F1234DECAFBADCAFEBABE 4E55DC4CE79978D88CA4D215949D2402 B78D6ACC99EA179B8DBB");
    let mut tx = Context::new(&key, &salt, None);
    assert_eq!(tx.protect_rtp(&rtp).unwrap(), srtp);
    let mut rx = Context::new(&key, &salt, None);
    assert_eq!(rx.unprotect_rtp(&srtp).unwrap(), rtp);
    assert_eq!(rx.unprotect_rtp(&srtp), Err(SrtpError::Replay));

    let rtcp = hex("81C8000BCAFEBABE ABABABABABABABABABABABABABABABAB");
    let srtcp =
        hex("81C8000BCAFEBABE 7128035BE487B9BDBEF89041F977A5A8 80000001 993E08CD54D6C1230798");
    tx.set_rtcp_index(1);
    assert_eq!(tx.protect_rtcp(&rtcp).unwrap(), srtcp);
    assert_eq!(rx.unprotect_rtcp(&srtcp).unwrap(), rtcp);
    assert_eq!(rx.unprotect_rtcp(&srtcp), Err(SrtpError::Replay));
    assert_eq!(rx.failures, 2);
}

fn rtp_packet(seq: u16, ssrc: u32, payload: &[u8]) -> Vec<u8> {
    let mut p = vec![0x80, 9];
    p.extend_from_slice(&seq.to_be_bytes());
    p.extend_from_slice(&u32::from(seq).wrapping_mul(160).to_be_bytes());
    p.extend_from_slice(&ssrc.to_be_bytes());
    p.extend_from_slice(payload);
    p
}

/// MS-SRTP profile: one-byte MKI (SR-03), tampering, wrong MKI, out-of-order inside the window,
/// too old packets, ROC rollover across sequence number 65535 → 0.
#[test]
fn profile_replay_and_rollover() {
    let key = [7u8; 16];
    let salt = [9u8; 14];
    let mut tx = Context::new(&key, &salt, Some(1));
    let mut rx = Context::new(&key, &salt, Some(1));
    let seqs: Vec<u16> = (65500..=65535).chain(0..40).collect();
    let packets: Vec<Vec<u8>> = seqs
        .iter()
        .map(|&s| {
            tx.protect_rtp(&rtp_packet(s, 0x1234, &[s as u8; 160]))
                .unwrap()
        })
        .collect();
    assert_eq!(packets[0].len(), 12 + 160 + 1 + 10);
    // Deliver with some reordering.
    let mut order: Vec<usize> = (0..packets.len()).collect();
    order.swap(10, 12);
    order.swap(40, 37);
    for &k in &order {
        let p = rx.unprotect_rtp(&packets[k]).unwrap();
        assert_eq!(p, rtp_packet(seqs[k], 0x1234, &[seqs[k] as u8; 160]));
    }
    // After the wrap the ROC is 1: a replay of the first packet (index 65500) is too old.
    assert_eq!(rx.unprotect_rtp(&packets[0]), Err(SrtpError::Replay));
    // Tampered payload, tag or MKI.
    let mut bad = tx.protect_rtp(&rtp_packet(40, 0x1234, &[1; 20])).unwrap();
    bad[15] ^= 1;
    assert_eq!(rx.unprotect_rtp(&bad), Err(SrtpError::Authentication));
    let mut bad = tx.protect_rtp(&rtp_packet(41, 0x1234, &[1; 20])).unwrap();
    let mki_at = bad.len() - 11;
    bad[mki_at] = 2;
    assert_eq!(rx.unprotect_rtp(&bad), Err(SrtpError::UnknownMki));
    assert_eq!(
        rx.unprotect_rtp(&[0x80, 0, 0]),
        Err(SrtpError::MalformedHeader)
    );
    // A different key never authenticates.
    let mut other = Context::new(&[8u8; 16], &salt, Some(1));
    let foreign = other
        .protect_rtp(&rtp_packet(42, 0x1234, &[1; 20]))
        .unwrap();
    assert_eq!(rx.unprotect_rtp(&foreign), Err(SrtpError::Authentication));
    // SRTCP with MKI, index shared by all streams.
    let mut rtcp = vec![0x80, 201, 0, 1];
    rtcp.extend_from_slice(&0x1234u32.to_be_bytes());
    let a = tx.protect_rtcp(&rtcp).unwrap();
    let b = tx.protect_rtcp(&rtcp).unwrap();
    assert_ne!(a, b);
    assert_eq!(rx.unprotect_rtcp(&b).unwrap(), rtcp);
    assert_eq!(
        rx.unprotect_rtcp(&a).unwrap(),
        rtcp,
        "older index inside the window"
    );
}

#[test]
fn bounded_number_of_streams() {
    let mut tx = Context::new(&[1; 16], &[2; 14], Some(0));
    let mut rx = Context::new(&[1; 16], &[2; 14], Some(0));
    for ssrc in 0..MAX_STREAMS as u32 {
        let p = tx.protect_rtp(&rtp_packet(1, ssrc, &[0; 4])).unwrap();
        rx.unprotect_rtp(&p).unwrap();
    }
    let extra = tx.protect_rtp(&rtp_packet(1, 999, &[0; 4]));
    assert_eq!(extra, Err(SrtpError::TooManyStreams));
}

/// One SSRTP wire packet (§2.2.1): header ‖ encrypted payload ‖ ESN ‖ MKI ‖ tag, built with the
/// transform that reproduces the §4.2 vector.
fn ssrtp_wire(header: &[u8], payload: &[u8], esn: u64, roc: u32) -> Vec<u8> {
    let keys = SessionKeys::derive(&arr(MASTER_KEY), &arr(MASTER_SALT), false);
    let p = ssrtp_protect(&keys, header, payload, esn, roc).unwrap();
    let mut w = header.to_vec();
    w.extend(p.encrypted_payload);
    w.extend(&esn.to_be_bytes()[2..]);
    w.push(1);
    w.extend(p.tag);
    w
}

/// [MS-SSRTP] §3.1.5.1.3.2 (SS-06): receive, then refuse tampering, replay, a wrong MKI and a
/// header extension; ESN, MKI and tag are removed.
#[test]
fn ssrtp_receive() {
    let mut rx = Context::new(&arr(MASTER_KEY), &arr(MASTER_SALT), Some(1));
    let header = hex("80728001AE773346DE1A3236");
    let payload = hex(PAYLOAD);
    let wire = ssrtp_wire(&header, &payload, 0x5E1A_3236_8001, 0);
    let plain = rx.unprotect_ssrtp(&wire).unwrap();
    assert_eq!(&plain[..12], &header[..]);
    assert_eq!(&plain[12..], &payload[..]);
    assert_eq!(
        rx.unprotect_ssrtp(&wire),
        Err(SrtpError::Replay),
        "same RTP index"
    );
    // The next packet (ESN not contiguous, §2.2.1) is accepted.
    let next = hex("80728002AE773346DE1A3236");
    assert!(rx
        .unprotect_ssrtp(&ssrtp_wire(&next, &payload, 0x5E1A_3236_9001, 0))
        .is_ok());
    let mut bad = ssrtp_wire(
        &hex("80728003AE773346DE1A3236"),
        &payload,
        0x5E1A_3236_9101,
        0,
    );
    let n = bad.len();
    bad[n - 12] ^= 1; // the ESN
    assert_eq!(rx.unprotect_ssrtp(&bad), Err(SrtpError::Authentication));
    let mut wrong_mki = ssrtp_wire(
        &hex("80728004AE773346DE1A3236"),
        &payload,
        0x5E1A_3236_9201,
        0,
    );
    wrong_mki[n - 11] = 2;
    assert_eq!(rx.unprotect_ssrtp(&wrong_mki), Err(SrtpError::UnknownMki));
    let with_ext = hex("90728005AE773346DE1A3236BEDE000100000000");
    assert_eq!(
        rx.unprotect_ssrtp(&ssrtp_wire(&hex("80728005AE773346DE1A3236"), &payload, 1, 0)[..0]),
        Err(SrtpError::MalformedHeader)
    );
    let mut ext_packet = with_ext.clone();
    ext_packet.extend(vec![0u8; 17]);
    assert_eq!(
        rx.unprotect_ssrtp(&ext_packet),
        Err(SrtpError::MalformedHeader),
        "header extension refused"
    );
    assert_eq!(rx.failures, 5);
}
