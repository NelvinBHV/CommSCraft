//! Read-only bounded SDP inspection (RFC 8866 + MS-SDPEXT context).
//!
//! This is NOT RFC 3264 offer/answer negotiation, ICE candidate selection,
//! codec negotiation or authorization to open RTP/SRTP sockets. Unknown `a=`
//! attributes are retained only as a count, not interpreted. MS-specific
//! extensions require separate validated handlers before real interop.

use std::fmt;

pub const MAX_SDP_BYTES: usize = 64 * 1024;
pub const MAX_SDP_LINES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdpError {
    TooLarge,
    Malformed,
    MissingRequired,
    DuplicateRequired,
    BadMedia,
}
impl fmt::Display for SdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SdpError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDescription {
    pub kind: String,
    pub port: u16,
    pub protocol: String,
    pub formats: Vec<String>,
    pub attribute_count: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdpSummary {
    pub media: Vec<MediaDescription>,
    pub session_attribute_count: usize,
}

// RFC 8866 §5.10: decimal seconds, optionally suffixed by d/h/m/s.
// Do not convert to a fixed-width number in this read-only inspector.
fn time_unit_token(token: &str) -> bool {
    let digits = if token
        .as_bytes()
        .last()
        .is_some_and(|b| matches!(*b, b'd' | b'h' | b'm' | b's'))
    {
        &token[..token.len() - 1]
    } else {
        token
    };
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// Parse only trusted structural fields: v/o/s/t and m. No media will be
/// negotiated based on this result. Strict CRLF is required on every line.
pub fn inspect_sdp(bytes: &[u8]) -> Result<SdpSummary, SdpError> {
    if bytes.is_empty() || bytes.len() > MAX_SDP_BYTES {
        return Err(SdpError::TooLarge);
    }
    let s = std::str::from_utf8(bytes).map_err(|_| SdpError::Malformed)?;
    if !s.ends_with("\r\n") || s.contains("\n\n") {
        return Err(SdpError::Malformed);
    }
    let mut v = false;
    let mut o = false;
    let mut subject = false;
    let mut time = false;
    let mut repeat = false;
    let mut zone = false;
    let mut media: Vec<MediaDescription> = Vec::new();
    let mut session_attributes = 0usize;
    let mut lines = 0usize;
    for line in s.split_terminator("\r\n") {
        lines += 1;
        if lines > MAX_SDP_LINES
            || line.is_empty()
            || line.len() > 2048
            || line
                .bytes()
                .any(|b| b == b'\r' || b == b'\n' || b == 0 || (b < 32 && b != b'\t'))
        {
            return Err(SdpError::Malformed);
        }
        if line.len() < 3 || !line.is_char_boundary(2) {
            return Err(SdpError::Malformed);
        }
        let (prefix, value) = line.split_at(2);
        if prefix.len() != 2
            || prefix.as_bytes()[1] != b'='
            || !prefix.as_bytes()[0].is_ascii_lowercase()
            || value.is_empty()
        {
            return Err(SdpError::Malformed);
        }
        match prefix {
            "v=" => {
                if v || o || !media.is_empty() {
                    return Err(SdpError::DuplicateRequired);
                }
                if value != "0" {
                    return Err(SdpError::Malformed);
                }
                v = true;
            }
            "o=" => {
                if !v || o || !media.is_empty() {
                    return Err(SdpError::DuplicateRequired);
                }
                // RFC 8866 §5.2: the origin has exactly six fields and its
                // session identifier/version are nonempty decimal digit strings.
                // Do not parse them into fixed-width integers: RFC 8866 does
                // not bound their numeric range.
                let fields: Vec<_> = value.split_ascii_whitespace().collect();
                if fields.len() != 6
                    || fields[1..=2]
                        .iter()
                        .any(|field| field.is_empty() || !field.bytes().all(|b| b.is_ascii_digit()))
                {
                    return Err(SdpError::Malformed);
                }
                // MS-SDPEXT §3.1.5.19: Microsoft SDP origin uses IN
                // with an actual IP4/IP6 literal, not an untrusted hostname.
                // RFC 8866 also permits FQDNs; this is a stricter MS profile.
                let valid_address = match fields[4] {
                    "IP4" => fields[5].parse::<std::net::Ipv4Addr>().is_ok(),
                    "IP6" => fields[5].parse::<std::net::Ipv6Addr>().is_ok(),
                    _ => false,
                };
                if fields[3] != "IN" || !valid_address {
                    return Err(SdpError::Malformed);
                }
                o = true;
            }
            "s=" => {
                if !o {
                    return Err(SdpError::Malformed);
                }
                if subject || !media.is_empty() {
                    return Err(SdpError::DuplicateRequired);
                }
                subject = true;
            }
            "t=" => {
                if !subject || !media.is_empty() || zone {
                    return Err(SdpError::Malformed);
                }
                // RFC 8866 §5.9: timestamps are arbitrary-length decimal
                // strings, not fixed-width u64 numbers (2036 rollover safe).
                let mut parts = value.split_ascii_whitespace();
                let valid_time = |part: Option<&str>| {
                    part.is_some_and(|digits| {
                        !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
                    })
                };
                if !valid_time(parts.next()) || !valid_time(parts.next()) || parts.next().is_some()
                {
                    return Err(SdpError::Malformed);
                }
                time = true;
                repeat = false;
                zone = false;
            }
            "r=" => {
                if !time || zone || !media.is_empty() {
                    return Err(SdpError::Malformed);
                }
                let parts: Vec<_> = value.split_ascii_whitespace().collect();
                if parts.len() < 3 || parts.iter().any(|part| !time_unit_token(part)) {
                    return Err(SdpError::Malformed);
                }
                repeat = true;
            }
            "z=" => {
                if !time || !repeat || zone || !media.is_empty() {
                    return Err(SdpError::Malformed);
                }
                let parts: Vec<_> = value.split_ascii_whitespace().collect();
                if parts.len() < 2
                    || parts.len() % 2 != 0
                    || parts
                        .iter()
                        .step_by(2)
                        .zip(parts.iter().skip(1).step_by(2))
                        .any(|(stamp, offset)| {
                            !stamp.bytes().all(|b| b.is_ascii_digit())
                                || stamp.is_empty()
                                || !time_unit_token(offset.strip_prefix('-').unwrap_or(offset))
                        })
                {
                    return Err(SdpError::Malformed);
                }
                zone = true;
            }
            "m=" => {
                if !time || media.len() >= 32 {
                    return Err(SdpError::BadMedia);
                }
                let mut p = value.split_ascii_whitespace();
                let kind = p.next().ok_or(SdpError::BadMedia)?;
                let port = p.next().ok_or(SdpError::BadMedia)?;
                let protocol = p.next().ok_or(SdpError::BadMedia)?;
                let formats: Vec<_> = p.map(str::to_owned).collect();
                // Port/count grammar is deliberately not supported. Reject it
                // rather than guessing a second number or advertised sockets.
                if kind.is_empty()
                    || !kind.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    || protocol.is_empty()
                    || protocol.len() > 64
                    || !protocol
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"/.-".contains(&b))
                    || formats.is_empty()
                    || formats.len() > 64
                    || formats.iter().any(|f| {
                        f.len() > 64
                            || !f
                                .bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"_./+-".contains(&b))
                    })
                {
                    return Err(SdpError::BadMedia);
                }
                let port = port.parse::<u16>().map_err(|_| SdpError::BadMedia)?;
                media.push(MediaDescription {
                    kind: kind.into(),
                    port,
                    protocol: protocol.into(),
                    formats,
                    attribute_count: 0,
                });
            }
            "a=" => {
                // RFC 8866 §5: session attributes follow the time descriptions.
                // Otherwise early attributes could be misclassified as trusted
                // session defaults despite an invalid SDP field order.
                if !time {
                    return Err(SdpError::Malformed);
                }
                if let Some(last) = media.last_mut() {
                    last.attribute_count += 1;
                } else {
                    session_attributes += 1;
                }
            }
            // Recognized fields remain inert in this read-only inspector.
            "i=" | "u=" | "e=" | "p=" | "c=" | "b=" | "k=" => {
                if !subject {
                    return Err(SdpError::Malformed);
                }
            }
            // RFC 8866 §5: extensions use a= attributes, not new fields.
            _ => return Err(SdpError::Malformed),
        }
    }
    if !(v && o && subject && time) {
        return Err(SdpError::MissingRequired);
    }
    Ok(SdpSummary {
        media,
        session_attribute_count: session_attributes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE: &str = "v=0\r\no=- 1 1 IN IP4 192.0.2.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0 1\r\nm=audio 49170 RTP/AVP 0 8\r\na=rtpmap:0 PCMU/8000\r\nm=video 0 RTP/AVP 96\r\na=rtpmap:96 H264/90000\r\n";
    #[test]
    fn inspection_does_not_negotiate_media() {
        let s = inspect_sdp(SAMPLE.as_bytes()).unwrap();
        assert_eq!(s.media.len(), 2);
        assert_eq!(s.media[0].kind, "audio");
        assert_eq!(s.media[1].port, 0);
        assert_eq!(s.session_attribute_count, 1);
        assert_eq!(s.media[0].attribute_count, 1);
    }
    #[test]
    fn rejects_truncated_and_malformed_sdp() {
        assert_eq!(inspect_sdp(b"v=0\n"), Err(SdpError::Malformed));
        assert_eq!(
            inspect_sdp(b"v=0\r\ns=-\r\nt=0 0\r\n"),
            Err(SdpError::Malformed)
        );
        assert_eq!(
            inspect_sdp(SAMPLE.replace("49170", "999999").as_bytes()),
            Err(SdpError::BadMedia)
        );
        assert_eq!(
            inspect_sdp(SAMPLE.replace("v=0", "v=1").as_bytes()),
            Err(SdpError::Malformed)
        );
        assert_eq!(
            inspect_sdp(SAMPLE.replace("s=-", "s=-\r\ns=-").as_bytes()),
            Err(SdpError::DuplicateRequired)
        );
    }
    #[test]
    fn rfc8866_origin_rejects_nonnumeric_session_identity_and_version() {
        for origin in [
            "o=- not-a-number 1 IN IP4 192.0.2.1",
            "o=- 1 not-a-number IN IP4 192.0.2.1",
            "o=- +1 1 IN IP4 192.0.2.1",
            "o=- 1 -1 IN IP4 192.0.2.1",
            "o=- 1 1.0 IN IP4 192.0.2.1",
        ] {
            let invalid = SAMPLE.replace("o=- 1 1 IN IP4 192.0.2.1", origin);
            assert_eq!(
                inspect_sdp(invalid.as_bytes()),
                Err(SdpError::Malformed),
                "{origin}"
            );
        }
        // Preserve arbitrarily long decimal identities without overflow.
        let long_id = "9".repeat(80);
        let valid = SAMPLE.replace(
            "o=- 1 1 IN IP4 192.0.2.1",
            &format!("o=- {long_id} 0 IN IP4 192.0.2.1"),
        );
        assert!(inspect_sdp(valid.as_bytes()).is_ok());
    }

    #[test]
    fn ms_sdpext_origin_requires_in_and_address_family_literal() {
        for origin in [
            "o=- 1 1 OTHER IP4 192.0.2.1",
            "o=- 1 1 IN IP5 192.0.2.1",
            "o=- 1 1 IN IP4 2001:db8::1",
            "o=- 1 1 IN IP6 192.0.2.1",
            "o=- 1 1 IN IP4 999.1.2.3",
            "o=- 1 1 IN IP4 host.example",
            "o=- 1 1 IN IP6 fe80::1%eth0",
        ] {
            let wire = SAMPLE.replace("o=- 1 1 IN IP4 192.0.2.1", origin);
            assert_eq!(
                inspect_sdp(wire.as_bytes()),
                Err(SdpError::Malformed),
                "{origin}"
            );
        }
        for origin in ["o=- 1 1 IN IP4 192.0.2.1", "o=- 1 1 IN IP6 2001:db8::1"] {
            let wire = SAMPLE.replace("o=- 1 1 IN IP4 192.0.2.1", origin);
            assert!(inspect_sdp(wire.as_bytes()).is_ok(), "{origin}");
        }
    }

    #[test]
    fn rfc8866_time_fields_accept_long_decimals_and_reject_invalid_values() {
        for invalid in ["-1 0", "1 +2", "1 2.5", "foo 0", "1 0x10", "1"] {
            let wire = SAMPLE.replace("t=0 0", &format!("t={invalid}"));
            assert_eq!(
                inspect_sdp(wire.as_bytes()),
                Err(SdpError::Malformed),
                "{invalid}"
            );
        }
        let oversized = "9".repeat(80);
        let wire = SAMPLE.replace("t=0 0", &format!("t={oversized} 0"));
        assert!(inspect_sdp(wire.as_bytes()).is_ok());
        let wire = SAMPLE.replace("t=0 0", &format!("t=0 {oversized}"));
        assert!(inspect_sdp(wire.as_bytes()).is_ok());
    }

    #[test]
    fn rfc8866_session_attributes_follow_timing_not_origin_or_name() {
        for misplaced in [
            SAMPLE.replace(
                "o=- 1 1 IN IP4 192.0.2.1\r\n",
                "a=x-mediabw:100\r\no=- 1 1 IN IP4 192.0.2.1\r\n",
            ),
            SAMPLE.replace("s=-\r\n", "s=-\r\na=x-devicecaps:1\r\n"),
        ] {
            assert_eq!(inspect_sdp(misplaced.as_bytes()), Err(SdpError::Malformed));
        }
        // Session- and media-level extension attributes remain countable only.
        let valid = SAMPLE.replace(
            "a=group:BUNDLE 0 1\r\n",
            "a=x-mediabw:100\r\na=group:BUNDLE 0 1\r\n",
        );
        let summary = inspect_sdp(valid.as_bytes()).unwrap();
        assert_eq!(summary.session_attribute_count, 2);
        assert_eq!(summary.media[0].attribute_count, 1);
    }

    #[test]
    fn rfc8866_repeat_and_zone_are_bounded_and_ordered() {
        for good in [
            "r=7d 1h 0 25h\r\n",
            "r=604800 3600 0 90000\r\nz=3730928400 -1h 3749680800 0\r\n",
        ] {
            let wire = SAMPLE.replace("t=0 0\r\n", &format!("t=0 0\r\n{good}"));
            assert!(inspect_sdp(wire.as_bytes()).is_ok(), "{good:?}");
        }
        for bad in [
            "r=1h 1h",
            "r=1h 1h +2",
            "r=1h 1h 1w",
            "z=3730928400 -1h",
            "r=1h 1h 0\r\nz=10",
            "r=1h 1h 0\r\nz=10 -1w",
            "r=1h 1h 0\r\nz=10 --1h",
            "r=1h 1h 0\r\nz=10 ---1h",
            "r=1h 1h 0\r\nz=10 +1h",
        ] {
            let wire = SAMPLE.replace("t=0 0\r\n", &format!("t=0 0\r\n{bad}\r\n"));
            assert_eq!(
                inspect_sdp(wire.as_bytes()),
                Err(SdpError::Malformed),
                "{bad:?}"
            );
        }
        let after_media = SAMPLE.to_owned() + "r=1h 1h 0\r\n";
        assert_eq!(
            inspect_sdp(after_media.as_bytes()),
            Err(SdpError::Malformed)
        );
    }

    #[test]
    fn rfc8866_rejects_time_description_after_zone_adjustments() {
        // RFC 8866 §§5.9–5.11: z= follows all t=/r= descriptions.
        for suffix in [
            "t=0 0\r\n",
            "t=1 2\r\nr=7d 1h 0\r\n",
        ] {
            let replacement = format!(
                "t=0 0\r\nr=7d 1h 0\r\nz=3730928400 -1h\r\n{suffix}"
            );
            let wire = SAMPLE.replace("t=0 0\r\n", &replacement);
            assert_eq!(inspect_sdp(wire.as_bytes()), Err(SdpError::Malformed));
        }
    }

    #[test]
    fn rfc8866_unknown_field_types_fail_closed() {
        for field in ["x=vendor-private", "q=unexpected", "y=opaque"] {
            let session = SAMPLE.replace("t=0 0\r\n", &format!("t=0 0\r\n{field}\r\n"));
            assert_eq!(
                inspect_sdp(session.as_bytes()),
                Err(SdpError::Malformed),
                "unknown session field {field}"
            );
            let media = format!("{SAMPLE}{field}\r\n");
            assert_eq!(
                inspect_sdp(media.as_bytes()),
                Err(SdpError::Malformed),
                "unknown media field {field}"
            );
        }
        let attribute = SAMPLE.replace(
            "a=group:BUNDLE 0 1\r\n",
            "a=x-vendor-private:opaque\r\na=group:BUNDLE 0 1\r\n",
        );
        assert!(inspect_sdp(attribute.as_bytes()).is_ok());
    }

    #[test]
    fn rejects_control_characters_and_unbounded_input() {
        assert_eq!(
            inspect_sdp(&vec![b'a'; MAX_SDP_BYTES + 1]),
            Err(SdpError::TooLarge)
        );
        assert_eq!(
            inspect_sdp(SAMPLE.replace("s=-", "s=\u{0000}").as_bytes()),
            Err(SdpError::Malformed)
        );
    }
}
