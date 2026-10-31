use super::*;

/// [MS-SDPEXT] §4.1.3 (pp. 53–55), audio section, line breaks of the printed page joined.
const SPEC_OFFER: &str = "v=0\r
o=- 0 0 IN IP4 172.24.32.152\r
s=session\r
c=IN IP4 172.24.32.152\r
b=CT:99980\r
t=0 0\r
a=x-mediabw:main-video send=585;recv=1416\r
a=x-devicecaps:audio:send,recv;video:send,recv\r
m=audio 50005 RTP/AVP 117 114 9 111 112 115 116 8 0 97 13 118 101\r
a=rtcp-fb:* x-message app send:dsh recv:dsh\r
a=rtcp-rsize\r
a=ice-ufrag: 6nxO\r
a=ice-pwd: G6rUJNNaobz8IdDZrAbyFDoO\r
a=candidate:1 1 UDP 2130706431 172.24.32.152 50005 typ host\r
a=candidate:1 2 UDP 2130705918 172.24.32.152 50009 typ host\r
a=candidate:2 1 TCP-PASS 6556159 172.29.105.171 53127 typ relay raddr 172.29.105.171 rport 53127\r
a=candidate:2 2 TCP-PASS 6556158 172.29.105.171 53127 typ relay raddr 172.29.105.171 rport 53127\r
a=candidate:3 1 UDP 16648703 172.29.105.171 59353 typ relay raddr 172.29.105.171 rport 59353\r
a=candidate:3 2 UDP 16648702 172.29.105.171 59627 typ relay raddr 172.29.105.171 rport 59627\r
a=candidate:4 1 TCP-ACT 7076863 172.29.105.171 53127 typ relay raddr 172.29.105.171 rport 53127\r
a=candidate:4 2 TCP-ACT 7076350 172.29.105.171 53127 typ relay raddr 172.29.105.171 rport 53127\r
a=candidate:5 1 TCP-ACT 1684797951 172.24.32.152 50004 typ srflx raddr 172.24.32.152 rport 50004\r
a=candidate:5 2 TCP-ACT 1684797438 172.24.32.152 50004 typ srflx raddr 172.24.32.152 rport 50004\r
a=label:main-audio\r
a=cryptoscale:1 client AES_CM_128_HMAC_SHA1_80 inline:15PHFDbUI8l9/bOHUYM9geb2IakQY3tMe31TgoPC|2^31|1:1\r
a=crypto:2 AES_CM_128_HMAC_SHA1_80 inline:C62B/j2xrqnk18t4bxXthuGv/Lxc9DmYDG4mnAOK|2^31|1:1\r
a=maxptime:200\r
a=rtcp:50009\r
a=rtpmap:117 G722/8000/2\r
a=fmtp:117 bitrate=128000\r
a=rtpmap:114 x-msrta/16000\r
a=fmtp:114 bitrate=29000\r
a=rtpmap:9 G722/8000\r
a=rtpmap:111 SIREN/16000\r
a=fmtp:111 bitrate=16000\r
a=rtpmap:112 G7221/16000\r
a=fmtp:112 bitrate=24000\r
a=rtpmap:115 x-msrta/8000\r
a=fmtp:115 bitrate=11800\r
a=rtpmap:116 AAL2-G726-32/8000\r
a=rtpmap:8 PCMA/8000\r
a=rtpmap:0 PCMU/8000\r
a=rtpmap:97 RED/8000\r
a=rtpmap:13 CN/8000\r
a=rtpmap:118 CN/16000\r
a=rtpmap:101 telephone-event/8000\r
a=fmtp:101 0-16\r
";

fn local() -> Local {
    Local {
        address: "192.0.2.5".parse().unwrap(),
        rtp_port: 40000,
        rtcp_port: 40001,
        key: Some(Key::new(1, [7; 30])),
        ice: Some(Ice {
            ufrag: "abcdefgh".into(),
            pwd: "0123456789abcdefghijklmnop".into(),
            candidates: vec![Candidate {
                foundation: "1".into(),
                component: 1,
                transport: Transport::Udp,
                priority: 2130706431,
                address: "192.0.2.5".parse().unwrap(),
                port: 40000,
                kind: CandidateType::Host,
                related: None,
            }],
        }),
        direction: Direction::SendRecv,
    }
}

