//! The video core of a call (function 28): H.264 UC access units to SRTP packets and back
//! ([MS-H264PF] through `commscraft-h264uc`, PL-01 … PL-09), RTCP sender/receiver reports and
//! the video feedback of [MS-RTP] §2.2.12 (PLI, VSR; RT-25, RT-26). Sans-I/O like [`Audio`]:
//! the caller moves packets and the clock.
//!
//! Security by design: our SSRC comes from our declared range and the peer's packets are taken
//! only from its declared range (SD-64); SRTP authentication comes before any parsing; a lost or
//! malformed access unit is never decoded but answered with a picture loss indication; RTP
//! sequence number 0 is never sent (note <2>).
//!
//! [`Audio`]: crate::Audio

use commscraft_h264uc as uc;
use commscraft_rtp as rtp;
use commscraft_sdp as sdp;
use commscraft_srtp as srtp;
use rtp::feedback::{self, Pli, Retransmit, Vsr, VsrEntry};

use crate::rate::{Estimator, SendRate};
use crate::{Error, PAIR_FAST_COUNT, PAIR_FAST_MS, RTCP_INTERVAL_MS};

/// The receiver-side limit is repeated in this many reports after a change (§3.2: "at least
/// five times").
const LIMIT_REPEATS: u8 = 5;
/// Packet loss notifications at most this often (§2.2.11.2).
const LOSS_NOTE_MS: u64 = 500;
/// Picture update requests through signalling at most this often (design value, like the
/// PLI schedule of RT-26 not more than about once a second).
const PICTURE_UPDATE_MS: u64 = 1000;
/// A new target is reported to the encoder when it moved by this percentage (design value).
const TARGET_STEP_PERCENT: u64 = 10;

/// The peer's stream moves to another SSRC of its range only after this much silence on the
/// current one (design: above the gap between two pictures at 7.5 fps).
const SSRC_SWITCH_MS: u64 = 200;

/// Largest RTP payload of video (design: room for SRTP, TURN and IP headers in 1500 bytes).
pub const VIDEO_MTU: usize = 1200;

/// What the video core reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoEvent {
    /// A complete access unit for the decoder.
    AccessUnit { data: Vec<u8>, idr: bool },
    /// The peer asks for a key frame (PLI, or a VSR with the key-frame flag).
    KeyFrameRequested,
    /// The peer asks for video with these parameters (VSR; `SOURCE_NONE`: stop sending).
    SourceRequested { msi: u32, entries: Vec<VsrEntry> },
    /// The peer ended its stream (RTCP BYE).
    Bye,
    /// What the path to the peer carries now (RT-30): send at most this bit rate.
    SendRate(u32),
    /// A picture was lost but the peer takes no PLI: ask for a full picture through signalling
    /// instead (SIP INFO `picture_fast_update`, function 31, PL-50).
    PictureUpdateNeeded,
}

/// NAL unit type of an IDR slice.
const NAL_IDR: u8 = 5;
/// Access units below this number of packets get no FEC (design).
const FEC_MIN_PACKETS: usize = 2;
/// Reported loss from which ordinary pictures get FEC, of 256 (design: about 1 %).
const FEC_LOSS: u8 = 3;
/// For how many reports after that (design: reports come every few seconds).
const FEC_REPORTS: u8 = 5;

/// A further simulcast stream we send (MS-H264PF §3.1.5.3, p. 23): its own priority ID and
/// SSRC, sequence numbers and packetizer; the first stream (priority ID 0) is the one of
/// [`Video`] itself.
struct Layer {
    prid: u8,
    sender: rtp::Sender,
    packetizer: uc::Packetizer,
}

