# RFC 3261 UAC, REGISTER and transaction clause ledger — research tranche 2

**2026-10-22 · documentation only · scope: SIP UAC / non-INVITE registration path.** The normative primary text is [RFC 3261 (June 2002)](https://www.rfc-editor.org/info/rfc3261/), subsequently updated by several RFCs including RFC 5922, 6026, 6665 and 8760. Each update requires feature-specific applicability review. This ledger does **not** claim analysis of every SIP clause or Microsoft extension.

## Normative rule / mechanism / consequence

| Rule | Citation | Role and condition | Rule (paraphrased), implications | Status |
|---|---|---|---|---|
| UAC-01 | RFC 3261 §8.1.3 | UAC receives response | Response proceeds transport → transaction layer → transaction user (TU), then method-specific processing. Distinguish parsing, matching, registrar behavior, and authentication. | directly inspected |
| UAC-02 | RFC 3261 §8.1.3.1 | transaction layer reports timeout | UAC **MUST** treat as receipt of 408. Fatal transport errors **MUST** be treated as 503; do not confuse the two in sign-in recovery. | directly inspected |
| UAC-03 | RFC 3261 §8.1.3.2 | unknown final response code | UAC **MUST** treat unrecognized final status as corresponding x00 class, and support all class x00 responses. | directly inspected |
| UAC-04 | RFC 3261 §8.1.3.3 | UAC receives response containing >1 Via header *value* | UAC **SHOULD** discard message. This is a conditional client processing recommendation, not a universal syntax prohibition nor MUST. | directly inspected |
| UAC-05 | RFC 3261 §8.1.3.4 | 3xx response | UAC **SHOULD** form new requests using Contact URI(s); check routing, authentication boundary, fork handling and Microsoft-specific redirects separately. | directly inspected; detail review pending |
| UAC-06 | RFC 3261 §8.1.3.5; §§22.2–22.3 | 401/407 | UAC **SHOULD** invoke authorization procedures and retry with credentials. Microsoft MS-SIPAE mechanisms replace/extend the generic specifics; a retry is a new transaction, not reuse of an already-consumed final response. | inspected UAC rule; MS details pending |
| REG-01 | RFC 3261 §10.2.3 | REGISTER binding query | No Contact in REGISTER leaves binding list unchanged; successful REGISTER returns existing bindings. Need independent state for query, add, refresh and unregister. | directly inspected |
| REG-02 | RFC 3261 §10.2.4 | REGISTER lifetime | UA is responsible for refreshing bindings; distinguish registration lifetime from security-context lifetime and TLS connection lifetime. | directly inspected; exact timing extraction pending |
| REG-03 | RFC 3261 §§10.2.1, 10.2.2, 10.2.8 | client constructs and processes REGISTER | Adding/removing contact bindings and error recovery are separate client operations; collect 423/Min-Expires, Contact/expires precedence and CSeq rules clause by clause. | sections identified; extraction incomplete |
| TX-01 | RFC 3261 §§17.1.2.1–17.1.2.2; Table 4 | non-INVITE transaction, such as REGISTER | Request/client-transaction phase and transport influence retransmissions. Timer E: retransmits on unreliable transport; Timer F: timeout (64 × T1); Timer K: completed-state linger for unreliable transport. Avoid inventing TLS retransmission behavior. | timer semantics checked; exact state table pending |
| TX-02 | RFC 3261 §17.1.3 | client matches response | Match response to *original* client transaction, considering the transaction's Via branch and CSeq method and associated matching rules; perform byte-level/profile analysis before implementing Microsoft variation. | section identified; exact ABNF/matching subtleties pending |
| TR-01 | RFC 3261 §§18.1, 18.3 | transport receives byte stream | Framing completeness is a distinct prerequisite to response-processing trust; reliable-stream boundaries cannot be treated as message boundaries. | section identified; full parsing rules pending |

## Reasoning across the layers (analytical inference, not normative quote)

A *single* success flag conflates at least five independent checks: syntactically complete response, matching transaction, result semantics, MS-SIPAE authentication/integrity, and completed MS-SIPREGE registration/provisioning. This follows from RFC 3261's transport/transaction/TU layering plus the official MS-SIPAE authentication purpose and MS-SIPREGE in-band-provisioning scope. Exact Microsoft-specific completion conditions remain unverified until the full current-revision specification body has been examined.

### Preliminary branching UAC model

```text
complete SIP response
  -> transaction lookup/match
    -> transaction layer error? timeout = 408 equivalent; fatal transport = 503 equivalent
    -> provisional 1xx: transaction remains in its permitted progressing state
    -> 2xx: method-specific REGISTER result; no implied authenticated session
    -> 3xx: consider redirect targets and new requests, preserve identity policy
    -> 401/407: authenticate and reissue by the applicable auth framework
    -> other final 4xx/5xx/6xx: method-specific policy and recovery
  -> Microsoft extension validation (exact profile unresolved)
  -> registration/provisioning state (MS-SIPREGE detail unresolved)
```

This is a *conceptual decision graph*, not an authoritative finite-state diagram of a Microsoft client. Further branches include 423, retransmission on unreliable transport, challenge token exchange, timeouts and negotiated feature policies.

## Primary Microsoft references and blocked acquisitions

- [MS-OCSPROT v9.0, 2025-05-20](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c): landing page and revision history readable. The hosted PDF fetch followed a redirect to a blocked host in the current web environment; **full normative body not yet extracted**.
- [MS-SIPAE v11.0, 2025-05-20](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd): landing page readable; PDF fetch redirected to a blocked host. **No invented MIC canonicalization or authentication tokens.**
- [MS-SIPREGE v17.1, 2025-08-19](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d): landing page readable; PDF fetch reported unsupported octet stream. **No claims of full clause review.**

These are tool/environment retrieval failures, not evidence that Microsoft specifications are private or unavailable. Alternative legitimate PDF/DOCX retrieval and revision archiving must be attempted. No licensed/proprietary product code should be copied.

## Document-system next deep-analysis gates

1. Obtain current and historical relevant MS-OCSPROT PDF/DOCX versions, extract every protocol reference and scenario into an exhaustive directed graph; select Skype for Business Windows Full Client roles, not every conceivable server-only protocol.
2. Read MS-SIPAE rev 11.0 body and normative references; map §2 data, §3 state and explicit challenge/MIC algorithm including exact on-wire input, key derivation and sequence rules; distinguish mechanisms by negotiated security scheme.
3. Read MS-SIPREGE rev 17.1 body, including request/response formats, in-band provisioning, GRUU/contact, registration refresh, error branches; correlate each change of state to MS-SIPAE and RFC 3261.
4. Analyze MS-CONMGMT, MS-SIPRE and MS-PRES, then contact, IM, conference, SDP, RTP/SRTP and edge/ICE dependent protocols.
5. Produce linked normative requirement IDs, document versions, actual sections, diagrams, contradiction log and compliance matrix before planning implementation. Real-server interoperability testing remains the owner's exclusive responsibility.

**Completion status: NOT COMPLETE. No protocol implementation in this tranche.**
