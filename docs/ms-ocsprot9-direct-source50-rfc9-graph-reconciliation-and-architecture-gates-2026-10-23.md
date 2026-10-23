# MS-OCSPROT 9.0 exact reference set: 50 Microsoft + 9 RFC, crosswalk repair and client review gates

**Original-document audit date: 2026-10-23; NOT evidence of complete full-client coverage.** Primary **[MS-OCSPROT] version v20250520/rev9.0, May 20 2025**, SHA-256 `b0380ba5606072eb071bd67ffa7561e12cd7e55ec9dbd23a6553558438524ff2` (original PDF bytes SHA verified locally), official Microsoft PDF https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OCSPROT/%5BMS-OCSPROT%5D.pdf . This document's actual **§1.2 “References”** does **not** separate normative and informative citations. Direct bibliography extraction from the source **§1.2** yields exactly **50 distinct Microsoft ID references + 9 distinct RFC references**. This is the overview's bibliography, **not** full *transitive normative closure* and **not** evidence that every protocol is active in the 2015 classic Windows desktop client.

## 1. Machine-readable master catalogs committed now

- [50 Microsoft source references and pinned edition+SHA/provenance](research-corpus/ms-ocsprot9-50-direct-ms-reference-edition-audit-2026-10-23.csv). Each row records exact Microsoft document ID, functional subsystem, §1.2 source classification, official Microsoft original PDF URL, local manifest-pinned publication date/revision/SHA256, physical original presence, primary-research link if established, and explicitly **normative closure open**. **50/50 directly cited first-party original PDFs are available** in the existing 150-PDF corpus; nevertheless **35/50 do not have a newly linked primary-clause research file in this targeted crosswalk** and **all 50 require exhaustive section/edition applicability validation**. The 15 crosswalk research links indicate selected-clause analysis, not completion.
- [9 directly cited RFC edition/date/link rows](research-corpus/ms-ocsprot9-nine-direct-rfc-versions-2026-10-23.csv): **RFC2118** (MPPC, 1997), RFC3261 (SIP, 2002), **RFC3265** (SIP events 2002, later superseded by RFC6665), RFC3325 (asserted identity 2002), RFC3550 (RTP 2003), RFC3551 (RTP A/V profile 2003), RFC3892 (Referred-By 2004), RFC6120 and RFC6121 (XMPP 2011). **RFC2118 is the actual Microsoft Point-to-Point Compression RFC, NOT the familiar RFC2119 key word requirements RFC**; the two cannot be silently interchanged.
- [21 MS-OCSPROT §2.5 use-case and dependencies trace](research-corpus/ms-ocsprot9-21-usecase-client-coverage-trace-2026-10-23.csv), [use-case role/state original audit](ms-ocsprot9-21-usecase-full-client-functional-protocol-trace-2026-10-23.md), [CONMGMT original transport certificate/keepalive audit](ms-conmgmt10-sip-discovery-tls-fqdn-keepalive-proxy-expiry-original-2026-10-23.md), and [SIPAE / SIPREGE auth and survivable failure source conflict](ms-sipae11-siprege17-security-association-survivable-mode-source-conflicts-2026-10-23.md).

## 2. Concrete extraction discrepancy in previous 150-PDF edge export (fixed by companion catalog, not silently overwritten)

The existing source [150-node classified edges](research-corpus/microsoft-originals-150-classified-reference-edges-2026-10-22.csv) contains for source **MS-OCSPROT** only **49 MS + 8 RFC reference edges**, whereas its original PDF **§1.2** has **50 MS + 9 RFC**. The exact discrepancy is:

| Original evidence | Existing 2026-10-22 graph | Correct conclusion |
|---|---|---|
| §1.2 **[MS-OCGCWEB]** | **ABSENT** from MS-OCSPROT-source edges | **Missing MS bibliography edge**. MS-OCGCWEB 20160914 physically available, but its client-era behavior is not proven by this citation |
| §1.2 **[MS-QoE]** | Stored as `MS-QOE` (upper-case normalization) | **Same document**, not a second missing original or a new protocol. New catalog preserves `MS-QoE` original spelling |
| §1.2 **[RFC3265]** | **ABSENT** | **Missing RFC bibliography edge**; critical for 2002-style SUBSCRIBE/NOTIFY per MS-OCSPROT, despite later RFC6665 supersession |
| §1.2 **[RFC2118]** | PRESENT | It describes Microsoft Point-to-Point Compression, not RFC2119; do not mislabel historical compression normative role |
| Entire §1.2 | graph `overview_unclassified` | **Correct classification**; overview does not furnish `normative` vs `informative` reference subsections |

**Repair method:** The two new catalog CSVs are authoritative **supplemental audit artifacts of this source's direct bibliography**. The old **3,060-edge extract remains a reproducible historical corpus artifact** and has **not** been manually edited or falsely marked complete. To rebuild global 150-PDF graph, use normalized ID mapping plus an explicit **original §1.2 cross-check** for all overviews; record skipped references and edge provenance, regenerate whole graph atomically, rather than inserting two rows and implying full global validity. The old manifest's source MS-OCSPROT `unclassified_MS=49`, `unclassified_RFC=8` is therefore a **document-specific extraction undercount**, not a proof those originals were missing.

## 3. Source chain for classic Windows Full Client and version boundary

