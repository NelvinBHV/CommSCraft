//! Fail-closed, read-only SIP receive transport framing.
//!
//! RFC 3261 §§7.5, 18.3: SIP over streams requires Content-Length and
//! may be fragmented or coalesced; leading CRLF is ignored.
//! This is an offline/testable read abstraction. It DOES NOT authenticate a
//! peer; a verified TLS channel is required by the live transport wrapper.

use crate::sip::{SipError, SipMessage};
use crate::stream::SipStreamDecoder;
use std::collections::VecDeque;
use std::io::{self, Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveError {
    Malformed(SipError),
    /// Connection closed while a SIP frame or CRLF was incomplete.
    Truncated,
    /// Socket failed; do not resume parsing on this connection.
    Transport(io::ErrorKind),
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiveEvent {
    Message(SipMessage),
    /// Not enough data yet, or a timeout. The incomplete frame remains buffered.
    Pending,
}

/// No write method is intentionally provided: SIP authentication and signing
/// must be integrated before network messages may be sent.
#[derive(Debug, Default)]
pub struct SipInbound {
    decoder: SipStreamDecoder,
    ready: VecDeque<SipMessage>,
    closed: bool,
}

impl SipInbound {
    pub fn new() -> Self { Self::default() }

    /// Call from a TLS-decrypted stream. `read` is invoked at most once;
    /// callers choose their own timeouts. A read timeout preserves framing.
    pub fn receive<R: Read>(&mut self, io: &mut R) -> Result<ReceiveEvent, ReceiveError> {
        if let Some(message) = self.ready.pop_front() {
            return Ok(ReceiveEvent::Message(message));
        }
        if self.closed { return Err(ReceiveError::Closed); }
        let mut buf = [0_u8; 8192];
        match io.read(&mut buf) {
            Ok(0) => {
                self.closed = true;
                if self.decoder.pending_bytes() > 0 { Err(ReceiveError::Truncated) }
                else { Err(ReceiveError::Closed) }
            }
            Ok(count) => match self.decoder.push(&buf[..count]) {
                Ok(frames) => {
                    self.ready.extend(frames);
                    Ok(self.ready.pop_front().map(ReceiveEvent::Message)
                        .unwrap_or(ReceiveEvent::Pending))
                }
                Err(error) => {
                    self.closed = true;
                    Err(ReceiveError::Malformed(error))
                }
            },
            Err(error) if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                | io::ErrorKind::Interrupted) => Ok(ReceiveEvent::Pending),
            Err(error) => {
                self.closed = true;
                Err(ReceiveError::Transport(error.kind()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Error, ErrorKind};
    const OK: &[u8] = b"SIP/2.0 200 OK\r\nContent-Length: 0\r\n\r\n";
    const TRY: &[u8] = b"SIP/2.0 100 Trying\r\nl: 0\r\n\r\n";

    struct Scripted {
        steps: VecDeque<Result<Vec<u8>, ErrorKind>>,
    }
    impl Scripted {
        fn from(steps: Vec<Result<Vec<u8>, ErrorKind>>) -> Self {
            Self { steps: steps.into() }
        }
    }
    impl Read for Scripted {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            match self.steps.pop_front() {
                Some(Ok(bytes)) => {
                    assert!(bytes.len() <= out.len());
                    out[..bytes.len()].copy_from_slice(&bytes);
                    Ok(bytes.len())
                }
                Some(Err(kind)) => Err(Error::from(kind)),
                None => Ok(0),
            }
        }
    }

    #[test]
    fn coalesced_frames_returned_without_second_read() {
        let mut input = Cursor::new([TRY, OK].concat());
        let mut receiver = SipInbound::new();
        assert!(matches!(receiver.receive(&mut input), Ok(ReceiveEvent::Message(_))));
        assert!(matches!(receiver.receive(&mut input), Ok(ReceiveEvent::Message(_))));
        assert_eq!(receiver.receive(&mut input), Err(ReceiveError::Closed));
    }
    #[test]
    fn fragment_then_timeout_retains_partial_message() {
        let mut input = Scripted::from(vec![
            Ok(OK[..10].to_vec()), Err(ErrorKind::TimedOut), Ok(OK[10..].to_vec()),
        ]);
        let mut receiver = SipInbound::new();
        assert_eq!(receiver.receive(&mut input), Ok(ReceiveEvent::Pending));
        assert_eq!(receiver.receive(&mut input), Ok(ReceiveEvent::Pending));
        assert!(matches!(receiver.receive(&mut input), Ok(ReceiveEvent::Message(_))));
    }
    #[test]
    fn truncated_frame_at_eof_rejected() {
        let mut input = Cursor::new(&OK[..OK.len()-1]);
        let mut receiver = SipInbound::new();
        assert_eq!(receiver.receive(&mut input), Ok(ReceiveEvent::Pending));
        assert_eq!(receiver.receive(&mut input), Err(ReceiveError::Truncated));
        assert_eq!(receiver.receive(&mut input), Err(ReceiveError::Closed));
    }
    #[test]
    fn missing_length_fails_closed() {
        let mut input = Cursor::new(b"SIP/2.0 200 OK\r\nVia: test\r\n\r\n");
        let mut receiver = SipInbound::new();
        assert_eq!(receiver.receive(&mut input), Err(ReceiveError::Malformed(SipError::MissingContentLength)));
        assert_eq!(receiver.receive(&mut input), Err(ReceiveError::Closed));
    }
    #[test]
    fn duplicate_length_fails_closed() {
        let mut input = Cursor::new(b"SIP/2.0 200 OK\r\nl: 0\r\nContent-Length: 0\r\n\r\n");
        let mut receiver = SipInbound::new();
        assert_eq!(receiver.receive(&mut input), Err(ReceiveError::Malformed(SipError::DuplicateContentLength)));
    }
    #[test]
    fn keepalive_crlf_is_skipped() {
        let mut input = Cursor::new([&b"\r\n\r\n"[..], OK].concat());
        let mut receiver = SipInbound::new();
        assert!(matches!(receiver.receive(&mut input), Ok(ReceiveEvent::Message(_))));
    }
    #[test]
    fn transport_error_poisoned() {
        let mut input = Scripted::from(vec![Err(ErrorKind::ConnectionReset), Ok(OK.to_vec())]);
        let mut receiver = SipInbound::new();
        assert_eq!(receiver.receive(&mut input), Err(ReceiveError::Transport(ErrorKind::ConnectionReset)));
        assert_eq!(receiver.receive(&mut input), Err(ReceiveError::Closed));
    }
    #[test]
    fn retryable_io_error_preserves_connection() {
        let mut input = Scripted::from(vec![Err(ErrorKind::Interrupted), Ok(OK.to_vec())]);
        let mut receiver = SipInbound::new();
        assert_eq!(receiver.receive(&mut input), Ok(ReceiveEvent::Pending));
        assert!(matches!(receiver.receive(&mut input), Ok(ReceiveEvent::Message(_))));
    }
    #[test]
    fn huge_header_is_rejected() {
        let mut input = Cursor::new(vec![b'A'; crate::sip::MAX_HEADER_BYTES + 100]);
        let mut receiver = SipInbound::new();
        let mut failed = false;
        for _ in 0..20 {
            match receiver.receive(&mut input) {
                Err(ReceiveError::Malformed(SipError::Oversize)) => { failed = true; break; },
                Ok(ReceiveEvent::Pending) => {},
                other => panic!("unexpected result: {other:?}"),
            }
        }
        assert!(failed);
    }
}
