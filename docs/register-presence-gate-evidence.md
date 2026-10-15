# CommSCraft REGISTER Presence-State gate — RFC / MS evidence

**Scope:** Source and offline tests only. `observe_register_with_presence` composes the current RFC 3261 non-INVITE REGISTER transaction gate, existing MS-SIPAE security association MIC/replay checks, and parsing of optional Microsoft `Presence-State`. No TLS socket, trusted network endpoint or production sign-in is established.

| Evidence ID | Normative reference | Source behavior | Adversarial / positive Rust test |
| --- | --- | --- | --- |
| CPG-01 | [RFC 3261 §§10, 17.1.3](https://www.rfc-editor.org/rfc/rfc3261#section-17.1.3) | A response must correlate with the ongoing REGISTER transaction; a wrong Call-ID never promotes state | `wrong_transaction_cannot_reveal_hint_or_advance_gate` |
| CPG-02 | [MS-SIPAE §3.2.5.2 steps 1–6](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) | Only after matching SA, server MIC and replay checks may metadata be returned; failed MIC cannot consume replay sequence | `synthetically_signed_correlated_200_may_expose_only_untrusted_hint`, `invalid_mic_does_not_release_hint_or_record_replay_number` |
| CPG-03 | [MS-SIPREGE §2.2.1.4](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/affa7b0a-c039-4699-9889-1da2e585ef39) | Optional header is parsed before any gate mutation; duplicate or malformed header rejected without accepting a transaction | `invalid_optional_metadata_does_not_consume_valid_mic_or_transaction` |
| CPG-04 | [RFC 3261 §§10.2, 17.1.2](https://www.rfc-editor.org/rfc/rfc3261#section-17.1.2) | A final 401 challenge or timeout never emits a registered hint | `challenge_never_exposes_microsoft_presence_hint`, `timed_out_gate_cannot_become_registered_by_late_mic` |
| CPG-05 | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) and local transport security rule | Disconnect makes an existing gate unusable; stale MIC cannot regain a session | `after_disconnect_a_valid_mic_is_no_longer_accepted` |

## Semantic and security boundaries

- All cryptographic test vectors are **synthetic**, using an explicit test-only TLS-DSK HMAC key. The test code does not acquire a server key through TLS or Windows SSPI.
- `GatedPresenceOutcome::untrusted_hint` is optional registrar metadata, **not** validated sign-in, presence publication or a security principal. The Microsoft MIC does not protect all response routing metadata, including Contact/GRUU, so actual transport peer validation remains a prerequisite for live use.
- A syntactically invalid optional header fails before mutating the security association. This is a **CommSCraft fail-closed local policy**; it does not claim the Microsoft standard requires discard for all malformed extensions.
- The version 1.0 full Windows-client parity gate remains closed.

## Executable evidence

`cargo test -p commscraft-register-events --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` are required in GitHub Actions on Linux and Windows. Until CI runs, authored tests are NOT counted as passed. Prior RFC/MS source hash ledgers remain unchanged.
