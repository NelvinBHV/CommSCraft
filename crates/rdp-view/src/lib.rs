//! The RDP viewer of application sharing (function 43): CommSCraft's adapter around IronRDP.
//!
//! The sharer is the RDP server, the viewer the RDP client. The bytes run over the RTP data
//! stream of `media::data` (MS-RTASPF); this crate only turns bytes into bytes and pictures and
//! knows no sockets. The connection sequence is our own ([MS-RDPBCGR] 20250520 §1.3.1.1) with
//! IronRDP's PDU types: X.224 Connection Request with standard RDP security, MCS Connect
//! Initial, Erect Domain, Attach User, Channel Join, Client Info, licensing (expected to end at
//! once), Demand/Confirm Active and the finalization. After that IronRDP's active stage decodes
//! the screen updates into an RGBA image. The Windows sharer draws through the graphics
//! pipeline ([MS-RDPEGFX]) on the dynamic channels (`drdynvc`), not with bitmap updates
//! (`tested-in-lab`): we ask for `drdynvc` as the Windows viewer does and offer the pipeline's
//! capability sets without AVC (H.264), so RemoteFX, planar and ClearCodec are decoded in
//! IronRDP's Rust code.
//!
//! What MS-RTASPF 20250520 requires: encryption level "None" (§1.5, p. 7), no bulk compression
//! between viewer and the sharer's side (§1.5), and no Security Exchange PDU (§3.1.5, p. 9) – so
//! we offer no encryption method and send none. The FriendlyName of [MS-RDPEMC] is the
//! participants' SIP URI (§3.1, p. 9); the sharer sends it on the multiparty channel `encomsp`,
//! which we join and whose data we take without acting on it (inference: viewing needs no
//! control request).
//!
//! Security by design: the input is hostile; PDUs are framed with IronRDP's length check and at
//! most [`MAX_PDU`] bytes, the input buffer is bounded, every decode error ends the session, the
//! screen size is bounded ([`MAX_SIDE`]).

use ironrdp_core::{decode, encode_vec, impl_as_any};
use ironrdp_dvc::DrdynvcClient;
use ironrdp_egfx::client::{GraphicsPipelineClient, GraphicsPipelineHandler};
use ironrdp_egfx::pdu::{
    CapabilitiesV103Flags, CapabilitiesV104Flags, CapabilitiesV107Flags, CapabilitiesV10Flags,
    CapabilitiesV8Flags, CapabilitySet as EgfxCapabilitySet,
};
use ironrdp_graphics::image_processing::PixelFormat;
use ironrdp_pdu::gcc::{
    ChannelDef, ChannelName, ChannelOptions, ClientCoreData, ClientCoreOptionalData,
    ClientEarlyCapabilityFlags, ClientGccBlocks, ClientNetworkData, ClientSecurityData, ColorDepth,
    ConnectionType, EncryptionMethod, HighColorDepth, RdpVersion, SecureAccessSequence,
    SupportedColorDepths,
};
use ironrdp_pdu::mcs;
use ironrdp_pdu::nego::{
    ConnectionConfirm, ConnectionRequest, NegoRequestData, RequestFlags, SecurityProtocol,
};
use ironrdp_pdu::rdp::capability_sets::{
    Bitmap, BitmapCache, BitmapDrawingFlags, Brush, CacheDefinition, CacheEntry, CapabilitySet,
    ClientConfirmActive, DemandActive, General, GeneralExtraFlags, GlyphCache, GlyphSupportLevel,
    Input, InputFlags, MultifragmentUpdate, OffscreenBitmapCache, Order, OrderFlags,
    OrderSupportExFlags, Pointer, SupportLevel, VirtualChannel, VirtualChannelFlags,
    BITMAP_CACHE_ENTRIES_NUM, GLYPH_CACHE_NUM, SERVER_CHANNEL_ID,
};
use ironrdp_pdu::rdp::client_info::{
    AddressFamily, ClientInfo, ClientInfoFlags, CompressionType, Credentials, ExtendedClientInfo,
    ExtendedClientOptionalInfo,
};
use ironrdp_pdu::rdp::finalization_messages::{ControlAction, ControlPdu, FontPdu, SynchronizePdu};
use ironrdp_pdu::rdp::headers::{
    BasicSecurityHeader, BasicSecurityHeaderFlags, ShareControlPdu, ShareDataPdu,
};
use ironrdp_pdu::rdp::ClientInfoPdu;
use ironrdp_pdu::x224::{X224Data, X224};
use ironrdp_pdu::Action;
use ironrdp_session::image::DecodedImage;
use ironrdp_session::{ActiveStage, ActiveStageBuilder, ActiveStageOutput};
use ironrdp_svc::{StaticChannelSet, SvcClientProcessor, SvcMessage, SvcProcessor};
use std::borrow::Cow;

