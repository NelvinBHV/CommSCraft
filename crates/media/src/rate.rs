//! Bandwidth estimation and rate adaptation: the receiver's estimate and congestion state that
//! go back to the sender in RTCP ([MS-RTP] §2.2.11.1, §2.2.11.11, §3.2; RT-28, RT-29), the
//! sender's target bit rate from what the peer reports (RT-30), and the resolution steps of
//! Skype for Business (BW-17).
//!
//! Documented: estimates MUST go back through the profile extension (§3.2, p. 39), how they are
//! computed is "outside the scope of this specification" (§3.2.5, p. 42), and received packet
//! pairs "MAY not be used" (§3.2, p. 39). Inference (ours): user-space polling cannot time
//! back-to-back packets, so the estimate comes from the media itself – it starts at our
//! declared capacity with a low confidence, falls with the received rate when loss or queueing
//! delay show congestion and grows back slowly while the path is clean (additive increase,
//! multiplicative decrease). The thresholds are design values.
//!
//! Security by design: every input is a number from the network – arithmetic saturates, rates
//! stay between [`FLOOR_BPS`] and the capacity, nothing grows with the input.

use commscraft_rtp as rtp;

/// Lowest estimate and target we report or use (design value: below it video is useless).
pub const FLOOR_BPS: u32 = 50_000;
/// IPv4 and UDP header octets counted per packet (§3.2.5 counts headers up to the network
/// layer; we assume IPv4).
const HEADER_OCTETS: u64 = 28;
/// Length of one measurement interval (design value).
pub const INTERVAL_MS: u64 = 1000;
/// Loss fraction (of 256) from which the path counts as congested, and below which as clean
/// (design values: about 10 % and 2 %).
const LOSS_CONGESTED: u8 = 26;
const LOSS_CLEAN: u8 = 5;
/// Queueing delay (ms above the lowest seen one-way delay) from which the path counts as
/// congested, and below which as clean (design values).
const DELAY_CONGESTED_MS: i64 = 100;
const DELAY_CLEAN_MS: i64 = 30;
/// How the lowest one-way delay is forgotten: one minimum per interval, the last 10 kept.
const BASE_WINDOWS: usize = 10;
/// Delay counts as congestion only in this many intervals in a row (a single burst after a
/// stall is no queue; design value).
const DELAY_STRIKES: u8 = 2;
/// A sequence number jump larger than this is a new start (e.g. a mixer switching the source),
/// not loss (RFC 3550 A.1 treats large jumps as a restart; design value).
const MAX_JUMP: i64 = 3000;

/// Congestion bits of the network congestion notification (§2.2.11.11, p. 25).
pub const UNCONGESTED_DELAY: u8 = 1 << 0;
pub const CONGESTED_DELAY: u8 = 1 << 1;
pub const UNCONGESTED_LOSS: u8 = 1 << 2;
pub const CONGESTED_LOSS: u8 = 1 << 3;

/// The receiver's view of one incoming stream.
#[derive(Debug, Clone)]
pub struct Estimator {
    capacity: u32,
    estimate: u32,
    confidence: u8,
    window_start: Option<u64>,
    window_octets: u64,
    /// One-way delay reference: first arrival (ms) and first RTP timestamp.
    origin: Option<(u64, u32)>,
    /// Lowest relative delay of each of the last intervals, and of the running one.
    base: Vec<i64>,
    running_min: Option<i64>,
    delay_sum: i64,
    delay_count: i64,
    /// Loss: highest sequence number (extended), its value at the interval start, packets
    /// received in the interval.
    max_seq: Option<u16>,
    ext_max: i64,
    window_base: Option<i64>,
    window_received: i64,
    /// Intervals in a row with a high queueing delay.
    delay_strikes: u8,
    /// The stream began in the running interval: its loss is not judged yet (a mixer starts
    /// forwarding mid-stream, the gap before is no loss on our path; design rule).
    warming_up: bool,
    congestion: u8,
    /// The congestion bits changed and are not yet reported.
    changed: bool,
}

/// What [`Estimator::update`] found for one interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub received_bps: u32,
    pub queueing_ms: i64,
    pub fraction_lost: u8,
}

