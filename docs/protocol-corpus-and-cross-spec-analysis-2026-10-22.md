# CommSCraft — protocol corpus and cross-specification analysis, phase 1

**22 October 2026 · research status: inventory and system-level synthesis, NOT exhaustive clause analysis.**

## Research discipline

The owner requests a **complete documentation-first reconstruction** of Skype for Business / Microsoft SIP client behavior before further implementation. The owner alone will perform tests against real servers. This document records *observed statements*, *interpretation*, and *unverified questions* separately. Never promote a schematic exchange to a normative wire trace. No production-code changes or server traffic are authorized by this document.

For each relevant standard, collect the full official published revision, publication date, superseding/updated-by RFCs, protocol roles, message ABNF/fields, timers, errors, state tables, cross-references, examples, and optional/required extension rules. Every eventual code change must cite a clause of the revision actually reviewed. The catalog below is **not a claim that each full specification has already been read**.

## Primary system-of-systems authority

- [MS-OCSPROT], *Skype for Business and Skype for Business Server Protocols Overview*, published **9.0 / 20 May 2025**, [Microsoft landing page](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c). This is the top-level system map. It explicitly spans instant messaging, presence, conferencing, voice, video and application sharing, and directs implementers to individual normative protocol documents for processing rules.
- [Microsoft Office protocol technical-document index](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-offprotlp/939e9850-c48d-464d-860e-78f8b281e7bd). Establishes the authoritative interdependent MS-* specification catalog.

## Corpus inventory: foundational RFCs

| Identifier | Role in client reconstruction | Priority | Source |
|---|---|---|---|
| RFC 3261 (June 2002) | SIP ABNF, URIs/headers, transactions, dialogs, REGISTER, security, errors, timers | foundational | https://www.rfc-editor.org/rfc/rfc3261 |
| RFC 3263 (June 2002) | DNS NAPTR/SRV/A/AAAA, service resolution, transport and failover | foundational | https://www.rfc-editor.org/rfc/rfc3263 |
| RFC 2782 (February 2000) | SRV record semantics and priority/weight | foundational | https://www.rfc-editor.org/rfc/rfc2782 |
| RFC 5922 (June 2010) | SIP-domain identity and certificate binding after discovery | foundational | https://www.rfc-editor.org/rfc/rfc5922 |
| RFC 2743 (January 2000) | GSS-API abstract security context and per-message services | foundational | https://www.rfc-editor.org/rfc/rfc2743 |
| RFC 2744 (January 2000) | GSS-API C bindings used for adapter comparison | supporting | https://www.rfc-editor.org/rfc/rfc2744 |
| RFC 4120 (July 2005) | Kerberos V5 authentication architecture | foundational | https://www.rfc-editor.org/rfc/rfc4120 |
| RFC 6665 (July 2012) | SIP event notification SUBSCRIBE/NOTIFY (updates 3265) | feature-critical | https://www.rfc-editor.org/rfc/rfc6665 |
| RFC 3856 (August 2004) | Presence event package | feature-critical | https://www.rfc-editor.org/rfc/rfc3856 |
| RFC 3863 (August 2004) | PIDF presence format | feature-critical | https://www.rfc-editor.org/rfc/rfc3863 |
| RFC 4566 (July 2006) | SDP session descriptions (superseded by RFC 8866; check deployed profile) | media | https://www.rfc-editor.org/rfc/rfc4566 |
| RFC 3264 (June 2002) | SDP offer-answer | media | https://www.rfc-editor.org/rfc/rfc3264 |
| RFC 3550 (July 2003) | RTP/RTCP | media | https://www.rfc-editor.org/rfc/rfc3550 |
| RFC 3711 (March 2004) | SRTP | media | https://www.rfc-editor.org/rfc/rfc3711 |

**Coverage caveat:** This inventory omits many ancillary RFCs until [MS-OCSPROT] and each MS-* document's exact normative reference lists have been traversed. RFC 3261 has subsequent updates; resolve applicable changes by client feature and negotiated protocol version, not by assuming its June 2002 text is the final word.

## Corpus inventory: Microsoft documents

| Identifier | Role | Verified official listing/source | Current revision evidence |
|---|---|---|---|
| MS-OCSPROT | high-level inter-protocol graph | https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c | 9.0 / 20 May 2025 |
| MS-CONMGMT | proxy auto-discovery, keepalives, TLS certificate requirements | https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-offprotlp/939e9850-c48d-464d-860e-78f8b281e7bd | pin PDF revision |
| MS-SIPAE | NTLM/Kerberos SIP authentication extensions | https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd | 11.0 / 20 May 2025 |
| MS-SIPREGE | REGISTER extensions and sign-in in-band provisioning | https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d | 17.1 / 19 Aug 2025 |
| MS-SIPRE | SIP routing and endpoint-identification extensions | https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-offprotlp/939e9850-c48d-464d-860e-78f8b281e7bd | pin PDF revision |
| MS-SIPAPP | SIP application-level extensions | same Microsoft official technical-doc index | pin PDF revision |
| MS-SIPCOMP | SIP compression negotiation/profile | same Microsoft official technical-doc index | pin PDF revision |
| MS-PRES | Microsoft presence extensions | same Microsoft official technical-doc index | pin PDF revision |
| MS-ABS | address book files/user lookup | same Microsoft official technical-doc index | pin PDF revision |
| MS-DLX | distribution list expansion | same Microsoft official technical-doc index | pin PDF revision |
| MS-CONFIM / MS-CONFPRO / MS-CONFBAS / MS-CONFAS | conferencing and provisioning/application sharing | same Microsoft official technical-doc index | pin each PDF revision |
| MS-OCSMP | online conference scheduling/management | same Microsoft official technical-doc index | pin PDF revision |
| MS-SDPEXT | Skype SDP extension profile | same Microsoft official technical-doc index | pin PDF revision |
| MS-SRTP / MS-SSRTP / MS-AVEDGEA | encrypted media and edge auth | same Microsoft official technical-doc index | pin each PDF revision |
| MS-PSOM | Web conference shared-object messaging | same Microsoft official technical-doc index | pin PDF revision |
| MS-KILE / MS-NLMP | Windows Kerberos/NTLM mechanics invoked by SIP auth | locate exact Windows Protocols published revisions and references in MS-SIPAE | not yet collected |

