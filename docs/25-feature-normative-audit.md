# RFC/MS normative and security source audit — 25 offline subsets
Date: 2026-10-17. Branch: integration/v030-sip-san-six-prs.
Result: **release BLOCKED**; source-to-test mapping reviewed, normative protocol compliance NOT verified.
Review limits: official Microsoft published-version indexes and public RFCs were checked; full version-pinned MS PDF clauses were **not independently compared line-by-line**. No live Microsoft enterprise tests were authorized. A green offline CI is never a substitute.

## Findings (risk descending)
- **B01 Critical**: 0/25 feature rows possess an independently verified exact MS revision + normative clause + fixture/negative-case + same-SHA execution + reviewer chain. Protocol compliance assertions prohibited.
- **B02 Critical**: 0/122 features have trusted, authorized server interoperability evidence. Offline MIC, SIP 200, SSPI handle, call draft or roster do not indicate login or functional parity.
- **B03 High**: 18 development features are local-only data/UI models, not MS protocol implementations. Aggregate 25/122 (20.5%) measures offline inventory only.
- **B04 High**: The `codecs` label overstates `sdp.rs`. [RFC 8866 January 2021 §5](https://www.rfc-editor.org/rfc/rfc8866#section-5) describes SDP field syntax; [RFC 3264 June 2002 §§5–8](https://www.rfc-editor.org/rfc/rfc3264) offer/answer is **not implemented**.
- **B05 Medium**: Previous audit erroneously identified desktop file `apps/commscraft-desktop/src/main.rs`; correct path is `apps/commscraft/src/main.rs`. Corrected in evidence ledger; no UI behavioral fixture verified.
- **B06 Medium**: MS-CONMGMT, MS-ABCH, MS-SIPRE, MS-OCSPROT precise editions remain unverified. Edition catalog checks for MS-SIPAE 11.0 (2025-05-20), MS-SIPREGE 17.1 (2025-08-19), MS-PRES 13.0 (2025-05-20), MS-SDPEXT 16.0 (2025-05-20), MS-CONFIM 11.0 (2025-05-20), MS-CONFBAS 15.0 (2025-05-20) do not prove clause implementation.
- **B07 Medium**: [MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3) requires four parallel SRV lookups; [RFC 2782 February 2000 §3](https://www.rfc-editor.org/rfc/rfc2782#section-3) specifies SRV selection. `discovery.rs` plans requests and prioritization but performs no DNS I/O and selects TLS-only candidates. Clarify compatibility limitation.
- **B08 Medium**: Unit fixtures are not independently linked to each MS MUST/SHOULD; real TLS peer/SPN/SIP-domain authorization, state recovery, policy denial and replay across reconnect require lab fixtures.
- **B09 Medium**: The original 7/122 matrix is a hash-pinned archival file; editing it caused CI integrity failure. Preserve unchanged and maintain overlay, source/fixture links and clear scope labels.

## Seven historical feature-by-feature findings
| Feature | RFC / Microsoft section evidence | Source and real fixture names | Missing |
| --- | --- | --- | --- |
| dns-discovery | RFC 2782 §3; MS-CONMGMT §3.1.5 (edition unpinned) | `discovery.rs`: `microsoft_names_cover_four_srv_services`, `weight_and_priority_follow_rfc2782`, `rejects_invalid_identity_or_hostnames` | Actual network lookup, MS precedence/fallback full test |
| auth | RFC 2743 §2; RFC 2104 §2; MS-SIPAE 11.0 code cites §§3.2.5.1–3.2.5.2 (unverified clause) | `security.rs`: `signed_response_establishes_then_replay_is_rejected`, `invalid_signature_never_consumes_replay_number` | Kerberos/NTLM/SSPI, authenticated connection and server binding |
| register | RFC 3261 §§10.2/17.1.2; MS-SIPREGE 17.1 code cites §§3.1, 4.1.1, 4.1.2 (unverified clause) | `registration.rs`: `microsoft_register_has_required_identity_and_capability`, `ms_sipae_challenge_cannot_be_auto_authenticated` | Live signed REGISTER, policy and token proof |
| presence-sub | RFC 6665 §4, RFC 3856 §6; MS-PRES 13.0 exact extension clause open | `presence.rs`: `creates_unsigned_but_valid_subscribe`, `receives_initial_notify_before_subscribe_response_and_rejects_replays` | BENOTIFY, category sync, live policy |
| im-1to1 | RFC 3261 §12, RFC 3428 §3; MS-SIPRE / MS-OCSPROT editions and exact clause open | `im.rs`: `invite_ack_message_and_sip_acceptance_not_read_receipt`, `malformed_and_replayed_incoming_text_is_rejected` | Authenticated IM and transport |
| codecs | RFC 8866 §5 syntax; RFC 3264 §§5–8 unimplemented; MS-SDPEXT 16.0 exact clause open | `sdp.rs`: `inspection_does_not_negotiate_media`, `rejects_truncated_and_malformed_sdp` | Offer-answer, codecs, media |
| windows-ui | No standalone RFC for local UI; MS-OCSPROT overview cannot prove UI parity | `apps/commscraft/src/main.rs`, NO feature-specific fixture identified | Windows UI workflow, accessibility and recovery testing |

## Remaining 18 local-only findings
The ledger `release/offline-feature-spec-audit.json` maps **each** of the following to its concrete Rust source and named positive/negative unit tests:
`contact-groups`, `presence-publish`, `im-history` in `client_offline.rs`;
`contact-search`, `contact-privacy`, `im-richtext` in `local_features.rs`;
`calendar-presence`, `photos`, `distribution-groups` in `auxiliary_offline.rs`;
`im-multiparty`, `im-file`, `audio-call`, `hold` in `interaction_offline.rs`;
`forwarding`, `transfer`, `conference-roster`, `devices`, `call-history` in `call_drafts.rs`.
All 18 are local models with no network protocol claim. For `transfer`, [RFC 3515 April 2003 §2](https://www.rfc-editor.org/rfc/rfc3515#section-2) concerns REFER and is **not implemented**. For `hold`, RFC 3264 offer/answer is **not implemented**. Local microphone labels are not OS device enumeration; local file-offer metadata is not a safe remote transfer; local contact blocking is not server-enforced privacy.

## Mandatory remediation / release decision
Each protocol change requires exact dated revision PDF, specific normative paragraph, concrete source and fixture, malicious-negative vector, exact CI SHA/result, threat analysis and independent reviewer decision. CI on both standard Ubuntu and Windows plus authorized sanitized lab/server traces are additionally required. Never mark a feature 'verified' from local code or a normative-reference hyperlink. No main merge or v0.30/v1.0 tag justified by this audit alone.

## Executable RFC trace gate (2026-10-17)

Four deliberately scoped RFC requirements now have uniquely identified records in `release/rfc-requirement-evidence.json` mapping normative document date/section/URL, a limited behavioral claim, Rust source and named positive and negative tests. `scripts/test_rfc_requirement_evidence.py` checks source and fixture existence and **fails closed** on unverifiable execution claims; it runs via the existing Python unittest discovery workflow. Initial state is `fixture_mapped_execution_pending` for all four. No `executed_same_sha` attestation has been made until successful exact-SHA CI results can be attached. No RFC claim constitutes Microsoft product interoperability; MS-specific clauses still require independently confirmed dated PDFs and manual review. No history ledger was modified. Next steps: establish same SHA results, then version-pin each MS requirement separately against its official text; do not use generic catalog links as proof.

## Addendum: 30 local offline subsets (2026-10-17)

The prior 25-feature assessment remains as a historical snapshot. Five newly authored local-only models bring development inventory to **30/122 (24.6%)**: `presence-alerts`, `im-tabs`, `im-importance`, `notifications`, `meeting-lobby`. All five are in `crates/commscraft-core/src/usability_offline.rs`, with separate Rust tests `alerts_reject_duplicate_and_injection`, `tabs_reject_duplicate_and_close_cleanly`, `importance_rejects_control_characters`, `notifications_are_bounded`, `lobby_is_terminal`. Positive and negative scenarios cover valid local state and rejection of duplicates, invalid text, capacity overflow or invalid terminal transitions.

Reference families **MS-PRES**, **MS-OCSPROT**, **MS-SIPRE**, and **MS-CONFBAS** are **contextual only** here; no revision-specific normative section has been implemented, verified or claimed for any of these five records. They do not send a presence NOTIFY, send an IM, deliver Windows notifications, enforce server importance policy or connect to a meeting lobby. See `release/offline-feature-spec-audit.json` for one record per feature and `release/offline-feature-development.json` for the immutable-source-safe overlay. Tests and release outcomes must be tied to the exact HEAD SHA; full verified interoperability remains **0/122**.