#[test]
fn parses_the_specification_offer() {
    let s = parse(SPEC_OFFER.as_bytes()).unwrap();
    let m = audio(&s).unwrap();
    assert_eq!(
        (m.port, m.proto.as_str(), m.formats.len()),
        (50005, "RTP/AVP", 13)
    );
    assert_eq!(codec_of(m, 9), Some(Codec::G722));
    assert_eq!(codec_of(m, 117), None, "G.722 stereo is not offered by us");
    assert_eq!(codec_of(m, 8), Some(Codec::Pcma));
    assert_eq!(dtmf_pt(m), Some(101));
    let ice = ice_of(m).unwrap();
    assert_eq!((ice.ufrag.as_str(), ice.candidates.len()), ("6nxO", 10));
    assert_eq!(ice.candidates[2].transport, Transport::TcpPassive);
    assert_eq!(ice.candidates[2].kind, CandidateType::Relay);
    assert_eq!(
        ice.candidates[2].related,
        Some(("172.29.105.171".parse().unwrap(), 53127))
    );
    let keys = keys_of(m);
    assert_eq!(
        keys.len(),
        1,
        "only a=crypto, a=cryptoscale is not used for 1:1"
    );
    assert_eq!((keys[0].tag, keys[0].mki), (2, Some(1)));
    assert_eq!(
        endpoints(&s, m).unwrap().1,
        ("172.24.32.152".parse().unwrap(), 50009)
    );
    // Candidate and crypto lines survive a round trip through our writer.
    for c in &ice.candidates {
        assert_eq!(&Candidate::parse(&c.attribute()).unwrap(), c);
    }
    assert_eq!(Key::parse(&keys[0].attribute()).unwrap(), keys[0]);
}

/// SD-03, SD-11, §4.3.1: answering the optional-encryption offer of the specification.
#[test]
fn answers_the_specification_offer() {
    let (sdp, n) = answer(
        SPEC_OFFER.as_bytes(),
        &local(),
        Key::new(9, [3; 30]),
        Encryption::Required,
    )
    .unwrap();
    assert!(
        sdp.contains("m=audio 40000 RTP/SAVP 9 8 0 101\r\n"),
        "{sdp}"
    );
    assert!(
        sdp.contains("a=crypto:2 AES_CM_128_HMAC_SHA1_80 inline:"),
        "same tag as the offer"
    );
    assert!(sdp.contains("|2^31|1:1\r\n"));
    assert!(sdp.contains("a=rtpmap:101 telephone-event/8000\r\na=fmtp:101 0-16\r\n"));
    assert!(
        sdp.contains("o=- 0 0 IN IP4 192.0.2.5\r\n"),
        "SD-02: versions stay 0"
    );
    assert_eq!(
        (n.codec, n.payload_type, n.dtmf_payload_type),
        (Codec::G722, 9, Some(101))
    );
    assert_eq!(n.remote_rtp, ("172.24.32.152".parse().unwrap(), 50005));
    assert_eq!(n.remote_key.as_ref().map(|k| k.tag), Some(2));
    // Our own answer parses again.
    let again = parse(sdp.as_bytes()).unwrap();
    assert_eq!(again.media[0].formats, vec![9, 8, 0, 101]);
}

#[test]
fn offer_and_answer_evaluation() {
    let l = local();
    let o = offer(
        &l,
        Encryption::Optional {
            allow_plaintext: false,
        },
    );
    assert!(o.contains("m=audio 40000 RTP/AVP 9 0 8 101\r\n"));
    assert!(o.contains("a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:"));
    assert!(offer(&l, Encryption::Required).contains("RTP/SAVP"));
    assert!(!offer(&l, Encryption::Off).contains("a=crypto"));
    let peer_key = Key::new(1, [5; 30]);
    let ans = format!(
        "v=0\r\no=- 0 0 IN IP4 198.51.100.7\r\ns=session\r\nc=IN IP4 198.51.100.7\r\nt=0 0\r\nm=audio 5000 RTP/SAVP 0 101\r\na=crypto:{}\r\na=rtpmap:0 PCMU/8000\r\na=rtpmap:101 telephone-event/8000\r\na=inactive\r\n",
        peer_key.attribute()
    );
    let n = answer_to_offer(ans.as_bytes(), l.key.as_ref(), Encryption::Required).unwrap();
    assert_eq!(
        (n.codec, n.remote_rtcp.1, n.direction),
        (Codec::Pcmu, 5001, Direction::Inactive)
    );
    assert_eq!(n.remote_key, Some(peer_key.clone()));
    // A plaintext answer is refused unless the policy allows it.
    let plain = "v=0\r\nc=IN IP4 198.51.100.7\r\nm=audio 5000 RTP/AVP 0\r\n";
    assert_eq!(
        answer_to_offer(
            plain.as_bytes(),
            l.key.as_ref(),
            Encryption::Optional {
                allow_plaintext: false
            }
        ),
        Err(Error::Unencrypted)
    );
    assert!(answer_to_offer(
        plain.as_bytes(),
        l.key.as_ref(),
        Encryption::Optional {
            allow_plaintext: true
        }
    )
    .is_ok());
    // A peer echoing our own key is refused (MS-SRTP §5.1).
    let echo = ans.replace(&peer_key.attribute(), &l.key.as_ref().unwrap().attribute());
    assert!(matches!(
        answer_to_offer(echo.as_bytes(), l.key.as_ref(), Encryption::Required),
        Err(Error::Crypto(_))
    ));
}

