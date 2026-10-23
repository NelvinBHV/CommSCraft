# Research log and continuation checkpoint

## 2026-10-23 — Initial catalog / first source chains

**Git branch:** `draft/protocol-spec-analysis-2026-10-23` from `main`. **No main changes, no PR/merge/tag/release, no protocol implementation and no network or interoperability tests with Skype for Business servers.** Only documentation files under `docs/protocol-research` were authored.

### Inspected evidence

- Repository instructions: `AGENTS.md`; `README.md`, `ROADMAP.md`, `FEATURE_PARITY.json`, `WINDOWS_REFERENCE_CASES.json`; pre-existing `docs/122-feature-rfc-ms-reaudit.md`, `docs/25-feature-normative-audit.md` and `docs/dns-register-clause-audit-2026-10-18.md`. The 122-feature audit says **0 normative-verified entries** and **0 independently interoperable features**.
- Official Microsoft published-version tables inspected for eight documents: MS-OCSPROT 9.0 (2025-05-20), MS-CONMGMT 10.0 (2025-05-20), MS-SIPAE 11.0 (2025-05-20), MS-SIPREGE 17.1 (**2025-08-19**), MS-SIPRE 13.0 (2025-05-20), MS-PRES 13.0 (2025-05-20), MS-CONFBAS 15.0 (2025-05-20), MS-SDPEXT 16.0 (2025-05-20). The catalogue contains official release URLs and prior versions where checked.
- Sections inspected from public Microsoft Learn: MS-OCSPROT §§2.2.2.1, 2.2.2.2, 2.2.3.1, 2.2.3.2, 2.5.2, 2.5.9 and 2.5.17; MS-CONMGMT §3.1.5; MS-SIPREGE §3.1.2.5.1; MS-SIPREGE §1.2.1. These section pages **do not automatically attest to the precise PDF release revision**. Text access to complete published MS-OCSPROT 2025 PDF was unavailable through research browser redirect restrictions. Historical MS-OCSPROT rev 4.4, dated 2015-03-30, was seen and a page-render used to verify historic structure, not used for modern normative behavior.
- Direct official RFC Editor text inspected: RFC 2782 (2000) selected §3 and RFC 3261 (2002) §§10.2.1–10.2.8, 10.3 (selected), 17.1.2 (selected). **Neither RFC is fully analyzed.**

### Authored checkpoint artifacts

- `README.md`: method, scope, completion/stop conditions, citation policy.
- `source-catalog.md`: published revisions, unpinned candidate documents, RFC seed, missing sources.
- `dependency-matrix.md`: draft source adjacency and unresolved normative closure.
- `discovery-registration-analysis.md`: actual selected Microsoft discovery and registration rules.
- `rfc3261-rfc2782-clause-review.md`: selected fixed-edition RFC data, transaction diagram and timers.
- `flows-and-states.md`: seven client-flow families with fact/inference/unknown annotations.
- `full-client-coverage.md`: whole-client area matrix, open feature-to-clause crosswalk.
- `full-122x93-traceability.md`: enumerates **all 122 feature IDs**, maps all **93** baseline reference cases (all referenced feature IDs resolve), flags **31** feature IDs with no direct scenario; snapshots exact baseline blob SHAs. Status is metadata only and **does not assert** 122 normatively complete.

### Blocking findings, not conjectures

- **B-01 / source edition:** current Microsoft published release PDF/DOCX full content, dated changelogs and applicability footnotes **not yet read**. Live web sections cannot substitute for edition-pinned line-by-line comparison.
- **B-02 / normative closure:** all MS-OCSPROT referenced MS-* specifications and recursive normative references **not exhaustively traversed**; no complete reference matrix.
- **B-03 / auth trust:** full MS-SIPAE 11.0, MS-CONMGMT TLS peer matching, Windows Kerberos/NTLM/SSPI, TLS-DSK proof/key schedule **not verified**; registration 200 cannot establish sign-in by itself.
- **B-04 / client coverage:** requirements for presence, contacts, address book, IM, media, ICE/TURN, conferences, sharing, enterprise voice, PSTN, policy failures and UI integrations **largely unexamined** at protocol-clause level.
- **B-05 / version conflicts:** MS-SIPREGE 17.1 has post-overview Aug 2025 update; v17.0 → v17.1 delta not examined. RFC 3265 → 6665, SDP 4566 → 8866, ICE 5245 → 8445 and historical GRUU draft references require intentional profile choices.
- **B-06 / baseline provenance:** README v0.14.0 vs other audit references to development v0.30; build and release provenance not reconciled. Exact Windows Full Client build baseline still UNPINNED.
- **B-07 / product proof:** no user-owned sanitized reference captures or server configuration evidence was supplied; no testing attempted (per instruction). This is independent of whether documents can be completed.

### Highest-value next research iteration, in order (no implementation)

1. Acquire **official current published** MS-OCSPROT 9.0 PDF or DOCX, record stable content digest and primary URL, read every client-relevant section; extract **all** linked MS-* identifiers and each normative reference with document version, index them in a machine-readable catalog.
2. Read and pin **MS-CONMGMT 10.0, MS-SIPAE 11.0, MS-SIPREGE 17.1, MS-SIPRE 13.0 and MS-PRES 13.0** complete applicable §§1–5 and appendices, with release-specific changes; reconcile earlier live selected snippets; capture every field, algorithm, state, timer, failure and cryptographic trust rule.
3. Expand to directories, IM and conference clusters (MS-ABS, MS-DLX, MS-ABCH, MS-SIPAPP, MS-CONFBAS/CONFPRO/CONFIM/CONFAV/CONFAS/XMLMC), then media and traversal clusters (SDPEXT, RTP/SRTP, ICE/TURN), then telephony and sharing.
4. Traverse RFC-3261 **Updates** chain and all RFC normative graphs; keep the Microsoft-required edition distinct from newer replacements. Build a complete release-specific dependency edge dataset.
5. Link each extracted requirement to `FEATURE_PARITY.json` and `WINDOWS_REFERENCE_CASES.json`, all positive/negative/policy/expiry/reconnect cases, and provenance/conflicts; review full scenario/transaction/dialog state machines.
6. Only after every source chain and open question is dispositioned produce **source-derived future implementation plan**. Do **not** implement, connect to servers, run live tests, merge or release. Continue on this exact branch.

**Quality gate:** Research documents are committed but **unreviewed draft research**; their paragraphs are not acceptance evidence and no coverage percentage should be construed as a compliance estimate. The completion checklist in README is entirely unchecked.

## How to resume

Start by inspecting this log, `source-catalog.md`, `README.md`, the latest branch commit history and `AGENTS.md`. Avoid redoing already cataloged Microsoft release-row checks except to confirm later updates. Close the earliest unresolved dependency per source, retain current status labels and exact edition/dates, record new confirmed references, and commit additional **documentation only** on this branch.
