# Offline authentication-binding chain security review

**Scope:** draft PRs #42–#46, source-only and offline-tested. Review date: 2026-10-21. No production TLS/SSPI/REGISTER interoperability claim.

## Findings

### F1 — Owner identifier collisions (high if reused in production)
`crates/commscraft-transport/src/offline_session_identity.rs`: `new_fixture(owner: u64)` accepts any nonzero caller-supplied ID. Two distinct owners with the same ID and the same generation can accept each other's `FixtureSessionHandle` through `check_fixture`. The existing cross-owner test only uses *different* owner IDs. **Required before production:** owner identity must originate from the transport's unforgeable per-connection state, not a caller-supplied number; add a same-ID distinct-owner negative test.

### F2 — Generation saturation (medium, latent)
`connect_fixture` uses `saturating_add(1)`; at `u64::MAX`, reconnect retains the generation and may leave a prior handle valid. **Required:** fail closed on overflow or use a non-repeating connection identity, and add an overflow test.

### F3 — Fixture MIC flag is not authentication (production blocker)
`crates/commscraft-transport/src/offline_register_security_handoff.rs` accepts `mic_verified_fixture: bool` from the caller. A true flag proves only that the fixture selected a branch of test logic; it cannot attest to Kerberos/SSPI, SPN, MIC or message integrity. `authenticated_sign_in()` remains false. **Required:** keep fixture API segregated and require an independently verified, session-bound security-adapter result in any future production design.

### F4 — REGISTER Via branch is not a server authentication proof (production blocker)
`FixtureRegisterSecurityHandoff` compares a caller-supplied branch string. Matching does not prove the response belongs to the actual TLS connection or security association. The two offline branch validators also differ (128 vs 255 byte maximum and permitted ASCII punctuation), so no production parser should infer a unified policy. **Required:** define one reviewed transaction identifier type and bind it to real transaction state, not raw caller strings.

### F5 — Disconnection and failed checks (review observation)
Current `accept_fixture` checks consumption, session lifecycle, branch and fixture MIC before setting `consumed=true`. Replays after successful acceptance and disconnected/reconnected handles fail closed. A failed MIC or branch check does not consume the handoff; this allows later corrected fixture observations and must not be assumed safe for a real protocol state machine without explicit retry policy.

## Normative references and traceability

- RFC 5922 (June 2010) §§7.1–7.3: SIP-domain identity validation on the actual TLS peer. https://www.rfc-editor.org/rfc/rfc5922
- RFC 3261 (June 2002) §17.1.3: SIP client transaction matching. https://www.rfc-editor.org/rfc/rfc3261
- Microsoft [MS-SIPAE] §3.2.5.2: security-association handling (publication revision must be verified before production work). https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/

Code: `crates/commscraft-core/src/tls_boundary.rs`, `crates/commscraft-core/src/offline_session_adapter.rs`, `crates/commscraft-transport/src/offline_session_identity.rs`, `crates/commscraft-transport/src/offline_register_security_handoff.rs`.

Existing tests: `fixture_session_rejects_certificate_spn_transaction_replay_and_disconnect`, `fixture_adapter_rejects_unverified_mic_replay_expiry_and_disconnect`, `rejects_cross_owner_and_reconnect_stale_handles`, `rejects_missing_security_wrong_transaction_and_replay`, `rejects_reconnect_disconnect_and_foreign_owner`.

**Disposition:** Offline proof-of-concept only. F1/F2 need corrective tests and fail-closed changes before any broader adapter work; F3/F4 remain production blockers under Issue #18. Do not merge, deploy, run network interoperability tests or claim sign-in.
