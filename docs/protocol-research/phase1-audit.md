# Phase 1 audit — verified source coverage and blockers

**Recorded 2026-10-23 · STATUS: PHASE 1 NOT COMPLETE.** Source identity and version/date cataloging are only one part of Phase 1. Direct normative references, complete current-original acquisition, update histories, cross-referenced non-overview protocols and exact client applicability are still mandatory. Do not infer completeness from counts.

## Objective checked coverage

| Gate | Observed status | Verdict |
| --- | --- | --- |
| MS-OCSPROT 9.0 (published 2025-05-20) overview families | All **6** published live protocol tables inspected | Source-list coverage achieved |
| MS-* entries in six overview families | **46/46 distinct IDs** with Microsoft landing/latest revision/date checked | Source metadata inventory achieved; applicability and earlier version history OPEN |
| First-tier §1.2.1 normative references | **14/46** documents checked; **32/46 still to examine** | NOT complete |
| Original indexed 2025 Microsoft PDF §1.2.1 text with screenshot | **5** published-edition PDF sources; partial reference pages | Edition-bound reference evidence for those sections only, not full document acquisition |
| Other §1.2.1 live Microsoft HTML pages | **10**, may predate current released PDF | Listed but release-edition comparison open |
| Direct normative edges and distinct target IDs | **213 edges, 131 distinct target IDs** (RFC, other MS, IETF drafts, ISO/W3C/other) | Partial **first-tier only**, recursive closure OPEN |
| RFC Editor candidate + directly named sources | **88** total RFC IDs, **58** RFC IDs named in inspected MS §1.2.1 lists | Full publication metadata/reference/update/errata graph OPEN |
| Entire exact published 2025+ MS PDFs/DOCXs stored with SHA-256 | **0** | **Blocked/unacquired**; indexed rendered snippets are not bytes |
| Historical MS editions and dated update comparisons | Selected release rows checked (not every revision/diff for 46 specs) | OPEN |
| Exact classic Windows Full Client build/applicability policy | Unpinned in repository WINDOWS_REFERENCE_CASES.json | OPEN |

## Published-edition §1.2.1 original pages independently inspected

- **MS-CONMGMT 10.0** (2025-05-20), [current indexed original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-CONMGMT/%5BMS-CONMGMT%5D.pdf), §1.2.1 PDF pp. 8 (index 7); text AND screenshot checked; **full PDF bytes/hash unavailable**.
- **MS-CONFBAS 15.0** (2025-05-20), [current indexed original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-CONFBAS/%5BMS-CONFBAS%5D.pdf), §1.2.1 PDF pp. 13–14 (indices 12–13); text AND screenshot checked; **full PDF bytes/hash unavailable**.
- **MS-OCAUTHWS 11.0** (2025-05-20), [current indexed original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-OCAUTHWS/%5BMS-OCAUTHWS%5D.pdf), §1.2.1 PDF pp. 11–13 (indices 10–12); text AND screenshot checked; **full PDF bytes/hash unavailable**.
- **MS-PRES 13.0** (2025-05-20), [current indexed original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-PRES/%5BMS-PRES%5D.pdf), §1.2.1 PDF pp. 15–16 (indices 14–15); text AND screenshot checked; **full PDF bytes/hash unavailable**.
- **MS-ECREST 8.0** (2025-05-20), [current indexed original PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-ECREST/%5BMS-ECREST%5D.pdf), §1.2.1 PDF page 6 (zero-index 5); text AND screenshot checked; **full PDF bytes/hash unavailable**.

These search-index exposed originals can be read/page-rendered, even though direct PDF/DOCX links for the current overview were blocked by redirects. They do **not** establish availability of the complete original downloaded bytes or SHA-256; their full document content remains unprocessed.

## §1.2.1 inspected only as HTML