/// The video stream of one call.
pub struct Video {
    pt: u8,
    tx_srtp: Option<srtp::Context>,
    rx_srtp: Option<srtp::Context>,
    sender: rtp::Sender,
    receiver: rtp::Receiver,
    remote_range: (u32, u32),
    remote_ssrc: Option<u32>,
    /// When the last packet of that SSRC arrived.
    remote_seen_ms: u64,
    packetizer: uc::Packetizer,
    /// Further simulcast streams (priority ID ≥ 1), from the stream layout.
    layers: Vec<Layer>,
    our_range: (u32, u32),
    random: [u8; 10],
    depacketizer: uc::Depacketizer,
    /// The payload type of FEC, if negotiated (PL-06, SD-28).
    fec_pt: Option<u8>,
    recovery: uc::fec::Recovery,
    /// Reports of the peer for which we go on protecting ordinary pictures after it reported loss.
    fec_reports: u8,
    cname: String,
    next_rtcp: u64,
    last_sr: Option<(u32, u64)>,
    vsr: Option<(Vsr, Retransmit)>,
    pli: Option<(Pli, Retransmit)>,
    next_request_id: u16,
    /// Whether the peer accepts our feedback (its `recv:` capabilities, SD-63).
    peer_accepts: sdp::video::Feedback,
    /// Our view of the incoming stream (RT-28, RT-29) and the peer's view of ours (RT-30).
    estimator: Estimator,
    send_rate: SendRate,
    reported_target: Option<u32>,
    /// Peer info until the peer reported a positive estimate for us (§3.2).
    peer_estimated: bool,
    limit_left: u8,
    /// Packet pair fast rate (§3.2.2): started once by a report about our stream.
    fast_started: bool,
    fast_left: u32,
    /// The lost packet to report and when the last notification went out.
    lost_seq: Option<u16>,
    last_loss_note: Option<u64>,
    /// When we last asked for a picture through signalling (no PLI negotiated).
    last_picture_update: Option<u64>,
    expected_seq: Option<u16>,
    pub sent_units: u64,
    pub received_units: u64,
    pub lost_units: u64,
}

impl std::fmt::Debug for Video {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Video")
            .field("sent_units", &self.sent_units)
            .field("received_units", &self.received_units)
            .field("lost_units", &self.lost_units)
            .finish_non_exhaustive()
    }
}

impl Video {
    /// `our_range`: the SSRC range we declared (our SSRC is its start); `layout`: the stream
    /// layout of what we send (one base layer, PRID 0); `random` from a CSPRNG.
    pub fn new(
        negotiated: &sdp::video::NegotiatedVideo,
        our_key: Option<&sdp::Key>,
        our_range: (u32, u32),
        layout: uc::StreamLayout,
        random: [u8; 10],
        cname: &str,
    ) -> Result<Video, Error> {
        let (tx_srtp, rx_srtp) = match (our_key, negotiated.remote_key.as_ref()) {
            (Some(o), Some(r)) => (
                Some(srtp::Context::new(&o.key, &o.salt, o.mki)),
                Some(srtp::Context::new(&r.key, &r.salt, r.mki)),
            ),
            (None, None) => (None, None),
            _ => return Err(Error::Video("only one side encrypts")),
        };
        let mut sender = rtp::Sender::new(random);
        sender.ssrc = our_range.0.max(1);
        sender.skip_sequence_zero();
        let packetizer =
            uc::Packetizer::new(VIDEO_MTU, 0, layout).map_err(|_| Error::Video("stream layout"))?;
        Ok(Video {
            pt: negotiated.payload_type,
            tx_srtp,
            rx_srtp,
            sender,
            receiver: rtp::Receiver::default(),
            remote_range: negotiated.ssrc_range,
            remote_ssrc: None,
            remote_seen_ms: 0,
            packetizer,
            layers: Vec::new(),
            our_range,
            random,
            depacketizer: uc::Depacketizer::new(),
            fec_pt: negotiated
                .fec_payload_type
                .filter(|f| *f != negotiated.payload_type),
            recovery: uc::fec::Recovery::default(),
            fec_reports: 0,
            cname: cname
                .chars()
                .filter(|c| c.is_ascii_graphic())
                .take(64)
                .collect(),
            next_rtcp: 0,
            last_sr: None,
            vsr: None,
            pli: None,
            next_request_id: 1,
            peer_accepts: negotiated.feedback,
            estimator: Estimator::new(sdp::video::MEDIABW_KBPS.saturating_mul(1000)),
            send_rate: SendRate::default(),
            reported_target: None,
            peer_estimated: false,
            limit_left: LIMIT_REPEATS,
            fast_started: false,
            fast_left: 0,
            lost_seq: None,
            last_loss_note: None,
            last_picture_update: None,
            expected_seq: None,
            sent_units: 0,
            received_units: 0,
            lost_units: 0,
        })
    }