~~~mermaid
flowchart TD
 O[MS-OCSPROT rev9 overview 2025-05-20] --> A[50 direct Microsoft references]
 O --> B[9 directly listed historical RFCs]
 A --> C[MS-CONMGMT rev10 TLS/DNS/SIP transport]
 C --> D[MS-SIPAE rev11 per-server SA and MIC]
 D --> E[MS-SIPREGE rev17.1 REGISTER/GRUU]
 E --> F[MS-PRES self presence and MS-SIP contact/group SUBSCRIBE]
 F --> G{Service availability?}
 G -->|Available| H[User/roster per-feature readiness]
 G -->|Unavailable| I[Survivable-mode profile: only self-subscription SHOULD]
 I --> J[Self-sub failure unregister/retry original-source ambiguity]
 A --> K[MCU/Edge/ICE/SRTP and optional workstreams]
 K --> L[Independent audio/video/sharing modality readiness]
~~~

**Synthesis:** MS-SIPREGE rev17.1 has source publication **August 19 2025**, later than MS-OCSPROT rev9 **May 20 2025**; cross-references may **drift between document releases** (overview itself warns §1.2). Only statements verified against a specific source footer are direct P facts. No proof the Windows 2015 or 2016 client necessarily implements all 2025 server-side clauses.

## 4. New norm and timeline conflicts uncovered

| ID | Direct primary evidence | Interpretation and outstanding decision |
|---|---|---|
| COV-X01 | MS-OCSPROT §1.2 vs 2026-10-22 source-edge CSV | 50/9 vs 49/8 missing **MS-OCGCWEB** and **RFC3265**, plus QoE casing. **Catalog corrected**, entire global graph regeneration OPEN |
| COV-X02 | MS-OCSPROT §§1.2,2.5.1–2.5.5 | Login example uses ordered broad phases but provisioning requests in §2.5.3 **can occur in any order**. A deterministic single-threaded request sequence cannot be presumed source-normative |
| COV-X03 | MS-CONMGMT rev10 §3.1.5 vs RFC3263 | Four Microsoft DNS SRV queries and optional DHCP 120 plus HTTPS-port fallback do not equal generic SIP RFC3263 discovery; TLS certificate rejection should not silently downgrade to TCP |
| COV-X04 | MS-SIPAE rev11 §§3.2.1,3.2.2 vs MS-SIPREGE rev17.1 §§3.1.1.5.1,.6 | **Separate** SA expiration (8h minus ticket/cert buffer), REGISTER binding expiry, contact/group SUBSCRIBE, self-PRES SUBSCRIBE and hop keepalive. One `isLoggedIn` state insufficient |
| COV-X05 | MS-SIPREGE rev17.1 §3.1.1.5.1 | Self-subscription failure has **SHOULD REGISTER Expiry=0** and **SHOULD retry** conditioned by survivable status, but no order/precedence; protocol recovery **UNRESOLVED** |
| COV-X06 | Same clause | `Presence-Service-Available=false` signals unavailable/survivable; closing sentence claims it marks **end** of survivable. **Value polarity inconsistency** (potential spec erratum, not an implementer authority to change value). |
| COV-X07 | MS-SIPAE rev11 §§3.2.5.1–.2 | TLS-DSK embedded TLS messages and derived SIP sign keys are **distinct from transport TLS** and SRTP media protection; using one TLS valid flag would incorrectly authorize unrelated contexts |
| COV-X08 | MS-SIPREGE rev17.1 §3.1.1.6 vs MS-CONMGMT rev10 §3.4.6 | REGISTER refresh **MUST use initial TCP flow**, while proxy idle close when also registrar can silently remove binding with **no deregister NOTIFY**. Recovery on a *new* TCP connection requires binding/state reconciliation, not blind reuse old socket |
| COV-X09 | MS-OCSPROT §2.5.16 vs MS-QoE rev12 §3.2 | Overview happy-path postcondition suggests database stored while the QoE monitoring agent **202 Accepted only confirms acceptance**; durable telemetry storage proof absent |
| COV-X10 | MS-OCSPROT §2.5.17 and §§2.5.19–.20 | Focus, C3P INFO 202, conference subscription, ASMCU ICE/media, and PSOM security join **asynchronous independent protocols**. MCU media and whiteboard cannot be labelled ready from one SIP response |

## 5. Proof gates for continuation

1. **Rebuild 150-PDF normative graph with reference-extraction verification**: source section classifier, casing normalizer, original PDF exact source footer, versioned RFC/Internet drafts, and clearly distinguish overview bibliography from lower protocol normative lists. The existing larger corpus reports 191 MS IDs not locally present (68 from normative section lists) across broad Windows/Office references, distinct from **0 missing immediate 50 OCSPROT Microsoft PDFs**.
2. Audit all original normative clauses of each 50 Microsoft document (not merely §1.3 summaries). For exact Windows client build mark whether mandatory, conditional, legacy-only or server-only; the current 50-row catalog **deliberately does not claim that coverage is done**.
3. Resolve **register/survivable** contradiction from MS-SIPREGE §3.1.1.5.1 against official errata/product notes and historic editions; source may require external official clarification.
4. Resolve **SRTP** MS-SRTP rev14 SHA256 vs MS-SDPEXT rev16 / MS-SSRTP rev11.1 SHA1; no guessed crypto algorithm or automatic fallback.
5. Every 21 use-case overview diagram must be expanded into packet/message header, timer, error and security state models cross-linked to original RFC edition and Windows target build, including legacy XMPP/chat support, A/V Edge, conference C3P and E911.
6. **No server traffic, no tests**, no code or design implementation until all research gates evidence CLOSED. No merge or release.

**Research status: MS-OCSPROT direct bibliography completely catalogued (50+9); full transitive RFC/MS closure and Windows Full Client behavior NOT complete.**
