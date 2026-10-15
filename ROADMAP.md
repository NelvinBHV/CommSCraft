## Current milestone v0.14.0 — fail-closed REGISTER response coordinator

- [x] Model MIC-checked REGISTER 200 separately from server-verified sign-in.
- [x] Close a single-exchange gate after malformed matching final 200/401,
  timeout or transport disconnect; allow unmatched responses to be ignored.
- [x] Ensure UI simulation never displays signed-in merely from SIP 200.
- [ ] Authenticated SSPI/GSS handshake on Windows, trusted TLS binding and
  actual SIP REGISTER/refresh transaction on a validated server.
- [ ] Complete full Windows-client feature parity and bidirectional interop.

## Current milestone v0.13.0 — offline RFC 3261 transaction router

- Added bounded concurrent non-INVITE response dispatcher and isolated reliable-TLS transaction identities.
- Added synthetic framed-wire integration tests and a read-only CLI demo.
- **No real REGISTER, signed network transmit, Kerberos interoperability, contacts, IM, voice/video, or Windows parity.**
- 1.0 remains conditional on real bidirectional Windows-Full-client feature parity evidence.

# Roadmap — source status as of 0.11.0

[x] source code implemented; [~] partial; [ ] missing. Builds and server
interoperability have **not** been verified in this environment.

## M0 — Native UI and offline models
- [x] Modular Rust workspace and local egui contacts/chat demo
- [x] Offline SIP message parsing, encoding and registration simulator

## M1 — Baseline protocol correctness
- [x] Bounded, strict SIP stream framing and Content-Length validation
- [x] RFC 3261 reliable non-INVITE client transaction matching and Timer F
- [x] Synthetic SIP fixtures and fragmentation tests
- [x] Sanitized offline inspector `sfb-inspect`
- [ ] Complete SIP grammar, INVITE/dialog states, full transaction timers and fuzzing

## M2 — Microsoft discovery and authentication (in progress)
- [x] Four MS-CONMGMT SIP DNS-SRV names and service ordering
- [x] RFC 2782 bounded, weighted SRV priority/weight ordering
- [x] CLI opt-in real DNS queries (4 parallel queries)
- [x] CLI opt-in TLS-only handshake with certificate-chain/hostname checks
- [x] Read-only MS-SIPAE 401/407 offer parsing (NTLM/Kerberos/TLS-DSK)
- [ ] DNSSEC, DHCP option 120, HTTPS autodiscover, proxies and full discovery routing
- [ ] Certificate and service-origin identity binding in full Microsoft context
- [ ] Long-lived async SIP/TLS transport and reconnect/keepalive management
- [x] Offline SA identity, incoming server MIC buffer, HMAC-SHA256 test verification and replay window (partial MS-SIPAE)
- [ ] Full MS-SIPAE Kerberos/NTLM GSS and TLS-DSK mutual-auth handshake + PRF
- [x] Windows-only Secur32 Kerberos credential handle + GSS token-source adapter (v0.9.0; unverified on Windows)
- [x] Native Windows Kerberos GSS MIC FFI + SA binding (source only, Windows compilation/unverified)
- [ ] Real verified MIC on a domain-joined Windows system, production REGISTER integration
- [x] Offline Authorization/Proxy-Authorization signed *draft* for a separately verified SA, OS-random crand / per-SA cnum (v0.7.0)
- [x] Offline v3 Kerberos token-bearing Authorization/Proxy-Authorization planning with pinned SPN and GSS provider boundary (v0.8.0)
- [ ] Native SSPI/GSS implementation, full v4 negotiation, authenticated real REGISTER and refresh
- [x] Offline REGISTER planner: epid, +sip.instance, Microsoft GRUU extraction, 401/407/423 and 200, CSeq and safe unregistration plans
- [x] Offline transaction-preflight + MIC/replay-verified REGISTER 200 observer (v0.11.0; Contact/GRUU remain untrusted without TLS peer evidence)
- [ ] Real authenticated REGISTER, security-association verification, provisioning and refresh
- [ ] Authorized end-to-end interop lab with SfB Server 2015/2019/SE

## M3 — Messaging and presence
- [x] Offline RFC 6665 PIDF presence SUBSCRIBE/NOTIFY state + replay/identity checks
- [x] Offline SIP IM dialog/message subset with synthetic tests
- [ ] Real Microsoft contact synchronization, enhanced MS-PRES categories and BENOTIFY, full IM/dialog processing
- [ ] End-to-end delivery acknowledgments and retry behaviors