impl Estimator {
    /// `capacity`: the most we can receive (bit/s), the estimate before any measurement.
    pub fn new(capacity: u32) -> Estimator {
        let capacity = capacity.max(FLOOR_BPS);
        Estimator {
            capacity,
            estimate: capacity,
            // Not measured yet: lowest but one (§2.2.11.1: 0 = least reliable).
            confidence: 2,
            window_start: None,
            window_octets: 0,
            origin: None,
            base: Vec::with_capacity(BASE_WINDOWS),
            running_min: None,
            delay_sum: 0,
            delay_count: 0,
            max_seq: None,
            ext_max: 0,
            window_base: None,
            window_received: 0,
            delay_strikes: 0,
            warming_up: false,
            congestion: 0,
            changed: false,
        }
    }

    /// A received, authenticated packet: its size (without IP/UDP), sequence number, RTP
    /// timestamp, clock rate (Hz) and arrival time (ms, any origin).
    pub fn on_packet(
        &mut self,
        octets: usize,
        sequence: u16,
        timestamp: u32,
        clock_hz: u32,
        arrival_ms: u64,
    ) {
        self.window_octets = self
            .window_octets
            .saturating_add(octets as u64)
            .saturating_add(HEADER_OCTETS);
        match self.max_seq {
            None => {
                self.max_seq = Some(sequence);
                self.ext_max = i64::from(sequence);
                self.window_base = Some(self.ext_max - 1);
                self.warming_up = true;
            }
            Some(m) => {
                let d = i64::from(sequence.wrapping_sub(m) as i16);
                let jump =
                    i64::from(sequence.wrapping_sub(m)).min(i64::from(m.wrapping_sub(sequence)));
                if jump > MAX_JUMP {
                    // A new start: this packet counts as the next one expected.
                    self.ext_max = self.ext_max.saturating_add(1);
                    self.max_seq = Some(sequence);
                } else if d > 0 {
                    self.ext_max = self.ext_max.saturating_add(d);
                    self.max_seq = Some(sequence);
                }
            }
        }
        self.window_received = self.window_received.saturating_add(1);
        // A clock that went back (wrapped) starts the delay reference again.
        if self.origin.is_some_and(|(t0, _)| arrival_ms < t0) {
            self.origin = None;
            self.base.clear();
            self.running_min = None;
        }
        let (t0, ts0) = *self.origin.get_or_insert((arrival_ms, timestamp));
        let khz = i64::from((clock_hz / 1000).max(1));
        // Relative one-way delay: arrival progress minus media-time progress (wrapping RTP
        // timestamps read as a signed difference).
        let media_ms = i64::from(timestamp.wrapping_sub(ts0) as i32) / khz;
        let arrived_ms = arrival_ms.saturating_sub(t0).min(i64::MAX as u64) as i64;
        let delay = arrived_ms.saturating_sub(media_ms);
        self.running_min = Some(self.running_min.map_or(delay, |m| m.min(delay)));
        self.delay_sum = self.delay_sum.saturating_add(delay);
        self.delay_count += 1;
    }