- **MS-SIPREGE 17.1**, [official §1.2.1 HTML](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/24cbca88-e618-4d70-979d-3c52d6ec5c04), HTML last updated 2025-08-19; **published-revision matching unverified**.
- **MS-SDPEXT 16.0**, [official §1.2.1 HTML](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/be2f02ae-6687-444c-b21b-64553bb00ed5), HTML last updated 2024-04-16; **published-revision matching unverified**.
- **MS-RTP 17.0**, [official §1.2.1 HTML](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtp/9258c646-3fa5-4702-a9cf-86b30029ca7a), HTML last updated 2024-04-16; **published-revision matching unverified**.
- **MS-PRES 13.0**, [official §1.2.1 HTML](https://learn.microsoft.com/fr-ca/openspecs/office_protocols/ms-pres/a25dbe23-7871-431a-a8a9-1ea95138a7ee), HTML last updated 2025-05-20; **published-revision matching unverified**.
- **MS-SIPAE 11.0**, [official §1.2.1 HTML](https://learn.microsoft.com/th-th/openspecs/office_protocols/ms-sipae/e77ebace-a758-4e64-b3db-327db63f9dea), HTML last updated 2024-08-20; **published-revision matching unverified**.
- **MS-OCSMP 7.0**, [official §1.2.1 HTML](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsmp/985ab5c8-ec6f-4303-b27e-bedc3834c669), HTML last updated 2025-03-18; **published-revision matching unverified**.
- **MS-CVWREST 7.0**, [official §1.2.1 HTML](https://learn.microsoft.com/th-th/openspecs/office_protocols/ms-cvwrest/b3e19a54-7499-4d03-bbb7-51e1a4e5902b), HTML last updated 2025-05-20; **published-revision matching unverified**.
- **MS-OCDISCWS 10.0**, [official §1.2.1 HTML](https://learn.microsoft.com/hr-hr/openspecs/office_protocols/ms-ocdiscws/967752c2-051e-48f9-834e-d7017bca2b60), HTML last updated 2024-08-20; **published-revision matching unverified**.
- **MS-ABS 10.1**, [official §1.2.1 HTML](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abs/2e6f2cd1-b0a5-4aea-8b53-048e7b217c1d), HTML last updated 2023-08-03; **published-revision matching unverified**.
- **MS-DLX 13.0**, [official §1.2.1 HTML](https://learn.microsoft.com/nb-no/openspecs/office_protocols/ms-dlx/1772eb00-a266-4158-bfb3-1964e579e6ba), HTML last updated 2025-05-20; **published-revision matching unverified**.

Notably MS-ABS latest version is **10.1 (2025-08-19)** while its §1.2.1 live HTML page is last updated **2023-08-03**. Other HTML pages also predate their catalog release; do **not** impute the published release's exact reference list to older HTML without comparison.

## First-tier normative §1.2.1 lists not yet examined (32)

- **MS-EUMR** rev 9.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-eumr/3ef7d3c4-2b15-4d79-a8e4-8eca927540ac)
- **MS-OCEXUM** rev 11.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocexum/d2018f70-8c96-481b-859e-d3b741618fa2)
- **MS-OCPSTN** rev 11.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocpstn/6e69d8d4-ba59-484f-9f2f-61d8ee4ed0d9)
- **MS-OCER** rev 11.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocer/8b2b5ee5-6051-4f66-b51e-71102ef7782f)
- **MS-QoE** rev 12.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-qoe/c1050431-f4b2-44d4-a425-63d75e99f5df)
- **MS-SIPCOMP** rev 10.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipcomp/eaf7bc25-7128-454a-841e-539dfca2207d)
- **MS-SIPRE** rev 13.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipre/ab4ab249-3796-4ed1-8cec-f496d81a1a83)
- **MS-RGSWS** rev 9.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rgsws/762316e0-3c6c-42c2-b3e8-41a9901e1b7c)
- **MS-PSOM** rev 11.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-psom/0b9a26ba-b92e-4a71-a1ea-205c5a0c250c)
- **MS-SIPAPP** rev 8.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipapp/2cc5f99e-7190-4672-bb24-6dc9c6347019)
- **MS-E911WS** rev 9.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-e911ws/ab5d7449-2c15-434b-bf65-fdf38b8ffabd)
- **MS-XCCOSIP** rev 7.0 (2024-08-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-xccosip/e1afc2e0-8b2d-4fa4-a162-788349202345)
- **MS-OCGCWEB** rev 3.0 (2018-04-27) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocgcweb/7bab1c71-de7b-424c-afaa-7678c3cc54ed)
- **MS-CONFPRO** rev 13.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confpro/ec4a3626-66f9-4270-8adc-5f81f7fdad8a)
- **MS-CONFIM** rev 11.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confim/6f14d1ec-c854-4360-bdda-a6c61a655a60)
- **MS-CONFAV** rev 11.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confav/8e51c8b8-9869-4bff-b67e-56dbffefaa77)
- **MS-CONFAS** rev 12.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confas/bd7cc980-49d4-4ccb-a9c6-f3fda247ec8f)
- **MS-XMLMC** rev 10.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-xmlmc/5b0ef2c2-f3f0-4223-8c40-aeb02d5d74d7)
- **MS-EUMSDP** rev 9.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-eumsdp/c01cded2-02a3-467a-a959-5ebdc91e542b)
- **MS-DTMF** rev 15.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-dtmf/7a2c80c0-5fa7-4b8c-9216-c33804f0e0d4)
- **MS-H264PF** rev 16.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-h264pf/f3024858-350e-49b0-9866-99c4fd2b912d)
- **MS-RTPRADEX** rev 13.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtpradex/c28f4661-9e7c-4319-b6e5-58d0dba6b823)
- **MS-RTASPF** rev 10.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtaspf/42824cea-064e-45b3-a81b-24584449d2eb)
- **MS-RTVPF** rev 11.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtvpf/3f37858b-c991-41c4-bbbd-df10fcffc89e)
- **MS-SRTP** rev 14.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-srtp/d9641c95-b152-4cc7-8311-d178f3241f1f)
- **MS-SSRTP** rev 11.1 (2025-09-16) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ssrtp/980fc2b6-ba83-4313-9f5a-cad50449e98e)
- **MS-AVEDGEA** rev 12.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-avedgea/56860f6a-2fcf-4cad-8ee5-466d8f382a45)
- **MS-ICE** rev 11.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice/4feb9bf6-dbe8-4713-a228-e7e697c57eae)
- **MS-ICE2** rev 19.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice2/8cd2fe91-1aa6-49e1-b482-f92b5c40b7e2)
- **MS-ICE2BWM** rev 9.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice2bwm/9da023f3-da10-430d-825b-1d9d7b2ac5ca)
- **MS-TURN** rev 21.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-turn/9e434b27-eb13-4249-b031-2d15c3835c8b)
- **MS-TURNBWM** rev 9.0 (2025-05-20) — [Microsoft published version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-turnbwm/7fd11bbc-3e10-476c-80be-9d62b1b35b1a)

