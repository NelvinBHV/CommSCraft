# MS-SIPAE security association — prototype v0.7.0

> Offline-only, test-key based, **not** a live sign-in or complete authentication implementation.
> No security assertion about a real Microsoft Skype for Business server is possible with this code.

This module is a clean-room reading of the publicly documented protocol, not
Microsoft or proprietary client source. Section numbers refer to **MS-SIPAE 11.0
(published 2025-05-20)** and relevant RFCs, although some online section text
may be from older updates.

## What exists

| Normative rule | Reference | Implementation | Verification |
|---|---|---|---|
| Client keeps an SA per protocol, realm, target and proxy/UAS | MS-SIPAE §3.2.1 | `security.rs::SaKey`, `AssociationTable` | origin mismatch, separated entries |
| SA phases establishing/established/expired | MS-SIPAE §§3.2.1, 3.2.6 | `SecurityAssociation` | transitions, expiration |
| Server `Authentication-Info`/`Proxy-Authentication-Info` | MS-SIPAE §§2.2.2, 3.2.5.2 | strict bounded parser | duplicate/missing/malformed rejection |
| Ordered UTF-8 signed data, each field in `<...>` | MS-SIPAE §3.2.5.2 step 2 | `server_mic_buffer()` | byte-exact regression assertion |
| Verify server `rspauth` *before* checking `snum` | MS-SIPAE §3.2.5.2 steps 3–6 | `verify_server_message()` | tampered MIC does not consume sequence |
| Replay sliding window `SnumHigh-256..=SnumHigh` | MS-SIPAE §§3.2.1, 3.2.5.2 | `ReplayWindow` | boundary / duplicate / too-old |
| TLS-DSK HMAC and server authentication key | MS-SIPAE §3.2.5.2 step 3; RFC 2104 | `TlsDskHmacSha256`, supplied key | public synthetic known HMAC-SHA256 fixture |
| 403 on not-yet-established SA invalidates it | MS-SIPAE §3.2.5.2 step 7 | `SecurityAssociation` | controlled state transition |
| Verified response can transition the *SA* to established | MS-SIPAE §3.2.5.2 step 7 | `SecurityAssociation` | synthetic end-to-end test |

The sender-specified spelling and case of the headers/URI/tag/realm/targetname
matter to the signed buffer. Header fields absent from the message are encoded
as empty `<>`, except conditionally omitted fields. The parser deliberately
rejects SIP address forms not covered by its small subset. RFC 3261 contains
additional address grammar not yet supported.

## Crucial missing components

1. The GSS provider for **Kerberos** (`GSS_Init_sec_context`, `GSS_VerifyMic`,
   ticket/SPN validation and `MS-KILE`) and **NTLM** (`MS-NLMP`, SSPI fixed
   message sequence 100) is **NOT** implemented. `ServerMicVerifier` is an
   adapter interface, not a cryptographic GSS implementation.
2. **TLS-DSK handshake, client certificate provisioning** (`MS-OCAUTHWS`),
   certificate validation, EAP-PRF key derivation and supported negotiated hash
   selection are **NOT** implemented. HMAC-SHA256 here is usable only if an
   external, trusted TLS-DSK implementation yields the correct **server** key.
   Plain SIP-over-TLS does *not* yield this key.
3. **Outgoing MS-SIPAE Authorization / Proxy-Authorization, cnum/crand,
   client-to-server MIC drafting** is now available **offline only** for an
   already-established SA with an externally supplied TLS-DSK client key.
   Version-4 waiting-for-signature, all 401/407 handshake legs, GSS token
   processing and actual network signing are **NOT** implemented. See
   [outgoing MIC details](client-mic-signing.md).
4. Real SIP transport, service-identity binding, transaction-integrated
   verification, REGISTER/refresh/provisioning and long-lived SA lifecycle
   are **NOT** wired into the desktop client. An `UnverifiedAcceptance`
   cannot be treated as a verified registration.
5. HMAC-SHA256 is only one possible TLS-DSK handshake hash; negotiated
   SHA-1 or other suites are not silently substituted. Unsupported ciphers
   fail by absence of a verifier.
6. The sample key `[0x5a; 32]` is **public test data**, never a live secret.
   This library cannot guarantee secret residency, memory locking or GSS
   provider correctness. The key wrapper attempts wipe-on-drop; auditing is
   required before credentials are handled.

## Offline demonstration

```bash
cargo test -p commscraft-core
cargo run -p commscraft-inspect -- --sa-demo
cargo run -p commscraft-inspect -- --signing-demo
```

`--sa-demo` verifies a fixture signed with the fixed public test key, then
rejects a replay. It sends **no packets** and accepts **no password**.

## Primary sources

- [MS-SIPAE overview and published revision](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd)
- [§3.2.1 Abstract Data Model](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/82a3b226-f1e4-460f-b823-ca42cc251410)
- [§3.2.5.1 Challenge handling](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9)
- [§3.2.5.2 Verify server messages](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c)
- [§3.2.6 Expiration](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/15c359a8-7824-45e7-b8e8-36c48d87b434)
- [RFC 2104 — HMAC](https://www.rfc-editor.org/rfc/rfc2104)
- [RFC 2743 — GSS-API](https://www.rfc-editor.org/rfc/rfc2743)
- [RFC 3261 — SIP](https://www.rfc-editor.org/rfc/rfc3261)