    /// A new stream layout of what we send (another size after a video source request, or
    /// another set of simulcast streams): the next access unit of every stream carries the full
    /// layout (§2.2.5.1: on the IDR that should follow). Priority ID 0 must be in it; every other
    /// priority ID becomes a stream of its own with the SSRC `range start + PRID` (inference:
    /// MS-RTP §3.1 lets related streams share the session by SSRC, MS-H264PF tells them apart by
    /// PRID, neither says which SSRC a stream takes), streams no longer in it end.
    pub fn set_layout(&mut self, layout: uc::StreamLayout) -> Result<(), Error> {
        let bad = Error::Video("stream layout");
        let span = self.our_range.1.saturating_sub(self.our_range.0);
        if layout.layers.iter().any(|l| u32::from(l.prid) > span) {
            return Err(bad);
        }
        self.packetizer
            .set_layout(layout.clone())
            .map_err(|_| bad)?;
        self.layers.retain(|l| layout.layer(l.prid).is_some());
        for l in &mut self.layers {
            l.packetizer.set_layout(layout.clone()).map_err(|_| bad)?;
        }
        let mut prids: Vec<u8> = layout
            .layers
            .iter()
            .map(|l| l.prid)
            .filter(|p| *p != 0)
            .collect();
        prids.sort_unstable();
        for prid in prids {
            if self.layers.iter().any(|l| l.prid == prid) {
                continue;
            }
            // Its own start of the sequence numbers (MS-SRTP §5.1: not close to 65535).
            let mut random = self.random;
            for (i, b) in random.iter_mut().enumerate().skip(4).take(2) {
                *b ^= prid.wrapping_mul(0x5b).wrapping_add(i as u8);
            }
            let mut sender = rtp::Sender::new(random);
            sender.ssrc = self.sender.ssrc.saturating_add(u32::from(prid));
            sender.skip_sequence_zero();
            let packetizer =
                uc::Packetizer::new(VIDEO_MTU, prid, layout.clone()).map_err(|_| bad)?;
            self.layers.push(Layer {
                prid,
                sender,
                packetizer,
            });
        }
        self.layers.sort_by_key(|l| l.prid);
        Ok(())
    }

    /// The priority IDs of the streams we send now.
    pub fn prids(&self) -> Vec<u8> {
        std::iter::once(0)
            .chain(self.layers.iter().map(|l| l.prid))
            .collect()
    }

    fn is_ours(&self, ssrc: u32) -> bool {
        ssrc == self.sender.ssrc || self.layers.iter().any(|l| l.sender.ssrc == ssrc)
    }

    /// The most we can receive (bit/s): the start value of our estimate and our receiver-side
    /// limit (default: our `x-mediabw` receive value).
    pub fn set_receive_capacity(&mut self, bps: u32) {
        self.estimator = Estimator::new(bps);
        self.limit_left = LIMIT_REPEATS;
    }

    /// Our current estimate of the incoming path (bit/s).
    pub fn receive_estimate(&self) -> u32 {
        self.estimator.estimate()
    }

    /// The bit rate the peer's reports allow us to send, if it reported anything.
    pub fn send_target(&self, now_ms: u64) -> Option<u32> {
        self.send_rate.target(now_ms)
    }

