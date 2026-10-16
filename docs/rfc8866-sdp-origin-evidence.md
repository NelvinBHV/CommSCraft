# RFC 8866 SDP origin-field validation evidence

**Scope:** bounded read-only SDP inspector, not offer/answer negotiation, codec selection, authentication or network media. Evidence status is source-authored until the pull request's CI completes.

| ID | Specification / version / section | Source behavior | Test |
| --- | --- | --- | --- |
| SDP-ORIGIN-01 | [RFC 8866 (January 2021) §5.2](https://www.rfc-editor.org/rfc/rfc8866#section-5.2): `o=<username> <sess-id> <sess-version> <nettype> <addrtype> <unicast-address>` | `crates/commscraft-core/src/sdp.rs` requires six separated fields, and only ASCII decimal digits in nonempty `sess-id` and `sess-version` | `rfc8866_origin_rejects_nonnumeric_session_identity_and_version`, negatives: text, plus, minus, decimal point |
| SDP-ORIGIN-02 | [RFC 8866 §5.2](https://www.rfc-editor.org/rfc/rfc8866#section-5.2): numeric *string* with no fixed-size integer limit | Test and parser leave large decimal identifiers intact rather than converting to `u64` | same test: 80-digit session identifier accepted |
| SDP-ORIGIN-03 | [MS-SDPEXT v16.0 (20 May 2025) §1.3](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/ecbc55ab-5787-42c2-9499-69b6f6ad52ae): Microsoft SDP extends SDP for A/V and application sharing | Inspector does not negotiate media or infer trusted session state from the `o=` field; Microsoft-specific session-version behavior remains outside this fix | existing `inspection_does_not_negotiate_media` test |

Microsoft publication index: [MS-SDPEXT published revision 16.0 (2025-05-20)](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/cd17a549-b948-42a6-aa6b-fa707710faac).

**Verification:** `cargo test -p commscraft-core --all-targets`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, Python RFC/MS historical integrity checks, standard hosted Ubuntu/Windows jobs. Report individual CI outcomes only after completion. No historical evidence manifests modified.

**Nonclaims:** not full RFC 8866 grammar compliance, not MS-SDPEXT parity, not authenticated SIP/SDP signaling, no real RTP/SRTP, no verified Skype for Business server interoperability. CommSCraft 1.0 remains BLOCKED.

## SDP timing fields (RFC 8866 §5.9)

| ID | Normative source | Source behavior | Regression tests |
| --- | --- | --- | --- |
| SDP-TIME-01 | [RFC 8866 (January 2021) §5.9](https://www.rfc-editor.org/rfc/rfc8866#section-5.9): `t=<start-time> <stop-time>` contains decimal time representations of arbitrary length and need not fit a 64-bit integer | `crates/commscraft-core/src/sdp.rs` `inspect_sdp` checks exactly two nonempty ASCII decimal fields without `u64` conversion, retaining the existing 2,048-character line bound | `rfc8866_time_fields_accept_long_decimals_and_reject_invalid_values`: 80-digit start and stop times accepted, signs, fractional/nondecimal and missing field denied |
| SDP-TIME-02 | [MS-SDPEXT revision 16.0 (20 May 2025) §1.3](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/ecbc55ab-5787-42c2-9499-69b6f6ad52ae): extensions to SDP used in Microsoft audio/video and application sharing negotiation | Parsing a time value alone does not imply successful offer/answer or a trusted Microsoft media session; no negotiation, sockets or credentials added | Existing `inspection_does_not_negotiate_media` and time negative tests |

Execution status: authored on PR branch; only GitHub Actions checks completed on the **new** head commit may establish test success. Historical SHA-256 ledgers remain unchanged; v1.0 stays BLOCKED.

## Microsoft MS-SDPEXT origin constraints — 2026-10-16

| Requirement | Normative reference and scope | Implementation | Offline test | Execution evidence |
| --- | --- | --- | --- | --- |
| MS-ORIGIN-01 | [MS-SDPEXT §3.1.5.19, public 2024-08-20 revision](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-SDPEXT/%5BMS-SDPEXT%5D-240820.pdf) constrains origin `nettype` to `IN` and `addrtype` to `IP4` / `IP6` | `crates/commscraft-core/src/sdp.rs::inspect_sdp` rejects alternative network/address types | `ms_sdpext_origin_requires_in_and_address_family_literal` negative non-IN and IP5 cases | Pending updated PR head CI; do not mark passed before GitHub runs complete |
| MS-ORIGIN-02 | Same Microsoft section requires dotted-decimal IPv4 or textual IPv6 literals matching their declared type | Parse `std::net::Ipv4Addr` or `Ipv6Addr` with no DNS/network use; reject malformed, hostname, wrong family and scoped IPv6 zone notation | Same test: positive IPv4 and IPv6; negative mismatch, malformed address, hostname and zone suffix | Pending updated PR head CI |
| RFC-ORIGIN-03 | [RFC 8866 (January 2021) §5.2](https://www.rfc-editor.org/rfc/rfc8866#section-5.2) ALSO permits a FQDN for IP4/IP6 origin | **Declared intentional restriction** to Microsoft's documented profile, NOT universal RFC 8866 origin parser conformance. Retain strict bounds on bytes/line counts | Same hostname-negative test | Source-only until CI |

The §3.1.5.19 normative wording was cross-referenced to the historical public Microsoft Open Specifications PDF; the [Microsoft Learn MS-SDPEXT publication index](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/cd17a549-b948-42a6-aa6b-fa707710faac) lists revisions. This offline parser does not interpret subsequent SDP offer/answer state changes, and ignores neither security policy nor authentication requirements by implication. No SIP/TLS, MIC, trusted sign-in, media sockets or feature-parity proof. Historical SHA-256 evidence untouched; v1.0 BLOCKED.