## M4 — Media and conferences
- [x] Bounded read-only RFC 8866 SDP structural inspection; no offer/answer
- [ ] MS-SDPEXT ICE/STUN/TURN RTP/SRTP and devices
- [ ] Conferencing focus/C3P and application sharing

## M5 — Product hardening
- [ ] Independently verified cross-platform builds, CI and security review
- [ ] Windows/macOS/Linux packages, signing, accessibility

## M6 — Strict Windows parity release gate (new, mandatory)
- [x] 122 Windows-desktop capability requirements mapped to protocol families
- [x] 93 separately evidenced Windows Full client acceptance scenarios based on Microsoft Server 2019/SE feature comparison
- [x] Machine-readable `FEATURE_PARITY.json` and fail-closed release policy
- [ ] Inventory of exact installed Windows Full client build, not only Microsoft documentation
- [ ] Every one of 93 reference cases and all 122 requirements complete and tested on actual Windows build and SfB Server
- [ ] v1.0 signed off after cross-client interoperability, security and performance validation

**Release rule:** Do not tag or advertise 1.0 until `python3 scripts/check_release.py --release`
passes *and* independent reviewers confirm evidence. Existing UI samples are not counted.

## v0.6.2 scope hardening (no false 1.0)

- [x] Immutable Windows Full client no-exclusions contract and source documentation.
- [x] Require pinned reference build, executable fingerprint and bidirectional evidence.
- [ ] Complete and test all Windows client features on supported SfB Server Subscription Edition.
- [ ] Pin real reference executable, OS, client build and actual test server from authorized lab.
- [ ] Demonstrate every policy, failure, accessibility and Office-integration parity case.
- [ ] Validate all mandatory client behaviors with the installed Windows Full client and human review.

## v0.7.0: next critical engineering tasks

- [x] Separate client/server TLS-DSK signing-direction key interfaces for offline HMAC SHA-256.
- [x] Exact outgoing sign-buffer order; safe request mutation and fail-closed sequence handling.
- [ ] Implement audited GSS/SSPI backend or TLS-DSK inner handshake, server identity checks, and bidirectional proof-of-possession.
- [ ] Real REGISTER with challenge/response, signed server response, and in-band provisioning, with authorized test-server traces.
- [ ] Authenticated presence/categories, messages, voice/media, conferencing and Office integration.
- [ ] Verify all 122 requirements / 93 minimum Windows Full reference scenarios with cross-client evidence before v1.0.

## v0.8.0: Kerberos challenge flow (offline integration boundary)

- [x] Pin the service principal; reject unexpected Kerberos realm-target pair and no NTLM fallback.
- [x] Bind challenged REGISTER to response Call-ID, CSeq and Via branch.
- [x] Plan an independent CSeq+1/register retry and serialize externally generated v3 GSS token.
- [x] Reject missing native GSS provider; tests with fake provider are offline only.
- [ ] Implement audited native GSS/SSPI provider and verify actual signed server response.
- [ ] Live authentication, registration, feature parity and Windows 1.0 remain blocked.

## v0.9.0: Windows SSPI token-source experiment

- [x] Add explicit Windows Kerberos credential acquisition and opaque token-source adapter.
- [x] Track incomplete initial GSS context without falsely claiming authentication.
- [ ] Compile and test on Windows using an available Rust toolchain.
- [ ] Execute permitted Active Directory / Skype for Business Server integration tests.
- [ ] Implement native GSS per-message MIC, signed REGISTER and replay binding.
- [ ] Continue to block 1.0 until all Windows features meet the release contract.

## v0.10.0: Windows Kerberos per-message MIC (experimental)

- [x] Add completed-context-gated `MakeSignature`/`VerifySignature` + SSPI size query.
- [x] Bind the Kerberos MIC provider to a single pinned `SaKey` (scheme, realm, target); reject missing context.
- [x] Connect native methods to offline SIPAE MIC buffer signer/verifier and preserve MIC-before-replay ordering.
- [x] Add proof ledger with normative MS/RFC and Windows API citations and negative source-contract checks.
- [ ] **Run** native Windows compile/SSPI tests, real signed SIPAE traces, interop and full Windows client parity.


### v0.12.0: TLS-bound receive-only channel (auth still blocked)
- Added DNS-domain-bounded TLS-SRV target validation, OS certificate checks,
  and no downgrade after handshake failure.
- Offline stream receive tests for `Content-Length`, CRLF keep-alive,
  partial reads, coalesced frames, timeout and fail-closed EOF.
- **NOT implemented:** SIP writes, completed authenticated REGISTER,
  Kerberos mutual-auth lab proof, Contact/GRUU service binding, and the
  remaining Windows client functionality. v1.0 cannot be released.