    /// Our SSRC (the start of our range).
    pub fn ssrc(&self) -> u32 {
        self.sender.ssrc
    }

    /// One access unit (Annex B) captured at `timestamp` (90 kHz) → the protected RTP packets.
    pub fn on_access_unit(&mut self, au: &[u8], timestamp: u32) -> Vec<Vec<u8>> {
        self.on_layer_access_unit(0, au, timestamp)
    }

    /// One access unit of the simulcast stream with priority ID `prid`; nothing for a stream
    /// that is not in the stream layout (§3.1.5.3: no NAL unit of a removed layer).
    pub fn on_layer_access_unit(&mut self, prid: u8, au: &[u8], timestamp: u32) -> Vec<Vec<u8>> {
        // FEC costs at least one packet per access unit, so it is sent where it is worth that
        // (design): always for IDR pictures, whose loss costs the most, otherwise after the peer
        // reported loss; never for a single packet, whose copy would help little more.
        let idr = uc::split_annex_b(au)
            .is_ok_and(|nals| nals.iter().any(|n| uc::nal_type(n) == Some(NAL_IDR)));
        let fec = self.fec_pt.filter(|_| idr || self.fec_reports > 0);
        let (sender, packetizer) = if prid == 0 {
            (&mut self.sender, &mut self.packetizer)
        } else {
            match self.layers.iter_mut().find(|l| l.prid == prid) {
                Some(l) => (&mut l.sender, &mut l.packetizer),
                None => return Vec::new(),
            }
        };
        let out = send_unit(
            sender,
            packetizer,
            &mut self.tx_srtp,
            self.pt,
            fec,
            au,
            timestamp,
        );
        if prid == 0 && !out.is_empty() {
            self.sent_units += 1;
        }
        out
    }

    /// A received RTP/SRTP packet; `arrival` in 90 kHz ticks. A lost access unit asks the
    /// peer for a key frame at once (PLI).
    pub fn on_rtp(&mut self, packet: &[u8], arrival: u32, now_ms: u64) -> Vec<VideoEvent> {
        if packet.len() > rtp::MAX_PACKET {
            return Vec::new();
        }
        let plain = match self.rx_srtp.as_mut() {
            Some(c) => match c.unprotect_rtp(packet) {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            },
            None => packet.to_vec(),
        };
        let Ok(parsed) = rtp::parse(&plain) else {
            return Vec::new();
        };
        let h = parsed.header;
        // SD-64: only SSRCs from the peer's range (MS-RTP §3.1, p. 37: with a range set, SSRC
        // throttling is off and anything outside it is dropped).
        let is_fec = self.fec_pt == Some(h.payload_type);
        if h.ssrc < self.remote_range.0
            || h.ssrc > self.remote_range.1
            || (h.payload_type != self.pt && !is_fec)
        {
            return Vec::new();
        }
        // Within the range the stream may change its SSRC: an A/V MCU forwards another simulcast
        // stream or source from another one (MS-H264PF §3.1.5.3: it forwards one of the streams). The new SSRC takes over once the
        // current one has been silent for a while, so two streams at once do not take turns
        // (design).
        match self.remote_ssrc {
            None => self.remote_ssrc = Some(h.ssrc),
            Some(s) if s != h.ssrc => {
                if now_ms.saturating_sub(self.remote_seen_ms) < SSRC_SWITCH_MS {
                    return Vec::new();
                }
                self.follow_ssrc(h.ssrc, now_ms);
            }
            _ => {}
        }
        self.remote_seen_ms = now_ms;
        if !self.receiver.on_packet(&h, arrival, true) {
            return Vec::new();
        }
        self.estimator
            .on_packet(packet.len(), h.sequence, h.timestamp, 90_000, now_ms);
        // A gap in the sequence numbers: report the first missing packet (§2.2.11.2).
        if let Some(e) = self.expected_seq {
            let ahead = h.sequence.wrapping_sub(e);
            if ahead > 0 && ahead < 0x8000 {
                self.lost_seq = Some(e);
            }
        }
        if self
            .expected_seq
            .is_none_or(|e| h.sequence.wrapping_sub(e) < 0x8000)
        {
            self.expected_seq = Some(h.sequence.wrapping_add(1));
        }
        let packets = if is_fec {
            self.recovery
                .push_fec(h.sequence, h.timestamp, parsed.payload)
        } else if self.fec_pt.is_some() {
            self.recovery.push(uc::fec::Media {
                seq: h.sequence,
                timestamp: h.timestamp,
                marker: h.marker,
                payload_type: h.payload_type,
                payload: parsed.payload.to_vec(),
            })
        } else {
            vec![uc::fec::Media {
                seq: h.sequence,
                timestamp: h.timestamp,
                marker: h.marker,
                payload_type: h.payload_type,
                payload: parsed.payload.to_vec(),
            }]
        };
        let mut events = Vec::new();
        // A recovered packet can claim another format: it counts as lost then.
        let pt = self.pt;
        for p in packets.into_iter().filter(|p| p.payload_type == pt) {
            let outputs = self.depacketizer.push(uc::RtpIn {
                seq: p.seq,
                timestamp: p.timestamp,
                marker: p.marker,
                payload: &p.payload,
            });
            for out in outputs {
                match out {
                    uc::Output::AccessUnit { data, idr, .. } => {
                        self.received_units += 1;
                        events.push(VideoEvent::AccessUnit { data, idr });
                    }
                    uc::Output::Lost { .. } => {
                        self.lost_units += 1;
                        self.request_key_frame(now_ms);
                        if !self.peer_accepts.recv_pli
                            && self
                                .last_picture_update
                                .is_none_or(|t| now_ms.saturating_sub(t) >= PICTURE_UPDATE_MS)
                        {
                            self.last_picture_update = Some(now_ms);
                            events.push(VideoEvent::PictureUpdateNeeded);
                        }
                    }
                }
            }
        }
        events
    }

