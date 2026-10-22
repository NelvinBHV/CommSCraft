# Cross-endpoint integrity, replay and routing identity — MS-SIPAE + MS-SIPRE

Research date: 2026-10-22. **Document analysis only; incomplete overall corpus.** No implementation or real-server tests.

## Direct original normative sources and revision caveat

1. [MS-SIPAE §3.3.5.3, Processing Authorized Messages from the SIP Client](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/47b1baf7-ceae-40bc-b1bf-8c26a1ec885a), HTML updated 2021-11-16.
2. [MS-SIPAE §3.2.5.2, Processing Authenticated Messages from the SIP Server](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c), HTML updated 2021-11-16.
3. [MS-SIPAE §3.2.5.1, Processing Challenges](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9), HTML updated 2021-08-17.
4. [MS-SIPRE §3.2.4.1, User Agent Operation](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipre/413f4e70-729e-40b9-b75b-8baa3ef2a329), HTML updated 2021-08-17.
5. [MS-RTP §1.4, Relationship to Other Protocols](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtp/dea055e9-5873-4bd2-aeaf-22c88fa56c05), Microsoft subsection.
6. [RFC 3261](https://www.rfc-editor.org/rfc/rfc3261), June 2002, §§12 and 17.

All MS HTML section dates precede some later published compiled MS-* revisions, including MS-SIPAE 11.0 (2025-05-20). **Edition correspondence must be verified against the actual published PDF**. The original subsection titles and roles are material: server §3.3.5.3 is NOT a client-signing prescription §3.2.4.1.

## Incoming vs outgoing protected SIP messages: evidence-backed comparison

| Dimension | Client sends; server verifies [MS-SIPAE §3.3.5.3] | Server sends; client verifies [MS-SIPAE §3.2.5.2] |
|---|---|---|
| Header/claim | Authorization or Proxy-Authorization with `response` | Authentication-Info or Proxy-Authentication-Info with `rspauth` |
| SA lookup | Server locates per-client SA via prior Via/Record-Route reference, opaque or endpoint ID | Client matches SA by scheme, realm, authentication target (Kerberos adjusts `sip/` service descriptor) |
| Directional nonce and sequence | `crand` and `cnum` | `srand` and `snum` |
| Signed values | ordered angle-delimited list involving scheme, nonce, seq, realm, target, Call-ID, CSeq number/method, From URI/tag, conditional To URI, To tag, conditional SIP/tel identity, Expires, conditional response code | similar ordered structure, but roles/available identity header sources differ; compare original normative text instead of assuming byte-equivalence |
| Cryptographic authority | NTLM/Kerberos GSS_VerifyMic or TLS-DSK HMAC using **client** authentication key | NTLM/Kerberos GSS_VerifyMic or TLS-DSK HMAC using **server** authentication key |
| NTLM SSPI signature verification | fixed message sequence number **100** | fixed message sequence number **100** |
| Integrity failure | Server rejects authorized request, 401/407 if request | Client discards failed message |
| Replay check | **per-SA cnum** freshness sliding window, reject duplicates or >256 behind maximum | **per-SA snum** freshness sliding window, reject duplicates or >256 behind maximum |
| Accepted next action | Version-4 server SA `waiting for signature` flag may clear; server removes authorization header and continues SIP processing | Client may mark SA established; verified 403 has special revocation path |

Detailed server-side verification in §3.3.5.3 step 2 lists **16 positions**; absence of a field in an applicable position produces `<>`, whereas omitted conditional positions produce **nothing**, and all header value case/syntax must be preserved. For version >=3, the outgoing client protected-identity fields may originate from **P-Asserted-Identity or P-Preferred-Identity** in that server-verification section; the incoming direction should not automatically inherit the same selection. This distinction matters for proxy/registrar behavior and signature canonicalization.

**Crucial evidence boundary:** Server-side verification is invaluable corroboration of expected protected client message format, but it is not proof that every client-side signing step in §3.2.4.1 has been extracted. That original client subsection and compiled edition remain open.

## MS-SIPRE endpoint identity requirements (§3.2.4.1)

The user agent **MUST** put its locally generated `epid` parameter in the **From** header of every request, whether or not inside a transaction. Dialog state defined by RFC 3261 §12 is extended with **remote epid**. A UAC copies it from the received To `epid` (or uses empty when absent); a UAS copies it from the From `epid` (or empty when absent). When addressing an existing SIP transaction with nonempty remote epid, the UA includes that epid on the To header. For calls to one specific endpoint, the UA **SHOULD** obtain AoR and endpoint epid from previous dialog or presence data, put the AoR in Request-URI/To URI and the epid in To.

This establishes a cross-spec identity chain: **registration Contact and endpoint identities (MS-SIPREGE / MS-SIPRE) → presence-distributed endpoint identity (MS-PRES) → exact endpoint-directed SIP dialogs (MS-SIPRE) → protected identity inputs if signing (MS-SIPAE)**. The arrows describe dependencies, not guaranteed identical strings or an unverified exact server exchange.

## Media link from original MS-RTP §1.4

SIP call initiation and RTP are connected through **MS-SIPRE §3.14**, while actual RTP transport protocol/IP/port is commonly conveyed using **MS-SDPEXT §3.1.5**. With multiple candidate paths, the source identifies **MS-ICE / MS-ICE2 §3.1.4.8.3** as advanced connectivity mechanisms. Thus media reconstruction requires at least: SIP session/dialog → SDP codecs and transport candidates → ICE/ICE2 path selection → RTP/RTCP processing and security. A SIP INVITE alone is neither a media negotiation trace nor proof of call establishment.

## Research / conflict checks

- **SEC-01:** Verify exact MS-SIPAE rev 11.0 versions of §§3.2.4.1, 3.2.5.1, 3.2.5.2 and 3.3.5.3; don't interpolate client signing byte representation from server counterpart alone.
- **SEC-02:** Preserve correct directional keys, `crand/cnum` versus `srand/snum`, identity header choice and two independent anti-replay windows.
- **SEC-03:** Determine precise semantics for mismatched/expired SA, signed re-challenge, clock skew, transaction retry and version 3/4 fields across both client and server roles.
- **ID-01:** Verify EPID computation (§3.2.3) and +sip.instance / GRUU / presence endpointId relations under MS-SIPRE §2.2.5, MS-SIPREGE and MS-PRES, including multiple endpoints per AoR.
- **MED-01:** Inspect complete MS-SIPRE §3.14, MS-SDPEXT §3.1.5, MS-ICE/ICE2 §3.1.4.8.3 and MS-RTP; determine historical Windows client/AV deployment version applicability.
- **REG-01:** Full MS-SIPREGE rev 17.1 registration and provisioning syntax/sequence still unreviewed.
- **CORPUS-01:** Complete transitive normative reference collection and implementation-independent full Windows client feature matrix still outstanding.

**Status:** newly verified first-party server-direction signature/replay rules, MS-SIPRE dialog EPID rules and MS-RTP inter-spec cross-references. Full specifications, published edition comparisons and complete client behavior remain incomplete. No source changes, real-server tests, merge or release.
