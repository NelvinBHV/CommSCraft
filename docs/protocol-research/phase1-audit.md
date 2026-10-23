# Phase 1 — formal source-acquisition and normative-reference audit

**Snapshot:** 2026-10-23. **Verdict: NOT COMPLETE — implementation remains locked.** This is a provenance audit, not a protocol implementation or compatibility attestation.

## Coverage, measured from the exact draft-branch JSON ledgers

| Required acquisition gate | Measured status | Closure |
| --- | --- | --- |
| MS-OCSPROT rev 9.0 (2025-05-20), all six overview system protocol tables | 6/6 tables; **46 distinct published MS protocol documents** | Named overview inventory OK; applicability and transitive references OPEN |
| Published version metadata (latest revision/date/direct official Microsoft landing) | **46/46** cataloged | Latest landing metadata complete for overview table items; exhaustive historical versions/diffs NOT complete |
| First-layer MS §1.2.1 normative reference lists | **45/46** documents inspected | **1 still inaccessible/not examined**; 33 HTML-only documents cannot claim release-text match |
| Original released MS §1.2.1 index/PDF reference page and visual inspection | **12** documents, matching published edition cover/date | Reference-page evidence only; no whole-PDF ingestion or binary SHA-256 |
| Normative directed source→target edges | **455** records to **199** distinct IDs | Includes 120 MS edges, 227 RFC edges, 28 historical IETF draft edges, 80 other standards. **Not transitive closure.** |
| RFC original fixed-edition seed/first-level references | **113** RFC IDs, of which **89** directly cited by investigated MS lists | RFC normative references, Updates/Obsoletes/Errata and dates for whole set NOT checked |
| Transitive MS Windows/Office specs outside overview | **12** distinct immediate Microsoft target IDs identified | Their precise versions, access, all further references and client applicability OPEN |
| Official current published PDF/DOCX binaries acquired and hashed | **0** | Browser-access redirects / external host and container DNS restrictions; no fabricated checksum |
| Version comparisons of all MS documents and current Windows Client/SfB Server applicability | Only selected revisions/date rows checked | NOT complete; latest exact reference Windows client C2R build remains UNPINNED |

## First-tier MS source still unverified

- **MS-AVEDGEA revision 12.0, 2025-05-20**, [official landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-avedgea/56860f6a-2fcf-4cad-8ee5-466d8f382a45): current release PDF download redirected to unsupported host, page-index current PDF not located, and no verifiable matching current live normative-reference §1.2.1 obtained. **DO NOT** substitute historical 2021 content. Exact source acquisition is a blocker.

The latest [MS-AVEDGEA published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-avedgea/56860f6a-2fcf-4cad-8ee5-466d8f382a45) reports **rev 12.0, 2025-05-20** (major update from rev 11.0, 2024-08-20). Direct current PDF download failed through redirect, and standalone fetch was inaccessible; a historical 2021 source and a third-party reproduction were found but are **not normative evidence for rev 12.0**.

## Release PDF §1.2.1 sections with verified dated text and screenshot (partial pages, NOT entire documents)

- **MS-CONMGMT rev 10.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-CONMGMT/%5BMS-CONMGMT%5D.pdf), PDF pp. 8 (index 7); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-CONFBAS rev 15.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-CONFBAS/%5BMS-CONFBAS%5D.pdf), PDF pp. 13–14 (indices 12–13); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-OCAUTHWS rev 11.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-OCAUTHWS/%5BMS-OCAUTHWS%5D.pdf), PDF pp. 11–13 (indices 10–12); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-PRES rev 13.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-PRES/%5BMS-PRES%5D.pdf), PDF pp. 15–16 (indices 14–15); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-ECREST rev 8.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-ECREST/%5BMS-ECREST%5D.pdf), PDF page 6 (zero-index 5); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-PSOM rev 11.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-PSOM/%5BMS-PSOM%5D.pdf), PDF page 12 (zero-index 11); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-E911WS rev 9.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-E911WS/%5BMS-E911WS%5D.pdf), PDF page 8 (zero-index 7); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-RTPRADEX rev 13.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-RTPRADEX/%5BMS-RTPRADEX%5D.pdf), PDF page 6 (zero-index 5); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-RTVPF rev 11.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-RTVPF/%5BMS-RTVPF%5D.pdf), PDF page 7 (zero-index 6); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-SSRTP rev 11.1** (2025-09-16), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-SSRTP/%5BMS-SSRTP%5D.pdf), PDF page 8 (zero-index 7); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-SIPRE rev 13.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-SIPRE/%5BMS-SIPRE%5D.pdf), PDF pages 15–18 (zero-index 14–17); text and page screenshot inspected; whole binary NOT downloaded/hashed.
- **MS-RTASPF rev 10.0** (2025-05-20), [published original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-RTASPF/%5BMS-RTASPF%5D.pdf), PDF page 6 (zero-index 5); text and page screenshot inspected; whole binary NOT downloaded/hashed.