/// Largest RDP PDU taken (design; [MS-RDPBCGR] PDUs are at most 16 bit long, fast-path 15 bit).
pub const MAX_PDU: usize = 64 * 1024;
/// Most bytes waiting for a whole PDU (design).
pub const MAX_BUFFER: usize = 4 * MAX_PDU;
/// Largest screen side taken (design).
pub const MAX_SIDE: u16 = 8192;

/// Why the session ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input broke the protocol (the step it broke).
    Protocol(String),
    /// The sharer refused the connection or ended it.
    Refused(String),
    TooLarge,
}

/// What the viewer reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Bytes to send to the sharer.
    Send(Vec<u8>),
    /// The session is active; the screen has this size.
    Active { width: u16, height: u16 },
    /// The screen changed (inclusive rectangle); read it with [`Viewer::image`].
    Updated {
        left: u16,
        top: u16,
        right: u16,
        bottom: u16,
    },
    /// The sharer ended the session.
    Ended(String),
}

/// Settings of the viewer.
#[derive(Debug, Clone)]
pub struct Config {
    /// The screen size we ask for (the sharer decides).
    pub width: u16,
    pub height: u16,
    /// Our SIP URI (the participant's name, MS-RTASPF §3.1).
    pub sip_uri: String,
    /// Our computer's name (at most 15 characters are sent).
    pub client_name: String,
}

/// The `encomsp` channel: joined, its data taken without answer.
#[derive(Debug, Default)]
struct Encomsp {
    received: usize,
}

impl_as_any!(Encomsp);

impl SvcProcessor for Encomsp {
    fn channel_name(&self) -> ChannelName {
        ChannelName::from_static(b"encomsp\0")
    }

    fn process(&mut self, payload: &[u8]) -> ironrdp_pdu::PduResult<Vec<SvcMessage>> {
        self.received = self.received.saturating_add(payload.len());
        Ok(Vec::new())
    }
}

impl SvcClientProcessor for Encomsp {}

/// The static channels we ask for: `drdynvc` and `encomsp`.
const STATIC_CHANNELS: usize = 2;

/// The graphics pipeline's handler: the capability sets without H.264 (AVC), whose decoder
/// runs only in the sandboxed window process; RemoteFX, planar and ClearCodec are decoded in
/// IronRDP's Rust code. Version 10.1 is left out (it has no flag to switch AVC off), 8.1 too.
struct Gfx;

impl GraphicsPipelineHandler for Gfx {
    fn capabilities(&self) -> Vec<EgfxCapabilitySet> {
        vec![
            EgfxCapabilitySet::V10_7 {
                flags: CapabilitiesV107Flags::SMALL_CACHE | CapabilitiesV107Flags::AVC_DISABLED,
            },
            EgfxCapabilitySet::V10_6 {
                flags: CapabilitiesV104Flags::SMALL_CACHE | CapabilitiesV104Flags::AVC_DISABLED,
            },
            EgfxCapabilitySet::V10_5 {
                flags: CapabilitiesV104Flags::SMALL_CACHE | CapabilitiesV104Flags::AVC_DISABLED,
            },
            EgfxCapabilitySet::V10_4 {
                flags: CapabilitiesV104Flags::SMALL_CACHE | CapabilitiesV104Flags::AVC_DISABLED,
            },
            EgfxCapabilitySet::V10_3 {
                flags: CapabilitiesV103Flags::AVC_DISABLED,
            },
            EgfxCapabilitySet::V10_2 {
                flags: CapabilitiesV10Flags::SMALL_CACHE | CapabilitiesV10Flags::AVC_DISABLED,
            },
            EgfxCapabilitySet::V10 {
                flags: CapabilitiesV10Flags::SMALL_CACHE | CapabilitiesV10Flags::AVC_DISABLED,
            },
            EgfxCapabilitySet::V8 {
                flags: CapabilitiesV8Flags::SMALL_CACHE,
            },
        ]
    }
}

