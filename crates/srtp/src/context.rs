//! SRTP/SRTCP cryptographic contexts for one direction of one SRTP session (MS-SRTP profile of
//! RFC 3711).
//!
//! Evidence: `docs/protocol/evidence/15-srtp-ssrtp-media-encryption.md` (SR-01 … SR-07, SR-10 …
//! SR-14). Sources: [MS-SRTP] 20250520 §3.1.3–3.1.5 (pp. 10–13), §5.1 (p. 15); RFC 3711 §3.1
//! (packet format), §3.3 (index, ROC, s_l, replay), §3.4 (SRTCP), §4.1.1 (AES-CM IV), §4.2
//! (HMAC-SHA1 tag over the authenticated portion ‖ ROC), Appendix A (index estimation).
//! Receiving SSRTP from an MCU (SS-01 … SS-06): [MS-SSRTP] 20250520 §2.2.1 (packet, pp. 10–12),
//! §3.1.5.1.1–3.1.5.1.3.2 (index, IV, receive steps, pp. 14–17); SSRTCP is SRTCP (§3.1.5.2).
//!
//! Security by design: everything is authenticated before anything is decrypted or any state
//! changes (the tag is compared in constant time); replay window of 64 per SSRC and for SRTCP;
//! encrypted payloads only (SR-06: the E bit of incoming SRTCP is not honoured); the number of
//! receive streams is bounded; a wrong MKI is refused; keys are wiped on drop; failures are only
//! counted, never logged with packet content (§3.1.5.1.2).

use std::collections::HashMap;

use hmac::{Hmac, Mac};
use sha1::Sha1;
use zeroize::Zeroize;

use crate::{aes_cm, ssrtp_iv, SessionKeys, SrtpError, AUTH_TAG_LEN};

/// Replay list size (SR-04).
pub const REPLAY_WINDOW: u64 = 64;
/// Bound of SSRCs per context (design; MS-SRTP allows several streams per session, SR-05).
pub const MAX_STREAMS: usize = 32;
/// SRTCP index is 31 bits (RFC 3711 §3.4); the master key lifetime for RTCP is 2^31 − 1 (SR-03).
const SRTCP_INDEX_MASK: u32 = 0x7fff_ffff;

impl Drop for SessionKeys {
    fn drop(&mut self) {
        self.encryption.zeroize();
        self.authentication.zeroize();
        self.salt.zeroize();
    }
}

/// Sliding replay window over a 48-bit (SRTP) or 31-bit (SRTCP) index (RFC 3711 §3.3.2).
#[derive(Debug, Clone, Default)]
struct Replay {
    highest: Option<u64>,
    /// Bit k set ⇔ index `highest − k` was received.
    bits: u64,
}

impl Replay {
    fn check(&self, index: u64) -> Result<(), SrtpError> {
        let Some(h) = self.highest else { return Ok(()) };
        if index > h {
            return Ok(());
        }
        let diff = h - index;
        if diff >= REPLAY_WINDOW {
            return Err(SrtpError::Replay);
        }
        if self.bits & (1 << diff) != 0 {
            return Err(SrtpError::Replay);
        }
        Ok(())
    }

    fn add(&mut self, index: u64) {
        match self.highest {
            Some(h) if index <= h => self.bits |= 1 << (h - index),
            Some(h) => {
                let shift = index - h;
                self.bits = if shift >= REPLAY_WINDOW {
                    0
                } else {
                    self.bits << shift
                } | 1;
                self.highest = Some(index);
            }
            None => {
                self.highest = Some(index);
                self.bits = 1;
            }
        }
    }
}

/// Per-SSRC transform independent parameters (MS-SRTP §3.1.3.2): ROC, s_l, replay list.
#[derive(Debug, Clone, Default)]
struct Stream {
    roc: u32,
    s_l: u16,
    replay: Replay,
}

impl Stream {
    /// RFC 3711 Appendix A: v ∈ {ROC−1, ROC, ROC+1}.
    fn estimate(&self, seq: u16) -> u32 {
        let (s_l, seq32) = (i32::from(self.s_l), i32::from(seq));
        if s_l < 32768 {
            if seq32 - s_l > 32768 {
                self.roc.wrapping_sub(1)
            } else {
                self.roc
            }
        } else if s_l - 32768 > seq32 {
            self.roc.wrapping_add(1)
        } else {
            self.roc
        }
    }

    /// RFC 3711 §3.3.1 update after authentication.
    fn update(&mut self, v: u32, seq: u16) {
        if v == self.roc.wrapping_add(1) {
            self.roc = v;
            self.s_l = seq;
        } else if v == self.roc && seq > self.s_l {
            self.s_l = seq;
        }
    }
}

fn index(roc: u32, seq: u16) -> u64 {
    (u64::from(roc) << 16) | u64::from(seq)
}

