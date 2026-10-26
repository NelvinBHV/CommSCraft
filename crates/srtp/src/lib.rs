//! SRTP key derivation and the SSRTP packet transform.
//!
//! Evidence: `docs/protocol/evidence/15-srtp-ssrtp-media-encryption.md` (SR-01 … SR-07,
//! SS-01 … SS-06, K-SR-1) and `docs/protocol/evidence/38-ietf-foundations-and-archive-sources.md`
//! (IE-09). Only AES-CM-128 with HMAC-SHA1-80 and key derivation rate 0 are supported, which is
//! the single suite MS-SDPEXT allows (SD-10).

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes128;
use hmac::{Hmac, Mac};
use sha1::Sha1;

/// Length of the authentication tag (HMAC-SHA1-80, SR-02).
pub const AUTH_TAG_LEN: usize = 10;

/// Key derivation labels (RFC 3711 §4.3.1–4.3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Label {
    RtpEncryption = 0,
    RtpAuthentication = 1,
    RtpSalt = 2,
    RtcpEncryption = 3,
    RtcpAuthentication = 4,
    RtcpSalt = 5,
}

/// Errors of the SRTP layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SrtpError {
    /// The RTP header is shorter than 12 bytes or its CSRC list is truncated.
    MalformedHeader,
    /// The ESN does not fit into 48 bits.
    EsnOutOfRange,
}

impl std::fmt::Display for SrtpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedHeader => f.write_str("malformed RTP header"),
            Self::EsnOutOfRange => f.write_str("encryption sequence number exceeds 48 bits"),
        }
    }
}

impl std::error::Error for SrtpError {}

/// AES-CM key stream: E(k, IV + i) for i = 0, 1, … (RFC 3711 §4.1.1), XORed into `data`.
fn aes_cm(key: &[u8; 16], iv: u128, data: &mut [u8]) {
    let cipher = Aes128::new(&(*key).into());
    for (i, chunk) in data.chunks_mut(16).enumerate() {
        let counter = iv.wrapping_add(i as u128);
        let mut block = counter.to_be_bytes().into();
        cipher.encrypt_block(&mut block);
        for (d, k) in chunk.iter_mut().zip(block.iter()) {
            *d ^= *k;
        }
    }
}

/// Session key derivation with key derivation rate 0 (RFC 3711 §4.3, SR-03, SS-05):
/// x = (label ‖ 0^48) XOR master salt, keystream = AES-CM(master key, x·2^16).
pub fn derive(master_key: &[u8; 16], master_salt: &[u8; 14], label: Label, len: usize) -> Vec<u8> {
    let mut x = [0u8; 16];
    for (dst, src) in x.iter_mut().zip(master_salt.iter()) {
        *dst = *src;
    }
    // The label occupies the byte that sits 48 bits above the end of the 112-bit salt.
    x[7] ^= label as u8;
    let iv = u128::from_be_bytes(x);
    let mut out = vec![0u8; len];
    aes_cm(master_key, iv, &mut out);
    out
}

/// Session keys for one direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionKeys {
    pub encryption: [u8; 16],
    pub authentication: [u8; 20],
    pub salt: [u8; 14],
}

fn to_array<const N: usize>(v: &[u8]) -> [u8; N] {
    let mut a = [0u8; N];
    for (dst, src) in a.iter_mut().zip(v.iter()) {
        *dst = *src;
    }
    a
}

impl SessionKeys {
    /// Keys for RTP (`rtcp = false`) or RTCP (`rtcp = true`).
    pub fn derive(master_key: &[u8; 16], master_salt: &[u8; 14], rtcp: bool) -> Self {
        let (e, a, s) = if rtcp {
            (
                Label::RtcpEncryption,
                Label::RtcpAuthentication,
                Label::RtcpSalt,
            )
        } else {
            (
                Label::RtpEncryption,
                Label::RtpAuthentication,
                Label::RtpSalt,
            )
        };
        Self {
            encryption: to_array(&derive(master_key, master_salt, e, 16)),
            authentication: to_array(&derive(master_key, master_salt, a, 20)),
            salt: to_array(&derive(master_key, master_salt, s, 14)),
        }
    }
}

/// Maximum value of the 48-bit encryption sequence number (SS-04).
pub const MAX_ESN: u64 = (1 << 48) - 1;

/// SSRTP IV (SS-03): (k_s·2^16) XOR ((ESN>>16)·2^64) XOR (ESN·2^16).
pub fn ssrtp_iv(session_salt: &[u8; 14], esn: u64) -> u128 {
    let mut salt = [0u8; 16];
    for (dst, src) in salt.iter_mut().zip(session_salt.iter()) {
        *dst = *src;
    }
    let ks = u128::from_be_bytes(salt); // already shifted by 16 bits
    let esn = u128::from(esn);
    ks ^ ((esn >> 16) << 64) ^ (esn << 16)
}

/// Result of the SSRTP transform for one RTP packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsrtpProtected {
    /// Encrypted payload (same length as the input payload).
    pub encrypted_payload: Vec<u8>,
    /// The authenticated block (SS-02); exposed for verification against test vectors.
    pub authenticated_block: Vec<u8>,
    /// HMAC-SHA1-80 authentication tag.
    pub tag: [u8; AUTH_TAG_LEN],
}

/// Applies the SSRTP transform (SS-01 … SS-05) to one RTP packet. The wire packet is
/// header ‖ encrypted payload ‖ ESN (6 bytes) ‖ MKI ‖ tag; assembling it is left to the caller.
pub fn ssrtp_protect(
    keys: &SessionKeys,
    rtp_header: &[u8],
    payload: &[u8],
    esn: u64,
    roc: u32,
) -> Result<SsrtpProtected, SrtpError> {
    if esn > MAX_ESN {
        return Err(SrtpError::EsnOutOfRange);
    }
    let first = rtp_header
        .first()
        .copied()
        .ok_or(SrtpError::MalformedHeader)?;
    let csrc_len = usize::from(first & 0x0f) * 4;
    let fixed = rtp_header.get(..12).ok_or(SrtpError::MalformedHeader)?;
    let csrc = rtp_header
        .get(12..12 + csrc_len)
        .ok_or(SrtpError::MalformedHeader)?;

    let mut encrypted = payload.to_vec();
    aes_cm(&keys.encryption, ssrtp_iv(&keys.salt, esn), &mut encrypted);

    let esn_bytes = esn.to_be_bytes();
    let esn48 = esn_bytes.get(2..).ok_or(SrtpError::EsnOutOfRange)?;
    let mut block = Vec::with_capacity(csrc.len() + encrypted.len() + 6 + 64 + 12 + 4);
    block.extend_from_slice(csrc);
    block.extend_from_slice(&encrypted);
    block.extend_from_slice(esn48);
    let padding = (64 - block.len() % 64) % 64;
    block.resize(block.len() + padding, 0);
    block.extend_from_slice(fixed);
    block.extend_from_slice(&roc.to_be_bytes());

    let tag = match <Hmac<Sha1> as Mac>::new_from_slice(&keys.authentication) {
        Ok(mut m) => {
            m.update(&block);
            to_array(&m.finalize().into_bytes())
        }
        Err(_) => [0u8; AUTH_TAG_LEN],
    };
    Ok(SsrtpProtected {
        encrypted_payload: encrypted,
        authenticated_block: block,
        tag,
    })
}

#[cfg(test)]
mod tests;
