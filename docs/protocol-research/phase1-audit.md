# Phase 1 — formal source-acquisition and normative-reference audit

**Snapshot:** 2026-10-23. **Verdict: NOT COMPLETE — implementation remains locked.** This is a provenance audit, not a protocol implementation or compatibility attestation.

## Coverage, measured from the exact draft-branch JSON ledgers

| Required acquisition gate | Measured status | Closure |
| --- | --- | --- |
| MS-OCSPROT rev 9.0 (2025-05-20), all six overview system protocol tables | 6/6 tables; **46 distinct published MS protocol documents** | Named overview inventory OK; applicability and transitive references OPEN |
| Published version metadata (latest revision/date/direct official Microsoft landing) | **46/46** cataloged | Latest landing metadata complete for overview table items; exhaustive historical versions/diffs NOT complete |
| First-layer MS §1.2.1 normative reference lists | **46/46** documents' direct §1.2.1 lists inspected | All overview first-tier reference lists now enumerated; 33 other HTML-only documents still lack release-PDF comparison |
| Original released MS §1.2.1 index/PDF reference page and visual inspection | **12** indexed current edition PDF sections, plus **1 uploaded MS-AVEDGEA full PDF SHA-256-verified** | Other 45 full binaries not acquired / edition comparisons open |
| Normative directed source→target edges | **460** records to **199** distinct IDs | Includes 120 MS edges, 228 RFC edges, 29 historical IETF draft edges, 83 other standards. **Not transitive closure.** |
| RFC original fixed-edition seed/first-level references | **113** RFC IDs, of which **89** directly cited by investigated MS lists | RFC normative references, Updates/Obsoletes/Errata and dates for whole set NOT checked |
| Transitive MS Windows/Office specs outside overview | **12** distinct immediate Microsoft target IDs identified | All 12 latest published version/date identities now verified in [transitive-microsoft-inventory.json](transitive-microsoft-inventory.json), but precise client-applicable editions, full PDFs, their §1.2.1 transitive references and applicability remain OPEN |
| Official current published PDF/DOCX binaries acquired and hashed | **1** | User-supplied original **MS-AVEDGEA 12.0** (40 pages, 1,562,486 bytes, SHA-256 `baa3cddc1e1bfb4b0c2b0a998c71b802f80d4e5e6bb30adc0d8d42ebbbb0c8ff`); all other release files not acquired |
| Version comparisons of all MS documents and current Windows Client/SfB Server applicability | Only selected revisions/date rows checked | NOT complete; latest exact reference Windows client C2R build remains UNPINNED |

## The former MS-AVEDGEA first-tier source blocker is resolved

A user-supplied original PDF for **MS-AVEDGEA revision 12.0, 2025-05-20** has been independently file-checked, read and visually inspected: **40 pages, 1,562,486 bytes**, SHA-256 `baa3cddc1e1bfb4b0c2b0a998c71b802f80d4e5e6bb30adc0d8d42ebbbb0c8ff`. Exact §1.2.1 p.7: **5 normative refs** (draft-deason-sip-soap-00, RFC2119, W3C XMLNS 2009, W3C XML Schema parts 1+2, Second Edition 2004). §1.2.2 p.8: **5 separately informative refs** (MS-ICE, MS-ICE2, MS-SIPRE, MS-TURN, XML10). Revision history p.3 and change tracking p.38 identify revision 12.0 as Major and specifically **§7 supported product list updated**. Appendix B p.36 explicitly lists Skype for Business 2019 and Skype for Business Server 2019/Subscription Edition. This evidence is now in [MS-AVEDGEA v12 source audit](ms-avedgea-12-source-audit.md).

**All 46 MS-OCSPROT overview first-tier normative lists have now been read in some form. Phase 1 remains NOT COMPLETE** because these are not all matched to exact released PDF editions, all relevant transitive MS/IETF/W3C reference chains remain open, and the entire prior-revision change matrix is incomplete. The uploaded PDF bytes are available in the conversation session but are **not committed to the GitHub repository**; the hash enables future provenance checking when the original is reacquired.

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

**Update:** Latest official publication metadata for **all twelve** non-overview Microsoft destination IDs is now in [transitive-microsoft-inventory.md](transitive-microsoft-inventory.md). Four have newer 2026 versions (MS-KILE 49.0, MS-NLMP 37.0, MS-WCCE 55.0, MS-RDPBCGR 62.0) which must not be assumed as 2025 SfB Client's exact applicable editions.

## Fixed-edition standards and access blockers

- Historical **draft-ietf-sip-gruu-10**, ICE-06/19, SIP-Outbound-11 and several other explicitly cited IETF draft snapshots; finalized standards cannot be substituted without comparison.
- Paid/controlled original editions include **ISO 3166-1:2013**, **ISO 8601:2004**, **ITU E.164:2005**, **ISO/IEC 14496-10:2014** (H264), **SMPTE VC-1 421M:2006**, **IEEE 802.1AB:2005**, **ISO/IEC 29500-1:2011**, depending on product capabilities and formal use.
- Current MS-OCSPROT 9.0 direct PDF/DOCX redirected to an inaccessible host; a historical rev 4.4 from 2015 is not current normative proof. Some latest current indexed PDFs were readable as search-index excerpts, but full original binary hashes have **not** been obtained.
- Per-[MS-*] revision history and changelog delta comparisons remain to be done for the entire relevant graph; explicit newer-than-overview releases are **MS-ABS 10.1 (Aug 19 2025), MS-SIPREGE 17.1 (Aug 19 2025), MS-SSRTP 11.1 (Sep 16 2025)**.

## Objective completion conditions

