//! Hop-by-hop keep-alive negotiation and the CRLFCRLF refresh (MS-CONMGMT).
//!
//! Evidence: `docs/protocol/evidence/02-conmgmt-connection.md` (CON-30 … CON-35, K-CON-1).
//! Source: [MS-CONMGMT] 20250520 §2.2.1–2.2.2 (pp. 10–12), §3.4 (pp. 16–19), example §4 (p. 21).
//!
//! Security by design: the server's timeout is bounded (an absurdly small value would make the
//! client flood the connection, a huge one is useless); any malformed or ambiguous answer means
//! "no keep-alive" (§3.4.5.3 already says so for missing or repeated headers).

use commscraft_sip_msg::Message;

/// The request header (§3.4.5.1): role UAC, only hop-hop; end-end and tcp SHOULD NOT be offered.
pub const REQUEST_VALUE: &str = "UAC;hop-hop=yes";
/// The keep-alive message (§2.2.2).
pub const MESSAGE: &[u8] = b"\r\n\r\n";
/// Accepted timeout range in seconds (inference/design; the recommended value is 300, <4> p. 23).
pub const MIN_TIMEOUT: u64 = 30;
pub const MAX_TIMEOUT: u64 = 3600;

/// Evaluates the `ms-keep-alive` header of the response to the negotiating request (§3.4.5.3).
/// `None`: negotiation failed, send no keep-alives.
pub fn negotiated_timeout(response: &Message) -> Option<u64> {
    if !response
        .status_code()
        .is_some_and(|c| (200..300).contains(&c))
    {
        return None;
    }
    let mut values = response.header_values("ms-keep-alive");
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    let mut parts = value.split(';').map(str::trim);
    if !parts.next()?.eq_ignore_ascii_case("UAS") {
        return None;
    }
    let mut hop_hop = false;
    let mut timeout = None;
    for p in parts {
        let (k, v) = p.split_once('=')?;
        let (k, v) = (k.trim(), v.trim());
        if k.eq_ignore_ascii_case("timeout") {
            if timeout.is_some()
                || v.is_empty()
                || v.len() > 6
                || !v.bytes().all(|b| b.is_ascii_digit())
            {
                return None;
            }
            timeout = v.parse::<u64>().ok();
        } else if k.eq_ignore_ascii_case("hop-hop") {
            hop_hop = v.eq_ignore_ascii_case("yes");
        }
    }
    // The only capability the client offers is hop-hop; the intersection must contain it.
    let t = timeout?;
    (hop_hop && (MIN_TIMEOUT..=MAX_TIMEOUT).contains(&t)).then_some(t)
}

/// Refresh timer of one connection (§3.4.2, §3.4.5.4). K-CON-1: §3.4.2 sets the timer to two
/// thirds of the timeout but says sending data resets it "to the time-out value"; the client
/// always uses two thirds, which keeps the connection alive in both readings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeepAlive {
    interval: u64,
    due: u64,
}

impl KeepAlive {
    pub fn new(timeout: u64, now: u64) -> Self {
        let interval = (timeout * 2 / 3).max(1);
        Self {
            interval,
            due: now.saturating_add(interval),
        }
    }

    pub fn interval(&self) -> u64 {
        self.interval
    }

    /// Any data sent on the connection restarts the timer.
    pub fn on_data_sent(&mut self, now: u64) {
        self.due = now.saturating_add(self.interval);
    }

    /// Returns the keep-alive message when it is due and restarts the timer.
    pub fn on_tick(&mut self, now: u64) -> Option<&'static [u8]> {
        if now < self.due {
            return None;
        }
        self.on_data_sent(now);
        Some(MESSAGE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(code: u16, values: &[&str]) -> Message {
        let mut m = Message::response(code, "x");
        for v in values {
            m.push_header("ms-keep-alive", v);
        }
        m
    }

    /// [MS-CONMGMT] §4.2 (p. 21) and the REGISTER response of [MS-SIPREGE] §4.1.1 (p. 94).
    #[test]
    fn example_responses() {
        assert_eq!(
            negotiated_timeout(&response(
                200,
                &["UAS; tcp=no; hop-hop=yes; end-end=no; timeout=300"]
            )),
            Some(300)
        );
        let k = KeepAlive::new(300, 1000);
        assert_eq!(k.interval(), 200);
    }

    #[test]
    fn failures_mean_no_keep_alive() {
        for (code, vals) in [
            (403, vec!["UAS; hop-hop=yes; timeout=300"]),
            (200, vec![]),
            (
                200,
                vec![
                    "UAS; hop-hop=yes; timeout=300",
                    "UAS; hop-hop=yes; timeout=300",
                ],
            ),
            (200, vec!["UAC; hop-hop=yes; timeout=300"]),
            (200, vec!["UAS; hop-hop=no; tcp=yes; timeout=300"]),
            (200, vec!["UAS; hop-hop=yes"]),
            (200, vec!["UAS; hop-hop=yes; timeout=1"]),
            (200, vec!["UAS; hop-hop=yes; timeout=99999999"]),
            (200, vec!["UAS; hop-hop=yes; timeout=-5"]),
            (200, vec!["UAS; hop-hop=yes; timeout=300; timeout=30"]),
            (200, vec!["UAS; hop-hop"]),
        ] {
            assert_eq!(
                negotiated_timeout(&response(code, &vals)),
                None,
                "{code} {vals:?}"
            );
        }
    }

    #[test]
    fn timer() {
        let mut k = KeepAlive::new(300, 0);
        assert_eq!(k.on_tick(199), None);
        assert_eq!(k.on_tick(200), Some(MESSAGE));
        k.on_data_sent(350);
        assert_eq!(k.on_tick(549), None);
        assert_eq!(k.on_tick(550), Some(MESSAGE));
    }
}
