//! NTLMv2 key derivation and message signing.
//!
//! Evidence: `docs/protocol/evidence/35-windows-ntlm-kerberos.md` (NL-01 … NL-05) and
//! `docs/protocol/evidence/03-sipae-authentication.md` (AE-24, AE-29, AE-30).
//!
//! Only the cryptographic primitives live here; message parsing and the SIP security
//! association are separate layers.

use hmac::{Hmac, Mac};
use md4::Md4;
use md5::{Digest, Md5};

/// Sequence number that MS-SIPAE fixes for SIP message signatures (AE-30).
pub const SIP_SEQUENCE_NUMBER: u32 = 100;

const CLIENT_SIGN_MAGIC: &[u8] = b"session key to client-to-server signing key magic constant\0";
const SERVER_SIGN_MAGIC: &[u8] = b"session key to server-to-client signing key magic constant\0";
const CLIENT_SEAL_MAGIC: &[u8] = b"session key to client-to-server sealing key magic constant\0";
const SERVER_SEAL_MAGIC: &[u8] = b"session key to server-to-client sealing key magic constant\0";

/// Direction of a key or signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    ClientToServer,
    ServerToClient,
}

fn utf16le(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn md5(parts: &[&[u8]]) -> [u8; 16] {
    let mut h = Md5::new();
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}

/// HMAC-MD5 over the concatenation of `parts`. HMAC accepts keys of any length, so the
/// constructor cannot fail; the fallback keeps the function total without panicking.
fn hmac_md5(key: &[u8], parts: &[&[u8]]) -> [u8; 16] {
    match <Hmac<Md5> as Mac>::new_from_slice(key) {
        Ok(mut m) => {
            for p in parts {
                m.update(p);
            }
            m.finalize().into_bytes().into()
        }
        Err(_) => [0u8; 16],
    }
}

/// RC4 stream cipher state (MS-NLMP uses RC4 for sealing and for the signature checksum).
#[derive(Clone)]
pub struct Rc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    /// Key schedule. An empty key yields the identity permutation (never used by NTLM).
    #[allow(clippy::indexing_slicing)] // u8 indices into a 256-entry table are always in bounds
    pub fn new(key: &[u8]) -> Self {
        let mut s = [0u8; 256];
        for (i, v) in s.iter_mut().enumerate() {
            *v = i as u8;
        }
        if !key.is_empty() {
            let mut j: u8 = 0;
            for i in 0..=255u8 {
                let k = key.get(usize::from(i) % key.len()).copied().unwrap_or(0);
                j = j.wrapping_add(s[usize::from(i)]).wrapping_add(k);
                s.swap(usize::from(i), usize::from(j));
            }
        }
        Self { s, i: 0, j: 0 }
    }

    /// XORs the key stream into `data`.
    #[allow(clippy::indexing_slicing)] // u8 indices into a 256-entry table are always in bounds
    pub fn apply(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[usize::from(self.i)]);
            self.s.swap(usize::from(self.i), usize::from(self.j));
            let t = self.s[usize::from(self.i)].wrapping_add(self.s[usize::from(self.j)]);
            *b ^= self.s[usize::from(t)];
        }
    }
}

/// NTOWFv2 (NL-02): HMAC_MD5(MD4(UTF-16LE(password)), UTF-16LE(uppercase(user) ‖ domain)).
pub fn ntowf_v2(password: &str, user: &str, domain: &str) -> [u8; 16] {
    let nt_hash: [u8; 16] = Md4::digest(utf16le(password)).into();
    let identity = utf16le(&(user.to_uppercase() + domain));
    hmac_md5(&nt_hash, &[&identity])
}

/// The `temp` structure of the NTLMv2 response (NL-02):
/// `01 01 Z(6) Time ClientChallenge Z(4) AvPairs Z(4)`.
pub fn ntlmv2_temp(time: [u8; 8], client_challenge: [u8; 8], av_pairs: &[u8]) -> Vec<u8> {
    let mut t = Vec::with_capacity(28 + av_pairs.len() + 4);
    t.extend_from_slice(&[0x01, 0x01, 0, 0, 0, 0, 0, 0]);
    t.extend_from_slice(&time);
    t.extend_from_slice(&client_challenge);
    t.extend_from_slice(&[0, 0, 0, 0]);
    t.extend_from_slice(av_pairs);
    t.extend_from_slice(&[0, 0, 0, 0]);
    t
}

