//! Test vectors from [MS-NLMP] §4.2.1 and §4.2.4 (NTLMv2), pp. 80–83.

use super::*;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

const SERVER_CHALLENGE: [u8; 8] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
const CLIENT_CHALLENGE: [u8; 8] = [0xaa; 8];
const RANDOM_SESSION_KEY: [u8; 16] = [0x55; 16];

fn av_pairs() -> Vec<u8> {
    let mut v = vec![0x02, 0x00, 0x0c, 0x00];
    v.extend(utf16le("Domain"));
    v.extend([0x01, 0x00, 0x0c, 0x00]);
    v.extend(utf16le("Server"));
    v.extend([0, 0, 0, 0]);
    v
}

#[test]
fn ntowf_v2_vector() {
    let k = ntowf_v2("Password", "User", "Domain");
    assert_eq!(to_hex(&k), "0c868a403bfd7a93a3001ef22ef02e3f");
}

#[test]
fn lmv2_vector() {
    let k = ntowf_v2("Password", "User", "Domain");
    let lm = lmv2_response(&k, SERVER_CHALLENGE, CLIENT_CHALLENGE);
    assert_eq!(
        to_hex(&lm),
        "86c35097ac9cec102554764a57cccc19aaaaaaaaaaaaaaaa"
    );
}

#[test]
fn nt_proof_and_session_base_key_vectors() {
    let k = ntowf_v2("Password", "User", "Domain");
    let temp = ntlmv2_temp([0; 8], CLIENT_CHALLENGE, &av_pairs());
    let proof = nt_proof_str(&k, SERVER_CHALLENGE, &temp);
    assert_eq!(to_hex(&proof), "68cd0ab851e51c96aabc927bebef6a1c");
    let sbk = session_base_key(&k, &proof);
    assert_eq!(to_hex(&sbk), "8de40ccadbc14a82f15cb0ad0de95ca3");
    assert_eq!(
        to_hex(&encrypt_session_key(&sbk, RANDOM_SESSION_KEY)),
        "c5dad2544fc9799094ce1ce90bc9d03e"
    );
}

#[test]
fn sign_and_seal_key_vectors() {
    assert_eq!(
        to_hex(&seal_key(&RANDOM_SESSION_KEY, Direction::ClientToServer)),
        "59f600973cc4960a25480a7c196e4c58"
    );
    assert_eq!(
        to_hex(&sign_key(&RANDOM_SESSION_KEY, Direction::ClientToServer)),
        "4788dc861b4782f35d43fd98fe1a2d39"
    );
}

#[test]
fn seal_and_signature_vector() {
    let seal = seal_key(&RANDOM_SESSION_KEY, Direction::ClientToServer);
    let sign = sign_key(&RANDOM_SESSION_KEY, Direction::ClientToServer);
    let mut sealing = Rc4::new(&seal);
    let mut data = utf16le("Plaintext");
    sealing.apply(&mut data);
    assert_eq!(to_hex(&data), "54e50165bf1936dc996020c1811b0f06fb5f");
    let sig = signature_ess(&sign, &mut sealing, 0, &utf16le("Plaintext"));
    assert_eq!(to_hex(&sig), "010000007fb38ec5c55d497600000000");
    assert_eq!(hex("7fb38ec5c55d4976"), sig[4..12].to_vec());
}

#[test]
fn connectionless_signature_carries_the_sip_sequence_number() {
    let sig = signature_connectionless(&[1; 16], &[2; 16], SIP_SEQUENCE_NUMBER, b"<NTLM>");
    assert_eq!(&sig[0..4], &[1, 0, 0, 0]);
    assert_eq!(&sig[12..16], &SIP_SEQUENCE_NUMBER.to_le_bytes());
}
