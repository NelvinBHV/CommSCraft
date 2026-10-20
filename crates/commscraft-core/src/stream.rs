//! A strict SIP stream decoder for an eventual authenticated TCP/TLS transport.
//!
//! RFC 3261 sections 7.4.2, 18.3: a stream needs Content-Length; input can
//! arrive fragmented, concatenated and prefixed by keepalive CRLFs.  This
//! module intentionally performs *no* network I/O or SIP authentication.

use crate::sip::{parse_frame, SipError, SipMessage, MAX_BODY_BYTES, MAX_HEADER_BYTES};

pub const MAX_FRAME_BYTES: usize = MAX_HEADER_BYTES + 4 + MAX_BODY_BYTES;
const INPUT_CHUNK_BYTES: usize = 8 * 1024;

/// A single-direction streaming SIP decoder. Do not reuse after an error.
#[derive(Default, Debug)]
pub struct SipStreamDecoder {
    pending: Vec<u8>,
    failed: bool,
}

impl SipStreamDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending_bytes(&self) -> usize {
        self.pending.len()
    }

    /// Returns all fully framed messages. A bad frame poisons the decoder,
    /// because there is no safe, general-purpose stream resynchronization.
    pub fn push(&mut self, input: &[u8]) -> Result<Vec<SipMessage>, SipError> {
        if self.failed {
            return Err(SipError::DecoderPoisoned);
        }
        let mut out = Vec::new();
        for part in input.chunks(INPUT_CHUNK_BYTES) {
            self.pending.extend_from_slice(part);
            if let Err(error) = self.drain_ready(&mut out) {
                self.pending.clear();
                self.failed = true;
                return Err(error);
            }
        }
        Ok(out)
    }

    fn drain_ready(&mut self, out: &mut Vec<SipMessage>) -> Result<(), SipError> {
        loop {
            // RFC 3261 section 7.5: ignore CRLF preceding the start-line.
            // A trailing partial CRLF remains until the next chunk.
            let mut count = 0;
            while self.pending.get(count..count + 2) == Some(&b"\r\n"[..]) {
                count += 2;
            }
            if count != 0 {
                self.pending.drain(..count);
            }
            if self.pending.is_empty() || self.pending.len() == 1 && self.pending[0] == b'\r' {
                return Ok(());
            }

            match parse_frame(&self.pending)? {
                Some((message, used)) => {
                    // Unlike message-oriented transports, TCP/TLS MUST carry
                    // Content-Length, including for zero-byte bodies.
                    if message.header("Content-Length").is_none() && message.header("l").is_none() {
                        return Err(SipError::MissingContentLength);
                    }
                    self.pending.drain(..used);
                    out.push(message);
                }
                None => {
                    if self.pending.len() > MAX_FRAME_BYTES {
                        return Err(SipError::Oversize);
                    }
                    return Ok(());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIRST: &[u8] = b"SIP/2.0 100 Trying\r\nl: 0\r\n\r\n";
    const SECOND: &[u8] = b"SIP/2.0 200 OK\r\nContent-Length: 4\r\n\r\nTEST";

    #[test]
    fn accepts_fragmentation_two_frames_and_keepalive() {
        let mut stream = SipStreamDecoder::new();
        let mut all = Vec::new();
        let wire = [&b"\r\n\r\n"[..], FIRST, SECOND].concat();
        for chunk in wire.chunks(3) {
            all.extend(stream.push(chunk).unwrap());
        }
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].body, b"TEST");
        assert_eq!(stream.pending_bytes(), 0);
    }

    #[test]
    fn strict_content_length_and_poison_after_malformed() {
        let mut stream = SipStreamDecoder::new();
        assert_eq!(
            stream.push(b"SIP/2.0 200 OK\r\nVia: test\r\n\r\n"),
            Err(SipError::MissingContentLength)
        );
        assert_eq!(stream.push(SECOND), Err(SipError::DecoderPoisoned));
    }

    #[test]
    fn incomplete_body_is_buffered() {
        let mut stream = SipStreamDecoder::new();
        assert!(stream.push(&SECOND[..SECOND.len() - 1]).unwrap().is_empty());
        let out = stream.push(&SECOND[SECOND.len() - 1..]).unwrap();
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn split_keepalive_does_not_corrupt_next_message() {
        let mut stream = SipStreamDecoder::new();
        assert_eq!(stream.push(b"\r").unwrap().len(), 0);
        assert_eq!(stream.push(b"\n").unwrap().len(), 0);
        assert_eq!(stream.push(FIRST).unwrap().len(), 1);
    }

    #[test]
    fn rejects_oversized_undelimited_header() {
        let mut stream = SipStreamDecoder::new();
        let error = stream.push(&vec![b'X'; MAX_HEADER_BYTES + 1]);
        assert_eq!(error, Err(SipError::Oversize));
    }
}
