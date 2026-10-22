# Verified RFC/MS protocol findings — REGISTER/authentication research tranche

Date: 2026-10-22. **Research-only, not an implementation specification or a completed exhaustive review.** Real-server testing is reserved exclusively to the project owner.

## Primary source / publication evidence

| ID | Official document | Published revision | Date | Evidence |
|---|---|---|---|---|
| MS-OCSPROT | [Skype for Business and Skype for Business Server Protocols Overview](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c) | 9.0 | 2025-05-20 | Microsoft Published Version table; previous 8.0 (2021-08-17) |
| MS-SIPAE | [SIP Authentication Extensions](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd) | 11.0 | 2025-05-20 | Microsoft Published Version table; previous 10.0 (2024-08-20) |
| MS-SIPREGE | [SIP Registration Extensions](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d) | 17.1 | 2025-08-19 | Microsoft Published Version table; previous 17.0 (2025-05-20) |
| RFC 3261 | [SIP](https://www.rfc-editor.org/info/rfc3261) | June 2002, proposed standard | 2002-06 | RFC Editor; **updated by multiple later RFCs**, see official RFC Editor metadata |
| RFC 3263 | [Locating SIP Servers](https://www.rfc-editor.org/info/rfc3263) | June 2002 | 2002-06 | RFC Editor |
| RFC 5922 | [SIP Domain Certificates](https://www.rfc-editor.org/info/rfc5922) | June 2010 | 2010-06 | RFC Editor |

**Source-access boundary:** Microsoft landing pages expose abstract, published-version history, and PDF/DOCX links, not all normative sections. Therefore exact Microsoft wire fields, signature canonicalization, complete exchange order, deadlines and endpoint quirks are **NOT verified in this tranche**. Full current-version PDF/DOCX, historical deployment-profile versions, changes/errata and their normative references must be acquired and read before extracting exact requirements. Do not infer full clause coverage from page metadata.

## Verified document-system relationships

1. **System scope:** MS-OCSPROT describes SIP-based Skype for Business clients spanning instant messaging, aggregated presence, voice, video, conferencing and application sharing, and explicitly delegates protocol-specific processing rules to the underlying individual specifications. Source: [MS-OCSPROT abstract](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c), abstract and Published Version (rev 9.0).
2. **Authentication purpose:** MS-SIPAE's official abstract identifies NTLM and Kerberos authentication schemes in the SIP authentication framework. Exact message behavior awaits full rev 11.0 normative sections. Source: [MS-SIPAE abstract](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd).
3. **Registration is not just REGISTER:** MS-SIPREGE's official abstract explicitly covers registration procedure extensions **and** format/mechanism for in-band provisioning needed for client sign-in. The dependency graph must not terminate at a 200 REGISTER. Source: [MS-SIPREGE abstract](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d).

## RFC 3261: extracted normative client-side landmarks

| ID | Section | Actual rule / model | Implication for joint analysis | Confidence |
|---|---|---|---|---|
| SIP-R01 | §7.3 and §8.1.3.3 | Multiple Via values can occur in SIP routing, but a UAC receiving **more than one Via header-field value in a response SHOULD discard it**. This is SHOULD, not a blanket parser-wide MUST ban. | distinguish UAC response policy from all message parsing and downstream proxy roles | directly verified from RFC text |
| SIP-R02 | §§10.2, 10.2.1, 10.2.2, 10.2.4, 10.2.8 | REGISTER has separate construction, binding-add/removal, refresh and error-handling processing. | registration maintenance is a stateful activity after initial registration; correlate with MS-SIPREGE details | section inventory verified; fine-grained rules pending extraction |
| SIP-R03 | §§17.1.2.1–17.1.2.2 | REGISTER is non-INVITE transaction class; the RFC defines its dedicated client transaction states and timers. For unreliable transport Timer E handles retransmits; Timer F (64×T1) is transaction timeout; Timer K handles termination waiting. | never apply the INVITE transaction's timer set to REGISTER; compare reliable transport behavior and Microsoft extensions | verified timer overview/section references |
| SIP-R04 | §17.1.3 | Client-side response matching is transaction-specific; Via branch and CSeq methodology must be analyzed along with transport and magic-cookie rules. | prevents treating a generic final response as success for unrelated client request | section located, clause details pending |
| SIP-R05 | §§18.3, 7.4.2, 7.5 | Transport framing and SIP body length are separate constraints from authentication status. | parser completeness is a precondition, not a proof of authentication | section locations verified; full normative detail pending |
| SIP-R06 | RFC Editor **Updates** metadata | RFC 3261 is updated by, among others, RFC 5922, RFC 6026, RFC 6665 and RFC 8760. | create a per-feature applicability and conflict ledger rather than treating June 2002 as self-contained | directly verified from RFC Editor record |

For all these entries, **section inventory is not equivalent to a full clause audit**. Further extraction must copy only small lawful excerpts as needed and otherwise paraphrase with precise section and role. Track UAC vs registrar vs proxy rules independently.

## Composite reconstruction: sign-in (dependency model, NOT exact message trace)

```text
Configured identity / policy
      |
      v
Service/proxy resolution (RFC 3263; MS-CONMGMT still to inspect)
      |
      v
Transport establishment and peer SIP identity (RFC 5922; MS-CONMGMT)
      |
      v
SIP messages + non-INVITE transaction (RFC 3261 §§7–8, 17–18)
      |
      v
REGISTER request and response processing (RFC 3261 §10 + MS-SIPREGE)
      |                         |
      |                         +-- registration challenge / policy branches (exact rev 17.1 TBD)
      v
SIP authentication framework / Kerberos or NTLM (MS-SIPAE rev 11.0)
      |  GSS-API + OS crypto dependencies (full references TBD)
      v
Protected subsequent requests, response verification (exact rules TBD)
      |
      v
Registered contact and in-band provisioning (MS-SIPREGE rev 17.1)
      |
      v
Authorized post-registration feature protocols (MS-OCSPROT reference matrix TBD)
```

**This is not a fixed sequential wire trace**: authentication may involve multiple exchanges, challenges, alternate routes, errors and selected authentication mechanisms; details cannot be established from the metadata pages alone.

## Conflict / ambiguity register

- **A1 — Via response handling:** avoid elevating RFC 3261 §8.1.3.3's SHOULD to an unconditional MUST or incorrectly claiming that multiple Via values cannot be represented in SIP at all. Determine Microsoft UAC-specific profile in MS-SIPAE/MS-SIPREGE once normative sections are read.
- **A2 — Document version compatibility:** 2025 publications can document functionality added after historical Windows Full Client releases. Determine supported deployment era and feature negotiations; maintain both publication and feature-level applicability.
- **A3 — Authentication-success boundary:** neither TLS handshake, matched REGISTER response, nor provisional security context is adequate evidence that the complete sign-in + provisioning contract is met. MS-SIPAE and MS-SIPREGE normative details must identify precisely which transitions are authorized.
- **A4 — Untested client behavior:** do not infer real-server acceptance from CI. The owner alone conducts authorized real-server interoperability tests.

## Research execution ledger

- **Done this tranche:** checked three official Microsoft Published Version tables and abstracts, RFC 3261 metadata and normative Via-response clause, RFC 3261 registration/transaction section inventory and timer landmark, RFC 3263/RFC 5922 sources, wrote cross-spec dependency and uncertainty register.
- **Not yet done:** download and fully review each canonical MS PDF/DOCX and all referenced documents; enumerate exact MS-OCSPROT protocol matrix; extract field ABNF, state machines, timer tables and byte-level MIC construction; explain all sign-in error branches; build feature-complete calls/conferencing/media analysis; owner verification.
- **Next evidence tranche:** detailed normative extraction of [MS-OCSPROT] references and scenarios, [MS-SIPAE] rev 11.0 §§2–3 (plus Windows-security transitive references), [MS-SIPREGE] rev 17.1 §§2–3; then integrate into a single conditionally branched message/state model. No source-code implementation during research phase.
