## 0.14.0 — Authenticated-response gating and truthful UI state (offline)

- Added `sfb-core::session::RegisterVerificationGate`: one-transaction coordinator
  tying RFC 3261 response matching to MS-SIPAE MIC/replay verification.
- For matching 200 responses, MIC verification is required before recording
  `MicCheckedNotSignedIn`; a valid test MIC never means a live login.
- Fail closed on invalid correlated 200 or malformed 401/407; ignore unrelated
  responses without consuming the gate, and terminate on timeout/TLS loss.
- Prevent simulated `RegistrationAccepted` UI event from becoming `Registered`;
  the demo instead shows `AcceptanceUnverified`.
- Added eleven module tests (authored, Rust runtime **not executed**),
  security evidence ledger, historical-hash regression check and CI job.
- Old v0.11–v0.13 hashed source files preserved; v1.0 parity gate still blocks.

## 0.13.0 — Bounded SIP response router (offline only)

- New `sfb-router` crate with 64 in-flight transactions, 128 recently closed identities and Timer F.
- Strict response match includes Via transport as well as branch, sent-by, Call-ID and CSeq.
- Duplicate final responses, mismatches, expired transactions and TLS disconnect state are separate outcomes.
- Added 11 unit tests, three framed-wire integration tests, offline `--router-demo` and separate CI workflow.
- Preserved v0.11 and v0.12 source-evidence hashes; added RFC/MS-to-code/test ledger.
- Rust compilation, server interoperability and 1.0 feature parity are still **unverified**.

## v0.12.0 — Read-only TLS SIP receive transport

- New `sfb-transport` crate with validated, domain-bound TLS targets,
  OS chain/hostname validation and no plaintext fallback.
- Existing opt-in TLS probe now goes through the same transport component.
- New `sfb-core::inbound` receive-only framed SIP stream with offline
  negative tests and explicit transient/fatal IO handling.
- Added section-specific RFC/MS evidence, hashes and static-check script.
- Release v1.0 remains blocked; no real REGISTER/sign-in is claimed.

## 0.11.0 — Offline REGISTER / MIC integration (2026-10-15)

- Add `RegisterFlow::observe_mic_checked_200`: RFC 3261 transaction identity preflight, cautious binding parsing, signed `Expires` consistency check, MS-SIPAE signature and replay validation **before committing** a 200 response.
- Introduce explicit `MicCheckedAcceptance` / `MicCheckedUnregistration` states: neither claims an authenticated live login or a trusted TLS/Contact identity.
- Introduce `MicCheckedRegisterResult::RegistrarReportedBinding` with clearly untrusted GRUU and an 80%-lifetime non-normative refresh suggestion.
- Add nine offline Rust integration tests including signed success/unregister, bad MIC, mismatched transaction, bad endpoint/expiry, missing Authentication-Info, untrusted GRUU, and preservation of old unverified `observe` behavior.
- Add requirement-to-code-to-test evidence, checksums and dedicated CI test command. **Rust compiler / Windows AD / real server interop remain unverified.**

## 0.10.0 — Kerberos native per-message MIC bridge (2026-10-15)

- Add `kerberos_mic.rs` context-bound adapter connecting existing MS-SIPAE
  `ServerMicVerifier` and `ClientMicSigner` to a trusted Kerberos per-message backend.
- Add Windows-only SSPI `QueryContextAttributesW`, `MakeSignature` and
  `VerifySignature` FFI with native signature sizing, bounded buffers and QOP checks.
- Keep incomplete contexts blocked; do not alter replay state on invalid MIC.
- Add offline Rust mock tests, Python source-level checks and a normative
  requirement-to-code-to-test ledger (`docs/kerberos-mic-evidence.md`).
- Repair obsolete `ChallengeOffer` test constructor to include `gssapi_data`.
- **Not verified:** Rust build, Windows SSPI execution and SfB server interoperability.
  No working login; 1.0 Windows parity is still blocked.

## 0.9.0 — Experimental Windows Kerberos SSPI token adapter (2026-10-15)

- Add Windows-only, RAII-owned `Secur32` Kerberos outbound credential/context
  adapter (`AcquireCredentialsHandleW`, `InitializeSecurityContextW`, `CompleteAuthToken`).
