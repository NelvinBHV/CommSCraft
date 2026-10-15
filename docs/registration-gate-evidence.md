# v0.14.0 — REGISTER response verification gate: RFC/MS traceability

**Evidence class:** source code, authored offline Rust tests, Python integrity checks.
**Not evidence of:** Rust binary compilation, execution of native SSPI,
active TLS trust, real Skype for Business registration, or Windows-client parity.
Reviewed against RFC 3261 and Microsoft Open Specifications on 2026-10-15.

| ID | Normative source | Implementation constraint | Code | Authored test / status |
|---|---|---|---|---|
| GATE-01 | [RFC 3261 §17.1.2](https://www.rfc-editor.org/rfc/rfc3261.html#section-17.1.2) | Non-INVITE provisional responses must not complete a registration; final 401/407/423 needs a new transaction | `crates/commscraft-core/src/session.rs::RegisterVerificationGate::observe` | `challenge_is_final_and_does_not_authenticate`, `challenged_final_response_must_not_be_followed_by_200` — Rust NOT RUN |
| GATE-02 | [RFC 3261 §17.1.3](https://www.rfc-editor.org/rfc/rfc3261.html#section-17.1.3) | Unrelated replies must not alter the pending transaction | `registration.rs::observe_mic_checked_200`, `session.rs::observe` | `unrelated_200_does_not_consume_gate` — Rust NOT RUN |
| GATE-03 | [MS-SIPAE §3.2.5.2 steps 1–4](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) | For a correlated 200, match SA by scheme/realm/target, create exact signed buffer and **verify server MIC** prior to any state advancement | `registration.rs::observe_mic_checked_200`, `security.rs::verify_server_message`, `session.rs::observe` | `synthetic_mic_checked_200_is_not_signed_in`, `untrusted_200_poisoned_without_consuming_synthetic_sa` — Rust NOT RUN |
| GATE-04 | [MS-SIPAE §3.2.5.2 steps 5–6](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) | Record an `snum` only after MIC and replay-window checks | `security.rs::SecurityAssociation::verify_server_message` | `bad_mic_in_synthetic_success_fails_closed` — Rust NOT RUN |
| GATE-05 | [MS-SIPREGE basic registration](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/bc4b0245-339d-490c-ba67-2410e1513ed9) | Do not equate a `200 OK` or GRUU claim with an authenticated, bound endpoint | `session.rs::GateState::MicCheckedNotSignedIn` and `SessionState::AcceptanceUnverified` | `demo_200_is_not_a_login`, `synthetic_mic_checked_200_is_not_signed_in` — Rust NOT RUN |
| GATE-06 | [RFC 3261 §17.1.2.2](https://www.rfc-editor.org/rfc/rfc3261.html#section-17.1.2.2) | Timer F expiry is terminal and late replies cannot revive the exchange | `session.rs::tick`, `session.rs::observe` | `timeout_closes_gate`, `late_reply_does_not_resume_timed_out_exchange` — Rust NOT RUN |
| GATE-07 | [RFC 3261 §18.4](https://www.rfc-editor.org/rfc/rfc3261.html#section-18.4) | Disconnect terminates current exchange; do not carry its gate across connections | `session.rs::disconnect` | `fatal_disconnect_prevents_reuse` — Rust NOT RUN |
| GATE-08 | [MS-SIPAE §3.3.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/aa0ce59d-a5e7-47c9-8a36-f2a2d32bc08c) | Missing or malformed challenge does not start authentication; errors remain fail-closed | `registration.rs::final_response`, `session.rs::observe` | `malformed_challenge_closes_matching_exchange` — Rust NOT RUN |

## Trust boundary / known limitations

1. Tests use only **public deterministic keys** and synthetic SIP messages. The test key is not a real TLS-DSK secret or GSS-derived session key.
2. The prior `RegisterFlow::observe_mic_checked_200` verifies signed fields, but MS-SIPAE does **not** include the full `Contact` header in the MIC input. `untrusted_gruu` is never promoted to a trusted route by this module.
3. `RegisterVerificationGate` does **not** own a live `TlsSipReceiver` or verify a real peer. The live wire-registration pipeline is not implemented. There is no SIP send method.
4. `SessionState::AcceptanceUnverified` is an **illustrative UI state**, never proof of a logged-in Skype for Business client.
5. Windows Kerberos, NTLM, TLS-DSK key exchange, provisioning, contacts, IM, media and conferencing remain unverified or missing.
6. Full release requires bidirectional, policy-dependent interoperability against a pinned Windows Full client and SfB Server Subscription Edition, per `V1_RELEASE_CONTRACT.json`.

## Executable checks available

```sh
python3 -m unittest discover -s scripts -p 'test*.py'
python3 scripts/check_v011_evidence.py
python3 scripts/check_v012_evidence.py
python3 scripts/check_v013_evidence.py
python3 scripts/check_v014_evidence.py
cargo test -p commscraft-core session --lib    # requires Rust; not executed here
python3 scripts/check_release.py --release # MUST FAIL until parity is verified
```

SHA-256 for authored source and this report is in `docs/evidence-v0.14.0.json`.
Those hashes protect local file integrity; they do not attest that the code compiles, or that it implements the Microsoft specification completely.
