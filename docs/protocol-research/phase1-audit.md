# Phase 1 audit and unresolved normative-reference closure

**Snapshot:** 2026-10-23; Git branch `draft/protocol-spec-analysis-2026-10-23`. **PHASE 1: NOT COMPLETE.** Phase 1 requires *every relevant RFC and Microsoft specification and its recursive normative references*, version/date/update provenance, actual source availability and explicit exclusions. A landing-page index alone cannot meet that criterion.

## Evidence-backed coverage

| Audit gate | Measured / verified current state | Disposition |
| --- | --- | --- |
| Six [MS-OCSPROT] 9.0 overview protocol lists | **6/6** live overview groups inspected | Direct named overview inventory DONE; may not include every transitive/client-relevant item |
| Identifiers listed in those groups | **46 distinct [MS-*]**, deduplicated | Inventory DONE, full client relevance still OPEN |
| Latest publication metadata of overview documents | **46/46** official Microsoft Learn published revision/date rows recorded in `ms-ocsprot-inventory.json` | Landing metadata recorded, older edition comparisons and changelogs OPEN |
| Content of current published 2025+ [MS-*] PDFs/DOCXs | **0** fetched/hashed against exact revision from this environment | BLOCKED / download redirect, not silently marked accessed |
| Direct §1.2.1 normative reference lists of overview group | **4/46** live sections inspected; **42** not yet | STILL OPEN; inspected sections are **not PDF-revision-attested** |
| Direct normative links from inspected sections | **71 edges**, **58** distinct direct target IDs; by type: microsoft_spec=28, rfc=33, ietf_draft=6, external=4 | Only first-level and partial; transitive graph OPEN |
| RFC Editor seed corpus | **71** distinct RFCs with official persistent info URIs; **25** named in the four inspected MS sections | Candidates plus limited direct normative references; update/errata/reference closure OPEN |
| IETF historical drafts | GRUU draft-ietf-sip-gruu-10; multiple 2005–2008 ICE/offer-answer drafts | Exact historic editions named, originals/access and obsolescence context OPEN |
| Non-IETF normative standards | ISO 3166-1:2013, ISO 8601:2004, SOAP 1.1, MSFT-H264UCConfig 2011 | Paywall/legacy origins and actual source access OPEN |
| Applicability to exact Windows Full Client build | Exact reference C2R/build remains UNPINNED in `WINDOWS_REFERENCE_CASES.json` | OPEN |
| Current revision ↔ earlier releases for each MS-* | Only selected prior edition rows checked, especially MS-ABS10.0→10.1, MS-SIPREGE17.0→17.1, MS-SSRTP11.0→11.1 | 46-document historical change matrix OPEN |

### The 42 overview specifications whose §1.2.1 normative reference section has NOT yet been extracted