    /// Another SSRC of the peer's range is the stream now: its own sequence numbers, packets and
    /// access units start afresh, and a key frame is asked for at once (the new stream may
    /// start between two of them).
    fn follow_ssrc(&mut self, ssrc: u32, now_ms: u64) {
        self.remote_ssrc = Some(ssrc);
        self.receiver = rtp::Receiver::default();
        self.depacketizer.restart();
        let recovered = self.recovery.recovered;
        self.recovery = uc::fec::Recovery::default();
        self.recovery.recovered = recovered;
        self.expected_seq = None;
        self.lost_seq = None;
        self.pli = None;
        self.request_key_frame(now_ms);
    }

    fn next_id(&mut self) -> u16 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        id
    }

    /// Packets recovered by FEC so far.
    pub fn recovered_packets(&self) -> u64 {
        self.recovery.recovered
    }

    /// The SSRC the feedback is about: the peer's stream, or before it sends the start of its
    /// range (inference, O-RT-1).
    fn media_ssrc(&self) -> u32 {
        self.remote_ssrc.unwrap_or(self.remote_range.0)
    }

    /// The sync frame request bits of a PLI (§2.2.12.1, p. 27: bit n = priority ID n): the
    /// stream that arrives; not known yet, every stream of the sender's layout – a mixer
    /// forwards one of them (MS-H264PF §3.1.5.3), maybe not the one with priority ID 0.
    fn sync_frame_bits(&self) -> u64 {
        match (self.depacketizer.prid(), self.depacketizer.layout()) {
            (Some(p), _) if p < 64 => 1 << p,
            (_, Some(l)) if l.present != 0 => l.present,
            _ => 1,
        }
    }

    /// Ask the peer for a key frame (extended PLI with a new request ID for the stream that
    /// arrives), unless one is still being repeated or the peer does not accept PLI.
    pub fn request_key_frame(&mut self, now_ms: u64) {
        if !self.peer_accepts.recv_pli || self.pli.as_ref().is_some_and(|(_, r)| !r.finished()) {
            return;
        }
        let id = self.next_id();
        self.pli = Some((
            Pli {
                sender_ssrc: self.sender.ssrc,
                media_ssrc: self.media_ssrc(),
                extended: Some((id, self.sync_frame_bits())),
            },
            Retransmit::new(now_ms, false),
        ));
    }

    /// Ask the peer for video (VSR for the source `msi`, e.g. `SOURCE_ANY` in a 1:1 call) with
    /// the given parameters and a key frame; a new request replaces the old one.
    pub fn request_source(&mut self, msi: u32, entry: VsrEntry, now_ms: u64) {
        if !self.peer_accepts.recv_src {
            return;
        }
        let id = self.next_id();
        self.vsr = Some((
            Vsr {
                sender_ssrc: self.sender.ssrc,
                // O-RT-1: the specification leaves the media source field open; it carries the
                // requested MSI, the source the request is about (inference).
                media_ssrc: msi,
                msi,
                request_id: id,
                key_frame: true,
                entries: vec![entry],
            },
            Retransmit::new(now_ms, true),
        ));
    }

    fn protect_rtcp(&mut self, p: &[u8]) -> Option<Vec<u8>> {
        match self.tx_srtp.as_mut() {
            Some(c) => c.protect_rtcp(p).ok(),
            None => Some(p.to_vec()),
        }
    }

    /// The RTCP packets due now: feedback being repeated (each on its own, reduced size) and
    /// every 5 s a sender report with our reception report.
    pub fn rtcp_due(&mut self, now_ms: u64, ntp: u64) -> Vec<Vec<u8>> {
        let mut plain = Vec::new();
        if let Some((p, r)) = self.pli.as_mut() {
            if r.due(now_ms) {
                if let Ok(b) = p.to_bytes() {
                    plain.push(b);
                }
            }
        }
        if let Some((v, r)) = self.vsr.as_mut() {
            if r.due(now_ms) {
                if let Ok(b) = v.to_bytes() {
                    plain.push(b);
                }
            }
        }
        // One measurement interval per second; a change of the congestion state or a lost packet
        // is reported at once, not with the next regular report (§4.11; ≤ every 500 ms).
        self.estimator.update(now_ms);
        let loss_due = self.lost_seq.is_some()
            && self
                .last_loss_note
                .is_none_or(|t| now_ms.saturating_sub(t) >= LOSS_NOTE_MS);
        let early = loss_due || self.estimator.congestion_changed();
        if now_ms >= self.next_rtcp || early {
            self.next_rtcp = now_ms
                + if self.fast_left > 0 {
                    self.fast_left -= 1;
                    PAIR_FAST_MS
                } else {
                    RTCP_INTERVAL_MS
                };
            let (lsr, dlsr) = self.last_sr.map_or((0, 0), |(mid, at)| {
                (
                    mid,
                    (now_ms.saturating_sub(at).saturating_mul(65536) / 1000)
                        .min(u64::from(u32::MAX)) as u32,
                )
            });
            let reports: Vec<rtp::ReportBlock> = self
                .remote_ssrc
                .map(|s| vec![self.receiver.report(s, lsr, dlsr)])
                .unwrap_or_default();
            // MS-RTP §2.2.8: one sender report, from the lowest SSRC, counting all streams.
            let info = rtp::SenderInfo {
                ntp,
                rtp_timestamp: self.sender.timestamp(),
                packets: self
                    .layers
                    .iter()
                    .fold(self.sender.packets, |n, l| n.wrapping_add(l.sender.packets)),
                octets: self
                    .layers
                    .iter()
                    .fold(self.sender.octets, |n, l| n.wrapping_add(l.sender.octets)),
            };
            // RT-28: our estimate of the peer's stream in every report – also before it sends,
            // so the peer (e.g. an A/V MCU) knows what it may send.
            let mut extensions = vec![self.estimator.extension(self.media_ssrc())];
            if self.limit_left > 0 {
                self.limit_left -= 1;
                extensions.push(rtp::Extension::ReceiverLimit(self.estimator.capacity()));
            }
            if !self.peer_estimated {
                extensions.push(rtp::Extension::PeerInfo {
                    ssrc: self.sender.ssrc,
                    inbound: self.estimator.capacity(),
                    outbound: self.estimator.capacity(),
                    no_cache: true,
                });
            }
            if let Some(c) = self.estimator.take_congestion(ntp) {
                extensions.push(c);
            }
            if loss_due {
                if let Some(seq) = self.lost_seq.take() {
                    extensions.push(rtp::Extension::PacketLoss { sequence: seq });
                    self.last_loss_note = Some(now_ms);
                }
            }
            // Packet pair (§3.2.6): a probe right before the report.
            plain.push(rtp::probe(self.sender.ssrc, info));
            plain.push(
                rtp::Report {
                    ssrc: self.sender.ssrc,
                    info: Some(info),
                    reports: &reports,
                    extensions: &extensions,
                    cname: &self.cname,
                    quality: None,
                }
                .build(),
            );
        }
        plain.iter().filter_map(|p| self.protect_rtcp(p)).collect()
    }

    /// A received RTCP packet: reports, BYE and the peer's feedback about our stream.
    pub fn on_rtcp(&mut self, packet: &[u8], now_ms: u64) -> Vec<VideoEvent> {
        let plain = match self.rx_srtp.as_mut() {
            Some(c) => match c.unprotect_rtcp(packet) {
                Ok(p) => p,
                Err(_) => return Vec::new(),
            },
            None => packet.to_vec(),
        };
        let Ok(items) = rtp::parse_rtcp(&plain) else {
            return Vec::new();
        };
        let (lo, hi) = self.remote_range;
        let in_range = |s: u32| s >= lo && s <= hi;
        let mut events = Vec::new();
        for i in items {
            match i {
                rtp::Rtcp::SenderReport {
                    ssrc,
                    info,
                    reports,
                    extensions,
                } if in_range(ssrc) => {
                    self.last_sr = Some(((info.ntp >> 16) as u32, now_ms));
                    self.on_peer_report(&reports, &extensions, now_ms, &mut events);
                }
                rtp::Rtcp::ReceiverReport {
                    ssrc,
                    reports,
                    extensions,
                } if in_range(ssrc) => {
                    self.on_peer_report(&reports, &extensions, now_ms, &mut events);
                }
                rtp::Rtcp::Bye { sources } if sources.iter().any(|s| in_range(*s)) => {
                    events.push(VideoEvent::Bye)
                }
                rtp::Rtcp::Feedback(feedback::Feedback::Pli(p))
                    if in_range(p.sender_ssrc) && self.is_ours(p.media_ssrc) =>
                {
                    events.push(VideoEvent::KeyFrameRequested)
                }
                rtp::Rtcp::Feedback(feedback::Feedback::Vsr(v)) if in_range(v.sender_ssrc) => {
                    if v.key_frame {
                        events.push(VideoEvent::KeyFrameRequested);
                    }
                    events.push(VideoEvent::SourceRequested {
                        msi: v.msi,
                        entries: v.entries,
                    });
                }
                _ => {}
            }
        }
        events
    }

    /// The peer's report: a block about our stream starts the packet pair fast rate once
    /// (§3.2.5); its extensions feed our send rate (RT-30); a packet loss notification asks for
    /// a recovery (§3.2.5: e.g. an I-frame).
    fn on_peer_report(
        &mut self,
        reports: &[rtp::ReportBlock],
        extensions: &[rtp::Extension],
        now_ms: u64,
        events: &mut Vec<VideoEvent>,
    ) {
        // The worst of our streams decides.
        let ours = reports
            .iter()
            .filter(|r| self.is_ours(r.ssrc))
            .map(|r| r.fraction_lost)
            .max();
        if let Some(lost) = ours {
            self.fec_reports = if lost >= FEC_LOSS {
                FEC_REPORTS
            } else {
                self.fec_reports.saturating_sub(1)
            };
        }
        if !self.fast_started && reports.iter().any(|r| r.ssrc == self.sender.ssrc) {
            self.fast_started = true;
            self.fast_left = PAIR_FAST_COUNT;
            self.next_rtcp = self.next_rtcp.min(now_ms + PAIR_FAST_MS);
        }
        for e in extensions {
            if self.send_rate.on_extension(e, self.sender.ssrc, now_ms) {
                // A positive estimate ends peer info and the fast rate (§3.2, §3.2.5).
                self.peer_estimated = true;
                self.fast_left = 0;
            }
            if matches!(e, rtp::Extension::PacketLoss { .. }) {
                events.push(VideoEvent::KeyFrameRequested);
            }
        }
        if let Some(t) = self.send_rate.target(now_ms) {
            let moved = self.reported_target.is_none_or(|r| {
                u64::from(t.abs_diff(r)) * 100 >= u64::from(r) * TARGET_STEP_PERCENT
            });
            if moved {
                self.reported_target = Some(t);
                events.push(VideoEvent::SendRate(t));
            }
        }
    }

    /// RTCP BYE when the call ends.
    pub fn bye(&mut self) -> Option<Vec<u8>> {
        let p = rtp::bye(self.sender.ssrc, &self.cname);
        self.protect_rtcp(&p)
    }
}

