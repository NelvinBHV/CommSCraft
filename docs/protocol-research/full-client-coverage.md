# Windows Full Client protocol coverage and requirement mapping (DRAFT)

**Baseline:** `FEATURE_PARITY.json` and `WINDOWS_REFERENCE_CASES.json` are the project reference inventories. The existing [122-feature re-audit](../122-feature-rfc-ms-reaudit.md) reports **122/122 entries inventoried but 0 normatively verified and 0/122 independently verified interoperability**; this is an evaluation of audit evidence, not proof no code exists. `docs/25-feature-normative-audit.md` reports overlapping gaps. This research branch does **not** modify immutable files or change those audit verdicts.

**Metadata discrepancy:** Repository README calls its project v0.14.0, while other Oct 2026 documents refer to an overlay and a v0.30 development/release gate. Those documents describe **different snapshots**; the exact current shipped/default-branch version and overlay state are **NOT resolved** by reading the README alone. Reconcile latest branch/commit and release evidence before treating a version label as baseline.

**Coverage scale:** `CATALOGED` = candidate document known; `SELECTED_CLAUSES` = a few sections actually inspected; `DEEP_REVIEW` = complete relevant sections, normative refs, errors, timer and security clauses crosschecked for pinned release. **No area qualifies for DEEP_REVIEW** as of 2026-10-23.

| Client capability / acceptance gate | Primary source candidates | Research coverage | Blocking unknowns |
| --- | --- | --- | --- |
| Startup, DNS, DHCP, roaming/internal/external discovery | MS-CONMGMT §§3.1.4–3.1.5; RFC 2782, 3361, 3263, 1034/1035 | SELECTED_CLAUSES | full return ordering, network policies, proxy, versioned client behavior |
| TLS / certificate trust / transport persistence | MS-CONMGMT, TLS RFC 5246/8446, PKIX 5280/9525 | CATALOGED | exact target matching, certificate lifecycle, revocation, keep-alives |
| Kerberos/NTLM/TLS-DSK authentication and service identity | MS-SIPAE, MS-KILE, MS-NLMP, MS-OCAUTHWS, RFC 2743/4120 | SELECTED_CLAUSES (only overview challenge) | complete token exchanges, proof, downgrade/fallback, replay |
| SIP REGISTER / refresh / sign-out / GRUU | MS-SIPREGE, MS-SIPRE, RFC 3261 | SELECTED_CLAUSES | client states, expiry, 423, all error codes, provisioning formats |
| Presence publish, receive, permissions / notes | MS-PRES; RFC 3265/6665, 3856, 3903 | CATALOGED | categories, privacy access, BENOTIFY, subscriptions and recovery |
| Contact groups, directory, photos, address book and search | MS-ABS, MS-DLX, MS-ABCH (candidate from existing audit) | CATALOGED | actual download/update protocols, LDAP/web services, delta sync, policies |
| One-to-one / multipart IM, persistence and file transfer | MS-SIPAPP, MS-SIPRE, MS-CONFIM, RFC 3261, 3428 | SELECTED_CLAUSES (overview §2.5.9) | message bodies, file mechanisms, rejection, read receipts, IM federation |
| Audio calling, Enterprise Voice and PSTN | MS-OCPSTN, MS-OCEXUM, MS-EUMR, MS-DTMF, MS-SDPEXT | CATALOGED | routing/policy, codecs, ringing, hold, transfer, recovery |
| Video, codecs and SRTP | MS-SDPEXT, MS-RTP/RTVPF/H264PF, MS-SRTP/SSRTP, RFC 3264, 3550, 3711 | CATALOGED | offer/answer, encryption, keying, codec formats, device switching |
| Edge, STUN/TURN and ICE interoperability | MS-ICE/ICE2/ICE2BWM, MS-TURN/TURNBWM, MS-AVEDGEA, RFC 5245/8445 | CATALOGED | generation negotiation, credential/ticket lifetimes, consent and failure |
| Conferences, lobby, roster, meeting controls | MS-CONFBAS, MS-CONFPRO, MS-CONFIM, MS-CONFAV, MS-CONFAS, MS-XMLMC | SELECTED_CLAUSES (overview §2.5.17) | C3P error bodies/authorization, conference lifecycle, multi-MCU |
| Desktop/app sharing, whiteboard and collaboration | MS-RTASPF, MS-RDPBCGR, MS-PSOM and referenced Office formats | CATALOGED | data-channel security, content rights, presenter controls, policy denied |
| Emergency calling / delegates / response groups / federated extensions | MS-E911WS, MS-RGSWS, MS-OCPSTN, MS-OCAUTHWS | CATALOGED | routing and product support under server/policy combinations |
| Windows integration (SSPI, devices, notifications, accessibility) | Microsoft platform specifications / Win32 APIs; product build / feature comparison | CATALOGED | exact Windows client build, OS behavior, client-only local features |
| Reconnect, timeout, retry, sign-out, trust revocation | RFC 3261 §§8–12,17; MS-CONMGMT/SIPAE/SIPREGE; media security | CATALOGED | exhaustive state transitions, timer values, replay and error mapping |