- **MS-ABS** (10.1; 2025-08-19): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abs/ddc59abf-1f42-4935-9b72-5314665dbcff)
- **MS-DLX** (13.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-dlx/fa0e41cd-2b34-4081-b3cf-59ecdad7a5da)
- **MS-CONMGMT** (10.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/9f6ebbf0-9000-4dc8-86cd-ebfe22467cbf)
- **MS-EUMR** (9.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-eumr/3ef7d3c4-2b15-4d79-a8e4-8eca927540ac)
- **MS-OCEXUM** (11.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocexum/d2018f70-8c96-481b-859e-d3b741618fa2)
- **MS-OCPSTN** (11.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocpstn/6e69d8d4-ba59-484f-9f2f-61d8ee4ed0d9)
- **MS-OCER** (11.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocer/8b2b5ee5-6051-4f66-b51e-71102ef7782f)
- **MS-QoE** (12.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-qoe/c1050431-f4b2-44d4-a425-63d75e99f5df)
- **MS-SIPCOMP** (10.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipcomp/eaf7bc25-7128-454a-841e-539dfca2207d)
- **MS-SIPAE** (11.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd)
- **MS-SIPRE** (13.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipre/ab4ab249-3796-4ed1-8cec-f496d81a1a83)
- **MS-RGSWS** (9.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rgsws/762316e0-3c6c-42c2-b3e8-41a9901e1b7c)
- **MS-PSOM** (11.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-psom/0b9a26ba-b92e-4a71-a1ea-205c5a0c250c)
- **MS-SIPAPP** (8.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipapp/2cc5f99e-7190-4672-bb24-6dc9c6347019)
- **MS-OCAUTHWS** (11.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocauthws/63749828-ab8d-4b24-941c-b11985d854c5)
- **MS-E911WS** (9.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-e911ws/ab5d7449-2c15-434b-bf65-fdf38b8ffabd)
- **MS-XCCOSIP** (7.0; 2024-08-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-xccosip/e1afc2e0-8b2d-4fa4-a162-788349202345)
- **MS-OCGCWEB** (3.0; 2018-04-27): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocgcweb/7bab1c71-de7b-424c-afaa-7678c3cc54ed)
- **MS-CONFBAS** (15.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confbas/6cb739fe-3a84-4266-8d52-0af777f6f1fa)
- **MS-CONFPRO** (13.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confpro/ec4a3626-66f9-4270-8adc-5f81f7fdad8a)
- **MS-CONFIM** (11.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confim/6f14d1ec-c854-4360-bdda-a6c61a655a60)
- **MS-CONFAV** (11.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confav/8e51c8b8-9869-4bff-b67e-56dbffefaa77)
- **MS-CONFAS** (12.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confas/bd7cc980-49d4-4ccb-a9c6-f3fda247ec8f)
- **MS-XMLMC** (10.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-xmlmc/5b0ef2c2-f3f0-4223-8c40-aeb02d5d74d7)
- **MS-OCSMP** (7.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsmp/53c3507c-c987-47a9-9bc8-bb2baa18e1b9)
- **MS-OCDISCWS** (10.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocdiscws/95554ed7-530a-4a27-beee-b353ea9f7e95)
- **MS-CVWREST** (7.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-cvwrest/62436ec9-e9c9-4e72-9a3f-3081a8f92f67)
- **MS-ECREST** (8.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ecrest/4454be95-159a-4c99-a75a-5c260609343a)
- **MS-EUMSDP** (9.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-eumsdp/c01cded2-02a3-467a-a959-5ebdc91e542b)
- **MS-DTMF** (15.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-dtmf/7a2c80c0-5fa7-4b8c-9216-c33804f0e0d4)
- **MS-H264PF** (16.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-h264pf/f3024858-350e-49b0-9866-99c4fd2b912d)
- **MS-RTPRADEX** (13.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtpradex/c28f4661-9e7c-4319-b6e5-58d0dba6b823)
- **MS-RTASPF** (10.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtaspf/42824cea-064e-45b3-a81b-24584449d2eb)
- **MS-RTVPF** (11.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtvpf/3f37858b-c991-41c4-bbbd-df10fcffc89e)
- **MS-SRTP** (14.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-srtp/d9641c95-b152-4cc7-8311-d178f3241f1f)
- **MS-SSRTP** (11.1; 2025-09-16): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ssrtp/980fc2b6-ba83-4313-9f5a-cad50449e98e)
- **MS-AVEDGEA** (12.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-avedgea/56860f6a-2fcf-4cad-8ee5-466d8f382a45)
- **MS-ICE** (11.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice/4feb9bf6-dbe8-4713-a228-e7e697c57eae)
- **MS-ICE2** (19.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice2/8cd2fe91-1aa6-49e1-b482-f92b5c40b7e2)
- **MS-ICE2BWM** (9.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice2bwm/9da023f3-da10-430d-825b-1d9d7b2ac5ca)
- **MS-TURN** (21.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-turn/9e434b27-eb13-4249-b031-2d15c3835c8b)
- **MS-TURNBWM** (9.0; 2025-05-20): [official published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-turnbwm/7fd11bbc-3e10-476c-80be-9d62b1b35b1a)

**Caution:** The other four (MS-SIPREGE, MS-SDPEXT, MS-RTP, MS-PRES) are *not fully studied documents*, merely their `§1.2.1` live reference lists. They remain open for published PDF version matching, `§1.2.2` informative reference expansion where useful and full later Phase-2 analysis.

## Authoritative retrieval/access findings