/// K-SD-3: the answer examples of §4.3.1.2/§4.3.2.2 carry truncated keys (38/39 base64
/// characters instead of 40); they are refused, as are other suites and long MKIs.
#[test]
fn key_attribute_checks() {
    assert_eq!(
        Key::parse(
            "2 AES_CM_128_HMAC_SHA1_80 inline:z8aIuyfeJZ2bkLVNPadciqjXwGMXo0s0IomrZr|2^31|1:1"
        ),
        Err(Error::Crypto("base64"))
    );
    let ok = "2 AES_CM_128_HMAC_SHA1_80 inline:C62B/j2xrqnk18t4bxXthuGv/Lxc9DmYDG4mnAOK";
    assert_eq!(Key::parse(ok).unwrap().mki, None);
    assert!(
        Key::parse(&format!("{ok}|2^31|1:4")).is_err(),
        "MKI length must be 1"
    );
    assert!(Key::parse(&format!("{ok}|2^31|300:1")).is_err());
    assert!(Key::parse(
        "2 AES_CM_128_HMAC_SHA1_32 inline:C62B/j2xrqnk18t4bxXthuGv/Lxc9DmYDG4mnAOK"
    )
    .is_err());
    assert!(
        Key::parse(&format!("{ok} KDR=1")).is_err(),
        "no session parameters"
    );
    assert!(
        !format!("{:?}", Key::new(1, [1; 30])).contains("key:"),
        "keys are never printed"
    );
}

#[test]
fn hostile_sdp() {
    assert_eq!(parse(&vec![b'a'; MAX_SDP + 1]), Err(Error::TooLarge));
    assert_eq!(parse(b"m=audio 1 RTP/AVP 0\r\n"), Err(Error::Syntax("v=0")));
    assert!(parse(b"v=0\r\nc=IN IP4 999.1.1.1\r\n").is_err());
    assert!(parse("v=0\r\n".repeat(600).as_bytes()).is_err());
    let many_m = format!("v=0\r\n{}", "m=audio 1 RTP/AVP 0\r\n".repeat(20));
    assert_eq!(parse(many_m.as_bytes()), Err(Error::TooLarge));
    // An encrypted offer without key, and offers without our codecs, are refused.
    let savp = "v=0\r\nc=IN IP4 198.51.100.7\r\nm=audio 5000 RTP/SAVP 0\r\n";
    assert!(matches!(
        answer(
            savp.as_bytes(),
            &local(),
            Key::new(1, [3; 30]),
            Encryption::Required
        ),
        Err(Error::Crypto(_))
    ));
    let opus =
        "v=0\r\nc=IN IP4 198.51.100.7\r\nm=audio 5000 RTP/AVP 111\r\na=rtpmap:111 opus/48000/2\r\n";
    assert_eq!(
        answer(
            opus.as_bytes(),
            &local(),
            Key::new(1, [3; 30]),
            Encryption::Off
        )
        .err(),
        Some(Error::NoAudio)
    );
    assert!(
        Candidate::parse("1 3 UDP 1 192.0.2.1 1 typ host").is_err(),
        "component"
    );
    assert!(Candidate::parse("1 1 SCTP 1 192.0.2.1 1 typ host").is_err());
    assert!(Candidate::parse("1 1 UDP 1 not-an-ip 1 typ host").is_err());
}