fn protect_rtp(srtp: &mut Option<srtp::Context>, packet: Vec<u8>) -> Option<Vec<u8>> {
    match srtp.as_mut() {
        Some(c) => c.protect_rtp(&packet).ok(),
        None => Some(packet),
    }
}

/// The protected RTP packets of one access unit of one stream, with FEC of payload type `fec`
/// behind them (§2.2.8.1.1: on the same SSRC, the last with the marker) when it has enough
/// packets; nothing if the access unit cannot be sent.
fn send_unit(
    sender: &mut rtp::Sender,
    packetizer: &mut uc::Packetizer,
    srtp: &mut Option<srtp::Context>,
    pt: u8,
    fec: Option<u8>,
    au: &[u8],
    timestamp: u32,
) -> Vec<Vec<u8>> {
    let Ok(payloads) = packetizer.packetize(au) else {
        return Vec::new();
    };
    let protect = fec.filter(|_| payloads.len() >= FEC_MIN_PACKETS);
    let mut sources = Vec::new();
    let mut out = Vec::with_capacity(payloads.len());
    for p in payloads {
        sender.skip_sequence_zero();
        if protect.is_some() {
            sources.push(uc::fec::Media {
                seq: sender.sequence(),
                timestamp,
                marker: p.marker,
                payload_type: pt,
                payload: p.data.clone(),
            });
        }
        let Ok(packet) = sender.packet_at(pt, p.marker, &p.data, timestamp) else {
            return Vec::new();
        };
        let Some(protected) = protect_rtp(srtp, packet) else {
            return Vec::new();
        };
        out.push(protected);
    }
    if let Some(fec_pt) = protect {
        let chunks: Vec<&[uc::fec::Media]> = sources.chunks(uc::fec::MAX_PROTECTED).collect();
        let last = chunks.len().saturating_sub(1);
        for (i, chunk) in chunks.into_iter().enumerate() {
            sender.skip_sequence_zero();
            let Ok(fec) = uc::fec::protect(chunk, sender.sequence()) else {
                continue;
            };
            let Ok(packet) = sender.packet_at(fec_pt, i == last, &fec, timestamp) else {
                continue;
            };
            if let Some(protected) = protect_rtp(srtp, packet) {
                out.push(protected);
            }
        }
    }
    out
}