To label **PHASE1_COMPLETE**, require **all** relevant MS/RFC/other references and exclusions traced recursively to a fixed point with edition/date/URI/section; all Microsoft §1.2.1 references verified from exact published edition; RFC Editor metadata, Updates/Updated By/Obsoletes/Obsoleted By/errata and normative references closed; every relevant source acquired/hash-checked or explicitly escalated for inaccessible originals; substantive previous-version deltas reviewed; Windows Full Client reference build and optional/policy-dependent features reconciled; independently review all unresolved discrepancies. None of the substantive closure prerequisites are met simply by enumerating Microsoft overview tables.

**Scope lock:** No protocol implementations, no real server traffic, no independent interoperability testing, no merge to main, no releases. Continue on `draft/protocol-spec-analysis-2026-10-23`.

## 2026-10-23 follow-on: 12 cross-library Microsoft reference sections now investigated

See [source-audited second-tier report](phase1-transitive-research-2026-10-23.md), [246 source-bound edges](transitive-reference-ledger.json), [146 new open target IDs](transitive-frontier.json). Eleven §1.2.1 reference lists were fully visible as *live official HTML*, and MS-RDPEMC was seen only in an indexed Microsoft search excerpt (full-page read failed). The separate **460/199 first-tier graph remains unchanged**. Second tier: **246 edges**, **183** distinct targets; **146** not seen in the first-tier target set (52 Microsoft/MC, 51 RFC, 43 external/draft). Combined target-union: **345**. No second-tier sources have an exact released-PDF reference-list edition match in this iteration. **MS-OAUTH2EX draft-v2-22 name vs v2-23 URL unresolved.** None of these counts implies complete normative closure or required Windows Full Client functionality. Phase 1 **INCOMPLETE**, implementation lock unchanged.

## 2026-10-23 RFC Editor release relation spot check

Ten original RFC-Editor publication/update/obsolescence headers were verified and recorded with exact RFC-Editor source URLs in [`rfc-version-relations-reviewed.json`](rfc-version-relations-reviewed.json): RFC 3265, 6665, 4566, 8866, 5245, 8445, 6125, 9525, 3548, 4648. [`rfc-inventory.json`](rfc-inventory.json) carries those verified relation metadata, preserving **113** initial RFC identifiers. **Full RFC normative references, errata and all other RFC update chains remain OPEN.** These status relations alone do **not** authorize substituting newer RFCs for explicitly cited older Microsoft editions.

## 2026-10-23 — RFC-first-hop P1-04 follow-on

See [original edition bibliography audit](rfc-first-hop-audit-2026-10-23.md) and [edge register](rfc-first-hop-reference-ledger.json): RFC 3261 §29 has 26 explicit normative edges; RFC 2782 has 9 **unclassified** distinct citations. 23 extra RFC identifiers were found outside the fixed 113 seed; no Microsoft first-/second-level reference ledgers or publication-check metrics change. MS-ABS and MS-SIPAE current release PDF attempts failed; 33 HTML-only edition comparisons and RFC normative/errata/recursion remain OPEN. **Phase 1 NOT COMPLETE, no implementation approval.**

## 2026-10-23 – Complete Phase-1 closure request: required gate check FAILED

[Formaler 58-Microsoft-Quellen-/12-Abschlussgates-Nachweis](phase1-closure-gates-2026-10-23.json) und [wissenschaftliches Quellenaudit](phase1-closure-audit-2026-10-23.md) werden aus den Draft-JSONs berechnet. **0/12 PASS; PHASE 1 NOT COMPLETE.** Überblick 13/46 genau revisionsbezogene §1.2.1-Referenzabschnitte (33 HTML-only), nur **eine** historisch gehashte vollständige Microsoft-PDF, die in dieser Sitzung nicht erneut zur Verfügung stand. Zwölf zweite Microsoft-Ausgaben weiterhin **0/12** original-PDF-abgeglichen (11 HTML, 1 Suchauszug MS-RDPEMC), 146 neue offene Referenzziele, RFC-Normrekursion/Errata offen. Exakter Windows Full Client 2019 Build, Server-Edition/Rollen/Policies und Norm-Versionseignung weiterhin nicht vorliegend. **Quellenkonflikt:** RFC 3261 (2002-06) §29 [6] nennt RFC3268 als 2002-06, RFC Editor Originalinfo für RFC3268 weist 2002-07 aus. Keine Implementierungs-/Live-/Release-Freigabe.

## 2026-10-23 – MS-ABS historisches Original-PDF-Audit ohne Freigabe der aktuellen Ausgabe

[MS-ABS Editionsaudit](ms-abs-edition-audit-2026-10-23.md) belegt die offiziellen PDF-§1.2.1-Seiten der **historischen** Revisionen **9.0 (2024-08-20)** und **10.0 (2025-05-20)** durch Text- und Seitenansicht: jeweils E164-2005, MC-RegEx, RFC2119 normativ und zwei gesonderte informative Quellen. In 10.0 nennt §8 ausdrücklich SfB 2019 / Server 2019 / Server Subscription Edition; §9 weist Änderungsschwerpunkte §1.1 und §8 aus. **Die gegenwärtig publizierte 10.1 (2025-08-19) wurde NICHT geprüft; vollständige Originaldateien wurden NICHT lokal bezogen/gehasht.** Darum bleibt MS-ABS aktuell `LIVE_HTML_SECTION_READ_RELEASE_EDITION_UNVERIFIED`: 13/46 aktuelle Editions-PDF-Belege und 33 HTML-only unverändert. Keine neuen Normkanten, keine zusätzlichen Gates PASS, keine Clientpflicht ohne separates Profil.