**Do not confuse the Microsoft overview's prose with the detailed MS-* normative processing rules.** The technical document and references must be read before claiming precise message fields or optional-branch applicability.

## Composite system model — provisional, not a certified wire transcript

### A. Sign-in and registration dependency graph

```text
User/endpoint identity & policy
    |
    v
Service-location and proxy selection -------- RFC 3263 / RFC 2782 / MS-CONMGMT
    |
    v
TLS connection + SIP service identity ------- RFC 5922 / MS-CONMGMT
    |
    v
SIP framing / transaction layer ------------ RFC 3261 §§7, 8, 17, 18
    |
    v
REGISTER request / challenge decision ------ RFC 3261 §10 / MS-SIPREGE
    |
    +-- authentication challenge ------------ MS-SIPAE / GSS/SSPI / Kerberos or NTLM
    |      |                                    RFC 2743 / RFC 4120 / MS-KILE / MS-NLMP
    |      v
    +-- protected REGISTER continuation ----- MS-SIPAE signature/MIC rules
    |
    v
Response and state validation -------------- RFC 3261 / MS-SIPAE / MS-SIPREGE
    |
    v
Registration binding / in-band provisioning - MS-SIPREGE
    |
    v
Post-registration feature protocols -------- MS-PRES / SIP events / messaging,
                                             MS-SIPAPP / conferencing / media
```

This graph shows **dependency order**, NOT exact message count. A particular server can choose a challenge mechanism, reject credentials, redirect, require retries, or deny provisioning. Exact ABNF and state transitions are still awaiting clause-level review.

### B. Crucial boundaries to resolve from normative text

1. **Discovery identity vs network endpoint:** DNS SRV target is not automatically the certificate identity. For SIP service identity, RFC 5922 §7.3 describes validation against the relevant SIP domain. Determine exactly how MS-CONMGMT qualifies proxy/discovery flows and home-pool changes.
2. **Transport establishment vs user authentication:** a successfully validated TLS peer does not imply Microsoft account sign-in; MS-SIPAE adds client authentication/SA exchanges. Separate these state machines rigorously.
3. **SIP transaction vs registration binding:** a 200 response to a correlated REGISTER transaction is not, by itself, a proven authenticated session. Determine exact signature input, replay/sequence state, response trust and registration metadata fields from the pinned MS-SIPAE and MS-SIPREGE documents.
4. **Provisioning vs feature availability:** registration and in-band provisioning may make different features usable or policy-denied; each feature has its own protocol and independent state.
5. **General SIP vs Microsoft-specific profile:** multiple Via headers are valid in general SIP routing; restricted local fixtures must not silently become global parser constraints. Determine behavior from RFC 3261 §§7.3/17 and Microsoft extension clauses.
6. **Windows Full Client parity:** a login trace does not establish presence, IM, call, meeting or media parity. Review [MS-OCSPROT] scenarios and each associated technical document individually.

## Clause-by-clause extraction template (not yet fulfilled)

For **every** document/section record: exact revision; normative requirement (MUST/SHOULD/MAY); condition/role; message field and ABNF; state before/after; errors/timeouts/cleanup; peer security and trust assumptions; cross-reference to other specifications; repository file/function if present; contradiction/ambiguity; confidence; owner's later test scenario. Never insert guessed field values or conflate client and server responsibilities.

## First unresolved deep-analysis questions

- What is the exhaustive [MS-OCSPROT] scenario-to-protocol matrix, including all conferencing/media paths and version-specific deployments?
- Which exact challenge/response messages, security token exchanges, MAC input canonicalization and nonce/sequence rules are prescribed in [MS-SIPAE] revision 11.0 and its referenced OS security specs?
- Which registration/provisioning payload formats, return codes, contact identifiers and sign-out/refresh/recovery sequences are required by [MS-SIPREGE] revision 17.1?
- Which [MS-CONMGMT] proxy-autodiscovery and keepalive mechanisms are normative for the selected deployment topology, and how do they affect TLS service identity?
- Which Microsoft extensions modify the standard SIP transaction/dialog event machinery, and what interactions are mandatory vs capability-negotiated?
- Which referenced protocol revisions must be selected to mirror the intended historical Windows Full Client rather than indiscriminately combining latest incompatible document revisions?

## Status and next evidence gate

**Accomplished:** public-document identifiers and source URLs inventoried, latest published versions of the three anchor Microsoft documents checked, a preliminary dependency graph and targeted clause-extraction tasks established.

**Not accomplished:** exhaustive document acquisition, reading and tracing of all clauses, version compatibility matrix, exact-bytes wire transcripts, full feature coverage, resolution of all protocol ambiguities or confirmation on a real server. None of these is claimed. Subsequent work must expand this corpus before further implementation.