All other investigated MS first-tier reference lists came from visible Microsoft Learn HTML sections; some are **older than the current published revision**. Example: MS-ABS latest rev 10.1 (2025-08-19) but its HTML §1.2.1 was last updated **2023-08-03**. Do NOT treat a current release metadata label as proving any live HTML normative list belongs to that edition. Document [MS-OCAUTHWS] has `draft-ietf-oauth-v2-22` in reference name but a URL pointing to `-23`; original draft mismatch still open.

## Non-overview Microsoft specifications discovered by explicit normative edges

- **MS-H26XPF** — referenced by MS-RTP; exact current/target published revision, normative closure and client applicability pending.
- **MS-KILE** — referenced by MS-SIPAE; exact current/target published revision, normative closure and client applicability pending.
- **MS-NLMP** — referenced by MS-SIPAE; exact current/target published revision, normative closure and client applicability pending.
- **MS-OAUTH2EX** — referenced by MS-OCAUTHWS; exact current/target published revision, normative closure and client applicability pending.
- **MS-OXWUMS** — referenced by MS-OCEXUM; exact current/target published revision, normative closure and client applicability pending.
- **MS-RDPBCGR** — referenced by MS-RTASPF; exact current/target published revision, normative closure and client applicability pending.
- **MS-RDPEMC** — referenced by MS-CONFAS, MS-RTASPF; exact current/target published revision, normative closure and client applicability pending.
- **MS-SDP** — referenced by MS-H264PF; exact current/target published revision, normative closure and client applicability pending.
- **MS-SIP** — referenced by MS-SIPREGE, MS-PRES, MS-CONFBAS; exact current/target published revision, normative closure and client applicability pending.
- **MS-WCCE** — referenced by MS-OCAUTHWS; exact current/target published revision, normative closure and client applicability pending.
- **MS-WSPOL** — referenced by MS-OCAUTHWS; exact current/target published revision, normative closure and client applicability pending.
- **MS-WSTEP** — referenced by MS-OCAUTHWS; exact current/target published revision, normative closure and client applicability pending.

Notably [MS-SIP](https://learn.microsoft.com/en-us/openspecs/windows_protocols/MS-SIP/0d72fb55-12ba-49f1-850d-f0bf3110bb1f) and [MS-H26XPF](https://learn.microsoft.com/en-us/openspecs/windows_protocols/MS-H26XPF/ebea5d4a-a184-4912-83a6-df29606aaa78) are from the Windows Open Specifications library, not MS-OCSPROT's six Office tables. RDP sharing further references MS-RDPBCGR and MS-RDPEMC. Kerberos/NTLM, OAuth/WS-Trust and certificate paths bring security dependencies beyond SIP.

## Fixed-edition standards and access blockers

- Historical **draft-ietf-sip-gruu-10**, ICE-06/19, SIP-Outbound-11 and several other explicitly cited IETF draft snapshots; finalized standards cannot be substituted without comparison.
- Paid/controlled original editions include **ISO 3166-1:2013**, **ISO 8601:2004**, **ITU E.164:2005**, **ISO/IEC 14496-10:2014** (H264), **SMPTE VC-1 421M:2006**, **IEEE 802.1AB:2005**, **ISO/IEC 29500-1:2011**, depending on product capabilities and formal use.
- Current MS-OCSPROT 9.0 direct PDF/DOCX redirected to an inaccessible host; a historical rev 4.4 from 2015 is not current normative proof. Some latest current indexed PDFs were readable as search-index excerpts, but full original binary hashes have **not** been obtained.
- Per-[MS-*] revision history and changelog delta comparisons remain to be done for the entire relevant graph; explicit newer-than-overview releases are **MS-ABS 10.1 (Aug 19 2025), MS-SIPREGE 17.1 (Aug 19 2025), MS-SSRTP 11.1 (Sep 16 2025)**.

## Objective completion conditions

To label **PHASE1_COMPLETE**, require **all** relevant MS/RFC/other references and exclusions traced recursively to a fixed point with edition/date/URI/section; all Microsoft §1.2.1 references verified from exact published edition; RFC Editor metadata, Updates/Updated By/Obsoletes/Obsoleted By/errata and normative references closed; every relevant source acquired/hash-checked or explicitly escalated for inaccessible originals; substantive previous-version deltas reviewed; Windows Full Client reference build and optional/policy-dependent features reconciled; independently review all unresolved discrepancies. None of the substantive closure prerequisites are met simply by enumerating Microsoft overview tables.

**Scope lock:** No protocol implementations, no real server traffic, no independent interoperability testing, no merge to main, no releases. Continue on `draft/protocol-spec-analysis-2026-10-23`.