## Out-of-overview transitive candidates and unacquired source editions

- **Windows:** MS-SIP, MS-KILE, MS-NLMP, MS-RDPBCGR, MS-H26XPF, MS-WCCE, MS-WSTEP, MS-WSPOL, MS-OAUTH2EX and other documents discovered in the normative edge dataset. Must find/review exact published revision, date and their own normative references; determine specific client relevance.
- **IETF:** legacy drafts (including **draft-ietf-sip-gruu-10**, early ICE/offer-answer drafts and **draft-ietf-oauth-v2-22**) are cited as such. Their final RFCs must not silently replace normative draft wire rules. The MS-OCAUTHWS published PDF reference heading mentions `draft-ietf-oauth-v2-22` while its cited URL targets `draft-ietf-oauth-v2-23`; inspect the dated original to resolve.
- **ISO/ITU:** MS-PRES normative ISO 3166-1:2013 and ISO 8601:2004, plus MS-ABS E.164:2005, are **paywalled** original editions. No source bytes acquired or edition-confirmed content summary.
- **W3C/OASIS:** SOAP 1.1/1.2, SAML, WS-Trust, WS-Addressing, WS-Federation, XML schema, WS-Security; versions and actual normative cross-links in ledger, full original text review not done.
- **MS-OCSPROT 9.0:** the direct Microsoft PDF/DOCX downloads redirected to a host rejected by available tool; historical overview 2015 revision 4.4 is not current normative evidence.

## Precise outstanding Phase-1 exit gates

1. Read the 32 remaining §1.2.1 sources from **their exact published release** or record explicitly where originals cannot be acquired. Collect 1.2.2 informative links as contextual clues.
2. Recursively follow relevant *normative* MS, IETF draft, standards and RFC references until fixed point; document explicit evidence-backed exclusions for optional/server-only dependencies. A first-tier overview list is not transitive closure.
3. Capture published release date, revision, previous revisions, substantive changes, complete original source URLs, download/access status and **SHA-256** for every relevant document. When inaccessible, maintain unresolved blocking status rather than claim verified.
4. Traverse RFC Editor original fixed-edition normative references and **Updates, Updated by, Obsoletes, Obsoleted by, errata**; separate historic Microsoft-defined profiles from modern replacements.
5. Reconcile exact Windows Full Client build / server 2019/SE supported roles, features and policy and map scope to 122 features / 93 baseline scenarios, without dropping unverified candidate sources.
6. Obtain documented complete reviewer signoff before marking Phase 1 complete; **no new protocol code, real server tests, main changes, merge, or release**.

## Readable and machine-readable continuation

- [MS-OCSPROT revision catalog](ms-ocsprot-inventory.md), [JSON inventory](ms-ocsprot-inventory.json)
- [Normative edges with individual source URL, date/section and access status](normative-reference-ledger.json)
- [RFC seeded ID corpus](rfc-inventory.json)
- [Central source catalog](source-catalog.md)
- [Research log](research-log.md)

**Conclusion:** **Phase 1 is NOT demonstrably complete**. The source graph is incomplete (32 first-layer documents still unexamined plus recursive references), so source or feature completeness may not be claimed.
