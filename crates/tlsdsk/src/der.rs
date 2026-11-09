//! The few DER structures TLS-DSK needs: writing the PKCS#10 request (RFC 2986), reading the
//! validity and public key of a certificate (RFC 5280 §4.1). The reader is bounded and returns
//! `None` for anything it does not understand.

/// A TLV with a definite length.
fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let n = content.len();
    if n < 0x80 {
        out.push(n as u8);
    } else {
        let bytes: Vec<u8> = n
            .to_be_bytes()
            .iter()
            .copied()
            .skip_while(|b| *b == 0)
            .collect();
        out.push(0x80 | bytes.len() as u8);
        out.extend(bytes);
    }
    out.extend_from_slice(content);
    out
}

fn seq(parts: &[&[u8]]) -> Vec<u8> {
    tlv(0x30, &parts.concat())
}

// Object identifiers (content octets).
const OID_CN: &[u8] = &[0x55, 0x04, 0x03];
const OID_EXTENSION_REQUEST: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x0e];
const OID_KEY_USAGE: &[u8] = &[0x55, 0x1d, 0x0f];
const OID_EXT_KEY_USAGE: &[u8] = &[0x55, 0x1d, 0x25];
const OID_CLIENT_AUTH: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x02];
const OID_SHA256_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];

/// CertificationRequestInfo: version 0, subject CN = `entity`, the public key, and an extension
/// request with key usage (digital signature, key encipherment) and extended key usage client
/// authentication.
pub fn certification_request_info(entity: &str, public_key_info: &[u8]) -> Vec<u8> {
    let version = tlv(0x02, &[0]);
    let cn = seq(&[&tlv(0x06, OID_CN), &tlv(0x0c, entity.as_bytes())]);
    let subject = seq(&[&tlv(0x31, &cn)]);
    let key_usage = seq(&[
        &tlv(0x06, OID_KEY_USAGE),
        &tlv(0x01, &[0xff]),
        &tlv(0x04, &tlv(0x03, &[0x05, 0xa0])),
    ]);
    let ext_key_usage = seq(&[
        &tlv(0x06, OID_EXT_KEY_USAGE),
        &tlv(0x04, &seq(&[&tlv(0x06, OID_CLIENT_AUTH)])),
    ]);
    let extensions = seq(&[&key_usage, &ext_key_usage]);
    let attribute = seq(&[&tlv(0x06, OID_EXTENSION_REQUEST), &tlv(0x31, &extensions)]);
    let attributes = tlv(0xa0, &attribute);
    seq(&[&version, &subject, public_key_info, &attributes])
}

/// CertificationRequest: the info, sha256WithRSAEncryption and the signature.
pub fn certification_request(info: &[u8], signature: &[u8]) -> Vec<u8> {
    let algorithm = seq(&[&tlv(0x06, OID_SHA256_RSA), &tlv(0x05, &[])]);
    let mut bits = vec![0u8];
    bits.extend_from_slice(signature);
    seq(&[info, &algorithm, &tlv(0x03, &bits)])
}

/// One TLV at the start of a buffer: tag, content, the whole element, the rest.
type Tlv<'a> = (u8, &'a [u8], &'a [u8], &'a [u8]);

/// The TLV at the start of `b`.
fn next(b: &[u8]) -> Option<Tlv<'_>> {
    let tag = *b.first()?;
    let first = *b.get(1)?;
    let (len, head) = match first {
        0..=0x7f => (usize::from(first), 2),
        0x81..=0x84 => {
            let n = usize::from(first & 0x7f);
            let bytes = b.get(2..2 + n)?;
            (
                bytes.iter().fold(0usize, |a, x| (a << 8) | usize::from(*x)),
                2 + n,
            )
        }
        _ => return None,
    };
    let end = head.checked_add(len)?;
    Some((tag, b.get(head..end)?, b.get(..end)?, b.get(end..)?))
}

/// What TLS-DSK needs from a certificate.
pub struct CertificateFacts {
    pub not_before: u64,
    pub not_after: u64,
    /// The SubjectPublicKeyInfo, as encoded.
    pub public_key_info: Vec<u8>,
}

/// Validity and public key of an X.509 certificate.
pub fn certificate_facts(cert: &[u8]) -> Option<CertificateFacts> {
    if cert.len() > 64 * 1024 {
        return None;
    }
    let (0x30, body, _, _) = next(cert)? else {
        return None;
    };
    let (0x30, tbs, _, _) = next(body)? else {
        return None;
    };
    let mut rest = tbs;
    // [0] version (optional), serial, signature algorithm, issuer.
    let (tag, _, _, r) = next(rest)?;
    rest = r;
    if tag == 0xa0 {
        rest = next(rest)?.3;
    }
    rest = next(rest)?.3;
    rest = next(rest)?.3;
    let (0x30, validity, _, r) = next(rest)? else {
        return None;
    };
    let (t1, nb, _, v) = next(validity)?;
    let (t2, na, _, _) = next(v)?;
    // Subject, then the public key info.
    let r = next(r)?.3;
    let (0x30, _, spki, _) = next(r)? else {
        return None;
    };
    Some(CertificateFacts {
        not_before: time(t1, nb)?,
        not_after: time(t2, na)?,
        public_key_info: spki.to_vec(),
    })
}

/// The RSAPublicKey inside a SubjectPublicKeyInfo (the BIT STRING's content).
pub fn rsa_public_key_of(spki: &[u8]) -> Option<&[u8]> {
    let (0x30, body, _, _) = next(spki)? else {
        return None;
    };
    let (_, _, _, rest) = next(body)?;
    let (0x03, bits, _, _) = next(rest)? else {
        return None;
    };
    match bits.split_first() {
        Some((0, key)) => Some(key),
        _ => None,
    }
}

/// UTCTime (0x17, YYMMDDHHMMSSZ) or GeneralizedTime (0x18, YYYYMMDDHHMMSSZ) as Unix seconds.
fn time(tag: u8, v: &[u8]) -> Option<u64> {
    let s = std::str::from_utf8(v).ok()?.strip_suffix('Z')?;
    let (year, rest) = match tag {
        0x17 if s.len() == 12 => {
            let y: u64 = s.get(..2)?.parse().ok()?;
            (if y < 50 { 2000 + y } else { 1900 + y }, s.get(2..)?)
        }
        0x18 if s.len() == 14 => (s.get(..4)?.parse().ok()?, s.get(4..)?),
        _ => return None,
    };
    let n = |i: usize| -> Option<u64> { rest.get(i..i + 2)?.parse().ok() };
    let (month, day, h, m, sec) = (n(0)?, n(2)?, n(4)?, n(6)?, n(8)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || h > 23 || m > 59 || sec > 60 {
        return None;
    }
    // Days from civil (proleptic Gregorian), 1970-01-01 = 0.
    let (y, mo) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = y / 400;
    let yoe = y - era * 400;
    let doy = (153 * mo + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = (era * 146_097 + doe).checked_sub(719_468)?;
    Some(days * 86_400 + h * 3600 + m * 60 + sec)
}

#[cfg(test)]
pub(crate) fn time_for_test(tag: u8, v: &[u8]) -> Option<u64> {
    time(tag, v)
}
