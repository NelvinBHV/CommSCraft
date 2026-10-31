# CommSCraft architecture (planned)

> **Last reviewed:** 2026-10-27 · **Last updated:** 2026-10-27 · **Change:** minor (M1 crates: `sip-msg`, `sipae`, `register`, `xml`, `provisioning`)

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
| `commscraft-srtp` | L1 | RFC 3711 key derivation, SSRTP packet transform | SR-*, SS-01…SS-05, IE-09 |
| `commscraft-turn-bwm` | L1 | MS-TURNBWM attribute types, XOR site addresses | BW-11, BW-12 |
| `commscraft-sipcomp` | L2 | MPPC compressor/decompressor with 8 KiB history and packet flags (no header: O-CP-1) | CP-05…CP-08, K-CP-1, O-CP-3, OB-01 |
| `commscraft-abs` | L3 | Address book block compression (LZ77 variant), CRC-32, file-name dates | AB-03, AB-04, K-AB-1, K-AB-4 |
| `commscraft-sip-msg` | L0 | SIP parsing, stream framing and serialization; bounded, fail-closed, injection-safe | RFC 3261 §7, §18.3, §25; AE-29 |
| `commscraft-sipae` | L1 | MS-SIPAE security associations, signature buffers, Authorization header, verification with replay window | AE-05, AE-10, AE-11, AE-29…AE-33, K-AE-4 |
| `commscraft-register` | L2 | Sign-in state machine: REGISTER → NTLM handshake → signed 200 OK → GRUU/expiry → refresh → sign-out; client version block (RG-50); deregister/survivable mode NOTIFY | AE-20…AE-33, RG-03…RG-32, RG-50, RE-03, RE-06, K-RG-4 |
| `commscraft-xml` | L0 | Bounded XML reader without DTD/entities, namespaces, explicit stack | docs/security.md |
| `commscraft-provisioning` | L3 | Provisioning SUBSCRIBE, `provisionGroupList` parser, validated ServerConfiguration (HTTPS URLs with domain check, SIP URIs, port ranges) | RG-40…RG-44, K-RG-4 |
| `commscraft-psom` | L3 | PSOM GenericInt, String, RPC frame | PO-04, PO-05, K-PO-2, K-PO-3 |

Dependencies (decision D-03, pure Rust): RustCrypto `md4`, `md-5`, `hmac`, `sha1`, `aes`, plus
`subtle` (constant-time comparison) and `zeroize` (wiping keys) in `sipae`. `sip-msg`, `sipcomp`,
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
| L3 | `presence`, `im`, `conf`, `telephony`, `exchange`, `webservices` | Feature protocols (MS-PRES, MS-CONFBAS/AV/IM/AS, MS-OCPSTN, EWS, UCWA, MS-DLX, MS-ABS, MS-E911WS) |
| L4 | `session` | One signed-in account: orchestrates features, exposes commands and state |
| L5 | `ui-egui` | Desktop user interface |
| adapters | `net-tokio`, `audio-pipewire`, `video-*`, `rdp-*`, `gssapi` | OS and library bindings (subject to D-03) |
| apps | `commscraft`, `commscraft-cli` | Desktop app; command-line tool for offline checks against fixtures |

## Linux integration (planned)

- Audio via PipeWire with echo cancellation; video capture via PipeWire/V4L2.
- Screen sharing viewer first; sharing one's own screen later via xdg-desktop-portal.
- Credentials and certificates in the system keyring (Secret Service).
- URL handlers for `lync:`, `lync15:` and `conf:` meeting links.
