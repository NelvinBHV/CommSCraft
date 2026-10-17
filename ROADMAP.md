# CommSCraft roadmap

**Stage:** experimental source baseline v0.14.0 · **next target:** v0.30.0 · **target application:** Skype for Business Windows Full desktop client with on-premises Server 2019 / Subscription Edition.

> **Last reviewed:** 2026-10-17 · **Reference:** [WordCraft's public roadmap format](https://github.com/storytold/wordcraft/blob/main/ROADMAP.md), adapted to CommSCraft's own feature inventory; this page uses original CommSCraft wording and no WordCraft code or assets. · **Development:** [certificate integration PR #35](https://github.com/NelvinBHV/CommSCraft/pull/35) and [roadmap PR #34](https://github.com/NelvinBHV/CommSCraft/pull/34).

This page is the **summary dashboard**. The machine-readable requirement inventory is [`FEATURE_PARITY.json`](FEATURE_PARITY.json), the independent Windows acceptance inventory is [`WINDOWS_REFERENCE_CASES.json`](WINDOWS_REFERENCE_CASES.json), and the detailed release requirements are in [`docs/release-policy.md`](docs/release-policy.md). The draft integration branch adds [a Python report generator](https://github.com/NelvinBHV/CommSCraft/blob/integration/v030-sip-san-six-prs/scripts/report_feature_parity.py) and [a dated progress report](https://github.com/NelvinBHV/CommSCraft/blob/integration/v030-sip-san-six-prs/docs/skype-for-business-feature-parity-status.md). Until merged, those are **draft changes**, not features of `main`.

## Headline numbers — 2026-10-17

| Metric | Value | Basis / interpretation |
| --- | ---: | --- |
| Windows Full Client feature requirements | **122** | counted in `FEATURE_PARITY.json` |
| Requirements marked `offline_subset` | **7 / 122 (5.7%)** | only an inventory label for partial offline code; **not** 5.7% usable parity |
| Requirements marked `not_started` | **115 / 122 (94.3%)** | conservative declared status even if isolated source may exist |
| Fully and independently verified features | **0 / 122 (0%)** | no real-client/server verification registered |
| Independent Windows reference cases | **93** | `WINDOWS_REFERENCE_CASES.json` |
| Reference cases with registered real-server interop proof | **0 / 93 (0%)** | not executed / not evidenced |
| v0.30.0 integration | **draft PR #35** | latest PR CI and review must be checked against its exact head |
| v1.0.0 | **BLOCKED** | complete Windows Full Client reference parity, reviewed security, `check_release.py --release` |

**No unsupported weighted parity or engineering-hours estimates.** The 5.7% partial-inventory ratio is not an end-to-end completeness measure. For a real user, authenticated sign-in, contacts, voice and collaboration have not been demonstrated. Work estimates require a measured throughput baseline, scoped reference environment and completed requirements; they are intentionally **not estimated** here.

## Feature breadth by area

| Area | Requirements | Registered offline partials | Verified | Primary stage |
| --- | ---: | ---: | ---: | --- |
| Identity / discovery / sign-in | 9 | 3 | 0 | v0.30–v0.50 |
| Contacts / presence | 22 | 1 | 0 | v0.60 |
| Instant messaging | 17 | 1 | 0 | v0.60 |
| Voice / telephony | 23 | 0 | 0 | v0.70 |
| Video / media | 5 | 1 | 0 | v0.70 |
| Meetings / sharing | 35 | 0 | 0 | v0.80 |
| Windows client / platform | 11 | 1 | 0 | v0.90 |
| **Total** | **122** | **7** | **0** | **v1.0 requires all** |

## Readiness by user workflow

| Workflow | Offline / source situation | Full Windows parity |
| --- | --- | --- |
| DNS and server lookup | Partial diagnostics; no complete real-world discovery | Not verified |
| Secure peer identity | RFC 5922 SAN tests and actual TLS peer certificate validation in receive-only transport; combined security draft PR #35 | Not verified |
| Kerberos / registration | Synthetic MIC/replay and REGISTER response handling only; not a connected authenticated sign-in | Not verified |
| Presence and contacts | Limited offline models and parsing | Not verified |
| Instant messaging | Offline partial models; no demonstrated bidirectional real-server messaging | Not verified |
| Audio / video calls | Incomplete; no verified full call flow | Not verified |
| Conferences / screen sharing | Not started per registry | Not verified |
| Windows UI and integrations | Partial local UI work; no Windows Full parity evidence | Not verified |

## Ranked next work

1. **Land verified v0.30.0 security integration:** CI of exact final PR #35 SHA; reconcile #26–#31, negative SAN and transport tests; independently review; follow [v0.30.0 gates](#completion-criteria-for-the-v0300-experimental-milestone).
2. **Close trust-binding gaps:** same original SIP-domain, TLS peer identity, service principal and REGISTER transaction; no downgrade or reuse after disconnection. Do not present a TLS handshake or synthetic MIC as authenticated sign-in.
3. **Complete native Windows SSPI / MS-SIPAE and authorized interoperability testing:** requires actual permitted Windows/SfB lab, pinned builds and sanitized proof; v0.50.0 gate.
4. **Deliver authenticated presence, contacts and IM** (v0.60), then **calling/media** (v0.70) and **conferencing/sharing** (v0.80) with negative-policy and recovery tests.
5. **Audit all 122 features and 93 reference scenarios** and finish independent security, packaging and parity release gates. No skipped Full Client features.

## Progress log

| Date | Change | Evidence | What it does **not** prove |
| --- | --- | --- | --- |
| 2026-10-17 | Coordinated certificate SAN hardening from #26–#31; URI length, control, duplicate-parameter and DNS-fallback safeguards | [draft PR #35](https://github.com/NelvinBHV/CommSCraft/pull/35), [SAN trace](https://github.com/NelvinBHV/CommSCraft/blob/integration/v030-sip-san-six-prs/docs/v030-certificate-integration-evidence.md) | Authenticated Windows sign-in or live interoperability |
| 2026-10-17 | Automatic milestone release blocker and tests | [`check_v030_milestone.py`](https://github.com/NelvinBHV/CommSCraft/blob/integration/v030-sip-san-six-prs/scripts/check_v030_milestone.py) | Independent sign-off without real artifacts |
| 2026-10-17 | Feature parity inventory: 122 requirements, 93 reference cases, 7 offline partials, 0 verified | `FEATURE_PARITY.json`, `WINDOWS_REFERENCE_CASES.json`, [report generator](https://github.com/NelvinBHV/CommSCraft/blob/integration/v030-sip-san-six-prs/scripts/report_feature_parity.py) | Usable feature completion or weighted parity |
| 2026-10-17 | WordCraft-style top-level status and milestone presentation adapted for CommSCraft | this file, [PR #34](https://github.com/NelvinBHV/CommSCraft/pull/34) | Changes on `main`, any new protocol implementation |

---

# Target milestone: v0.30.0 (planned, not released)

The repository currently describes **v0.14.0** as the implemented experimental source baseline. **v0.30.0** is the requested next target, not a claim of completed authentication, Skype for Business interoperability or a published release.

## Completion criteria for the v0.30.0 experimental milestone

- [ ] Coordinate and integrate SIP-domain certificate SAN hardening from PRs #26–#31 on **one reviewed integration branch**. Resolve shared `crates/commscraft-transport/src/sip_certificate.rs` conflicts deliberately, including overlapping #26/#27 evidence. Verify RFC 5922 (June 2010) §§7.1–7.3, RFC 5280 (May 2008) §4.2.1.6 and RFC 3261 (June 2002) §§19.1.1, 19.1.4, 25.1 against source paths, negative DER/URI SAN fixtures and executed results.
- [ ] Preserve the strict DNS/original SIP service reference domain, RFC 5922 URI/DNS SAN precedence, no wildcards, no silent TLS downgrade, no accidental CN fallback, and prevent user-provided flags from declaring a trusted peer. The *actual* peer certificate must be checked on the same TLS connection; separate host/SRV/CNAME authority from SIP domain identity.
- [ ] Keep Kerberos SPN, REGISTER transaction, Microsoft Security Association and MIC/replay acceptance distinct from offline hostname or SAN acceptance. Map publicly documented MS-SIPAE §3.2.5.2 and MS-CONMGMT §3.1.5 to precise source/tests, recording Microsoft Open Specifications publication revision from the authoritative source at the time of implementation.
- [ ] Establish deterministic, bounded offline tests for hostile SIP URI encoding, malformed certs/SAN, duplicate and reserved parameter names, SDP frame and unknown field handling, SIP response framing and status codes, failed trust/identity/policy decisions.
- [ ] Prove one consolidated SHA on ordinary GitHub-hosted `ubuntu-latest` and `windows-latest`: Rust formatting check, all workspace targets/tests, strict Clippy, Python unittest/evidence and `scripts/check_standard_runners.py`, with no hidden continue-on-error or skipped success. Track historical rustfmt baseline separately per Issue #9; do **not** mutate historic SHA-256 evidence.
- [ ] Record per-check CI run/job links, source-to-fixture mapping and explicit tested-offline vs unverified status. Require independent security review before any assertion of real authentication or interop.

**Not included in v0.30.0 claims:** a working authenticated production sign-in, real SfB connectivity, full Windows Full Client feature parity, or v1.0 eligibility. Live Microsoft testing requires explicit authorization, a pinned lab and sanitized independently reviewed evidence. Do not tag v0.30.0 until all scope-appropriate gates above are fulfilled and reviewed.

---

# Planned development stages from v0.30.0 to v1.0 (not released)

These are **proposed engineering gates**, not claims that particular functionality already exists. Each intermediate version requires an explicit scope review, source-to-test mapping, and successful new-head CI before any tag. Work may be reordered as security evidence requires; moving to the next stage is not automatic.

| Planned milestone | Required deliverables and independently checkable exit criteria | Status |
| --- | --- | --- |
| **v0.30.0 — certificate identity and CI integration** | Meet all v0.30.0 gates above: one consolidated branch for #26–#31, correct original SIP-domain trust, bounded hostile-input tests and ordinary Ubuntu/Windows checks at one SHA | **OPEN** |
| **v0.40.0 — trustworthy transport and transaction binding** | Bind actual verified TLS peer cert identity and pinned service-origin/SPN to the same reliable SIP transport/REGISTER transaction; reject mismatched SRV target, downgraded TLS, changed endpoints, stale/replayed state; offline adversarial tests and security review | **OPEN** |
| **v0.50.0 — Windows Kerberos/MS-SIPAE interoperability** | Complete native multi-step SSPI/GSS SA negotiation, bidirectional signing/verification, replay behavior, and actual authorized Windows lab REGISTER/refresh/unregister evidence with exact builds and sanitized traces; a parsed 200, acquired handle or offline MIC alone does **not** count | **OPEN** |
| **v0.60.0 — real presence, contacts and messaging** | Implement authenticated, policy-aware sync, subscribe/notify and bidirectional IM handling with retry/recovery, server deny cases, bounded parsing and actual Windows/SfB interoperability evidence | **OPEN** |
| **v0.70.0 — media, calls, devices** | Review and implement required offer/answer, ICE/STUN/TURN, SRTP, call state, device selection, mute/hold, and negative authorization/failure paths, with security and accessibility evidence against reference client | **OPEN** |
| **v0.80.0 — conferencing and collaboration** | Conferencing/focus, participants, transfers and sharing capabilities under server policy, including denied and disconnected recovery scenarios; repeatable bidirectional evidence and privacy review | **OPEN** |
| **v0.90.0 — complete Windows Full Client feature parity** | Every requirement in `FEATURE_PARITY.json` and every case in `WINDOWS_REFERENCE_CASES.json` implemented and independently verified with exact pinned reference-client and server builds, allow/deny, accessibility, recovery and Office integration; no exclusions | **OPEN** |
| **v0.99.0 — release candidate and independent security review** | Cross-platform release CI, format migration without historical hash edits, packaging/signing, threat model and penetration review, reproducible evidence/artefact hashes, and no outstanding high-severity risks; demonstrate the strict release gate passes | **OPEN** |
| **v1.0.0 — fully evidenced Windows Full Client parity** | **All** `V1_RELEASE_CONTRACT.json` and `docs/release-policy.md` conditions satisfied, independently reviewed real end-to-end interoperability, performance, security, accessibility, policy denied/recovery, with `python3 scripts/check_release.py --release` passing. No override or nominal version-only promotion. | **BLOCKED** |

## Cross-stage rules and reference evidence

- Keep the three statuses explicit: **source-only**, **tested-offline**, and **independently verified interoperable**. GitHub Actions success proves only the checks actually executed; it does not prove enterprise identity, Kerberos authentication or parity.
- For every protocol-affecting commit, record the **precise public document revision/date, URL and section**, code path, positive/negative test fixture, actual run/job outcome and any unverified integration dependency. Baseline examples: [RFC 3261 (June 2002) §§10,17,19](https://www.rfc-editor.org/rfc/rfc3261), [RFC 5922 (June 2010) §§7.1–7.3](https://www.rfc-editor.org/rfc/rfc5922), [RFC 8866 (January 2021) §5](https://www.rfc-editor.org/rfc/rfc8866), [MS-SIPAE §§3.2.4–3.2.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/), [MS-SIPREGE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/) and [MS-CONMGMT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/). Pin exact applicable Microsoft publication revisions at implementation time, not by assumption in this planning document.
- No real credentials or productive network tests without explicit authorization. Fail closed on certificate/hostname/SPN, trust chain, MIC/replay, malicious SIP/SDP data, and policy denial. Only `ubuntu-latest` / `windows-latest` GitHub-hosted runners; no paid CI and no TLS downgrade.
- Preserve immutable source archives, historical evidence ledgers and recorded SHA-256 bytes. Treat rustfmt migration [Issue #9](https://github.com/NelvinBHV/CommSCraft/issues/9) as provenance-controlled.
- **No merge to `main`, release tag or v1.0 claim by automation.** Owner approval and independent security/interoperability signoff are mandatory for release gating; missing authorized lab results are blockers, not checkboxes to waive.

---

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
- [x] Sanitized offline inspector `commscraft-inspect`
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