- Use Kerberos package explicitly; pin the requested service principal;
  do not downgrade to NTLM/Negotiate or serialize native handles.
- Fix offline Kerberos initiation to allow nonempty GSS CONTINUE_NEEDED tokens;
  keep incomplete contexts in `AwaitingContextContinuation`, never authenticated.
- Add `sfb-inspect --sspi-check` local credential-acquisition self-check,
  native ABI layout unit test and Windows CI smoke step.
- Add MS-SIPAE/RFC/Win32 API traceability at `docs/windows-sspi-kerberos.md`.
- **Unverified:** Windows compilation, native domain authentication, GSS MIC,
  live REGISTER, server interoperability and v1.0 feature parity.

## 0.8.0 — Kerberos REGISTER challenge planning (2026-10-15)

- Add `kerberos.rs` offline GSS/SSPI provider boundary (default denies all credentials).
- Pin trusted `sip/<FQDN>` from deployment policy; select exactly one Kerberos offer, reject untrusted target, ambiguity and downgrade.
- Check challenge Call-ID/CSeq/Via branch binding and retry with new REGISTER CSeq/branch.
- Generate MS-SIPAE v3 Authorization/Proxy-Authorization with bounded external GSS token (`gssapi-data`).
- Add redacted challenge token parsing, safe `SipMessage` debug output, offline negative tests, CLI failure demonstration, documentation and CI smoke check.
- **No live Kerberos implementation, verified server MIC, TLS-DSK/NTLM auth or client sign-in.** Release 1.0 blocked.
- Rust compilation/test execution not confirmed in authoring environment.

## 0.7.0 — Offline MS-SIPAE outgoing MIC (2026-10-15)

- Add `ClientMicSigner` interface and separately keyed TLS-DSK HMAC-SHA256 client signer. Never reuse the server direction key.
- Build §3.2.4.1 / §3.3.5.3 exact-case, ordered outgoing MIC buffer and client `Authorization` / `Proxy-Authorization` headers for already-established, locally verified SAs only.
- Generate `crand` via OS CSPRNG and advance per-SA `cnum` only after valid signed drafts; negative tests for forged headers, mismatched CSeq, invalid state, expiration and failing signer.
- Synthetic end-to-end sign demo, deterministic HMAC test vector independently checked by Python, and CLI/CI smoke test.
- Recheck MS-SIPAE 11.0 and MS-SIPREGE 17.1 current published revision; release parity requirements unchanged (122/93).
- **No real authentication, interoperability, verified REGISTER or operational features added. Rust build/test execution remains unverified due missing compiler.**

## 0.6.2 — Windows Full-Client 1.0 release contract

- Froze an explicit no-exclusions acceptance scope for Windows Full Client 1.0.
- Release evidence now requires an exact four-part Windows reference build and SHA-256 fingerprint.
- Every Windows reference case requires two-way cross-client test results, policy-on/off behavior, negative/recovery tests and real artifacts.
- Added tests protecting the contract against narrowing and placeholder reference builds.
- **No new SIP/media interoperability was implemented.** v1.0 remains blocked.

## 0.6.1 (2026-10-14) — Strict Full Windows-client parity contract

- Expanded to 122 distinct client requirements, including 55 previously hidden or aggregated Windows behaviors.
- Introduced 93 independent acceptance scenarios from the official Microsoft Windows Full client feature-comparison categories; the list is a *minimum* until the exact installed client build is inventoried.
- Hardened v1.0 release gate to require per-scenario real-server/cross-client evidence, exact Windows build pinning, feature inventory review and difference resolution.
- Added negative tests for deleted reference scenarios and broken mappings.
- No new working sign-in, messaging or media functionality; v1.0 is **not complete**.

## 0.6.0 (2026-10-14) — Windows feature parity scope and offline primitives

- Added 67-item machine-readable Windows desktop client acceptance matrix; fail-closed v1.0 gate.
- Offline RFC 6665/RFC 3856 PIDF SUBSCRIBE + NOTIFY dialog/state/CSeq validation.
- Offline IM INVITE response/ACK state and text/plain MESSAGE request/response handling.
- Offline bounded RFC 8866 SDP structural inspector; no media-negotiation claims.
- Added synthetic `sfb-inspect --protocol-demo` and unit/negative tests.
- Still no real GSS/TLS-DSK client, secure login, contacts, IM transport, AV, conference or screen sharing.
- Rust build and cargo test execution still unverified in this offline authoring environment.

