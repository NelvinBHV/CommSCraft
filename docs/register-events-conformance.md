# Microsoft / IETF normative test evidence: REGISTER Presence-State

**Scope:** `crates/commscraft-register-events`, plus the new `tests/conformance.rs` integration tests. **Offline only; not SIPAE authenticated.**

| Requirement | Source | Implementation | Test | Proof level |
| --- | --- | --- | --- | --- |
| Presence-State optional and at most one in a 200 REGISTER response | [MS-SIPREGE §2.2.1.4](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/affa7b0a-c039-4699-9889-1da2e585ef39) | `parse_presence_state` | `duplicated_header_is_rejected` | Source + CI test |
| register-action = added, fixed or refreshed | MS-SIPREGE §2.2.1.4 and §3.1.2.5.1 | `RegisterAction` | `accepted_registration_has_untrusted_added_hint` | Source + CI test |
| primary-cluster-type and connection fields have a specified grammar | MS-SIPREGE §2.2.1.4 | `ClusterState` | `fixed_and_remote_cluster_fields_match_microsoft_abnf` | Source + CI test |
| Reject invalid grammar and untrusted control characters | MS-SIPREGE §2.2.1.4, [RFC 5234](https://www.rfc-editor.org/rfc/rfc5234) | `ParseError`, header-length limit | `unauthorized_extension_and_header_injection_fail_closed` | Defensive implementation, source + CI test |
| RFC REGISTER handling is distinct from Microsoft SA establishment | [RFC 3261 §10](https://www.rfc-editor.org/rfc/rfc3261#section-10), MS-SIPREGE §3.1.2.5.1, [MS-SIPAE §3.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9) | Parser returns `PresenceHint` only | `unknown_or_non_ok_responses_do_not_create_login_state` | Static API guarantee + CI test |

**Important:** The test suite does not show that `Presence-State` came from a mutually authenticated server. The parser only handles an untrusted header. It must not change the 'signed in' UI state without TLS identity, MS-SIPAE MIC verification and a real REGISTER flow.

Evidence classes: Rust source = authored, GitHub Actions success = executed tests for this crate, and real Skype for Business Server interoperability = **not demonstrated**.