enum State {
    WaitConfirm,
    WaitConnectResponse,
    WaitAttach,
    WaitJoin { remaining: Vec<u16> },
    WaitDemandActive,
    WaitFinalization { share_id: u32, font_map: bool },
    Active(Box<ActiveStage>),
    Ended,
}

/// The RDP viewer: bytes from the sharer in, bytes to the sharer and screen updates out.
pub struct Viewer {
    config: Config,
    state: State,
    buffer: Vec<u8>,
    io_channel: u16,
    user_channel: u16,
    /// The static channels' IDs from the server, in the order we asked for them
    /// ([`STATIC_CHANNELS`]).
    static_channels: Vec<u16>,
    image: Option<DecodedImage>,
}

/// The cookie's name: the first 9 characters of our SIP URI with its scheme, as the Windows
/// viewer sends it (for sip:alice@contoso.com: `Cookie: mstshash=sip:alice`, `tested-in-lab`); printable characters only.
fn cookie_name(sip_uri: &str) -> String {
    let uri = sip_uri.trim();
    let uri = if uri.starts_with("sip:") {
        uri.to_string()
    } else {
        format!("sip:{uri}")
    };
    uri.chars()
        .filter(|c| c.is_ascii_graphic())
        .take(9)
        .collect()
}

fn protocol(step: &str, e: impl std::fmt::Display) -> Error {
    Error::Protocol(format!("{step}: {e}"))
}

fn x224<T>(pdu: T) -> Result<Vec<u8>, Error>
where
    X224<T>: ironrdp_core::Encode,
{
    encode_vec(&X224(pdu)).map_err(|e| protocol("encode", e))
}

impl Viewer {
    /// A new viewer and the first bytes to send (the X.224 Connection Request with standard RDP
    /// security, [MS-RDPBCGR] §2.2.1.1: `requestedProtocols` PROTOCOL_RDP = 0).
    pub fn new(config: Config) -> Result<(Viewer, Vec<u8>), Error> {
        let request = ConnectionRequest {
            // The routing cookie `mstshash` ([MS-RDPBCGR] §2.2.1.1) as the Windows viewer
            // sends it (`tested-in-lab`).
            nego_data: Some(NegoRequestData::cookie(cookie_name(&config.sip_uri))),
            flags: RequestFlags::empty(),
            protocol: SecurityProtocol::empty(),
            correlation_info: None,
        };
        let bytes = x224(request)?;
        Ok((
            Viewer {
                config,
                state: State::WaitConfirm,
                buffer: Vec::new(),
                io_channel: 0,
                user_channel: 0,
                static_channels: Vec::new(),
                image: None,
            },
            bytes,
        ))
    }

    /// The decoded screen (RGBA, `width * 4` bytes per row) once the session is active.
    pub fn image(&self) -> Option<(u16, u16, &[u8])> {
        self.image
            .as_ref()
            .map(|i| (i.width(), i.height(), i.data()))
    }

    /// Whether the session is active.
    pub fn active(&self) -> bool {
        matches!(self.state, State::Active(_))
    }