    /// Ends the measurement interval if it is due (the first call only starts one). Returns
    /// the interval's figures when one ended.
    pub fn update(&mut self, now_ms: u64) -> Option<Interval> {
        let Some(start) = self.window_start else {
            self.window_start = Some(now_ms);
            return None;
        };
        let span = now_ms.saturating_sub(start);
        if span < INTERVAL_MS {
            return None;
        }
        // RFC 3550 §6.4.1 style: lost of expected, scaled to 256.
        let fraction_lost = match self.window_base {
            Some(b) if self.ext_max > b => {
                let expected = self.ext_max - b;
                let lost = (expected - self.window_received).max(0);
                (lost.saturating_mul(256) / expected).min(255) as u8
            }
            _ => 0,
        };
        if self.max_seq.is_some() {
            self.window_base = Some(self.ext_max);
        }
        self.window_received = 0;
        let received_bps = (self.window_octets.saturating_mul(8_000) / span.max(1))
            .min(u64::from(u32::MAX)) as u32;
        let base = self
            .base
            .iter()
            .copied()
            .chain(self.running_min)
            .min()
            .unwrap_or(0);
        let queueing_ms = if self.delay_count > 0 {
            (self.delay_sum / self.delay_count).saturating_sub(base)
        } else {
            0
        };
        if let Some(m) = self.running_min.take() {
            if self.base.len() >= BASE_WINDOWS {
                self.base.remove(0);
            }
            self.base.push(m);
        }
        self.window_start = Some(now_ms);
        self.window_octets = 0;
        self.delay_sum = 0;
        self.delay_count = 0;

        self.delay_strikes = if queueing_ms >= DELAY_CONGESTED_MS {
            self.delay_strikes.saturating_add(1)
        } else {
            0
        };
        let delay_bits = if self.delay_strikes >= DELAY_STRIKES {
            CONGESTED_DELAY
        } else if queueing_ms <= DELAY_CLEAN_MS {
            UNCONGESTED_DELAY
        } else {
            self.congestion & (CONGESTED_DELAY | UNCONGESTED_DELAY)
        };
        let judged_loss = if std::mem::take(&mut self.warming_up) {
            0
        } else {
            fraction_lost
        };
        let loss_bits = if judged_loss >= LOSS_CONGESTED {
            CONGESTED_LOSS
        } else if judged_loss <= LOSS_CLEAN {
            UNCONGESTED_LOSS
        } else {
            self.congestion & (CONGESTED_LOSS | UNCONGESTED_LOSS)
        };
        let bits = delay_bits | loss_bits;
        if bits != self.congestion {
            self.congestion = bits;
            self.changed = true;
        }
        if bits & (CONGESTED_DELAY | CONGESTED_LOSS) != 0 {
            // Multiplicative decrease below what actually got through.
            let base = self.estimate.min(received_bps.max(FLOOR_BPS));
            self.estimate = (base / 20 * 17).max(FLOOR_BPS);
            // A measured limit: high confidence.
            self.confidence = 12;
        } else if bits == UNCONGESTED_DELAY | UNCONGESTED_LOSS {
            // Additive-like increase (5 % per interval), and never below what a clean path just
            // carried (with 20 % headroom); at most the capacity.
            self.estimate = self
                .estimate
                .saturating_add(self.estimate / 20)
                .max(received_bps.saturating_add(received_bps / 5))
                .min(self.capacity);
            if received_bps > 0 {
                self.confidence = self.confidence.max(6);
            }
        }
        Some(Interval {
            received_bps,
            queueing_ms,
            fraction_lost,
        })
    }

    pub fn estimate(&self) -> u32 {
        self.estimate
    }

