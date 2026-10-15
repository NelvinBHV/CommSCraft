# SFB Client RS

**Open-source, clean-room Skype for Business client research project in Rust.**

> **Version 0.14.0: security-gated offline REGISTER response coordinator plus previous router/TLS modules.**
> **NOT a working Skype for Business sign-in client.** No authenticated server
> sessions, real presence or messaging, calls, conferences, file sharing or desktop sharing are implemented.
> **v1.0 requires full Windows-client parity, not a version label.**

Inspired by the modular Rust architecture and original implementation approach
of [storytold/wordcraft](https://github.com/storytold/wordcraft). This is an
independent project, not affiliated with Microsoft or storytold.

## New in v0.14.0: secure REGISTER response gate (offline)

`RegisterVerificationGate` in `sfb-core::session` joins the existing
REGISTER transaction observer and Microsoft security association in a
single *offline* acceptance boundary. A correlated 200 OK needs a valid
MS-SIPAE MIC and replay validation before the gate can report
`MicCheckedNotSignedIn`. This is **NOT a verified live sign-in**: a
trusted TLS channel, native server-authenticated GSS context and real
interop test are still missing. A matching but malformed or unsigned 200
poisons this exchange, while a response for a different transaction leaves
it pending. Challenges and 423 responses require a fresh transaction.

The desktop demonstration no longer exposes `Registered` for a simulated
200 response. It reports `AcceptanceUnverified`, even after the happy-path
demo sequence.

```sh
cargo test -p sfb-core session --lib
python3 scripts/check_v014_evidence.py
python3 -m unittest discover -s scripts -p 'test*.py'
python3 scripts/check_release.py --release  # intentionally fails
```

[Read normative requirement-to-source-to-test evidence](docs/registration-gate-evidence.md).
These are authored Rust tests, **not executed** in this environment; there is
no confirmed Windows/SfB Server interoperability or signed-off 1.0 parity.

## Previously in v0.13.0: reliable transport response routing (offline)

New `sfb-router` crate tracks up to 64 simultaneous non-INVITE SIP client
transactions and correlates provisional/final replies by Via branch, sent-by,
transport, Call-ID, CSeq number and method. Duplicate/late finals, Timer F
timeouts, malformed and unrelated replies cannot promote a registration state.
A synthetic end-to-end framed-message scenario is available using:

```sh
cargo test -p sfb-router --all-targets
cargo run -p sfb-inspect -- --router-demo
python3 scripts/check_v013_evidence.py
```

[Complete RFC/MS references, code references, authored tests and evidence
limitations](docs/response-router-evidence.md). This is offline code; it cannot
transmit SIP, authenticate a user or receive real registrar replies. **Rust
compilation/Windows SfB interoperability are not yet verified.**

## Previously in v0.11.0: transaction-bound MIC-checked REGISTER responses

`RegisterFlow::observe_mic_checked_200` now performs a non-mutating RFC 3261
response transaction preflight, validates the claimed endpoint binding and a
strict Contact/Expires consistency subset, then verifies the MS-SIPAE MIC and
257-number replay window **before** committing the response. Invalid signatures,
foreign transactions and malformed bindings leave the transaction open and do
not consume a sequence number. A locally suggested refresh occurs at 80% of
reported lifetime, which is a non-normative heuristic.

**Important protocol limit:** MS-SIPAE does *not* sign `Contact` or the GRUU.
The result explicitly exposes `untrusted_gruu`, not a trusted routing target.
Only a separately verified TLS peer and channel can protect those fields; that
channel is **not** connected to the registration observer. This is an offline
integration component, **not** a working Skype for Business server sign-in.

See [normative mapping and test evidence](docs/verified-registration-evidence.md),
including negative tests and verification blockers.

## Previously in v0.10.0: Windows Kerberos MIC signing and verification

The experimental Windows Kerberos provider now exposes bounded SSPI
`MakeSignature` and `VerifySignature` operations after a completed context.
`KerberosMicBridge` binds the context to the expected Kerberos SA target and
connects the native operations to the existing offline `ClientMicSigner` and
`ServerMicVerifier` interfaces. Native signature buffer capacity comes from
`QueryContextAttributesW(SECPKG_ATTR_SIZES)`; tests cover fail-closed binding,
bounds and a synthetic MIC path, **not a real GSS exchange**.

**[Requirement-to-code-to-test evidence](docs/kerberos-mic-evidence.md)** records
all normative references, available checks, and missing Windows/SfB server
interop evidence. Native Windows compilation, real server MIC verification and
full authenticated SIP REGISTER are **still unverified**. The v1.0 parity release
gate remains closed.

## Previously in v0.9.0: Windows Kerberos SSPI token source

`WindowsKerberosProvider` wraps Windows `AcquireCredentialsHandleW` and
`InitializeSecurityContextW` with explicitly pinned Kerberos (no NTLM/Negotiate
fallback), RAII native-handle cleanup and bounded GSS tokens. The **initial
GSS context may legitimately return CONTINUE_NEEDED**; SIP token planning no
longer confuses that with a valid login. On Windows, `sfb-inspect --sspi-check`
tries a **local credential-handle acquisition only** (no service ticket or
network I/O). See [SSPI design, API references and limitations](docs/windows-sspi-kerberos.md).

**Native MIC FFI is now authored, but has NOT been compiled or tested on Windows. Real GSS completion over SIP and authenticated REGISTER remain unimplemented.** This release is not a working Windows SfB client.

## v1.0 target: complete Windows client parity

The **full Skype for Business 2019 Windows desktop client** on SfB Server 2019/SE is the acceptance baseline, not just a chat mock. The
[122-requirement feature acceptance matrix](FEATURE_PARITY.json) and
[93 individually testable Windows reference cases](WINDOWS_REFERENCE_CASES.json)
cover discovery,
mutual authentication, contacts, privacy-aware presence, conversations, files,
Enterprise Voice/PSTN, call handling, video, ICE/TURN/SRTP, conferences,
application/desktop sharing, platform integration and release security.

**Current milestone is 0.11.0 and NOT 1.0.** The release gate
`python3 scripts/check_release.py --release` fails until every supported
Windows feature is verified with real-server interoperability evidence.
`python3 scripts/check_release.py` validates the manifest while work is ongoing.
See also [Windows parity and release validation](docs/windows-release-validation.md).

## What is implemented

- Native egui desktop **demo** with local contacts, conversations, presence UI,
  SIP/REGISTER diagnostic controls and MS-SIPAE challenge scheme display.
- `sfb-core` with an offline Microsoft-specific REGISTER planner/observer, bounded SIP parser, validated encoding, fragment-aware stream
  decoder, reliable non-INVITE client transaction subset and Timer F model.
- Microsoft-specific DNS SRV planning per **[MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3)**:
  four SIP SRV names, internal/external precedence and TLS fallbacks.
- RFC 2782 priority/weighted SRV ordering, bounded and deterministically testable.
- `sfb-probe`, a *separate opt-in CLI*, can resolve the four SRV names in parallel
  using system DNS and perform a **TLS-only certificate/hostname validated
  handshake** to an authorized host. **It never sends a SIP request.**
- Offline transaction-preflight + MS-SIPAE MIC-checked REGISTER response acceptance (no trusted Contact/GRUU or live connection).
- Read-only REGISTER/registration response modelling: Microsoft epid/`+sip.instance`/Event/GRUU, 423 Min-Expires, new CSeq/branch, own-contact unregistration and strict **UnverifiedAcceptance** state. See [registration protocol](docs/registration-protocol.md).
- Offline MS-SIPAE security-association core: bounded `Authentication-Info` parsing,
  byte-ordered server MIC buffer, HMAC-SHA256 over a separately supplied TLS-DSK
  server key, 257-value anti-replay window and SA phases. New v0.7.0 **outgoing
  MIC signing of offline SIP drafts** with per-SA `cnum`, OS-random `crand`,
  distinct TLS-DSK client key, and `Authorization`/`Proxy-Authorization` generation.
  **No real TLS-DSK handshake, native GSS provider, signed-in REGISTER or authenticated
  SIP transport.** See [client signing](docs/client-mic-signing.md) and
  [security-association notes](docs/sipae-security-association.md).
- New v0.8.0 **offline Kerberos GSS-token handoff** after a correctly matched REGISTER
  challenge: explicit trusted `sip/<FQDN>` pinning, independent retry transaction,
  fail-closed Kerberos-only selection, bounded base64 token serialization, v3
  `Authorization`/`Proxy-Authorization` and no fake login. The default GSS provider
  explicitly rejects all authentication. See [Kerberos negotiation](docs/kerberos-negotiation.md).
- Read-only parsing of 401/407 challenges for Kerberos, NTLM, TLS-DSK and
  Digest based on [MS-SIPAE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/e60fa322-b73b-4e83-9ddb-4b20bd17a4a1).
  **No full authentication mechanism or server handshake is implemented.** The offline REGISTER module never opens a socket or verifies an SA. Even a synthetic successful REGISTER is never reported as an authenticated sign-in.
- Offline SIP PIDF presence `SUBSCRIBE` draft and `NOTIFY` observation: dialog IDs,
  event, subscription state, CSeq replay and content-type validation. No
  BENOTIFY, enhanced category XML, publication or live subscription.
- Offline IM dialog subset: INVITE result/ACK state, unsigned text/plain MESSAGE
  draft, response correlation and inbound text validation. No true INVITE/ACK
  transaction, route-set, session authentication or remote delivery.
- Read-only, bounded SDP structural inspection (not codec negotiation, ICE,
  RTP/SRTP or media playback).
- `sfb-inspect` to inspect synthetic local SIP fixtures without printing
  sensitive headers or authentication fields.

## Run locally

Install stable [Rust](https://rustup.rs) and development packages needed for
`eframe` on your OS. Linux may need X11/GTK and OpenSSL development libraries.

```bash
cargo test -p sfb-core
cargo run -p sfb-desktop                 # offline mock UI
cargo run -p sfb-inspect -- --demo       # offline SIP inspector
cargo run -p sfb-inspect -- --register-demo  # synthetic 423 and 401 flow
cargo run -p sfb-inspect -- --sa-demo        # synthetic HMAC verification + replay
cargo run -p sfb-inspect -- --signing-demo   # synthetic client-side MIC, no packets
cargo run -p sfb-inspect -- --kerberos-plan-demo # fails closed; uses NoKerberosProvider
cargo run -p sfb-inspect -- --sspi-check       # Windows-only local credential check, no network
cargo run -p sfb-inspect -- --protocol-demo  # offline presence/IM/SDP sequence
python3 scripts/check_release.py          # validate 122 requirements and 93 Windows cases
cargo run -p sfb-inspect -- tests/fixtures/sip/challenge.sip

# These default to OFFLINE. Only the second and third commands access the network:
cargo run -p sfb-probe -- --domain example.com
cargo run -p sfb-probe -- --domain example.com --dns
cargo run -p sfb-probe -- --domain example.com --tls-probe
```

**Important:** Only run live DNS/TLS probes against systems where you have
permission to perform connection tests. Neither probe logs in nor sends any
SIP packets, but they still create network traffic. The TLS probe constrains
endpoints to the requested SIP domain and its subdomains and stops on TLS
errors; there is no certificate-check bypass or TCP downgrade. This conservative
scope can exclude legitimate on-premises deployments whose SRV targets live in
another DNS domain.

**Verification status:** No `cargo`, `rustc`, or package-registry access was
available in the environment where this snapshot was produced. Rust builds,
format/lint and tests could not be run here. The source and manifest/file
consistency were checked statically. GitHub Actions is configured to exercise
build, tests and lint after publication. A live handshake must not be confused
with server identity authorization, authenticated SIP registration, or functional
Skype for Business interoperability.

## Workspace layout

```text
apps/sfb-desktop/          native offline UI
apps/sfb-inspect/          local SIP fixture analyzer, no networking
apps/sfb-probe/            explicit opt-in DNS-SRV and TLS handshake diagnostics
crates/sfb-core/src/       REGISTER planner/observer, parser, stream, SIP transactions, discovery,
                           challenge parser, offline Kerberos token handoff, Windows-only native SSPI token source, SA/MIC/replay, PIDF presence, IM, SDP
crates/sfb-core/tests/     offline fixtures and integration tests
tests/fixtures/sip/       credential-free synthetic SIP samples
tests/vectors/            independent public-key outgoing MIC test vector
docs/protocol-matrix.md   implementation-to-RFC/MS-spec coverage
docs/spec-traceability.md explicit normative links and scope decisions
docs/security.md          trust boundaries and remaining risks
docs/architecture.md      implemented vs future architecture
docs/registration-protocol.md RFC/MS REGISTER mapping and threat model
docs/sipae-security-association.md MS-SIPAE §3.2 verification mapping and gaps
docs/client-mic-signing.md MS-SIPAE §3.2.4.1 outgoing MIC and gaps
docs/kerberos-negotiation.md MS-SIPAE §3.2.5.1 Kerberos offline challenge planning
docs/windows-sspi-kerberos.md Windows native SSPI token source, limitations and API traceability
docs/windows-parity.md    Windows functional acceptance matrix from v0.6.0
docs/windows-release-validation.md  real-server/windows evidence and release signoff policy
WINDOWS_REFERENCE_CASES.json  93 independent Windows Full client reference scenarios
FEATURE_PARITY.json        machine-readable 122-feature release obligations
scripts/check_release.py  fail-closed Windows parity release gate
ROADMAP.md                progress and gaps
CHANGELOG.md              version changes
.github/workflows/       GitHub Actions CI
```

## Normative references

| Scope | Reference |
|---|---|
| SIP message framing and non-INVITE transactions | [RFC 3261](https://www.rfc-editor.org/rfc/rfc3261) |
| Microsoft-specific client DNS-SRV discovery | [MS-CONMGMT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3) |
| DNS SRV priority and weight | [RFC 2782](https://www.rfc-editor.org/rfc/rfc2782) |
| SIP server routing and security | [RFC 3263](https://www.rfc-editor.org/rfc/rfc3263) |
| Microsoft SIP authentication extensions | [MS-SIPAE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9) |
| Windows desktop client feature baseline | [Microsoft Full Windows client feature matrix](https://learn.microsoft.com/en-us/skypeforbusiness/plan/feature-comparison) |
| Microsoft REGISTER and provisioning | [MS-SIPREGE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d) |
| SIP presence subscriptions | [RFC 6665](https://www.rfc-editor.org/rfc/rfc6665) / [RFC 3856](https://www.rfc-editor.org/rfc/rfc3856) |
| Presence and privacy | [MS-PRES](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/741d682d-590b-4600-916e-8deeee426a5c) |
| Skype for Business IM use case | [MS-OCSPROT §2.5.9](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/f27a1335-dfb8-43b3-80d3-5354757c7c0c) |
| SDP base syntax | [RFC 8866](https://www.rfc-editor.org/rfc/rfc8866) |
| Codec and SDP extensions | [MS-SDPEXT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/cd17a549-b948-42a6-aa6b-fa707710faac) |

Microsoft's [MS-OCSPROT registration use case](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/45e475a0-b534-469a-892e-6bd6ae42fc6e)
requires authentication, a GRUU and registration/provisioning capabilities.
None can be inferred merely from DNS resolution or a successful TLS handshake.

The intended eventual compatibility target is licensed **on-premises Skype for
Business Server** (2015/2019/Subscription Edition), after authorized testing.
Skype for Business Online is retired. UCWA/mobile and Microsoft Teams
interoperability are separate protocol workstreams.

## Project principles

1. Original, clean-room implementation using published RFCs and MS Open Specifications.
2. Security by default: TLS-only opt-in probes, no invalid certificate acceptance,
   no automatic downgrade, no credential dumping and no unimplemented sign-in UI.
3. Reproducible protocol fixtures, tests and explicit gaps in the coverage table.
4. Protocol, networking, authentication, media and UI designed as separate layers.
5. Microsoft's trademarks remain Microsoft's; no copied binaries or branded assets.

## Publishing to GitHub

```bash
git init
git add .
git commit -m "Add MS-SIPAE offline association verification and replay protection"
git branch -M main
# Create your own empty repository first, then:
git remote add origin https://github.com/YOUR-USERNAME/sfb-client-rs.git
git push -u origin main
```

The archive has **not** been published to GitHub automatically. MIT license.

### Binding full Windows-client v1.0 contract

`V1_RELEASE_CONTRACT.json` explicitly forbids shipping v1.0 with reduced client
functionality. The Microsoft feature matrix and the pinned, real Windows Full client
executable are **both** required as reference sources. Every existing Windows-client
capability must be implemented and independently tested against Skype for Business
Server Subscription Edition with real Windows↔Rust client evidence. `v0.10.0` adds a bounded native MIC adapter; it does not provide interoperable communications.

See `docs/windows-release-validation.md` and run
`python3 scripts/check_release.py --release` to inspect remaining blockers.


## v0.12.0: read-only TLS SIP receive transport

The `sfb-transport` crate implements strict target restriction, OS-trusted
TLS hostname/certificate checking and a **receive-only** `TlsSipReceiver`.
The `sfb-probe --tls-probe` handshake now uses this same transport. The
`SipInbound` reader from `sfb-core` can be tested without opening sockets;
it handles fragmented/coalesced SIP frames, timeouts, EOF and malformed data.
No public SIP write method is available: verified TLS **does not** satisfy
Microsoft SIPAE server/client Security Association requirements. The client
cannot register or sign in with this version.

Evidence: `docs/tls-inbound-evidence.md` and `docs/evidence-v0.12.0.json`.
Commands: `cargo test -p sfb-core inbound`, `cargo test -p sfb-transport`,
`cargo test --workspace --all-targets`, and
`python3 scripts/check_v012_evidence.py`.