    /// Takes bytes from the sharer (in order, as `media::data` hands them over).
    pub fn on_bytes(&mut self, bytes: &[u8]) -> Result<Vec<Event>, Error> {
        if matches!(self.state, State::Ended) {
            return Ok(Vec::new());
        }
        if self.buffer.len().saturating_add(bytes.len()) > MAX_BUFFER {
            self.state = State::Ended;
            return Err(Error::TooLarge);
        }
        self.buffer.extend_from_slice(bytes);
        let mut out = Vec::new();
        loop {
            let info = match ironrdp_pdu::find_size(&self.buffer) {
                Ok(Some(info)) => info,
                Ok(None) => break,
                Err(e) => return self.fail(protocol("framing", e)),
            };
            if info.length > MAX_PDU || info.length == 0 {
                return self.fail(Error::TooLarge);
            }
            if self.buffer.len() < info.length {
                break;
            }
            let pdu: Vec<u8> = self.buffer.drain(..info.length).collect();
            // IronRDP is third-party code: should it panic on hostile input, the session ends
            // instead of the client (security by design).
            let handled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.on_pdu(info.action, &pdu, &mut out)
            }));
            match handled {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return self.fail(e),
                Err(_) => return self.fail(Error::Protocol("decoder failure".into())),
            }
            if matches!(self.state, State::Ended) {
                break;
            }
        }
        Ok(out)
    }

    fn fail(&mut self, e: Error) -> Result<Vec<Event>, Error> {
        self.state = State::Ended;
        self.buffer.clear();
        Err(e)
    }

    fn on_pdu(&mut self, action: Action, pdu: &[u8], out: &mut Vec<Event>) -> Result<(), Error> {
        let state = std::mem::replace(&mut self.state, State::Ended);
        let next = match state {
            State::Active(mut stage) => {
                if self.on_active(&mut stage, action, pdu, out)? {
                    State::Ended
                } else {
                    State::Active(stage)
                }
            }
            _ if action != Action::X224 => {
                return Err(Error::Protocol("fast-path before activation".into()))
            }
            State::WaitConfirm => {
                let confirm = decode::<X224<ConnectionConfirm>>(pdu)
                    .map_err(|e| protocol("connection confirm", e))?
                    .0;
                match confirm {
                    ConnectionConfirm::Response { protocol: p, .. } if p.is_empty() => {}
                    ConnectionConfirm::Response { protocol: p, .. } => {
                        return Err(Error::Refused(format!("security protocol {p:?}")))
                    }
                    ConnectionConfirm::Failure { code } => {
                        return Err(Error::Refused(format!("{code:?}")))
                    }
                }
                out.push(Event::Send(self.connect_initial()?));
                State::WaitConnectResponse
            }
            State::WaitConnectResponse => {
                let data = decode::<X224<X224Data<'_>>>(pdu)
                    .map_err(|e| protocol("connect response", e))?
                    .0;
                let response = decode::<mcs::ConnectResponse>(data.data.as_ref())
                    .map_err(|e| protocol("connect response", e))?;
                let blocks = response.conference_create_response.gcc_blocks();
                if !blocks.security.encryption_method.is_empty() {
                    return Err(Error::Refused("RDP encryption required".into()));
                }
                self.io_channel = blocks.network.io_channel;
                self.static_channels = blocks
                    .network
                    .channel_ids
                    .iter()
                    .copied()
                    .take(STATIC_CHANNELS)
                    .collect();
                out.push(Event::Send(x224(mcs::ErectDomainPdu {
                    sub_height: 0,
                    sub_interval: 0,
                })?));
                out.push(Event::Send(x224(mcs::AttachUserRequest)?));
                State::WaitAttach
            }
            State::WaitAttach => {
                let confirm = decode::<X224<mcs::AttachUserConfirm>>(pdu)
                    .map_err(|e| protocol("attach user", e))?
                    .0;
                self.user_channel = confirm.initiator_id;
                let mut remaining = vec![self.user_channel, self.io_channel];
                remaining.extend(self.static_channels.iter().copied());
                for &channel_id in &remaining {
                    out.push(Event::Send(x224(mcs::ChannelJoinRequest {
                        initiator_id: self.user_channel,
                        channel_id,
                    })?));
                }
                State::WaitJoin { remaining }
            }
            State::WaitJoin { mut remaining } => {
                let confirm = decode::<X224<mcs::ChannelJoinConfirm>>(pdu)
                    .map_err(|e| protocol("channel join", e))?
                    .0;
                if confirm.result != 0 {
                    return Err(Error::Refused(format!(
                        "channel {} not joined",
                        confirm.requested_channel_id
                    )));
                }
                let before = remaining.len();
                remaining.retain(|&c| c != confirm.requested_channel_id);
                if remaining.len() == before {
                    return Err(Error::Protocol("unexpected channel join confirm".into()));
                }
                if remaining.is_empty() {
                    out.push(Event::Send(self.client_info()?));
                    State::WaitDemandActive
                } else {
                    State::WaitJoin { remaining }
                }
            }
            State::WaitDemandActive => self.on_demand_active(pdu, out)?,
            State::WaitFinalization { share_id, font_map } => {
                self.on_finalization(share_id, font_map, pdu, out)?
            }
            State::Ended => State::Ended,
        };
        self.state = next;
        Ok(())
    }

    /// MCS Connect Initial with the GCC blocks ([MS-RDPBCGR] §2.2.1.3): no encryption method
    /// (MS-RTASPF §1.5), the `encomsp` channel, 32-bit colour.
    fn connect_initial(&self) -> Result<Vec<u8>, Error> {
        let width = self.config.width.clamp(200, MAX_SIDE);
        let height = self.config.height.clamp(200, MAX_SIDE);
        let name: String = self
            .config
            .client_name
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(15)
            .collect();
        let blocks = ClientGccBlocks {
            core: ClientCoreData {
                version: RdpVersion::V5_PLUS,
                desktop_width: width,
                desktop_height: height,
                color_depth: ColorDepth::Bpp8,
                sec_access_sequence: SecureAccessSequence::Del,
                keyboard_layout: 0,
                client_build: 0,
                client_name: if name.is_empty() {
                    "commscraft".into()
                } else {
                    name
                },
                keyboard_type: ironrdp_pdu::gcc::KeyboardType::IBM_ENHANCED,
                keyboard_subtype: 0,
                keyboard_functional_keys_count: 12,
                ime_file_name: String::new(),
                optional_data: ClientCoreOptionalData {
                    post_beta2_color_depth: Some(ColorDepth::Bpp8),
                    client_product_id: Some(1),
                    serial_number: Some(0),
                    high_color_depth: Some(HighColorDepth::Bpp24),
                    supported_color_depths: Some(
                        SupportedColorDepths::BPP32
                            | SupportedColorDepths::BPP24
                            | SupportedColorDepths::BPP16
                            | SupportedColorDepths::BPP15,
                    ),
                    early_capability_flags: Some(
                        ClientEarlyCapabilityFlags::SUPPORT_ERR_INFO_PDU
                            | ClientEarlyCapabilityFlags::WANT_32_BPP_SESSION
                            | ClientEarlyCapabilityFlags::SUPPORT_DYN_VC_GFX_PROTOCOL,
                    ),
                    dig_product_id: Some(String::new()),
                    connection_type: Some(ConnectionType::NotUsed),
                    server_selected_protocol: Some(SecurityProtocol::empty()),
                    desktop_physical_width: None,
                    desktop_physical_height: None,
                    desktop_orientation: None,
                    desktop_scale_factor: None,
                    device_scale_factor: None,
                },
            },
            security: ClientSecurityData {
                encryption_methods: EncryptionMethod::empty(),
                ext_encryption_methods: 0,
            },
            // As the Windows viewer asks for them (`tested-in-lab`): the dynamic channels, which
            // carry the graphics pipeline the sharer draws with, then the multiparty channel.
            network: Some(ClientNetworkData {
                channels: vec![
                    ChannelDef {
                        name: ChannelName::from_static(b"drdynvc\0"),
                        options: ChannelOptions::INITIALIZED
                            | ChannelOptions::ENCRYPT_RDP
                            | ChannelOptions::COMPRESS_RDP,
                    },
                    ChannelDef {
                        name: ChannelName::from_static(b"encomsp\0"),
                        options: ChannelOptions::INITIALIZED,
                    },
                ],
            }),
            cluster: None,
            monitor: None,
            message_channel: None,
            multi_transport_channel: None,
            monitor_extended: None,
        };
        let initial = mcs::ConnectInitial::with_gcc_blocks(blocks)
            .map_err(|e| protocol("connect initial", e))?;
        let data = encode_vec(&initial).map_err(|e| protocol("encode", e))?;
        x224(X224Data {
            data: Cow::Owned(data),
        })
    }

    /// The Client Info PDU ([MS-RDPBCGR] §2.2.1.11): no password, no compression
    /// (MS-RTASPF §1.5); the user name is our SIP URI (inference: the sharer shows the
    /// participant by the multiparty channel, not by this name).
    fn client_info(&self) -> Result<Vec<u8>, Error> {
        let user: String = self
            .config
            .sip_uri
            .trim_start_matches("sip:")
            .chars()
            .filter(|c| !c.is_control())
            .take(255)
            .collect();
        let info = ClientInfoPdu {
            security_header: BasicSecurityHeader {
                flags: BasicSecurityHeaderFlags::INFO_PKT,
            },
            client_info: ClientInfo {
                credentials: Credentials {
                    username: user,
                    password: String::new(),
                    domain: None,
                },
                password_cookie: None,
                code_page: 0,
                flags: ClientInfoFlags::MOUSE
                    | ClientInfoFlags::UNICODE
                    | ClientInfoFlags::LOGON_NOTIFY
                    | ClientInfoFlags::NO_AUDIO_PLAYBACK
                    | ClientInfoFlags::VIDEO_DISABLE,
                compression_type: CompressionType::K8,
                alternate_shell: String::new(),
                work_dir: String::new(),
                extra_info: ExtendedClientInfo {
                    address_family: AddressFamily::INET,
                    address: "0.0.0.0".into(),
                    dir: String::new(),
                    optional_data: ExtendedClientOptionalInfo::default(),
                },
            },
        };
        let data = encode_vec(&info).map_err(|e| protocol("encode", e))?;
        x224(mcs::SendDataRequest {
            initiator_id: self.user_channel,
            channel_id: self.io_channel,
            user_data: Cow::Owned(data),
        })
    }

    /// Licensing ends at once (a License Error PDU STATUS_VALID_CLIENT, [MS-RDPBCGR]
    /// §2.2.1.12) or is skipped; then the Demand Active PDU, answered with Confirm Active and
    /// the client's finalization PDUs (§2.2.1.13.2, §2.2.1.14–2.2.1.18).
    fn on_demand_active(&mut self, pdu: &[u8], out: &mut Vec<Event>) -> Result<State, Error> {
        let ind =
            mcs::decode_send_data_indication(pdu).map_err(|e| protocol("demand active", e))?;
        // A licensing PDU (security header with SEC_LICENSE_PKT, §2.2.1.12): the sharer has
        // nothing to license; a license request would need an answer we do not give (open), so
        // anything but the end of licensing stalls. Tried only when the PDU is no Share Control
        // PDU, whose length field could carry the same bit.
        let license = matches!(ind.user_data, [lo, hi, ..]
            if u16::from_le_bytes([*lo, *hi]) & BasicSecurityHeaderFlags::LICENSE_PKT.bits() != 0);
        let ctx = match ironrdp_pdu::rdp::headers::decode_share_control(ind) {
            Ok(ctx) => ctx,
            Err(_) if license => return Ok(State::WaitDemandActive),
            Err(e) => return Err(protocol("demand active", e)),
        };
        let ShareControlPdu::ServerDemandActive(demand) = ctx.pdu else {
            return Ok(State::WaitDemandActive);
        };
        let (width, height) = demand
            .pdu
            .capability_sets
            .iter()
            .find_map(|c| match c {
                CapabilitySet::Bitmap(b) => Some((b.desktop_width, b.desktop_height)),
                _ => None,
            })
            .unwrap_or((self.config.width, self.config.height));
        if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
            return Err(Error::Protocol(format!("screen size {width}x{height}")));
        }
        let confirm = ShareControlPdu::ClientConfirmActive(confirm_active(width, height));
        let share_id = ctx.share_id;
        let mut buf = ironrdp_core::WriteBuf::new();
        ironrdp_pdu::rdp::headers::encode_share_control(
            self.user_channel,
            self.io_channel,
            share_id,
            confirm,
            &mut buf,
        )
        .map_err(|e| protocol("confirm active", e))?;
        out.push(Event::Send(buf.filled().to_vec()));
        for data in [
            ShareDataPdu::Synchronize(SynchronizePdu {
                target_user_id: self.user_channel,
            }),
            ShareDataPdu::Control(ControlPdu {
                action: ControlAction::Cooperate,
                grant_id: 0,
                control_id: 0,
            }),
            ShareDataPdu::Control(ControlPdu {
                action: ControlAction::RequestControl,
                grant_id: 0,
                control_id: 0,
            }),
            ShareDataPdu::FontList(FontPdu::default()),
        ] {
            let mut buf = ironrdp_core::WriteBuf::new();
            ironrdp_pdu::rdp::headers::encode_share_data(
                self.user_channel,
                self.io_channel,
                share_id,
                data,
                &mut buf,
            )
            .map_err(|e| protocol("finalization", e))?;
            out.push(Event::Send(buf.filled().to_vec()));
        }
        self.image = Some(DecodedImage::new(PixelFormat::RgbA32, width, height));
        Ok(State::WaitFinalization {
            share_id,
            font_map: false,
        })
    }

    /// The sharer's finalization PDUs end with the Font Map PDU (§2.2.1.22); then the active
    /// stage takes over.
    fn on_finalization(
        &mut self,
        share_id: u32,
        font_map: bool,
        pdu: &[u8],
        out: &mut Vec<Event>,
    ) -> Result<State, Error> {
        let ind = mcs::decode_send_data_indication(pdu).map_err(|e| protocol("finalization", e))?;
        if ind.channel_id != self.io_channel {
            return Ok(State::WaitFinalization { share_id, font_map });
        }
        let ctx = ironrdp_pdu::rdp::headers::decode_share_data(ind)
            .map_err(|e| protocol("finalization", e))?;
        if !matches!(ctx.pdu, ShareDataPdu::FontMap(_)) {
            return Ok(State::WaitFinalization { share_id, font_map });
        }
        let mut channels = StaticChannelSet::new();
        let gfx = GraphicsPipelineClient::new(Box::new(Gfx), None);
        channels.insert(DrdynvcClient::new().with_dynamic_channel(gfx));
        channels.insert(Encomsp::default());
        if let Some(&id) = self.static_channels.first() {
            channels.attach_channel_id(std::any::TypeId::of::<DrdynvcClient>(), id);
        }
        if let Some(&id) = self.static_channels.get(1) {
            channels.attach_channel_id(std::any::TypeId::of::<Encomsp>(), id);
        }
        let stage = ActiveStageBuilder {
            static_channels: channels,
            user_channel_id: self.user_channel,
            io_channel_id: self.io_channel,
            message_channel_id: None,
            share_id,
            compression_type: None,
            enable_server_pointer: false,
            pointer_software_rendering: true,
        }
        .build();
        let (width, height) = self
            .image
            .as_ref()
            .map(|i| (i.width(), i.height()))
            .unwrap_or((self.config.width, self.config.height));
        out.push(Event::Active { width, height });
        Ok(State::Active(Box::new(stage)))
    }

    fn on_active(
        &mut self,
        stage: &mut ActiveStage,
        action: Action,
        pdu: &[u8],
        out: &mut Vec<Event>,
    ) -> Result<bool, Error> {
        let Some(image) = self.image.as_mut() else {
            return Err(Error::Protocol("no screen".into()));
        };
        let outputs = stage
            .process(image, action, pdu, None)
            .map_err(|e| protocol("screen update", e))?;
        let mut ended = false;
        for o in outputs {
            match o {
                ActiveStageOutput::ResponseFrame(bytes) => out.push(Event::Send(bytes)),
                ActiveStageOutput::GraphicsUpdate(r) => out.push(Event::Updated {
                    left: r.left,
                    top: r.top,
                    right: r.right,
                    bottom: r.bottom,
                }),
                ActiveStageOutput::Terminate(reason) => {
                    out.push(Event::Ended(format!("{reason:?}")));
                    ended = true;
                }
                ActiveStageOutput::DeactivateAll => {
                    // Reactivation (§1.3.1.3) is not taken yet: the session ends.
                    out.push(Event::Ended("deactivated".into()));
                    ended = true;
                }
                _ => {}
            }
        }
        Ok(ended)
    }
}