/// AES-CM IV (RFC 3711 §4.1.1): (k_s·2^16) ⊕ (SSRC·2^64) ⊕ (i·2^16).
fn iv(salt: &[u8; 14], ssrc: u32, i: u64) -> u128 {
    let mut s = [0u8; 16];
    for (d, v) in s.iter_mut().zip(salt.iter()) {
        *d = *v;
    }
    u128::from_be_bytes(s) ^ (u128::from(ssrc) << 64) ^ (u128::from(i) << 16)
}

fn hmac(key: &[u8; 20]) -> Result<Hmac<Sha1>, SrtpError> {
    <Hmac<Sha1> as Mac>::new_from_slice(key).map_err(|_| SrtpError::Key)
}

fn tag(key: &[u8; 20], parts: &[&[u8]]) -> Result<[u8; AUTH_TAG_LEN], SrtpError> {
    let mut m = hmac(key)?;
    for p in parts {
        m.update(p);
    }
    let full = m.finalize().into_bytes();
    let mut t = [0u8; AUTH_TAG_LEN];
    for (d, s) in t.iter_mut().zip(full.iter()) {
        *d = *s;
    }
    Ok(t)
}

/// Constant-time check of an 80-bit tag.
fn verify(key: &[u8; 20], parts: &[&[u8]], received: &[u8]) -> Result<(), SrtpError> {
    let mut m = hmac(key)?;
    for p in parts {
        m.update(p);
    }
    m.verify_truncated_left(received)
        .map_err(|_| SrtpError::Authentication)
}

/// Length of the RTP header incl. CSRC list and header extension (RFC 3550 §5.1, §5.3.1).
fn rtp_header_len(p: &[u8]) -> Result<usize, SrtpError> {
    let first = *p.first().ok_or(SrtpError::MalformedHeader)?;
    if first >> 6 != 2 {
        return Err(SrtpError::MalformedHeader);
    }
    let mut len = 12 + usize::from(first & 0x0f) * 4;
    if first & 0x10 != 0 {
        len += 4 + usize::from(be16(p, len + 2)?) * 4;
    }
    if p.len() < len {
        return Err(SrtpError::MalformedHeader);
    }
    Ok(len)
}

fn be16(p: &[u8], at: usize) -> Result<u16, SrtpError> {
    let b = p.get(at..at + 2).ok_or(SrtpError::MalformedHeader)?;
    Ok(u16::from_be_bytes(
        b.try_into().map_err(|_| SrtpError::MalformedHeader)?,
    ))
}

fn be32(p: &[u8], at: usize) -> Result<u32, SrtpError> {
    let b = p.get(at..at + 4).ok_or(SrtpError::MalformedHeader)?;
    Ok(u32::from_be_bytes(
        b.try_into().map_err(|_| SrtpError::MalformedHeader)?,
    ))
}

/// The cryptographic context of one direction (MS-SRTP §3.1.3.1: one for all send streams, one
/// for all receive streams of a session).
pub struct Context {
    rtp: SessionKeys,
    rtcp: SessionKeys,
    mki: Option<u8>,
    streams: HashMap<u32, Stream>,
    /// Sender: next SRTCP index. Receiver: replay window of SRTCP.
    rtcp_index: u32,
    rtcp_replay: Replay,
    /// Number of rejected packets (MS-SRTP §3.1.5.1.2: count, do not log each).
    pub failures: u64,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("mki", &self.mki)
            .field("streams", &self.streams.len())
            .field("failures", &self.failures)
            .finish_non_exhaustive()
    }
}

impl Context {
    /// `mki`: the one-byte master key identifier (SR-03: mandatory in MS-SRTP; `None` only for
    /// RFC 3711 test vectors).
    pub fn new(master_key: &[u8; 16], master_salt: &[u8; 14], mki: Option<u8>) -> Self {
        Context {
            rtp: SessionKeys::derive(master_key, master_salt, false),
            rtcp: SessionKeys::derive(master_key, master_salt, true),
            mki,
            streams: HashMap::new(),
            rtcp_index: 0,
            rtcp_replay: Replay::default(),
            failures: 0,
        }
    }

    /// Sets the next SRTCP index of a send context (RFC 3711 §3.4 starts at 0).
    pub fn set_rtcp_index(&mut self, index: u32) {
        self.rtcp_index = index & SRTCP_INDEX_MASK;
    }

    fn trailer_len(&self) -> usize {
        usize::from(self.mki.is_some()) + AUTH_TAG_LEN
    }

    fn stream(&mut self, ssrc: u32, seq: u16) -> Result<&mut Stream, SrtpError> {
        if !self.streams.contains_key(&ssrc) && self.streams.len() >= MAX_STREAMS {
            return Err(SrtpError::TooManyStreams);
        }
        Ok(self.streams.entry(ssrc).or_insert_with(|| Stream {
            roc: 0,
            s_l: seq,
            replay: Replay::default(),
        }))
    }

