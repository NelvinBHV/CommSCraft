//! Bounds on what waits in the event queues between worker threads and the session loop
//! (issue #253). The queues stay `std::sync::mpsc` channels; what can arrive at network or
//! frame rate is counted here: a producer takes a place before sending, the consumer gives it
//! back on receipt.
//!
//! - Decoded pictures ([`FRAMES`]): a picture is dropped when the places are taken (the
//!   session loop is behind; a newer picture follows).
//! - Meeting content events ([`CONTENT`]): never dropped (that would corrupt the shown state);
//!   the content thread stops reading the network until places are free (TCP pushes back),
//!   and ends the content session if they stay taken ([`CONTENT_WAIT_MS`]).
//!
//! The other queues carry control events bounded by the calls themselves (gathering done,
//! path selected, notices, the end): a few per call.

use std::sync::atomic::{AtomicUsize, Ordering};

/// A counted bound on queued items.
pub struct Backlog {
    queued: AtomicUsize,
    max: usize,
}

impl Backlog {
    pub const fn new(max: usize) -> Self {
        Backlog {
            queued: AtomicUsize::new(0),
            max,
        }
    }

    /// Takes a place; `false` when all are taken (the item is not sent).
    pub fn enter(&self) -> bool {
        self.step(|n| (n < self.max).then_some(n + 1))
    }

    /// Gives a place back (the consumer took an item, or sending failed).
    pub fn leave(&self) {
        self.step(|n| n.checked_sub(1));
    }

    /// Applies `f` to the count atomically; `false` when `f` refuses.
    fn step(&self, f: impl Fn(usize) -> Option<usize>) -> bool {
        let mut n = self.queued.load(Ordering::Acquire);
        loop {
            let Some(next) = f(n) else { return false };
            match self
                .queued
                .compare_exchange_weak(n, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return true,
                Err(now) => n = now,
            }
        }
    }

    /// Items waiting now.
    pub fn queued(&self) -> usize {
        self.queued.load(Ordering::Acquire)
    }

    /// Whether `n` more items fit.
    pub fn room_for(&self, n: usize) -> bool {
        self.queued().saturating_add(n) <= self.max
    }
}

/// Decoded pictures (received and self view) waiting for the session loop: a few frames.
pub static FRAMES: Backlog = Backlog::new(8);
/// Meeting content events and lines waiting for the session loop.
pub static CONTENT: Backlog = Backlog::new(4096);
/// How long the content thread waits for free places before it ends the content session.
pub const CONTENT_WAIT_MS: u64 = 10_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_are_bounded_and_given_back() {
        let b = Backlog::new(2);
        assert!(b.enter() && b.enter());
        assert!(!b.enter());
        b.leave();
        assert!(b.enter());
        b.leave();
        b.leave();
        b.leave();
        assert_eq!(b.queued(), 0);
        assert!(b.room_for(2) && !b.room_for(3));
    }
}