/// The Confirm Active PDU's capability sets ([MS-RDPBCGR] §2.2.1.13.2): no orders, no caches,
/// no bulk compression; the screen comes through the graphics pipeline (or bitmap updates).
fn confirm_active(width: u16, height: u16) -> ClientConfirmActive {
    ClientConfirmActive {
        originator_id: SERVER_CHANNEL_ID,
        pdu: DemandActive {
            source_descriptor: "COMMSCRAFT".into(),
            capability_sets: vec![
                CapabilitySet::General(General {
                    extra_flags: GeneralExtraFlags::FASTPATH_OUTPUT_SUPPORTED
                        | GeneralExtraFlags::NO_BITMAP_COMPRESSION_HDR,
                    ..Default::default()
                }),
                CapabilitySet::Bitmap(Bitmap {
                    pref_bits_per_pix: 32,
                    desktop_width: width,
                    desktop_height: height,
                    desktop_resize_flag: false,
                    drawing_flags: BitmapDrawingFlags::ALLOW_SKIP_ALPHA,
                }),
                CapabilitySet::Order(Order::new(
                    OrderFlags::NEGOTIATE_ORDER_SUPPORT | OrderFlags::ZERO_BOUNDS_DELTAS_SUPPORT,
                    OrderSupportExFlags::empty(),
                    0,
                    0,
                )),
                CapabilitySet::BitmapCache(BitmapCache {
                    caches: [CacheEntry {
                        entries: 0,
                        max_cell_size: 0,
                    }; BITMAP_CACHE_ENTRIES_NUM],
                }),
                CapabilitySet::Input(Input {
                    input_flags: InputFlags::SCANCODES | InputFlags::UNICODE,
                    keyboard_layout: 0,
                    keyboard_type: Some(ironrdp_pdu::gcc::KeyboardType::IBM_ENHANCED),
                    keyboard_subtype: 0,
                    keyboard_function_key: 12,
                    keyboard_ime_filename: String::new(),
                }),
                CapabilitySet::Pointer(Pointer {
                    color_pointer_cache_size: 0,
                    pointer_cache_size: 0,
                }),
                CapabilitySet::Brush(Brush {
                    support_level: SupportLevel::Default,
                }),
                CapabilitySet::GlyphCache(GlyphCache {
                    glyph_cache: [CacheDefinition {
                        entries: 0,
                        max_cell_size: 0,
                    }; GLYPH_CACHE_NUM],
                    frag_cache: CacheDefinition {
                        entries: 0,
                        max_cell_size: 0,
                    },
                    glyph_support_level: GlyphSupportLevel::None,
                }),
                CapabilitySet::OffscreenBitmapCache(OffscreenBitmapCache {
                    is_supported: false,
                    cache_size: 0,
                    cache_entries: 0,
                }),
                CapabilitySet::VirtualChannel(VirtualChannel {
                    flags: VirtualChannelFlags::NO_COMPRESSION,
                    chunk_size: Some(0),
                }),
                CapabilitySet::MultiFragmentUpdate(MultifragmentUpdate {
                    max_request_size: 8 * 1024 * 1024,
                }),
            ],
        },
    }
}

#[cfg(test)]
mod tests;
