# Evidence: REGISTER Presence-State (CommSCraft independent crate)

Status: source authored on feature branch. Rust execution remains unconfirmed until GitHub Actions completes. No live Skype for Business authentication.

| Requirement | Normative reference | Source / test |
| --- | --- | --- |
| At most one optional Presence-State header in REGISTER 200 | MS-SIPREGE §2.2.1.4 | parse_presence_state / optional_header_absent, duplicate_header_is_rejected |
| Added, refreshed, fixed action values | MS-SIPREGE §§2.2.1.4 and 3.1.2.5.1 | RegisterAction / actions_from_microsoft_spec |
| Central/remote and connected status | MS-SIPREGE §2.2.1.4 | ClusterState / extended_cluster_state |
| User services unavailable, case-insensitive ABNF token | MS-SIPREGE §2.2.1.4; RFC 5234 §2.3 | PresenceHint / extended_cluster_state, accepts_case_insensitive_user_services_token |
| Reject malformed/oversized input | Defensive constraints on MS-SIPREGE ABNF, RFC 5234 §2.3 (case-insensitive string literals), and RFC 3261 §20 | malformed_extensions_are_rejected, bounded_header, rejects_header_injection_and_nul |
| Parsed header is NOT authenticated sign-in | MS-SIPREGE §3.1.2.5.1 (security association per MS-SIPAE §3.2); RFC 3261 §10.3 | PresenceHint / no_authenticated_state_is_constructed |

Microsoft:
- https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/affa7b0a-c039-4699-9889-1da2e585ef39
- https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010
IETF:
- https://www.rfc-editor.org/rfc/rfc3261#section-10.3
- https://www.rfc-editor.org/rfc/rfc5234#section-2.3

LIMITATIONS: This is a deliberately strict parser for known MS-SIPREGE fields; future server variants/extensions may need explicit compatibility analysis. It does not send packets and never marks the account online. Historic v0.14.0 code remains pending full import.
