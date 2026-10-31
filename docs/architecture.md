# CommSCraft architecture (planned)

> **Last reviewed:** 2026-10-27 · **Last updated:** 2026-10-27 · **Change:** minor (first adapter `net`, first app `commscraft-cli`)

Proposal for M0; it becomes binding once decisions D-01, D-03 and D-07 in
[`decisions.md`](decisions.md) are taken. Rules for working in the code are in
[`AGENTS.md`](../AGENTS.md).

## Principles

- **Sans-I/O protocol core:** every protocol (SIP transaction and dialog layer, authentication, ICE,
  TURN, SRTP, conference control) is a state machine that consumes bytes/events and emits
  bytes/events. Sockets, timers, the OS and the UI live in adapter crates. This makes the published
  test vectors and recorded fixtures directly testable offline.
- **Evidence IDs in code:** each requirement implemented carries its evidence ID (e.g. `AE-24`).
- **Microsoft variants are explicit:** where Microsoft deviates from the RFC (MS-TURN attribute
  numbers and padding, MS-ICE2 duplicate check with the misprinted CRC table, NTLM sequence number
  100, SHA-1 in the endpoint ID derivation), the variant is a named, tested code path.

## Crates that exist (2026-10-27)

| Crate | Layer | Job | Evidence |
|---|---|---|---|
| `commscraft-sip-identity` | L0 | EPID → `+sip.instance` (SHA-1, K-RE-1) | RE-02, RE-06, RE-07 |
| `commscraft-ntlm` | L1 | NTLMv2 keys, ESS signatures, connectionless SIP signing (seq 100); CHALLENGE parsing and AUTHENTICATE construction with MIC, flag policy (NTLMv2 + ESS + 128 + KEY_EXCH required) | NL-01…NL-05, AE-24, AE-30, OB-43 |
| `commscraft-ice` | L1 | CRC-32/FINGERPRINT incl. the MS-ICE2 duplicate variant, candidate priorities | IC-12, IC-14, IE-03…IE-05, OB-09 |
| `commscraft-g711` | L1 | ITU-T G.711 A-law/µ-law from the decision values of the Recommendation | G7-01…G7-05 |
| `commscraft-g722` | L1 | ITU-T G.722 SB-ADPCM 64 kbit/s with QMF; ADPCM bit-exact against the ITU test sequences | G2-01…G2-06, K-G2-1 |
| `commscraft-audio-alsa` | L2 | Audio device adapter (D-11): ALSA API, PipeWire through its ALSA plugin; mono 16-bit at 8/16 kHz, checked configuration, xrun recovery | D-03, D-11 |
| `commscraft-srtp` | L1 | RFC 3711 key derivation, SSRTP packet transform, SRTP/SRTCP cryptographic contexts of the MS-SRTP profile (MKI, replay window 64, ROC) | SR-*, SS-01…SS-05, IE-09 |
| `commscraft-sdp` | L3 | SDP for audio: bounded parser, offer/answer, a=crypto keys, ICE-19 attributes, encryption policy | SD-01…SD-05, SD-10, SD-11, SD-14, SD-25, SD-41, K-SD-3 |
| `commscraft-stun` | L1 | STUN messages in RFC 5389 and STUN-02/MS-TURN format, MESSAGE-INTEGRITY (SHA-1, legacy, MS-TURN SHA-256), FINGERPRINT, addresses | TU-10…TU-15, IC-02…IC-04, IC-13, IE-02, IE-03, O-IC-2 |
| `commscraft-avedgea` | L3 | Media relay credentials (MS-AVEDGEA): SERVICE body, checked answer with relays | AV-01…AV-09, K-AV-1 |
| `commscraft-ice-agent` | L2 | ICE agent per media stream (MS-ICE2, UDP): pairs, checks in STUN-02 and RFC 5389, peer-reflexive learning, triggered checks, regular nomination, keep-alive, consent freshness | IC-01…IC-17, K-IC-2, O-IC-2 |
| `commscraft-call-media` | L2 | Network side of a call's media: UDP sockets, MS-TURN relays, ICE agent, audio core; local and final SDP description, direct media for peers without ICE | OCS-UC12, IC-07, IC-09, TU-24, TU-25 |
| `commscraft-media` | L2 | Audio media core of a call: codec, RTP, SRTP, jitter buffer (60 ms, bounded), loss concealment, DTMF, RTCP reports and BYE, STUN/RTP/RTCP classification | RT-01…RT-10, SR-*, PL-30, G7-*, G2-* |
| `commscraft-turn` | L2 | MS-TURN client: Allocate with digest challenge, sequence numbers, Send/Data Indication, Set Active Destination, refresh, teardown, retransmission, TCP framing, pseudo-TLS | TU-02, TU-04, TU-10…TU-27 |
| `commscraft-rtp` | L1 | RTP header, sender, reception statistics, RTCP SR/RR/SDES/BYE, telephone events | RT-01…RT-10, RT-20, RT-21, PL-30 |
| `commscraft-turn-bwm` | L1 | MS-TURNBWM attribute types, XOR site addresses | BW-11, BW-12 |
| `commscraft-sipcomp` | L2 | MPPC compressor/decompressor with 8 KiB history and packet flags (no header: O-CP-1) | CP-05…CP-08, K-CP-1, O-CP-3, OB-01 |
| `commscraft-abs` | L3 | Address book block compression (LZ77 variant), CRC-32, file-name dates | AB-03, AB-04, K-AB-1, K-AB-4 |
| `commscraft-sip-msg` | L0 | SIP parsing, stream framing and serialization; bounded, fail-closed, injection-safe; UAC dialogs (RFC 3261 §12) | RFC 3261 §7, §18.3, §25; AE-29 |
| `commscraft-sipae` | L1 | MS-SIPAE security associations, signature buffers, Authorization header, verification with replay window | AE-05, AE-10, AE-11, AE-29…AE-33, K-AE-4 |
| `commscraft-register` | L2 | Sign-in state machine: REGISTER → NTLM handshake → signed 200 OK → GRUU/expiry → refresh → sign-out; client version block (RG-50); deregister/survivable mode NOTIFY; signed home pool redirect (O-RG-1); keep-alive negotiation (CON-30…CON-35) | AE-20…AE-33, RG-03…RG-32, RG-50, RE-03, RE-06, K-RG-4 |
| `commscraft-url` | L0 | Checked DNS host names and HTTPS URLs, user-domain test | docs/security.md |
| `commscraft-discovery` | L2 | Autodiscover (HTTPS only, Domain resource, redirect limits) and DNS SRV/fallback ordering (TLS only, domain filter) | AD-02…AD-13, CON-10…CON-18, OB-06, K-AD-1 |
| `commscraft-net` | adapter | TLS client (rustls + ring, system trust store, optional enterprise CA), blocking SIP connection over the bounded decoder, OS random source, HTTPS GET for Autodiscover, DNS SRV client | CON-01…CON-04, CON-11, AD-02…AD-04, [SFB-TLS] |
| `commscraft-cli` | app | Owner's interop tool: discovery (Autodiscover, DNS SRV), sign in, keep-alive, provisioning summary, presence, IM, interactive mode on the session engine (incl. audio calls), sign out; `--discover-only`; `--authorized` required | IT-16, IT-19 |
| `commscraft-presence` | L3 | Enhanced presence: publish own userState, batch presence queries (multipart/RLMI/categories), roaming contact list | PR-05, PR-10…PR-17, PR-30…PR-36, SIP-01/02/04/07, K-PR-1 |
| `commscraft-dlx` | L3 | Address book search (MS-DLX SearchAbEntry): SOAP request, bounded answer parser with SIP URI check | DL-01…DL-10, K-DL-3, OB-44 |
| `commscraft-conf` | L3 | Group IM: C3P addConference/addUser (join, dial-out), roster (RFC 4575 merge, bounded), IMDN, Ms-Sender, invitations | CB-10…CB-37, PV-01…PV-11, CI-01…CI-12, K-CI-3 |
| `commscraft-im` | L3 | 1:1 IM: message-session SDP (offer/check), ACK for 2xx and failures, MESSAGE, typing INFO, handling of the peer's requests (plain text only) | OCS-UC09, CI-02, OB-16 |
| `commscraft-session` | L4 | Session engine (sans-I/O): registration, bootstrap (endpoint publication, contact list), long-lived presence subscriptions with NOTIFY/refresh/resubscribe, client transactions with timeouts, incoming and outgoing IM conversations, events for a UI | OCS-UC03, PR-11…PR-33, SIP-04/07, AE-29/32, RFC 3261/3265 |
| `commscraft-xml` | L0 | Bounded XML reader without DTD/entities, namespaces, explicit stack | docs/security.md |
| `commscraft-provisioning` | L3 | Provisioning SUBSCRIBE, `provisionGroupList` parser, validated ServerConfiguration (HTTPS URLs with domain check, SIP URIs, port ranges) | RG-40…RG-44, K-RG-4 |
| `commscraft-psom` | L3 | PSOM GenericInt, String, RPC frame | PO-04, PO-05, K-PO-2, K-PO-3 |