## Traceability / data model (not yet populated with 122 per-clause records)

Required final per-capability record:

`feature_id → windows_scenario_id(s) → Microsoft release revision/date/§ → RFC edition/date/§ → message/field/guard/timer/error/threat → confidence → policy cases → recovery cases → open issues → approved implementation requirement ID (only after completion)`.

Exactly **122/122 catalog IDs and 93 reference scenarios** are intended as the baseline, not 122 normatively audited paragraphs. Neither feature name nor raw count establishes completeness; new actual-client differences may increase the target set.

### Example mapping: existing IDs from original inventory

| Feature ID | Scenario ID when verified | Source clause to inspect | Completion proof needed |
| --- | --- | --- | --- |
| `dns-discovery` | to cross-link | MS-CONMGMT §3.1.5; RFC 2782 §3 | every candidate/failure/override, signed endpoint selection, policy |
| `auth` | to cross-link | MS-SIPAE §§2–3; RFC 2743 | all auth mechanisms, mutual proof, replay, error and credential lifecycle |
| `register` | to cross-link | MS-SIPREGE §§2–3; RFC 3261 §10 | registration, GRUU, expiry, provisioning, sign-out and failures |
| `presence-sub` | `status-watch` (present in WINDOWS_REFERENCE_CASES) | MS-PRES, RFC 3265/6665 | categories, privacy, initial/late NOTIFY, resubscribe |
| `im-1to1` | to cross-link | MS-OCSPROT §2.5.9; MS-SIPAPP; RFC 3261 | INVITE/ACK/MESSAGE, negative/timeouts and delivery semantics |
| `codecs` | to cross-link | MS-SDPEXT; RFC 3264/8866 | codec/SDP compatibility, negotiation, recovery, media |

## Negative-case matrix all feature families must satisfy (specification study first)

For every protocol transaction, document independently: (a) happy path, (b) malformed/oversized messages and unsupported versions, (c) cryptographic identity failure and replay, (d) server policy denial and missing entitlement/service, (e) expiry/timeouts/transport failover, (f) resource cleanup on sign-out, (g) race/reordering/retransmission, (h) cross-version negotiation and unsupported extension behavior. A local mock or syntax parser cannot substitute for real server semantics.

## What will count as Phase-4 completion

1. Full **version-pinned source-to-clause extraction**, with all relevant and excluded MS-OCSPROT/expanded normative cross-links explained.
2. One complete per-feature requirement graph for every Windows build & server profile, including exact on-wire data and negative paths.
3. Side-by-side RFC-vs-Microsoft conflicts and product-version delta table with decisions.
4. Full sequence/state/timer tables; per-state trust prerequisites and threat assumptions.
5. Reviewed, source-driven **future** implementation plan; separate authorization to actually implement.
6. User-owned interop validation (not performed by this research effort). No release gate relaxation.

The fully enumerated baseline ID map is in [full-122x93-traceability.md](full-122x93-traceability.md); all 93 Windows-case feature references resolve, and 31 features have no baseline case. **No claim of complete documentation or client parity is made.**
