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

## SDP session attribute ordering — October 16, 2026

| ID | Published normative basis | Implementation | Regression cases | Verification status |
| --- | --- | --- | --- | --- |
| SDP-ORDER-01 | [RFC 8866 (January 2021) §5](https://www.rfc-editor.org/rfc/rfc8866#section-5): session-level `a=` occurs **after** the time description (`t=`, optional `r=`/`z=`), before media sections | `crates/commscraft-core/src/sdp.rs::inspect_sdp`: reject any attribute before first valid `t=`; preserve existing session versus media attribute counters and bounded read-only behavior | `rfc8866_session_attributes_follow_timing_not_origin_or_name`: early `a=x-mediabw` before `o=` and early `a=x-devicecaps` after `s=` both fail; correctly placed Microsoft extension attribute counts remain intact | Authored in PR #33; CI outcome pending for latest head |
| SDP-ORDER-02 | [MS-SDPEXT §1.3](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/ecbc55ab-5787-42c2-9499-69b6f6ad52ae) identifies session-level `x-mediabw` and `x-devicecaps` extension attributes | Continue to **count** extension attributes only, without interpreting capabilities, credentials, bandwidth authorizations or creating media connections | Same test, valid Microsoft-style `x-mediabw` session attribute and media-level counts; no interoperability claim | Source authored; Windows server/client lab proof absent |

The required `AGENTS.md` evidence levels remain distinct: source-authored is not CI-passed, and CI-passed offline is not independently verified Microsoft interoperability. Existing historical RFC/MS SHA-256 manifests are unmodified. Required Linux and Windows workflows must pass on standard hosted runners before merge. Full Windows client parity and CommSCraft v1.0 remain BLOCKED.

## Repeat/zone scheduling field hardening — 2026-10-16

| ID | Normative citation | Code | Positive/negative Rust test | Evidence level |
| --- | --- | --- | --- | --- |
| SDP-TIME-03 | [RFC 8866 (January 2021) §5.10](https://www.rfc-editor.org/rfc/rfc8866#section-5.10): `r=` carries repeat interval, duration and at least one offset; decimal with optional case-sensitive d/h/m/s | `crates/commscraft-core/src/sdp.rs::time_unit_token`, `inspect_sdp`: validate required fields, unit syntax and placement after `t=`, before media | `rfc8866_repeat_and_zone_are_bounded_and_ordered`: valid 7d/1h and numeric offsets, invalid plus sign/unknown w suffix/missing offset and late repeat | Source-authored; updated-head CI pending |
| SDP-TIME-04 | [RFC 8866 (January 2021) §5.11](https://www.rfc-editor.org/rfc/rfc8866#section-5.11): optional zone adjustment is ordered pairs of absolute adjustment times and signed offsets, linked to preceding repeat times | `inspect_sdp`: require prior `r=`, one `z=` in a time description, even number of tokens, decimal adjustment time and optional negative offset with permitted unit | same test: valid `z=3730928400 -1h 3749680800 0`, orphan z, odd token count and bad unit rejected | Source-authored; updated-head CI pending |
| MS-CONTEXT-03 | [MS-SDPEXT §1.3](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/ecbc55ab-5787-42c2-9499-69b6f6ad52ae): Microsoft A/V session negotiation uses SDP | Read-only scheduling grammar does not activate Microsoft negotiation, SIP authentication, media sockets, SRTP or policy acceptance | existing `inspection_does_not_negotiate_media` | No SfB lab interoperability evidence |

Deliberate limitations: this is structural, bounded offline SDP inspection, not complete RFC 8866 scheduling semantics or Microsoft SDP parity. The parser does not calculate or schedule repeats. All historical SHA-256 files remain unchanged; all Ubuntu/Windows Rust, Clippy, Python RFC/MS evidence checks are mandatory. CommSCraft v1.0 remains BLOCKED.

## SDP signed zone-offset ambiguity — 2026-10-16

| Evidence ID | Normative reference | Code mapping | Regression mapping | Executed status |
| --- | --- | --- | --- | --- |
| SDP-TIME-05 | [RFC 8866 (January 2021) §5.11](https://www.rfc-editor.org/rfc/rfc8866#section-5.11) zone offsets use an optional single sign followed by a time value. | `crates/commscraft-core/src/sdp.rs::inspect_sdp`: replace `trim_start_matches('-')` (which consumed arbitrarily many minus signs) with `strip_prefix('-')` (at most one); the existing decimal/time-unit validator now rejects all remaining signs. | `rfc8866_repeat_and_zone_are_bounded_and_ordered`: negative `z=10 --1h`, `z=10 ---1h`, `z=10 +1h`; previously supported `z=3730928400 -1h` remains positive. | **Source-authored, not yet CI-confirmed** on commit after this documentation change. |
| SDP-TIME-06 | [MS-SDPEXT §1.3](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/ecbc55ab-5787-42c2-9499-69b6f6ad52ae) defines Microsoft extensions to SDP; this is RFC-level scheduling input validation, not a Microsoft-specific wire feature. | Same read-only `inspect_sdp`, no media, no credential use, no negotiation. | Existing offline regression above; live SfB evidence absent. | **Interoperability unverified**. |

The parser remains a deliberate bounded subset of RFC 8866; no semantic schedule calculation, complete MS-SDPEXT acceptance or Windows full client parity claimed. Historical SHA-256 manifests unchanged. Required checks: standard GitHub-hosted Linux/Windows workspace tests, Clippy and RFC/MS Python evidence. Do not merge until passed; v1.0 remains **BLOCKED**.