    /// RTP → SRTP (RFC 3711 §3.3): encrypt the payload, append MKI and tag.
    pub fn protect_rtp(&mut self, packet: &[u8]) -> Result<Vec<u8>, SrtpError> {
        let hlen = rtp_header_len(packet)?;
        let seq = be16(packet, 2)?;
        let ssrc = be32(packet, 8)?;
        // The sender's ROC counts wraps of its own sequence numbers.
        let st = self.stream(ssrc, seq)?;
        let v = st.estimate(seq);
        st.update(v, seq);
        let i = index(v, seq);
        let mut out = packet.to_vec();
        let (_, payload) = out.split_at_mut(hlen);
        aes_cm(&self.rtp.encryption, iv(&self.rtp.salt, ssrc, i), payload);
        let t = tag(&self.rtp.authentication, &[&out, &v.to_be_bytes()])?;
        if let Some(m) = self.mki {
            out.push(m);
        }
        out.extend_from_slice(&t);
        Ok(out)
    }

    /// SRTP → RTP: authenticate (constant time), replay check, decrypt, then update ROC/s_l and
    /// the replay list.
    pub fn unprotect_rtp(&mut self, packet: &[u8]) -> Result<Vec<u8>, SrtpError> {
        let r = self.unprotect_rtp_inner(packet);
        if r.is_err() {
            self.failures = self.failures.saturating_add(1);
        }
        r
    }

    fn unprotect_rtp_inner(&mut self, packet: &[u8]) -> Result<Vec<u8>, SrtpError> {
        let body_len = packet
            .len()
            .checked_sub(self.trailer_len())
            .ok_or(SrtpError::MalformedHeader)?;
        let (body, trailer) = packet.split_at(body_len);
        let hlen = rtp_header_len(body)?;
        let (mki, received_tag) = trailer.split_at(usize::from(self.mki.is_some()));
        if self.mki.is_some() && mki.first().copied() != self.mki {
            return Err(SrtpError::UnknownMki);
        }
        let seq = be16(body, 2)?;
        let ssrc = be32(body, 8)?;
        let (v, i) = match self.streams.get(&ssrc) {
            Some(st) => {
                let v = st.estimate(seq);
                let i = index(v, seq);
                st.replay.check(i)?;
                (v, i)
            }
            None if self.streams.len() >= MAX_STREAMS => return Err(SrtpError::TooManyStreams),
            None => (0, index(0, seq)),
        };
        verify(
            &self.rtp.authentication,
            &[body, &v.to_be_bytes()],
            received_tag,
        )?;
        let mut out = body.to_vec();
        let (_, payload) = out.split_at_mut(hlen);
        aes_cm(&self.rtp.encryption, iv(&self.rtp.salt, ssrc, i), payload);
        let st = self.stream(ssrc, seq)?;
        st.update(v, seq);
        st.replay.add(i);
        Ok(out)
    }

    /// SSRTP → RTP ([MS-SSRTP] §3.1.5.1.3.2): authenticate the rearranged block (CSRC ‖
    /// encrypted payload ‖ ESN ‖ zero padding to 64 octets ‖ fixed header ‖ ROC) in constant
    /// time, check replay by the RTP index (never by the ESN, §3.1.5.1.1), decrypt with the ESN
    /// IV, then update ROC/s_l and the replay list; ESN, MKI and tag are removed. MKI is
    /// mandatory (§2.2.1). Inference (fail closed): §2.2.1 places the payload right after the
    /// CSRC list, so packets with a header extension are refused.
    pub fn unprotect_ssrtp(&mut self, packet: &[u8]) -> Result<Vec<u8>, SrtpError> {
        let r = self.unprotect_ssrtp_inner(packet);
        if r.is_err() {
            self.failures = self.failures.saturating_add(1);
        }
        r
    }

