//! Offline response dispatcher for reliable (TCP/TLS) non-INVITE SIP transactions.
//!
//! RFC 3261 §§17.1.2, 17.1.3 and 18.4; MS-SIPAE §4.2 (REGISTER challenge
//! example), MS-OCSPROT §2.5.2. This is NOT a live SIP stack or authentication.
//! A transport/driver MUST tie the entire router to exactly one TLS connection;
//! this module does not establish trust, send packets, sign or verify MICs.
//!
//! Invariants:
//! * Strictly bounded pending transactions and recently closed identities.
//! * A reply with wrong CSeq/Call-ID/Via transport/branch never reaches a flow.
//! * Duplicated final replies never emit another final event.
//! * 401/407/423 are FINAL; retries require a freshly branched transaction.
//! * Disconnect terminates all requests. Time uses caller's monotonic clock.

use sfb_core::sip::{SipMessage, StartLine};
use sfb_core::transaction::{NonInviteClientTransaction, TransactionError, TransactionEvent,
    TransactionKey};
use std::collections::VecDeque;
use std::fmt;

pub const MAX_IN_FLIGHT: usize = 64;
pub const MAX_RECENTLY_CLOSED: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutedEvent {
    Provisional { id: TransactionId, code: u16 },
    Final { id: TransactionId, code: u16 },
    TimedOut { id: TransactionId },
    TransportLost { id: TransactionId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterError {
    Transaction(TransactionError),
    CapacityExceeded,
    DuplicateIdentity,
    IdExhausted,
    UnknownResponse,
    DuplicateOrLateResponse,
    Expired,
    Disconnected,
}
impl fmt::Display for RouterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for RouterError {}

#[derive(Debug)]
struct OpenTransaction {
    id: TransactionId,
    transaction: NonInviteClientTransaction,
}

/// This object is not clonable: one instance represents one bounded,
/// not-yet-authenticated local transaction namespace for a single connection.
#[derive(Debug, Default)]
pub struct ResponseRouter {
    open: Vec<OpenTransaction>,
    recently_closed: VecDeque<TransactionKey>,
    next_id: u64,
    disconnected: bool,
}

impl ResponseRouter {
    pub fn new() -> Self { Self::default() }
    pub fn len(&self) -> usize { self.open.len() }
    pub fn is_empty(&self) -> bool { self.open.is_empty() }
    pub fn is_disconnected(&self) -> bool { self.disconnected }

    fn remember(&mut self, key: TransactionKey) {
        if self.recently_closed.len() == MAX_RECENTLY_CLOSED {
            self.recently_closed.pop_front();
        }
        self.recently_closed.push_back(key);
    }

    /// Register an already-constructed request before an external sender
    /// transmits it. No send API is available here. An authenticated retry
    /// must use a new RFC 3261 branch/CSeq, never reuse a completed identity.
    pub fn track(&mut self, request: &SipMessage, now_ms: u64)
        -> Result<TransactionId, RouterError>
    {
        if self.disconnected { return Err(RouterError::Disconnected); }
        let transaction = NonInviteClientTransaction::new(request, now_ms)
            .map_err(RouterError::Transaction)?;
        let key = transaction.key();
        if self.open.iter().any(|entry| entry.transaction.key() == key)
            || self.recently_closed.iter().any(|closed| closed == key)
        { return Err(RouterError::DuplicateIdentity); }
        if self.open.len() >= MAX_IN_FLIGHT { return Err(RouterError::CapacityExceeded); }
        let next = self.next_id.checked_add(1).ok_or(RouterError::IdExhausted)?;
        self.next_id = next;
        let id = TransactionId(next);
        self.open.push(OpenTransaction { id, transaction });
        Ok(id)
    }

    /// Dispatch a framed response. Malformed, unsolicited, duplicated or
    /// expired replies do not commit application state. A successful Final is
    /// ONLY a SIP transaction result, NOT evidence of verified server identity.
    pub fn response(&mut self, message: &SipMessage, now_ms: u64)
        -> Result<RoutedEvent, RouterError>
    {
        if self.disconnected { return Err(RouterError::Disconnected); }
        if !matches!(&message.start_line, StartLine::Response { .. }) {
            return Err(RouterError::Transaction(TransactionError::NotResponse));
        }
        let key = TransactionKey::from_message(message).map_err(RouterError::Transaction)?;
        let Some(position) = self.open.iter().position(|entry| entry.transaction.key() == &key) else {
            if self.recently_closed.contains(&key) { return Err(RouterError::DuplicateOrLateResponse); }
            return Err(RouterError::UnknownResponse);
        };
        let entry = &mut self.open[position];
        let id = entry.id;
        match entry.transaction.receive_response(message, now_ms) {
            Ok(TransactionEvent::Provisional(code)) => Ok(RoutedEvent::Provisional { id, code }),
            Ok(TransactionEvent::Final(code)) => {
                let finished = self.open.remove(position);
                self.remember(finished.transaction.key().clone());
                Ok(RoutedEvent::Final { id, code })
            }
            Ok(TransactionEvent::TimedOut) | Err(TransactionError::TimedOut) => {
                let timed_out = self.open.remove(position);
                self.remember(timed_out.transaction.key().clone());
                Err(RouterError::Expired)
            }
            Err(error) => Err(RouterError::Transaction(error)),
        }
    }

    /// Advance Timer F (default 64*T1=32s). No Timer E retransmits on TLS.
    pub fn tick(&mut self, now_ms: u64) -> Vec<RoutedEvent> {
        if self.disconnected { return Vec::new(); }
        let mut expired = Vec::new();
        let mut position = 0;
        while position < self.open.len() {
            if self.open[position].transaction.tick(now_ms).is_some() {
                let entry = self.open.remove(position);
                self.remember(entry.transaction.key().clone());
                expired.push(RoutedEvent::TimedOut { id: entry.id });
            } else { position += 1; }
        }
        expired
    }

    /// Any TLS EOF, fatal framing failure, handshake failure or connection
    /// replacement MUST abort every outstanding transaction; never reuse
    /// this response router on another peer or socket.
    pub fn disconnect(&mut self) -> Vec<RoutedEvent> {
        if self.disconnected { return Vec::new(); }
        self.disconnected = true;
        let open = std::mem::take(&mut self.open);
        open.into_iter().map(|entry| RoutedEvent::TransportLost { id: entry.id }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sfb_core::sip::illustrative_register;

    fn request(seq: u32, branch: &str) -> SipMessage {
        let mut msg = illustrative_register("sip:alice@example.org").unwrap();
        for (name, value) in &mut msg.headers {
            if name == "CSeq" { *value = format!("{seq} REGISTER"); }
            if name == "Via" { *value = format!("SIP/2.0/TLS edge.example.org:5061;branch={branch}"); }
        }
        msg
    }
    fn response(req: &SipMessage, code: u16) -> SipMessage {
        SipMessage {
            start_line: StartLine::Response { code, reason: "Synthetic".into() },
            headers: ["Via", "Call-ID", "CSeq"].iter().map(|name|
                ((*name).into(), req.header(name).unwrap().into())).collect(),
            body: Vec::new(),
        }
    }
    fn different(i: usize) -> SipMessage {
        request(i as u32 + 1, &format!("z9hG4bK-unique-{i:06}"))
    }
    #[test]
    fn concurrent_transactions_route_out_of_order() {
        let mut router = ResponseRouter::new();
        let a = different(1);
        let b = different(2);
        let a_id = router.track(&a, 0).unwrap();
        let b_id = router.track(&b, 0).unwrap();
        assert_ne!(a_id, b_id);
        assert_eq!(router.response(&response(&b, 100), 1),
            Ok(RoutedEvent::Provisional { id: b_id, code: 100 }));
        assert_eq!(router.response(&response(&a, 401), 2),
            Ok(RoutedEvent::Final { id: a_id, code: 401 }));
        assert_eq!(router.response(&response(&b, 200), 3),
            Ok(RoutedEvent::Final { id: b_id, code: 200 }));
        assert!(router.is_empty());
    }
    #[test]
    fn challenge_is_final_and_retries_need_new_identity() {
        let mut router = ResponseRouter::new();
        let old = different(4);
        let first = router.track(&old, 0).unwrap();
        assert_eq!(router.response(&response(&old, 407), 1),
            Ok(RoutedEvent::Final { id: first, code: 407 }));
        assert_eq!(router.track(&old, 2), Err(RouterError::DuplicateIdentity));
        let retry = different(5);
        assert!(router.track(&retry, 2).is_ok());
    }
    #[test]
    fn rejects_wrong_cseq_call_id_branch_and_transport_without_consumption() {
        let mut router = ResponseRouter::new();
        let req = different(3);
        let id = router.track(&req, 0).unwrap();
        for key in ["CSeq", "Call-ID", "Via"] {
            let mut invalid = response(&req, 200);
            let header = &mut invalid.headers.iter_mut().find(|(name,_)| name == key).unwrap().1;
            if key == "CSeq" { *header = "999 REGISTER".into(); }
            else { header.push_str("-different"); }
            assert_eq!(router.response(&invalid, 1), Err(RouterError::UnknownResponse));
        }
        let mut changed_transport = response(&req, 200);
        changed_transport.headers[0].1 = changed_transport.headers[0].1.replace("SIP/2.0/TLS", "SIP/2.0/TCP");
        assert_eq!(router.response(&changed_transport, 1), Err(RouterError::UnknownResponse));
        assert_eq!(router.len(), 1);
        assert_eq!(router.response(&response(&req, 200), 2), Ok(RoutedEvent::Final { id, code: 200 }));
    }
    #[test]
    fn duplicate_final_reply_is_quarantined() {
        let mut router = ResponseRouter::new();
        let req = different(6);
        router.track(&req, 0).unwrap();
        assert!(matches!(router.response(&response(&req, 200), 1), Ok(RoutedEvent::Final { .. })));
        assert_eq!(router.response(&response(&req, 200), 2), Err(RouterError::DuplicateOrLateResponse));
    }
    #[test]
    fn timer_f_is_not_reset_by_provisional_responses() {
        let mut router = ResponseRouter::new();
        let req = different(7);
        let id = router.track(&req, 0).unwrap();
        router.response(&response(&req, 100), TIMER_F_MS - 1).unwrap();
        assert_eq!(router.tick(TIMER_F_MS), vec![RoutedEvent::TimedOut { id }]);
        assert_eq!(router.response(&response(&req, 200), TIMER_F_MS + 1),
            Err(RouterError::DuplicateOrLateResponse));
    }
    #[test]
    fn late_final_is_rejected_without_tick() {
        let mut router = ResponseRouter::new();
        let req = different(8);
        router.track(&req, 0).unwrap();
        assert_eq!(router.response(&response(&req, 200), TIMER_F_MS), Err(RouterError::Expired));
        assert!(router.is_empty());
    }
    #[test]
    fn non_response_is_never_routed() {
        let mut router = ResponseRouter::new();
        let req = different(9);
        router.track(&req, 0).unwrap();
        assert_eq!(router.response(&req, 1),
            Err(RouterError::Transaction(TransactionError::NotResponse)));
        assert_eq!(router.len(), 1);
    }
    #[test]
    fn capacity_is_bounded_and_recovers() {
        let mut router = ResponseRouter::new();
        for i in 0..MAX_IN_FLIGHT { router.track(&different(i), 0).unwrap(); }
        assert_eq!(router.len(), MAX_IN_FLIGHT);
        assert_eq!(router.track(&different(MAX_IN_FLIGHT), 0), Err(RouterError::CapacityExceeded));
        assert_eq!(router.tick(TIMER_F_MS).len(), MAX_IN_FLIGHT);
        router.track(&different(MAX_IN_FLIGHT), TIMER_F_MS + 1).unwrap();
    }
    #[test]
    fn disconnect_terminates_all_and_prevents_peer_reuse() {
        let mut router = ResponseRouter::new();
        let req = different(20);
        let id = router.track(&req, 0).unwrap();
        assert_eq!(router.disconnect(), vec![RoutedEvent::TransportLost { id }]);
        assert!(router.disconnect().is_empty());
        assert_eq!(router.track(&different(21), 5), Err(RouterError::Disconnected));
        assert_eq!(router.response(&response(&req, 200), 5), Err(RouterError::Disconnected));
    }
    #[test]
    fn invalid_reply_does_not_close_other_transactions() {
        let mut router = ResponseRouter::new();
        let a = different(10);
        let b = different(11);
        router.track(&a, 0).unwrap();
        router.track(&b, 0).unwrap();
        let mut bad = response(&a, 200);
        bad.headers.push(("Via".into(), a.header("Via").unwrap().into()));
        assert_eq!(router.response(&bad, 1), Err(RouterError::Transaction(TransactionError::DuplicateHeader("Via"))));
        assert_eq!(router.len(), 2);
    }
    #[test]
    fn exceeding_recent_history_evicts_oldest() {
        let mut router = ResponseRouter::new();
        for i in 0..MAX_RECENTLY_CLOSED + 1 {
            let req = different(i);
            router.track(&req, i as u64).unwrap();
            router.response(&response(&req, 200), i as u64 + 1).unwrap();
        }
        let old = different(0);
        assert_eq!(router.response(&response(&old, 200), 300), Err(RouterError::UnknownResponse));
        assert_eq!(router.response(&response(&different(1), 200), 300), Err(RouterError::DuplicateOrLateResponse));
    }
}