- Current [MS-OCSPROT 9.0 published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c), released **2025-05-20**. On 2026-10-23 the landing [PDF] download `https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OCSPROT/%5BMS-OCSPROT%5D.pdf` redirected to `https://officeprotocoldoc.z19.web.core.windows.net/files/MS-OCSPROT/%5BMS-OCSPROT%5D.pdf` which was rejected by the research browser. The [DOCX] link `.../files/MS-OCSPROT/%5BMS-OCSPROT%5D-250520.docx` redirected to the matching `z19.web.core.windows.net` host and was also rejected. **No file bytes/hash obtained.** This is **a browser/access restriction**, not proof the underlying docs do not exist.
- Microsoft's HTML reference list for [MS-SIPREGE §1.2.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/24cbca88-e618-4d70-979d-3c52d6ec5c04) shows last update 2025-08-19; [MS-SDPEXT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/be2f02ae-6687-444c-b21b-64553bb00ed5) and [MS-RTP](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtp/9258c646-3fa5-4702-a9cf-86b30029ca7a) §1.2.1 pages show last update 2024-04-16 despite May 2025 published revisions. [MS-PRES §1.2.1](https://learn.microsoft.com/fr-ca/openspecs/office_protocols/ms-pres/a25dbe23-7871-431a-a8a9-1ea95138a7ee) has a functional French-Canadian page with English reference text, last update 2025-05-20; the en-US equivalent was inaccessible in this browser. Each page displays an authorization warning **and** visible normative-reference content. Visibility of text **does not certify** the published release's PDF content.
- [MS-PRES §1.2.1](https://learn.microsoft.com/fr-ca/openspecs/office_protocols/ms-pres/a25dbe23-7871-431a-a8a9-1ea95138a7ee) explicitly marks **ISO 3166-1:2013** and **ISO 8601:2004** as **paid to download**. These reference editions must stay marked paywalled/unverified; newer free guidance is not the same edition.
- [MS-RTP §1.2.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtp/9258c646-3fa5-4702-a9cf-86b30029ca7a) points to MS-H26XPF and 2011 *UCConfig H.264* PDF; [MS-SIPREGE and MS-PRES] cite **MS-SIP** from the **Windows Protocols** library. Additional Windows specs MS-KILE, MS-NLMP, MS-RDPBCGR, and MS-ABCH are pending explicit versioned landing verification and transitive applicability decisions.
- The Skype for Business 2019/SE target needs actual client build, server deployment roles and policy surface to decide conditional reachability. Missing product configuration does **not** authorize dropping optional/deployment-specific specs; mark relevance unresolved.

## Why not claim Phase 1 finished

Even with all 46 overview-document metadata recorded, **at least 42 first-layer normative lists** are outstanding; additional cross-linked Microsoft specs and RFC normative chains are still unexpanded. The current published document downloads have not been acquired/reconciled at the binary level, there is no complete prior-release/update matrix for 46 specs, and editions are not pinned for all external normative standards. **Therefore the user-mandated Phase 1 completion condition is demonstrably false.** Continue only on this Draft branch, no code/tests/server traffic/merge/release.

## Objective exit criteria for a future evidence-backed Phase 1 finish

1. Obtain each **actually applicable** published Microsoft PDF/DOCX; record release URL, version/date, SHA-256, successful parse/page counts and version-to-HTML comparison. Alternatively document exact unattainability and explicitly agree on a suitable authoritative surrogate before closing that document.
2. For all 46 overview documents plus relevant non-overview Windows/Office docs: capture **§1.2.1 direct normative targets**, resolve target exact editions, **§1.2.2 informative references** as context, iterate cross-document until fixed point, with traceable approved exclusion reason for server-only items.
3. For all applicable RFCs: capture publication metadata, RFC normative references, **Updates/Updated By/Obsoletes/Obsoleted By** and errata; preserve Microsoft's historic normative profile and compare replacements.
4. Compare every relevant Microsoft document's published version and dated history with earlier versions and the Windows Full Client/Server 2019/SE deployment compatibility profile.
5. Resolve all access, paywall, historic draft and product-applicability unknowns through evidence or record explicit source-inaccessible blocking disposition; include exact follow-up/source acquisition.
6. Verify inventory counts mechanically, have reviewer sign off **all** per-document access/reference statuses, and only then mark `PHASE1_COMPLETE`. Not done in this pass.

## Continuation and source-of-truth pointers

- [ms-ocsprot-inventory.json](ms-ocsprot-inventory.json) — 46 versioned overview IDs and primary URLs.
- [normative-reference-ledger.json](normative-reference-ledger.json) — 71 source-linked edges from four live §1.2.1 sections, all revision-binding flagged unverified.
- [rfc-inventory.json](rfc-inventory.json) — 71 RFC seeds, distinct direct normative/secondary status.
- [ms-ocsprot-inventory.md](ms-ocsprot-inventory.md) — human-readable full overview version/source table.
- [research-log.md](research-log.md) — next steps, strict no-implementation gate.