    fn unprotect_ssrtp_inner(&mut self, packet: &[u8]) -> Result<Vec<u8>, SrtpError> {
        const ESN_LEN: usize = 6;
        let mki = self.mki.ok_or(SrtpError::UnknownMki)?;
        let body_len = packet
            .len()
            .checked_sub(ESN_LEN + 1 + AUTH_TAG_LEN)
            .ok_or(SrtpError::MalformedHeader)?;
        let (body, trailer) = packet.split_at(body_len);
        let hlen = rtp_header_len(body)?;
        let first = *body.first().ok_or(SrtpError::MalformedHeader)?;
        if first & 0x10 != 0 {
            return Err(SrtpError::MalformedHeader);
        }
        let (esn, rest) = trailer.split_at(ESN_LEN);
        let (pkt_mki, received_tag) = rest.split_at(1);
        if pkt_mki.first().copied() != Some(mki) {
            return Err(SrtpError::UnknownMki);
        }
        let esn_value = esn.iter().fold(0u64, |a, b| (a << 8) | u64::from(*b));
        let seq = be16(body, 2)?;
        let ssrc = be32(body, 8)?;
        let (v, i) = match self.streams.get(&ssrc) {
            Some(st) => {
                let v = st.estimate(seq);
                let i = index(v, seq);
                st.replay.check(i)?;
                (v, i)
            }
            None if self.streams.len() >= MAX_STREAMS => return Err(SrtpError::TooManyStreams),
            None => (0, index(0, seq)),
        };
        let fixed = body.get(..12).ok_or(SrtpError::MalformedHeader)?;
        let csrc = body.get(12..hlen).ok_or(SrtpError::MalformedHeader)?;
        let encrypted = body.get(hlen..).ok_or(SrtpError::MalformedHeader)?;
        let mut block = Vec::with_capacity(csrc.len() + encrypted.len() + ESN_LEN + 64 + 16);
        block.extend_from_slice(csrc);
        block.extend_from_slice(encrypted);
        block.extend_from_slice(esn);
        let padding = (64 - block.len() % 64) % 64;
        block.resize(block.len() + padding, 0);
        block.extend_from_slice(fixed);
        verify(
            &self.rtp.authentication,
            &[&block, &v.to_be_bytes()],
            received_tag,
        )?;
        let mut out = body.to_vec();
        let (_, payload) = out.split_at_mut(hlen);
        aes_cm(
            &self.rtp.encryption,
            ssrtp_iv(&self.rtp.salt, esn_value),
            payload,
        );
        let st = self.stream(ssrc, seq)?;
        st.update(v, seq);
        st.replay.add(i);
        Ok(out)
    }

    /// RTCP → SRTCP (RFC 3711 §3.4): the first 8 octets stay clear, the rest is encrypted;
    /// E bit set (SR-06: always encrypted), 31-bit index shared by all streams of the direction.
    pub fn protect_rtcp(&mut self, packet: &[u8]) -> Result<Vec<u8>, SrtpError> {
        if packet.len() < 8 || packet.first().map(|b| b >> 6) != Some(2) {
            return Err(SrtpError::MalformedHeader);
        }
        let ssrc = be32(packet, 4)?;
        let i = self.rtcp_index;
        self.rtcp_index = (i + 1) & SRTCP_INDEX_MASK;
        let mut out = packet.to_vec();
        let (_, rest) = out.split_at_mut(8);
        aes_cm(
            &self.rtcp.encryption,
            iv(&self.rtcp.salt, ssrc, u64::from(i)),
            rest,
        );
        out.extend_from_slice(&(0x8000_0000 | i).to_be_bytes());
        let t = tag(&self.rtcp.authentication, &[&out])?;
        if let Some(m) = self.mki {
            out.push(m);
        }
        out.extend_from_slice(&t);
        Ok(out)
    }

    /// SRTCP → RTCP. The E bit is ignored: every packet is treated as encrypted (SR-06).
    pub fn unprotect_rtcp(&mut self, packet: &[u8]) -> Result<Vec<u8>, SrtpError> {
        let r = self.unprotect_rtcp_inner(packet);
        if r.is_err() {
            self.failures = self.failures.saturating_add(1);
        }
        r
    }

    fn unprotect_rtcp_inner(&mut self, packet: &[u8]) -> Result<Vec<u8>, SrtpError> {
        let auth_len = packet
            .len()
            .checked_sub(self.trailer_len())
            .filter(|&l| l >= 12)
            .ok_or(SrtpError::MalformedHeader)?;
        let (authenticated, trailer) = packet.split_at(auth_len);
        let (mki, received_tag) = trailer.split_at(usize::from(self.mki.is_some()));
        if self.mki.is_some() && mki.first().copied() != self.mki {
            return Err(SrtpError::UnknownMki);
        }
        if authenticated.first().map(|b| b >> 6) != Some(2) {
            return Err(SrtpError::MalformedHeader);
        }
        let e_index = be32(authenticated, auth_len - 4)?;
        let i = e_index & SRTCP_INDEX_MASK;
        self.rtcp_replay.check(u64::from(i))?;
        verify(&self.rtcp.authentication, &[authenticated], received_tag)?;
        let ssrc = be32(authenticated, 4)?;
        let mut out = authenticated
            .get(..auth_len - 4)
            .ok_or(SrtpError::MalformedHeader)?
            .to_vec();
        let (_, rest) = out.split_at_mut(8);
        aes_cm(
            &self.rtcp.encryption,
            iv(&self.rtcp.salt, ssrc, u64::from(i)),
            rest,
        );
        self.rtcp_replay.add(u64::from(i));
        Ok(out)
    }
}