Dependencies (decision D-03, pure Rust): RustCrypto `md4`, `md-5`, `hmac`, `sha1`, `aes`, plus
`subtle` (constant-time comparison) and `zeroize` (wiping keys) in `sipae`. The adapter `net` adds
`rustls` (with `ring`), `rustls-native-certs` and `getrandom`. `sip-msg`, `sipcomp`,
`abs`, `psom` and `turn-bwm` use only the standard library. Security rules: [`security.md`](security.md).

## Layers (proposal)

A crate may only depend on crates in lower layers.

| Layer | Crate | Job |
|---|---|---|
| L0 | `sip-msg` | SIP/SDP parsing and serialization, bounded and hostile-input safe |
| L0 | `xml` | Bounded XML reader/writer without external entities |
| L1 | `auth` | NTLM (MS-NLMP, SIP signatures per MS-SIPAE), Kerberos/SPNEGO via adapter, TLS-DSK |
| L1 | `stun-turn-ice` | MS-TURN, MS-ICE2, bandwidth extensions (MS-TURNBWM, MS-ICE2BWM) |
| L1 | `srtp` | SRTP/SSRTP per MS-SRTP |
| L2 | `sip-core` | Transactions, dialogs, registration, provisioning, keep-alive, compression (optional) |
| L2 | `media` | RTP/RTCP (MS-RTP), payload formats (H.264 UC, DTMF, redundancy), jitter buffer |
| L3 | `presence`, `im`, `dlx`, `conf`, `telephony`, `exchange`, `webservices` | Feature protocols (MS-PRES, MS-CONFBAS/AV/IM/AS, MS-OCPSTN, EWS, UCWA, MS-DLX, MS-ABS, MS-E911WS) |
| L4 | `session` | One signed-in account: orchestrates features, exposes commands and state |
| L5 | `ui-egui` | Desktop user interface |
| adapters | `net-tokio`, `audio-pipewire`, `video-*`, `rdp-*`, `gssapi` | OS and library bindings (subject to D-03) |
| apps | `commscraft`, `commscraft-cli` | Desktop app; command-line tool for offline checks against fixtures |

## Linux integration (planned)

- Audio via PipeWire with echo cancellation; video capture via PipeWire/V4L2.
- Screen sharing viewer first; sharing one's own screen later via xdg-desktop-portal.
- Credentials and certificates in the system keyring (Secret Service).
- URL handlers for `lync:`, `lync15:` and `conf:` meeting links.
