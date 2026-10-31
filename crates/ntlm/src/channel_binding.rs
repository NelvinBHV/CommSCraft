//! Channel bindings for NTLM over TLS (issue #42): the MsvAvChannelBindings AV pair carries the
//! MD5 hash of a `gss_channel_bindings_struct` ([MS-NLMP] 20260330 §2.2.2.1, p. 31;
//! §3.1.5.1.2), serialised per RFC 4121 §4.1.1.2 (every integer and length field as four octets,
//! little-endian; lengths included even when zero; addresses unused). The application data is
//! the `tls-server-end-point` binding of RFC 5929 §4: the unique prefix and a colon (RFC 5056
//! §2.1), then the hash of the server certificate as sent in the TLS Certificate message, with
//! the hash function of the certificate's signature algorithm (MD5 and SHA-1 replaced by
//! SHA-256).
//!
//! Security by design: the DER walk is bounded (four-octet lengths at most, every slice checked);
//! a certificate whose signature algorithm has no single hash (RSASSA-PSS, Ed25519 – undefined in
//! RFC 5929 §4.1) or is unknown yields `None`, and the caller sends the "no binding" value Z(16)
//! (§2.2.2.1) rather than guessing.

use md5::{Digest, Md5};

/// One DER TLV: (tag, contents, rest).
fn tlv(p: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = p.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (len, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let n = usize::from(first & 0x7f);
        if n == 0 || n > 4 {
            return None;
        }
        let bytes = rest.get(..n)?;
        let len = bytes.iter().fold(0usize, |a, &b| (a << 8) | usize::from(b));
        (len, rest.get(n..)?)
    };
    Some((tag, rest.get(..len)?, rest.get(len..)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hash {
    Sha224,
    Sha256,
    Sha384,
    Sha512,
}

/// The hash of RFC 5929 §4.1 for the certificate's `signatureAlgorithm` OID (DER contents).
fn hash_for(oid: &[u8]) -> Option<Hash> {
    const RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01];
    const ECDSA_SHA1: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x01];
    const ECDSA_SHA2: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03];
    const DSA_SHA1: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x38, 0x04, 0x03];
    const DSA_SHA2: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x03];
    if oid == ECDSA_SHA1 || oid == DSA_SHA1 {
        return Some(Hash::Sha256);
    }
    let (prefix, last) = oid.split_at_checked(oid.len().checked_sub(1)?)?;
    match (prefix, last) {
        // md5WithRSA (4), sha1WithRSA (5) → SHA-256; sha256/384/512/224WithRSA (11–14).
        (RSA, [4 | 5 | 11]) => Some(Hash::Sha256),
        (RSA, [12]) => Some(Hash::Sha384),
        (RSA, [13]) => Some(Hash::Sha512),
        (RSA, [14]) => Some(Hash::Sha224),
        (ECDSA_SHA2, [1]) => Some(Hash::Sha224),
        (ECDSA_SHA2, [2]) => Some(Hash::Sha256),
        (ECDSA_SHA2, [3]) => Some(Hash::Sha384),
        (ECDSA_SHA2, [4]) => Some(Hash::Sha512),
        (DSA_SHA2, [1]) => Some(Hash::Sha224),
        (DSA_SHA2, [2]) => Some(Hash::Sha256),
        _ => None,
    }
}

/// `tls-server-end-point:` ‖ hash(certificate), or `None` when RFC 5929 defines no binding.
pub fn tls_server_end_point(cert_der: &[u8]) -> Option<Vec<u8>> {
    // Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm, signatureValue }
    let (0x30, cert, _) = tlv(cert_der)? else {
        return None;
    };
    let (0x30, _tbs, rest) = tlv(cert)? else {
        return None;
    };
    let (0x30, alg, _) = tlv(rest)? else {
        return None;
    };
    let (0x06, oid, _) = tlv(alg)? else {
        return None;
    };
    let digest = match hash_for(oid)? {
        Hash::Sha224 => sha2::Sha224::digest(cert_der).to_vec(),
        Hash::Sha256 => sha2::Sha256::digest(cert_der).to_vec(),
        Hash::Sha384 => sha2::Sha384::digest(cert_der).to_vec(),
        Hash::Sha512 => sha2::Sha512::digest(cert_der).to_vec(),
    };
    let mut out = b"tls-server-end-point:".to_vec();
    out.extend_from_slice(&digest);
    Some(out)
}

/// MD5 of the `gss_channel_bindings_struct` with only `application_data` set (RFC 4121
/// §4.1.1.2): initiator and acceptor address type and length are zero.
pub fn bindings_hash(application_data: &[u8]) -> [u8; 16] {
    let mut h = Md5::new();
    h.update([0u8; 16]);
    h.update((application_data.len() as u32).to_le_bytes());
    h.update(application_data);
    h.finalize().into()
}

/// The MsvAvChannelBindings value for a TLS server certificate, if RFC 5929 defines one.
pub fn for_certificate(cert_der: &[u8]) -> Option<[u8; 16]> {
    tls_server_end_point(cert_der).map(|a| bindings_hash(&a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn der(tag: u8, body: &[u8]) -> Vec<u8> {
        let mut v = vec![tag];
        if body.len() < 0x80 {
            v.push(body.len() as u8);
        } else {
            v.extend_from_slice(&[0x81, body.len() as u8]);
        }
        v.extend_from_slice(body);
        v
    }

    /// A synthetic certificate: the hash covers the DER octets as they are.
    fn cert(oid: &[u8]) -> Vec<u8> {
        let mut tbs_body = der(0x02, &[1]);
        tbs_body.extend_from_slice(&[0; 200]);
        let mut alg = der(0x06, oid);
        alg.extend_from_slice(&[5, 0]);
        let mut body = der(0x30, &tbs_body);
        body.extend(der(0x30, &alg));
        body.extend(der(0x03, &[0, 1]));
        der(0x30, &body)
    }

    /// Recomputed: the expected values come from an independent computation with Python's
    /// `hashlib` (MD5(Z(16) ‖ LE32(len) ‖ "tls-server-end-point:" ‖ H(cert))).
    #[test]
    fn recomputed_bindings() {
        let rsa = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01];
        let hex = |b: [u8; 16]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        let with = |last: u8| [&rsa[..], &[last]].concat();
        for (oid, expect) in [
            (with(0x0b), "4880f3418ea9073c003cfd141086a451"),
            (with(0x05), "4b0df6d2b53ba8035fdebc44e1bf85f9"),
            (with(0x0c), "2f002853e8d65609363b7969680d8d0b"),
            (
                vec![0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x04],
                "1c50b076eeece7e2634c38158a595ec1",
            ),
        ] {
            let c = cert(&oid);
            assert_eq!(hex(for_certificate(&c).unwrap()), expect, "{oid:02x?}");
        }
    }

    #[test]
    fn undefined_or_broken_certificates() {
        // RSASSA-PSS (1.2.840.113549.1.1.10) and Ed25519 (1.3.101.112): no binding defined.
        assert!(for_certificate(&cert(&[
            0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0a
        ]))
        .is_none());
        assert!(for_certificate(&cert(&[0x2b, 0x65, 0x70])).is_none());
        let good = cert(&[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b]);
        for cut in 0..good.len() - 4 {
            assert!(
                tls_server_end_point(&good[..cut]).is_none(),
                "truncated at {cut}"
            );
        }
        assert!(
            tls_server_end_point(&[0x30, 0x85, 1, 2, 3, 4, 5]).is_none(),
            "5-octet length"
        );
        assert!(tls_server_end_point(&[]).is_none());
    }
}