    pub fn confidence(&self) -> u8 {
        self.confidence
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// The estimate as profile extension (16-octet form with the confidence) about `ssrc`.
    pub fn extension(&self, ssrc: u32) -> rtp::Extension {
        rtp::Extension::Bandwidth {
            ssrc,
            bps: self.estimate,
            confidence: Some(self.confidence),
        }
    }

    /// The congestion notification if the state changed since the last call (§2.2.11.11;
    /// example §4.11: the receiver notifies the sender of changes).
    pub fn take_congestion(&mut self, ntp: u64) -> Option<rtp::Extension> {
        if !std::mem::take(&mut self.changed) {
            return None;
        }
        Some(rtp::Extension::Congestion {
            ntp,
            info: self.congestion,
        })
    }

    /// Whether the congestion state changed and is not yet reported.
    pub fn congestion_changed(&self) -> bool {
        self.changed
    }

    /// Whether the last interval found congestion.
    pub fn congested(&self) -> bool {
        self.congestion & (CONGESTED_DELAY | CONGESTED_LOSS) != 0
    }
}

/// The sender's side: what the peer tells about our stream, turned into a target bit rate.
#[derive(Debug, Clone, Default)]
pub struct SendRate {
    /// The peer's bandwidth estimate for our SSRC (positive values only).
    estimate: Option<u32>,
    receiver_limit: Option<u32>,
    policy: Option<u32>,
    turn: Option<u32>,
    modality_limit: Option<u32>,
    /// Back-off in percent after congestion notifications, recovering over time.
    scale: Option<(u32, u64)>,
}

/// Percent of the target taken off per congestion notification and recovered per second
/// (design values).
const BACKOFF_PERCENT: u32 = 70;
const RECOVER_PERCENT_PER_S: u64 = 5;
const MIN_SCALE: u32 = 25;

impl SendRate {
    /// One profile extension from the peer's report; `ours`: our SSRC. Returns true if the
    /// peer gave a positive estimate for us (ends peer info exchange, §3.2).
    pub fn on_extension(&mut self, e: &rtp::Extension, ours: u32, now_ms: u64) -> bool {
        match *e {
            rtp::Extension::Bandwidth { ssrc, bps, .. } if ssrc == ours => {
                // Special values (−3, −5, −6) are not estimates.
                if bps > 0 && bps < rtp::BANDWIDTH_SEND_TRAIN {
                    self.estimate = Some(bps);
                    return true;
                }
            }
            rtp::Extension::ReceiverLimit(b) if b > 0 => self.receiver_limit = Some(b),
            rtp::Extension::PolicyBandwidth(b) if b > 0 => self.policy = Some(b),
            rtp::Extension::TurnBandwidth(b) if b > 0 => self.turn = Some(b),
            rtp::Extension::ModalityLimit { modality: 2, bps } if bps > 0 => {
                self.modality_limit = Some(bps)
            }
            rtp::Extension::Congestion { info, .. }
                if info & (CONGESTED_DELAY | CONGESTED_LOSS) != 0 =>
            {
                let now = self.scale_at(now_ms);
                self.scale = Some(((now * BACKOFF_PERCENT / 100).max(MIN_SCALE), now_ms));
            }
            _ => {}
        }
        false
    }

    fn scale_at(&self, now_ms: u64) -> u32 {
        match self.scale {
            None => 100,
            Some((p, at)) => {
                let back = now_ms.saturating_sub(at) / 1000 * RECOVER_PERCENT_PER_S;
                (u64::from(p).saturating_add(back)).min(100) as u32
            }
        }
    }

    /// The bit rate to send at most, or `None` while the peer told us nothing.
    pub fn target(&self, now_ms: u64) -> Option<u32> {
        let limit = [
            self.estimate,
            self.receiver_limit,
            self.policy,
            self.turn,
            self.modality_limit,
        ]
        .into_iter()
        .flatten()
        .min()?;
        let scaled = u64::from(limit) * u64::from(self.scale_at(now_ms)) / 100;
        Some((scaled.min(u64::from(u32::MAX)) as u32).max(FLOOR_BPS))
    }
}

/// One resolution step: size and the payload bit rates it needs and uses (BW-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub width: u32,
    pub height: u32,
    /// Below this bit rate a client switches to the next lower resolution.
    pub min_bps: u32,
    /// The bit rate at the highest frame rate.
    pub max_bps: u32,
}

const fn step(width: u32, height: u32, min_kbps: u32, max_kbps: u32) -> Step {
    Step {
        width,
        height,
        min_bps: min_kbps * 1000,
        max_bps: max_kbps * 1000,
    }
}

/// The H.264 resolutions with their minimum and maximum payload bit rates, largest first
/// (BW-17: Microsoft Learn, "Plan network requirements for Skype for Business", table "Video
/// Resolution Bandwidth"; 16:9 and 4:3 sizes of a row share its rates).
pub const STEPS: [Step; 13] = [
    step(1920, 1080, 1500, 4000),
    step(1280, 720, 700, 2500),
    step(960, 540, 500, 2000),
    step(848, 480, 400, 1500),
    step(640, 480, 300, 800),
    step(640, 360, 300, 800),
    step(480, 270, 200, 450),
    step(424, 320, 200, 450),
    step(424, 240, 100, 350),
    step(320, 240, 100, 350),
    step(320, 180, 15, 250),
    step(212, 160, 15, 250),
    // Below the table (our own smallest size): the same rates as 320x180.
    step(160, 120, 15, 250),
];

/// The step of a size, if it is one of [`STEPS`].
pub fn step_of(width: u32, height: u32) -> Option<Step> {
    STEPS
        .iter()
        .copied()
        .find(|s| s.width == width && s.height == height)
}
