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
