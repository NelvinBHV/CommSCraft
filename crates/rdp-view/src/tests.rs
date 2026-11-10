use super::*;
use ironrdp_pdu::gcc::{
    ConferenceCreateResponse, ServerCoreData, ServerCoreOptionalData, ServerGccBlocks,
    ServerNetworkData, ServerSecurityData,
};
use ironrdp_pdu::nego::ResponseFlags;
use ironrdp_pdu::rdp::capability_sets::ServerDemandActive;
use ironrdp_pdu::rdp::headers::{ShareControlHeader, ShareDataHeader, StreamPriority};

const IO: u16 = 1003;
const ENC: u16 = 1004;
const USER: u16 = 1007;
const SHARE: u32 = 0x0001_03ea;

fn config() -> Config {
    Config {
        width: 1280,
        height: 720,
        sip_uri: "sip:alice@contoso.com".into(),
        client_name: "alice-pc".into(),
    }
}

fn indication(channel: u16, data: Vec<u8>) -> Vec<u8> {
    x224(mcs::SendDataIndication {
        initiator_id: 1002,
        channel_id: channel,
        user_data: Cow::Owned(data),
    })
    .unwrap()
}

fn share_control(pdu: ShareControlPdu) -> Vec<u8> {
    indication(
        IO,
        encode_vec(&ShareControlHeader {
            share_control_pdu: pdu,
            pdu_source: 1002,
            share_id: SHARE,
        })
        .unwrap(),
    )
}

fn share_data(pdu: ShareDataPdu) -> Vec<u8> {
    share_control(ShareControlPdu::Data(ShareDataHeader {
        share_data_pdu: pdu,
        stream_priority: StreamPriority::Medium,
        compression_flags: ironrdp_pdu::rdp::headers::CompressionFlags::empty(),
        compression_type: CompressionType::K8,
    }))
}

fn connect_response(encryption: EncryptionMethod) -> Vec<u8> {
    let blocks = ServerGccBlocks {
        core: ServerCoreData {
            version: RdpVersion::V5_PLUS,
            optional_data: ServerCoreOptionalData::default(),
        },
        network: ServerNetworkData {
            channel_ids: vec![ENC],
            io_channel: IO,
        },
        security: if encryption.is_empty() {
            ServerSecurityData::no_security()
        } else {
            ServerSecurityData {
                encryption_method: encryption,
                encryption_level: ironrdp_pdu::gcc::EncryptionLevel::ClientCompatible,
                server_random: Some([7; 32]),
                server_cert: vec![1; 16],
            }
        },
        message_channel: None,
        multi_transport_channel: None,
    };
    let response = mcs::ConnectResponse {
        conference_create_response: ConferenceCreateResponse::new(USER, blocks).unwrap(),
        called_connect_id: 1,
        domain_parameters: mcs::DomainParameters::target(),
    };
    x224(X224Data {
        data: Cow::Owned(encode_vec(&response).unwrap()),
    })
    .unwrap()
}

fn sends(events: &[Event]) -> Vec<&Vec<u8>> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Send(b) => Some(b),
            _ => None,
        })
        .collect()
}

/// Drives the connection sequence up to the Demand Active PDU; returns the viewer.
fn until_demand_active() -> Viewer {
    let (mut v, first) = Viewer::new(config()).unwrap();
    let request = decode::<X224<ConnectionRequest>>(&first).unwrap().0;
    assert!(request.protocol.is_empty(), "standard RDP security");
    let confirm = x224(ConnectionConfirm::Response {
        flags: ResponseFlags::empty(),
        protocol: SecurityProtocol::empty(),
    })
    .unwrap();
    let out = v.on_bytes(&confirm).unwrap();
    let initial = decode::<X224<X224Data<'_>>>(sends(&out)[0]).unwrap().0;
    let initial = decode::<mcs::ConnectInitial>(initial.data.as_ref()).unwrap();
    let blocks = initial.conference_create_request.gcc_blocks();
    assert!(blocks.security.encryption_methods.is_empty());
    let channels = &blocks.network.as_ref().unwrap().channels;
    assert_eq!(channels[0].name.as_str(), Some("encomsp"));
    let out = v
        .on_bytes(&connect_response(EncryptionMethod::empty()))
        .unwrap();
    assert_eq!(sends(&out).len(), 2, "erect domain and attach user");
    let attach = x224(mcs::AttachUserConfirm {
        result: 0,
        initiator_id: USER,
    })
    .unwrap();
    let out = v.on_bytes(&attach).unwrap();
    assert_eq!(sends(&out).len(), 3, "user, I/O and encomsp channel");
    // The three confirms in one go, split across calls in the middle of a PDU.
    let mut joins = Vec::new();
    for c in [USER, IO, ENC] {
        joins.extend(
            x224(mcs::ChannelJoinConfirm {
                result: 0,
                initiator_id: USER,
                requested_channel_id: c,
                channel_id: c,
            })
            .unwrap(),
        );
    }
    assert!(v.on_bytes(&joins[..5]).unwrap().is_empty());
    let out = v.on_bytes(&joins[5..]).unwrap();
    let info = mcs::decode_send_data_indication(sends(&out)[0]);
    // A Send Data Request is not an indication: decode it as request instead.
    assert!(info.is_err());
    let request = decode::<X224<mcs::SendDataRequest<'_>>>(sends(&out)[0])
        .unwrap()
        .0;
    let info = decode::<ClientInfoPdu>(request.user_data.as_ref()).unwrap();
    assert_eq!(info.client_info.credentials.username, "alice@contoso.com");
    assert!(info.client_info.credentials.password.is_empty());
    assert!(!info
        .client_info
        .flags
        .contains(ClientInfoFlags::COMPRESSION));
    v
}