# Changelog

## 0.5.0 — Incoming MS-SIPAE signature and replay primitives (2026-10-14)

- Add offline MS-SIPAE SecurityAssociation/AssociationTable, with scheme/realm/
  target/UAS-or-proxy identity, bounded lifetime and establishing/established/expired states.
- Parse server Authentication-Info/Proxy-Authentication-Info safely; produce the
  ordered angle-bracket MIC buffer from §3.2.5.2 without altering wire casing.
- Add HMAC-SHA256 MIC verification against an externally supplied TLS-DSK server
  authentication key (RFC 2104), plus a 257-value inclusive replay window. MIC
  MUST validate before a sequence number is accepted.
- Add synthetically signed/invalid fixture, negative tests and `sfb-inspect --sa-demo`.
- Add exact normative mapping in `docs/sipae-security-association.md`.
- **Not a real login:** GSS/TLS-DSK handshake, trusted service identity,
  outgoing signing and authenticated network REGISTER remain unimplemented.
- **Build not tested in authoring environment:** missing Rust toolchain.


## 0.4.0 — Offline REGISTER state and Microsoft endpoint identity (2026-10-14)

- Add `registration.rs` offline REGISTER/response planner and observer (never sends SIP).
- Construct RFC 3261 / MS-SIPREGE REGISTER envelope with `epid`, `+sip.instance`, `Event: registration`, `Supported: gruu-10` and bound Contact.
- Parse matching 200 OK Contact Microsoft `gruu`, per-Contact expiry, handle 401/407 as unauthenticated challenge, 423 with new CSeq/branch and bounded Min-Expires.
- Add own-Contact unregister / refresh plans, Timer F observation and unit coverage.
- Add `sfb-inspect --register-demo` offline scenario; RFC/MS traceability, security notes and CI smoke check.
- **No live REGISTER / GSS or authenticated MS-SIPAE SA**. No successful Rust builds are claimed.


## 0.3.0 — Microsoft discovery & TLS diagnostics (2026-10-14)

- Expanded the MS-CONMGMT DNS name plan to all four SIP SRV records;
  added restricted TLS candidate precedence and port-443 fallback targets.
- Added deterministic, bounded RFC 2782 SRV priority/weight selection with
  injected random source and regression tests.
- Added `sfb-probe`, a separate explicit-opt-in CLI for parallel SRV DNS
  lookup and a TLS-only, system-trust certificate/hostname-verified handshake;
  it **never sends SIP packets or credentials**.
- Implemented read-only MS-SIPAE 401/407 auth scheme/parameter parsing for
  NTLM, Kerberos, TLS-DSK and Digest with redacted Debug output.
- Added synthetic multi-challenge fixture tests and offline GUI/CLI diagnostics.
- Updated the protocol matrix, architecture, threat model and roadmap.

**Limitations:** Not a functional Skype for Business sign-in client. No
Microsoft auth exchange, REGISTER over network, presence or IM. Build and tests
are not verified locally because the Rust toolchain is absent.

# Changelog

## 0.2.0 — Incremental offline protocol milestone (2026-10-14)

- Introduced a bounded incremental TCP/TLS SIP stream decoder that rejects
  absent Content-Length and invalid framing.
- Added a deterministic, reliable-transport-only non-INVITE client transaction
  model with branch, sent-by, Call-ID and CSeq correlation and Timer F timeout.
- Hardened outgoing wire serialization against CR/LF injection and oversized
  input; made encoding fallible.
- Added synthetic REGISTER/100/401 fixtures and tests for every two-chunk split.
- Added a local file/`--demo` SIP inspector that avoids printing identities.
- Added RFC 3261 transaction controls to the native diagnostics tab.
- Updated coverage, architecture, security docs and CI test steps.

No networking, authenticated sign-in, remote contacts, remote messaging,
audio/video, or server interoperability was introduced in this release.

**Build note:** Rust toolchain and network access were unavailable to the
artifact-building environment; the code was not compiled or run there.

## 0.1.0 — Initial offline prototype

- Clean-room Rust workspace with native egui local-only conversation demo.
- Basic SIP parser, REGISTER illustration, discovery names and session UI model.
