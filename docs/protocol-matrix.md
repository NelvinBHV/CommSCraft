# Protocol coverage matrix — v0.10.0

**All statuses are research-module coverage only**, not claims of build success,
client feature completion, real-server compatibility or Microsoft certification.
The code is not yet an operational Skype for Business client.

| Layer | Specification | Implemented in repo | Still missing for real Windows parity |
|---|---|---|---|
| SIP framing, encoding and stream | [RFC 3261 §§7, 18.3](https://www.rfc-editor.org/rfc/rfc3261) | Bounded UTF-8/CRLF framing, safe outbound encoding, streaming decoder | Complete ABNF, routing, multipart, retransmissions, dialog and transaction variants |
| SIP non-INVITE transactions | RFC 3261 §17.1.2 | Reliable-client model, correlation, provisional/final/timeout states | Live TLS SIP transaction loop, server transactions, proper response retransmissions |
| DNS-SRV and TLS probe | [MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3), [RFC 2782](https://www.rfc-editor.org/rfc/rfc2782), [RFC 3263](https://www.rfc-editor.org/rfc/rfc3263) | 4 SRV query names, weighted/priority ordering, opt-in TLS-only diagnostic | Full MS client discovery, proxy/DHCP, proper service identity association, persistent transport |
| Challenge inspection | [MS-SIPAE §3.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9) | Read-only 401/407 Kerberos/NTLM/TLS-DSK offer parser including redacted `gssapi-data` | Actual GSS/SSPI/TLS-DSK credentials/handshake/token exchange |
| Kerberos challenge retry (new in 0.8.0) | [MS-SIPAE §3.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9), §4.2, RFC 2743 | Offline one-step v3 GSS token handoff; pinned SPN, CSeq+1, Via branch, 401/407 identity and source, no native provider | Real GSS/SSPI provider, multi-round and v4 negotiation, verifiable Kerberos-signed server response, live registration |
| Server response MIC and SA | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c), [RFC 2104](https://www.rfc-editor.org/rfc/rfc2104) | Offline incoming HMAC-SHA256 with external server-direction key, replay window, SA state | Trusted server identity and handshake, GSS verification, live response binding |
| **Outgoing client MIC (new in 0.7.0)** | [MS-SIPAE §§3.2.4.1, 3.3.5.3](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/47b1baf7-ceae-40bc-b1bf-8c26a1ec885a) | **Offline** ordered client MIC buffer, separate external client HMAC key, OS-random `crand`, per-SA `cnum`, Authorization / Proxy-Authorization | Handshake, actual GSS signer, v4 challenge-first-signature cases, live signed sessions |
| REGISTER/unregistration and GRUU | [MS-SIPREGE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d), [RFC 3261 §10](https://www.rfc-editor.org/rfc/rfc3261) | Offline REGISTER draft/observer, `epid`, `+sip.instance`, GRUU subset, `423` planning, honest `UnverifiedAcceptance` | Real signed REGISTER/refresh and in-band provisioning |
| Presence subscription | [RFC 6665](https://www.rfc-editor.org/rfc/rfc6665), [MS-PRES](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/741d682d-590b-4600-916e-8deeee426a5c) | Offline PIDF SUBSCRIBE/NOTIFY dialog, event/CSeq/state checks | MS enhanced category parsing/publication, contacts, privacy, BENOTIFY, real subscriptions |
| IM SIP subset | RFC 3261 §12, [MS-OCSPROT §2.5.9](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/f27a1335-dfb8-43b3-80d3-5354757c7c0c) | Offline INVITE/ACK/MESSAGE state and UTF-8 content checks | Live dialogs, error handling, Microsoft IM modalities, receipts/history, transfers |
| SDP | [RFC 8866](https://www.rfc-editor.org/rfc/rfc8866), [MS-SDPEXT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/cd17a549-b948-42a6-aa6b-fa707710faac) | Bounded read-only field inspection | RFC 3264 offers/answers, codec negotiation, ICE/STUN/TURN, SRTP, media |
| Conferences/screen sharing/Office | MS-CONFBAS/MS-CONFAV/MS-PSOM and other Office specifications | None | All conference, content, enterprise voice and Office integration implementations |
| Windows Full client parity | [Microsoft Server desktop feature comparison](https://learn.microsoft.com/en-us/skypeforbusiness/plan/feature-comparison) | 122 requirements and 93 minimum independent scenarios; release gate denies v1.0 | All real-server/Windows↔Rust test evidence, accessibility/platform integration and reviewed sign-off |

## Security and test evidence

The outgoing HMAC vector in `tests/vectors/client-mic-v4.json` uses a **public
synthetic client key**, separately from the public server-direction fixture.
`scripts/test_mic_vectors.py` independently verifies the vector with Python.
The Rust tests exercise the draft signing interface but could **not be run
here** without `rustc`/`cargo`; CI must check them. Real credentials and
network sessions must not be enabled on this codebase.

The authoritative progress for the intended final product is the fail-closed
`FEATURE_PARITY.json`, `WINDOWS_REFERENCE_CASES.json` and
`V1_RELEASE_CONTRACT.json`, **not this protocol table**.

| Native Windows Kerberos token source (v0.9.0) | [MS-SIPAE §3.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9), RFC 2743, Microsoft Win32 SSPI docs | Windows-only credential acquisition and bounded token generation, support initial `CONTINUE_NEEDED` offline | Windows build unverified, no server MIC verifier, authenticated REGISTER or live network flow |

## v0.10.0 MIC extension

| Feature | Spec | New source code | Outstanding evidence |
|---|---|---|---|
| Native Windows Kerberos per-message MIC | MS-SIPAE §§3.2.4.1/3.2.5.2, RFC 2743 §2.3, Windows SSPI MakeSignature/VerifySignature | `windows_sspi.rs`, `kerberos_mic.rs`; bounded signed inputs and context/SA target checks | Windows native compilation, live Kerberos server MIC verification, production registration integration **pending** |

See [exact source/requirement/test mapping](kerberos-mic-evidence.md).