/// Function 43 over RDP: the whole connection sequence against a modelled sharer, up to the
/// active session with the sharer's screen size.
#[test]
fn connection_sequence() {
    let mut v = until_demand_active();
    // Licensing ends at once: a licensing PDU is passed over.
    let license = indication(IO, vec![0x80, 0x00, 0x00, 0x00, 0xff, 0x03, 0x10, 0x00]);
    assert!(v.on_bytes(&license).unwrap().is_empty());
    let demand = share_control(ShareControlPdu::ServerDemandActive(ServerDemandActive {
        pdu: DemandActive {
            source_descriptor: "RDP".into(),
            capability_sets: vec![CapabilitySet::Bitmap(Bitmap {
                pref_bits_per_pix: 32,
                desktop_width: 800,
                desktop_height: 600,
                desktop_resize_flag: false,
                drawing_flags: BitmapDrawingFlags::empty(),
            })],
        },
    }));
    let out = v.on_bytes(&demand).unwrap();
    assert_eq!(
        sends(&out).len(),
        5,
        "confirm active and four finalization PDUs"
    );
    assert!(!v.active());
    let mut fin = share_data(ShareDataPdu::Synchronize(SynchronizePdu {
        target_user_id: USER,
    }));
    fin.extend(share_data(ShareDataPdu::FontMap(FontPdu::default())));
    let out = v.on_bytes(&fin).unwrap();
    assert!(out.contains(&Event::Active {
        width: 800,
        height: 600
    }));
    assert!(v.active());
    let (w, h, data) = v.image().unwrap();
    assert_eq!((w, h, data.len()), (800, 600, 800 * 600 * 4));
}

/// A sharer that wants RDP encryption is refused (MS-RTASPF §1.5: level "None").
#[test]
fn encryption_refused() {
    let (mut v, _) = Viewer::new(config()).unwrap();
    let confirm = x224(ConnectionConfirm::Response {
        flags: ResponseFlags::empty(),
        protocol: SecurityProtocol::empty(),
    })
    .unwrap();
    v.on_bytes(&confirm).unwrap();
    let r = v.on_bytes(&connect_response(EncryptionMethod::BIT_128));
    assert!(matches!(r, Err(Error::Refused(_))), "{r:?}");
    // Ended: further input is ignored.
    assert!(v.on_bytes(&confirm).unwrap().is_empty());
}

/// Hostile input: a TLS-only answer, garbage, an oversized length and a flood end the session.
#[test]
fn hostile_input() {
    let (mut v, _) = Viewer::new(config()).unwrap();
    let tls = x224(ConnectionConfirm::Response {
        flags: ResponseFlags::empty(),
        protocol: SecurityProtocol::SSL,
    })
    .unwrap();
    assert!(matches!(v.on_bytes(&tls), Err(Error::Refused(_))));

    let (mut v, _) = Viewer::new(config()).unwrap();
    assert!(v.on_bytes(&[0x03, 0x00, 0x00, 0x08, 1, 2, 3, 4]).is_err());

    let (mut v, _) = Viewer::new(config()).unwrap();
    assert!(v.on_bytes(&[0x03, 0x00, 0x00, 0x00]).is_err(), "length 0");

    let (mut v, _) = Viewer::new(config()).unwrap();
    // A TPKT header announcing 65535 bytes, then never enough: bounded by MAX_BUFFER.
    let mut flood = vec![0x03, 0x00, 0xff, 0xff];
    flood.resize(1000, 0);
    assert!(v.on_bytes(&flood).unwrap().is_empty());
    let big = vec![0u8; MAX_BUFFER];
    assert_eq!(v.on_bytes(&big), Err(Error::TooLarge));

    // Fast-path before activation.
    let (mut v, _) = Viewer::new(config()).unwrap();
    assert!(v.on_bytes(&[0x00, 0x04, 0x00, 0x00]).is_err());
}