/// NTProofStr = HMAC_MD5(ResponseKeyNT, ServerChallenge ‖ temp) (NL-02).
pub fn nt_proof_str(
    response_key_nt: &[u8; 16],
    server_challenge: [u8; 8],
    temp: &[u8],
) -> [u8; 16] {
    hmac_md5(response_key_nt, &[&server_challenge, temp])
}

/// SessionBaseKey = HMAC_MD5(ResponseKeyNT, NTProofStr); for NTLMv2 also the KeyExchangeKey.
pub fn session_base_key(response_key_nt: &[u8; 16], nt_proof: &[u8; 16]) -> [u8; 16] {
    hmac_md5(response_key_nt, &[nt_proof])
}

/// LMv2 response = HMAC_MD5(ResponseKeyLM, ServerChallenge ‖ ClientChallenge) ‖ ClientChallenge.
pub fn lmv2_response(
    response_key_lm: &[u8; 16],
    server_challenge: [u8; 8],
    client_challenge: [u8; 8],
) -> [u8; 24] {
    let mac = hmac_md5(response_key_lm, &[&server_challenge, &client_challenge]);
    let mut out = [0u8; 24];
    for (dst, src) in out
        .iter_mut()
        .zip(mac.iter().chain(client_challenge.iter()))
    {
        *dst = *src;
    }
    out
}

/// Encrypts the random session key with the KeyExchangeKey (NEGOTIATE_KEY_EXCH).
pub fn encrypt_session_key(key_exchange_key: &[u8; 16], random_session_key: [u8; 16]) -> [u8; 16] {
    let mut k = random_session_key;
    Rc4::new(key_exchange_key).apply(&mut k);
    k
}

/// SIGNKEY (NL-03): MD5(ExportedSessionKey ‖ magic constant incl. the terminating NUL).
pub fn sign_key(exported_session_key: &[u8; 16], direction: Direction) -> [u8; 16] {
    let magic = match direction {
        Direction::ClientToServer => CLIENT_SIGN_MAGIC,
        Direction::ServerToClient => SERVER_SIGN_MAGIC,
    };
    md5(&[exported_session_key, magic])
}

/// SEALKEY (NL-03) for a 128-bit session (NEGOTIATE_128).
pub fn seal_key(exported_session_key: &[u8; 16], direction: Direction) -> [u8; 16] {
    let magic = match direction {
        Direction::ClientToServer => CLIENT_SEAL_MAGIC,
        Direction::ServerToClient => SERVER_SEAL_MAGIC,
    };
    md5(&[exported_session_key, magic])
}

/// Signature with extended session security and key exchange (NL-04):
/// `01 00 00 00 ‖ RC4(HMAC_MD5(SignKey, SeqNum ‖ message)[0..8]) ‖ SeqNum`, using the given
/// RC4 sealing state.
pub fn signature_ess(sign_key: &[u8; 16], sealing: &mut Rc4, seq: u32, message: &[u8]) -> [u8; 16] {
    let seq_le = seq.to_le_bytes();
    let mac = hmac_md5(sign_key, &[&seq_le, message]);
    let mut checksum = [0u8; 8];
    for (dst, src) in checksum.iter_mut().zip(mac.iter()) {
        *dst = *src;
    }
    sealing.apply(&mut checksum);
    let mut out = [0u8; 16];
    let parts = [1u8, 0, 0, 0].into_iter().chain(checksum).chain(seq_le);
    for (dst, src) in out.iter_mut().zip(parts) {
        *dst = src;
    }
    out
}

/// Connectionless signature as used for SIP (NL-05, AE-30): the RC4 state is re-keyed for every
/// message with MD5(SealKey ‖ SeqNum), and the sequence number comes from the application.
pub fn signature_connectionless(
    sign_key: &[u8; 16],
    seal_key: &[u8; 16],
    seq: u32,
    message: &[u8],
) -> [u8; 16] {
    let per_message = md5(&[seal_key, &seq.to_le_bytes()]);
    let mut sealing = Rc4::new(&per_message);
    signature_ess(sign_key, &mut sealing, seq, message)
}

/// Lower-case Base16 as used in the SIP `response`/`rspauth` parameters (AE-30).
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests;
