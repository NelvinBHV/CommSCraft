**Drittstufen-Nachtrag 24.10.2026:** Aus 52 bereits vorliegenden MS-/MC-PDFs 48 ausdrücklich normative §1.2.1-Verzeichnisse mit 827 Kanten erfasst (398 MS/MC, 267 RFC, 162 externe/historische), vier Schema-§1.1-Verzeichnisse mit 67 bewusst *unklassifizierten* Literaturhinweisen separat dokumentiert. 236 neue exakte Roh-Zielkennungen, davon 86 MS/MC; Raw-Label-Gesamtunion 581 (keine Normkanonisierung oder Clientpflicht). [Originale & Graph](third-ring-original-bibliography-audit-2026-10-24.md) · [Kanten-TSV GZIP](third-ring-bibliography-edges-2026-10-24.tsv.gz). Original-PDFs bleiben vereinbarungsgemäß beim Nutzer, auf Bedarf gezielt anzufordern. Phase 1 weiterhin **0/12** bestanden; keine Implementierung oder Live-Tests.

# Official Microsoft and RFC protocol document catalog — dated Phase-1 inventory

**Original snapshot 2026-10-23; latest source-acquisition update 2026-10-24. Phase 1 NOT COMPLETE.** All 46 current overview §1.2.1 PDF source sections have evidence; 464 directed first-tier normative edges, 199 distinct first-tier targets. **12/12** exact current second-tier Microsoft §1.2.1 bibliographies now independently checked from complete SHA-256-verified PDFs; **246** existing second-tier edges, 183 second-tier targets. Original PDFs for **all 52** previously discovered Microsoft/MC third-tier target IDs are present across Office/Exchange/SharePoint/Windows sources, but **no recursive normative closure/client feature requirement has been established**. Total 822 hashed PDF files across three upload tranches (74 Office, 297 Exchange/SharePoint, 451 Windows), including duplicates and historical editions. Full source binaries **not publicly archived in GitHub**. No historical full-text diff, standards closure, signed-in/live interoperability or Phase-1 gate approval.

## Canonical datasets

- [MS overview inventory JSON](ms-ocsprot-inventory.json) and [human readable version table](ms-ocsprot-inventory.md).
- [Twelve additional Microsoft/Windows/Exchange/SharePoint source versions](transitive-microsoft-inventory.md) and [JSON](transitive-microsoft-inventory.json) — latest published metadata verified, applicability and recursive references OPEN.
- [MS-AVEDGEA 12.0 uploaded original and SHA-256 provenance](ms-avedgea-12-source-audit.md) — closes the last overview first-tier §1.2.1 source list, 5 normative / 5 informative refs.
- [Normative reference graph](normative-reference-ledger.json) — source-pinned §1.2.1 MS→target edges, reference types and edition/access flags.
- [RFC fixed-edition source ledger](rfc-inventory.json) — direct vs seed evidence, canonical RFC Editor links, update-chain status.
- [Phase-1 formal coverage audit](phase1-audit.md) — exact missing sources/access blockers and acceptance criteria.
- [Research/continuation log](research-log.md) — dated snapshots, limitations and no-code constraint.

## Microsoft source revisions, publication dates, exact source URL and acquisition status

| MS identifier | Family (MS-OCSPROT §) | Published revision | Published date | Official Microsoft landing | Direct §1.2.1 evidence |
| --- | --- | --- | --- | --- | --- |
| MS-ABS | directory §2.2.1 | 10.1 | 2025-08-19 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abs/ddc59abf-1f42-4935-9b72-5314665dbcff) | LIVE_HTML_SECTION_READ_RELEASE_EDITION_UNVERIFIED |
| MS-DLX | directory §2.2.1 | 13.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-dlx/fa0e41cd-2b34-4081-b3cf-59ecdad7a5da) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-CONMGMT | sip §2.2.2.1 | 10.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/9f6ebbf0-9000-4dc8-86cd-ebfe22467cbf) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-EUMR | sip §2.2.2.1 | 9.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-eumr/3ef7d3c4-2b15-4d79-a8e4-8eca927540ac) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-OCEXUM | sip §2.2.2.1 | 11.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocexum/d2018f70-8c96-481b-859e-d3b741618fa2) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-OCPSTN | sip §2.2.2.1 | 11.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocpstn/6e69d8d4-ba59-484f-9f2f-61d8ee4ed0d9) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-OCER | sip §2.2.2.1 | 11.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocer/8b2b5ee5-6051-4f66-b51e-71102ef7782f) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-PRES | sip §2.2.2.1 | 13.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/741d682d-590b-4600-916e-8deeee426a5c) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-QoE | sip §2.2.2.1 | 12.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-qoe/c1050431-f4b2-44d4-a425-63d75e99f5df) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-SIPCOMP | sip §2.2.2.1 | 10.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipcomp/eaf7bc25-7128-454a-841e-539dfca2207d) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-SIPAE | sip §2.2.2.1 | 11.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd) | LIVE_HTML_SECTION_READ_RELEASE_EDITION_UNVERIFIED |
| MS-SIPRE | sip §2.2.2.1 | 13.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipre/ab4ab249-3796-4ed1-8cec-f496d81a1a83) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-SIPREGE | sip §2.2.2.1 | 17.1 | 2025-08-19 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d) | LIVE_HTML_SECTION_READ_RELEASE_EDITION_UNVERIFIED |
| MS-RGSWS | sip §2.2.2.1 | 9.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rgsws/762316e0-3c6c-42c2-b3e8-41a9901e1b7c) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-PSOM | sip §2.2.2.1 | 11.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-psom/0b9a26ba-b92e-4a71-a1ea-205c5a0c250c) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-SIPAPP | sip §2.2.2.1 | 8.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipapp/2cc5f99e-7190-4672-bb24-6dc9c6347019) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-OCAUTHWS | sip §2.2.2.1 | 11.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocauthws/63749828-ab8d-4b24-941c-b11985d854c5) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-E911WS | sip §2.2.2.1 | 9.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-e911ws/ab5d7449-2c15-434b-bf65-fdf38b8ffabd) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-XCCOSIP | sip §2.2.2.1 | 7.0 | 2024-08-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-xccosip/e1afc2e0-8b2d-4fa4-a162-788349202345) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-OCGCWEB | sip §2.2.2.1 | 3.0 | 2018-04-27 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocgcweb/7bab1c71-de7b-424c-afaa-7678c3cc54ed) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-CONFBAS | conference §2.2.2.2 | 15.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confbas/6cb739fe-3a84-4266-8d52-0af777f6f1fa) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-CONFPRO | conference §2.2.2.2 | 13.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confpro/ec4a3626-66f9-4270-8adc-5f81f7fdad8a) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-CONFIM | conference §2.2.2.2 | 11.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confim/6f14d1ec-c854-4360-bdda-a6c61a655a60) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-CONFAV | conference §2.2.2.2 | 11.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confav/8e51c8b8-9869-4bff-b67e-56dbffefaa77) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-CONFAS | conference §2.2.2.2 | 12.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confas/bd7cc980-49d4-4ccb-a9c6-f3fda247ec8f) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-XMLMC | conference §2.2.2.2 | 10.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-xmlmc/5b0ef2c2-f3f0-4223-8c40-aeb02d5d74d7) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-OCSMP | http §2.2.2.3 | 7.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsmp/53c3507c-c987-47a9-9bc8-bb2baa18e1b9) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-OCDISCWS | http §2.2.2.3 | 10.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocdiscws/95554ed7-530a-4a27-beee-b353ea9f7e95) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-CVWREST | http §2.2.2.3 | 7.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-cvwrest/62436ec9-e9c9-4e72-9a3f-3081a8f92f67) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-ECREST | http §2.2.2.3 | 8.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ecrest/4454be95-159a-4c99-a75a-5c260609343a) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-EUMSDP | media §2.2.3.1 | 9.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-eumsdp/c01cded2-02a3-467a-a959-5ebdc91e542b) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-DTMF | media §2.2.3.1 | 15.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-dtmf/7a2c80c0-5fa7-4b8c-9216-c33804f0e0d4) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-H264PF | media §2.2.3.1 | 16.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-h264pf/f3024858-350e-49b0-9866-99c4fd2b912d) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-RTP | media §2.2.3.1 | 17.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtp/26dda764-6e96-456b-810b-9ae0275af564) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-RTPRADEX | media §2.2.3.1 | 13.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtpradex/c28f4661-9e7c-4319-b6e5-58d0dba6b823) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-RTASPF | media §2.2.3.1 | 10.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtaspf/42824cea-064e-45b3-a81b-24584449d2eb) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-RTVPF | media §2.2.3.1 | 11.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtvpf/3f37858b-c991-41c4-bbbd-df10fcffc89e) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-SDPEXT | media §2.2.3.1 | 16.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/cd17a549-b948-42a6-aa6b-fa707710faac) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-SRTP | media §2.2.3.1 | 14.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-srtp/d9641c95-b152-4cc7-8311-d178f3241f1f) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-SSRTP | media §2.2.3.1 | 11.1 | 2025-09-16 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ssrtp/980fc2b6-ba83-4313-9f5a-cad50449e98e) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-AVEDGEA | ice §2.2.3.2 | 12.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-avedgea/56860f6a-2fcf-4cad-8ee5-466d8f382a45) | VERIFIED_FROM_USER_SUPPLIED_RELEASE_PDF_SHA256 |
| MS-ICE | ice §2.2.3.2 | 11.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice/4feb9bf6-dbe8-4713-a228-e7e697c57eae) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-ICE2 | ice §2.2.3.2 | 19.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice2/8cd2fe91-1aa6-49e1-b482-f92b5c40b7e2) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-ICE2BWM | ice §2.2.3.2 | 9.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice2bwm/9da023f3-da10-430d-825b-1d9d7b2ac5ca) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |
| MS-TURN | ice §2.2.3.2 | 21.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-turn/9e434b27-eb13-4249-b031-2d15c3835c8b) | SECTION_VERIFIED_FROM_RELEASE_PDF |
| MS-TURNBWM | ice §2.2.3.2 | 9.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-turnbwm/7fd11bbc-3e10-476c-80be-9d62b1b35b1a) | FULL_USER_ZIP_ORIGINAL_PDF_SHA256_SECTION_VERIFIED_NOT_PUBLIC_ARCHIVED |

**Known May 2025 overview versus newer releases:** MS-ABS rev 10.1 (2025-08-19), MS-SIPREGE rev 17.1 (2025-08-19), MS-SSRTP rev 11.1 (2025-09-16). Change details and historical revision comparison NOT yet read for all editions. Source landing metadata does not prove release PDF normative text has been parsed.

## Source-pinned first-layer normative sections

**Current published original PDF reference pages (12 index views; uploaded full MS-AVEDGEA v12 separately):** [MS-CONMGMT 10.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-CONMGMT/%5BMS-CONMGMT%5D.pdf), [MS-CONFBAS 15.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-CONFBAS/%5BMS-CONFBAS%5D.pdf), [MS-OCAUTHWS 11.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-OCAUTHWS/%5BMS-OCAUTHWS%5D.pdf), [MS-PRES 13.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-PRES/%5BMS-PRES%5D.pdf), [MS-ECREST 8.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-ECREST/%5BMS-ECREST%5D.pdf), [MS-PSOM 11.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-PSOM/%5BMS-PSOM%5D.pdf), [MS-E911WS 9.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-E911WS/%5BMS-E911WS%5D.pdf), [MS-RTPRADEX 13.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-RTPRADEX/%5BMS-RTPRADEX%5D.pdf), [MS-RTVPF 11.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-RTVPF/%5BMS-RTVPF%5D.pdf), [MS-SSRTP 11.1](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-SSRTP/%5BMS-SSRTP%5D.pdf), [MS-SIPRE 13.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-SIPRE/%5BMS-SIPRE%5D.pdf), [MS-RTASPF 10.0](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-RTASPF/%5BMS-RTASPF%5D.pdf). Page numbers and visual inspection are recorded individually in the JSON reference graph.

**HTML-only reference lists (current edition match unverified):** [MS-SIPREGE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/24cbca88-e618-4d70-979d-3c52d6ec5c04), [MS-SDPEXT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/be2f02ae-6687-444c-b21b-64553bb00ed5), [MS-RTP](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtp/9258c646-3fa5-4702-a9cf-86b30029ca7a), [MS-SIPAE](https://learn.microsoft.com/th-th/openspecs/office_protocols/ms-sipae/e77ebace-a758-4e64-b3db-327db63f9dea), [MS-OCSMP](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsmp/985ab5c8-ec6f-4303-b27e-bedc3834c669), [MS-CVWREST](https://learn.microsoft.com/th-th/openspecs/office_protocols/ms-cvwrest/b3e19a54-7499-4d03-bbb7-51e1a4e5902b), [MS-OCDISCWS](https://learn.microsoft.com/hr-hr/openspecs/office_protocols/ms-ocdiscws/967752c2-051e-48f9-834e-d7017bca2b60), [MS-ABS](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abs/2e6f2cd1-b0a5-4aea-8b53-048e7b217c1d), [MS-DLX](https://learn.microsoft.com/nb-no/openspecs/office_protocols/ms-dlx/1772eb00-a266-4158-bfb3-1964e579e6ba), [MS-ICE2](https://learn.microsoft.com/sk-sk/openspecs/office_protocols/ms-ice2/8f113283-5259-43f8-a4e7-e48826b30659), [MS-CONFPRO](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confpro/0a7b1315-9ba1-44dd-b667-71cd53001c24), [MS-ICE2BWM](https://learn.microsoft.com/nb-no/openspecs/office_protocols/ms-ice2bwm/3b5b53f6-364c-4fc7-9461-8398e5226f30), [MS-EUMR](https://learn.microsoft.com/fr-be/openspecs/office_protocols/ms-eumr/13a6c308-b6c4-45ec-8066-b5b0cd1aba6b), [MS-CONFAS](https://learn.microsoft.com/uk-ua/openspecs/office_protocols/ms-confas/18121bb6-f1d2-4bbb-8636-809908680934), [MS-ICE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice/bc736014-7a9b-4920-b251-5f8367216e20), [MS-TURN](https://learn.microsoft.com/nb-no/openspecs/office_protocols/ms-turn/be08fae9-014d-44e9-9386-338d4a74d36d), [MS-SIPAPP](https://learn.microsoft.com/sr-cyrl-rs/openspecs/office_protocols/ms-sipapp/74dabe8d-3f6c-4383-988c-32b65aea3f8b), [MS-CONFAV](https://learn.microsoft.com/fi-fi/openspecs/office_protocols/ms-confav/b8836030-4028-428e-9ee2-28a5a2cf8af9), [MS-TURNBWM](https://learn.microsoft.com/hr-hr/openspecs/office_protocols/ms-turnbwm/fd36f6a0-50e0-4b24-909a-17309095f471), [MS-QoE](https://learn.microsoft.com/cs-cz/openspecs/office_protocols/ms-qoe/42bde001-68b7-48e7-afdd-97c3556f610a), [MS-OCER](https://learn.microsoft.com/de-at/openspecs/office_protocols/ms-ocer/199e4333-7be3-43bd-80fa-6463d5f5b7e4), [MS-DTMF](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-dtmf/d66cd508-c8bb-44ed-8a38-79d75385934c), [MS-SRTP](https://learn.microsoft.com/de-de/openspecs/office_protocols/ms-srtp/4a2c4129-f1a8-45c8-90ab-73562af9741e), [MS-OCEXUM](https://learn.microsoft.com/lb-lu/openspecs/office_protocols/ms-ocexum/98c07460-792c-4466-8d19-a8b69012b0db), [MS-XCCOSIP](https://learn.microsoft.com/nb-no/openspecs/office_protocols/ms-xccosip/5b58e6cb-d7ef-4493-9da2-b824b6c18ac8), [MS-XMLMC](https://learn.microsoft.com/hr-hr/openspecs/office_protocols/ms-xmlmc/c8e9ff63-f060-4084-9d8b-51b2ea71b5a3), [MS-CONFIM](https://learn.microsoft.com/lv-lv/openspecs/office_protocols/ms-confim/5b25e886-289a-43e8-85c7-f87003f576b2), [MS-OCPSTN](https://learn.microsoft.com/fi-fi/openspecs/office_protocols/ms-ocpstn/db5f4eb9-ff09-4c1c-af80-5f1c7f4c72e5), [MS-SIPCOMP](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipcomp/7fbb715f-8c9d-44f7-83a9-6fb346ae2964), [MS-OCGCWEB](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocgcweb/a1766684-7cc3-4e42-9112-d0199f02cb51), [MS-EUMSDP](https://learn.microsoft.com/kk-kz/openspecs/office_protocols/ms-eumsdp/5c51fc24-f18d-4766-8b38-cd9ccfd01197), [MS-RGSWS](https://learn.microsoft.com/it-ch/openspecs/office_protocols/ms-rgsws/02a4c060-ce47-488e-bca4-9ff8929a274b), [MS-H264PF](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-h264pf/981f1404-3f38-449f-a9fd-09fd83f460c0). Some HTML pages predate the current published edition by years.

**MS-AVEDGEA rev 12.0 §1.2.1 now verified** from the uploaded, SHA-256-checked 40-page current release PDF: **5 normative** references, **5 informative** references (§1.2.2), exact keys and fixed versions in [MS-AVEDGEA source audit](ms-avedgea-12-source-audit.md). All **46/46** first-tier lists inspected, but only **13** released-edition PDF sections are corroborated (12 indexed pages + uploaded complete MS-AVEDGEA); the other 33 remain HTML-edition-unconfirmed.


## RFC original editions: distinguish confirmed direct Microsoft §1.2.1 references vs research candidates

| RFC | Original RFC Editor source | Direct MS normative references / seed |
| --- | --- | --- |
| RFC950 | [RFC Editor](https://www.rfc-editor.org/info/rfc950) | MS-SIPREGE |
| RFC1034 | [RFC Editor](https://www.rfc-editor.org/info/rfc1034) | MS-PRES |
| RFC1035 | [RFC Editor](https://www.rfc-editor.org/info/rfc1035) | MS-CONMGMT |
| RFC1321 | [RFC Editor](https://www.rfc-editor.org/info/rfc1321) | MS-TURN |
| RFC1341 | [RFC Editor](https://www.rfc-editor.org/info/rfc1341) | MS-PRES |
| RFC1521 | [RFC Editor](https://www.rfc-editor.org/info/rfc1521) | MS-SDPEXT |
| RFC2045 | [RFC Editor](https://www.rfc-editor.org/info/rfc2045) | MS-CONFIM, MS-OCPSTN |
| RFC2046 | [RFC Editor](https://www.rfc-editor.org/info/rfc2046) | MS-CONFIM, MS-SIPRE |
| RFC2104 | [RFC Editor](https://www.rfc-editor.org/info/rfc2104) | MS-SIPAE, MS-SRTP |
| RFC2111 | [RFC Editor](https://www.rfc-editor.org/info/rfc2111) | MS-OCPSTN |
| RFC2118 | [RFC Editor](https://www.rfc-editor.org/info/rfc2118) | MS-SIPCOMP |
| RFC2119 | [RFC Editor](https://www.rfc-editor.org/info/rfc2119) | MS-SIPREGE, MS-SDPEXT, MS-RTP, MS-PRES, MS-CONMGMT, MS-CONFBAS, MS-OCAUTHWS, MS-SIPAE, MS-OCSMP, MS-CVWREST, MS-OCDISCWS, MS-ECREST, MS-ABS, MS-DLX, MS-ICE2, MS-CONFPRO, MS-ICE2BWM, MS-EUMR, MS-CONFAS, MS-ICE, MS-TURN, MS-SIPAPP, MS-CONFAV, MS-TURNBWM, MS-QoE, MS-OCER, MS-DTMF, MS-SRTP, MS-OCEXUM, MS-XCCOSIP, MS-PSOM, MS-E911WS, MS-RTPRADEX, MS-RTVPF, MS-XMLMC, MS-CONFIM, MS-OCPSTN, MS-SIPCOMP, MS-OCGCWEB, MS-SSRTP, MS-EUMSDP, MS-SIPRE, MS-RTASPF, MS-RGSWS, MS-H264PF |
| RFC2131 | [RFC Editor](https://www.rfc-editor.org/info/rfc2131) | MS-CONMGMT |
| RFC2132 | [RFC Editor](https://www.rfc-editor.org/info/rfc2132) | MS-CONMGMT |
| RFC2141 | [RFC Editor](https://www.rfc-editor.org/info/rfc2141) | MS-SIPRE |
| RFC2198 | [RFC Editor](https://www.rfc-editor.org/info/rfc2198) | MS-RTPRADEX |
| RFC2246 | [RFC Editor](https://www.rfc-editor.org/info/rfc2246) | MS-SIPAE, MS-TURN |
| RFC2327 | [RFC Editor](https://www.rfc-editor.org/info/rfc2327) | MS-EUMR |
| RFC2387 | [RFC Editor](https://www.rfc-editor.org/info/rfc2387) | MS-OCSMP |
| RFC2459 | [RFC Editor](https://www.rfc-editor.org/info/rfc2459) | MS-CONMGMT |
| RFC2616 | [RFC Editor](https://www.rfc-editor.org/info/rfc2616) | MS-OCSMP, MS-CVWREST, MS-OCDISCWS, MS-ECREST, MS-DLX, MS-TURN, MS-PSOM, MS-E911WS |
| RFC2716 | [RFC Editor](https://www.rfc-editor.org/info/rfc2716) | MS-SIPAE |
| RFC2743 | [RFC Editor](https://www.rfc-editor.org/info/rfc2743) | MS-SIPAE |
| RFC2782 | [RFC Editor](https://www.rfc-editor.org/info/rfc2782) | MS-CONMGMT |
| RFC2818 | [RFC Editor](https://www.rfc-editor.org/info/rfc2818) | MS-OCAUTHWS, MS-SIPAE, MS-OCSMP, MS-CVWREST, MS-OCDISCWS, MS-ECREST, MS-DLX, MS-TURN |
| RFC2976 | [RFC Editor](https://www.rfc-editor.org/info/rfc2976) | MS-CONFBAS, MS-EUMR, MS-XMLMC, MS-CONFIM |
| RFC3261 | [RFC Editor](https://www.rfc-editor.org/info/rfc3261) | MS-SIPREGE, MS-SDPEXT, MS-PRES, MS-CONMGMT, MS-CONFBAS, MS-OCAUTHWS, MS-SIPAE, MS-OCDISCWS, MS-DLX, MS-CONFPRO, MS-EUMR, MS-SIPAPP, MS-CONFAV, MS-OCER, MS-OCEXUM, MS-XCCOSIP, MS-CONFIM, MS-OCPSTN, MS-SIPCOMP, MS-SIPRE |
| RFC3262 | [RFC Editor](https://www.rfc-editor.org/info/rfc3262) | MS-SDPEXT |
| RFC3263 | [RFC Editor](https://www.rfc-editor.org/info/rfc3263) | candidate only; no direct source proven yet |
| RFC3264 | [RFC Editor](https://www.rfc-editor.org/info/rfc3264) | MS-SDPEXT, MS-CONFAS, MS-CONFAV, MS-XMLMC, MS-SIPRE |
| RFC3265 | [RFC Editor](https://www.rfc-editor.org/info/rfc3265) | MS-SIPREGE, MS-PRES, MS-CONFBAS, MS-CONFIM, MS-SIPRE |
| RFC3280 | [RFC Editor](https://www.rfc-editor.org/info/rfc3280) | MS-CONFBAS, MS-OCAUTHWS |
| RFC3311 | [RFC Editor](https://www.rfc-editor.org/info/rfc3311) | MS-CONFBAS, MS-CONFIM |
| RFC3323 | [RFC Editor](https://www.rfc-editor.org/info/rfc3323) | MS-SIPAE |
| RFC3325 | [RFC Editor](https://www.rfc-editor.org/info/rfc3325) | MS-SIPAE, MS-SIPRE |
| RFC3326 | [RFC Editor](https://www.rfc-editor.org/info/rfc3326) | MS-SIPRE |
| RFC3327 | [RFC Editor](https://www.rfc-editor.org/info/rfc3327) | MS-SIPRE |
| RFC3361 | [RFC Editor](https://www.rfc-editor.org/info/rfc3361) | MS-CONMGMT |
| RFC3389 | [RFC Editor](https://www.rfc-editor.org/info/rfc3389) | MS-SDPEXT |
| RFC3428 | [RFC Editor](https://www.rfc-editor.org/info/rfc3428) | MS-CONFIM |
| RFC3489 | [RFC Editor](https://www.rfc-editor.org/info/rfc3489) | MS-TURN |
| RFC3515 | [RFC Editor](https://www.rfc-editor.org/info/rfc3515) | MS-SIPAPP |
| RFC3548 | [RFC Editor](https://www.rfc-editor.org/info/rfc3548) | MS-SIPAE, MS-SIPRE |
| RFC3550 | [RFC Editor](https://www.rfc-editor.org/info/rfc3550) | MS-RTP, MS-QoE, MS-SSRTP, MS-RTASPF, MS-H264PF |
| RFC3551 | [RFC Editor](https://www.rfc-editor.org/info/rfc3551) | MS-SDPEXT, MS-RTP, MS-QoE |
| RFC3581 | [RFC Editor](https://www.rfc-editor.org/info/rfc3581) | candidate only; no direct source proven yet |
| RFC3605 | [RFC Editor](https://www.rfc-editor.org/info/rfc3605) | MS-SDPEXT |
| RFC3611 | [RFC Editor](https://www.rfc-editor.org/info/rfc3611) | MS-QoE |
| RFC3629 | [RFC Editor](https://www.rfc-editor.org/info/rfc3629) | MS-CONFBAS |
| RFC3711 | [RFC Editor](https://www.rfc-editor.org/info/rfc3711) | MS-SRTP, MS-SSRTP |
| RFC3840 | [RFC Editor](https://www.rfc-editor.org/info/rfc3840) | MS-SIPAPP |
| RFC3852 | [RFC Editor](https://www.rfc-editor.org/info/rfc3852) | MS-CONFPRO |
| RFC3856 | [RFC Editor](https://www.rfc-editor.org/info/rfc3856) | candidate only; no direct source proven yet |
| RFC3863 | [RFC Editor](https://www.rfc-editor.org/info/rfc3863) | MS-PRES, MS-E911WS, MS-SIPRE |
| RFC3891 | [RFC Editor](https://www.rfc-editor.org/info/rfc3891) | MS-SIPAPP |
| RFC3892 | [RFC Editor](https://www.rfc-editor.org/info/rfc3892) | MS-SIPAE, MS-EUMR, MS-SIPRE |
| RFC3903 | [RFC Editor](https://www.rfc-editor.org/info/rfc3903) | candidate only; no direct source proven yet |
| RFC3966 | [RFC Editor](https://www.rfc-editor.org/info/rfc3966) | MS-CVWREST, MS-OCPSTN, MS-SIPRE |
| RFC3986 | [RFC Editor](https://www.rfc-editor.org/info/rfc3986) | MS-OCDISCWS, MS-OCGCWEB, MS-SIPRE |
| RFC4028 | [RFC Editor](https://www.rfc-editor.org/info/rfc4028) | MS-CONFBAS, MS-SIPAE, MS-CONFAS, MS-CONFIM, MS-SIPRE |
| RFC4119 | [RFC Editor](https://www.rfc-editor.org/info/rfc4119) | MS-E911WS, MS-SIPRE |
| RFC4120 | [RFC Editor](https://www.rfc-editor.org/info/rfc4120) | MS-SIPAE |
| RFC4121 | [RFC Editor](https://www.rfc-editor.org/info/rfc4121) | MS-SIPAE |
| RFC4122 | [RFC Editor](https://www.rfc-editor.org/info/rfc4122) | MS-SIPRE |
| RFC4145 | [RFC Editor](https://www.rfc-editor.org/info/rfc4145) | MS-SDPEXT |
| RFC4235 | [RFC Editor](https://www.rfc-editor.org/info/rfc4235) | MS-SDPEXT, MS-SIPRE |
| RFC4244 | [RFC Editor](https://www.rfc-editor.org/info/rfc4244) | MS-SIPRE |
| RFC4320 | [RFC Editor](https://www.rfc-editor.org/info/rfc4320) | candidate only; no direct source proven yet |
| RFC4346 | [RFC Editor](https://www.rfc-editor.org/info/rfc4346) | MS-SIPAE, MS-TURN, MS-SIPCOMP |
| RFC4353 | [RFC Editor](https://www.rfc-editor.org/info/rfc4353) | MS-CONFBAS |
| RFC4480 | [RFC Editor](https://www.rfc-editor.org/info/rfc4480) | MS-PRES |
| RFC4559 | [RFC Editor](https://www.rfc-editor.org/info/rfc4559) | MS-OCAUTHWS |
| RFC4566 | [RFC Editor](https://www.rfc-editor.org/info/rfc4566) | MS-SDPEXT, MS-SIPAPP, MS-CONFAV, MS-XMLMC, MS-CONFIM, MS-EUMSDP, MS-SIPRE |
| RFC4568 | [RFC Editor](https://www.rfc-editor.org/info/rfc4568) | MS-SDPEXT |
| RFC4571 | [RFC Editor](https://www.rfc-editor.org/info/rfc4571) | MS-SDPEXT, MS-ICE2, MS-ICE |
| RFC4574 | [RFC Editor](https://www.rfc-editor.org/info/rfc4574) | MS-SDPEXT, MS-CONFAS, MS-CONFAV |
| RFC4575 | [RFC Editor](https://www.rfc-editor.org/info/rfc4575) | MS-CONFBAS, MS-CONFAS, MS-CONFAV |
| RFC4585 | [RFC Editor](https://www.rfc-editor.org/info/rfc4585) | MS-SDPEXT, MS-RTP |
| RFC4627 | [RFC Editor](https://www.rfc-editor.org/info/rfc4627) | MS-OCDISCWS |
| RFC4648 | [RFC Editor](https://www.rfc-editor.org/info/rfc4648) | candidate only; no direct source proven yet |
| RFC4722 | [RFC Editor](https://www.rfc-editor.org/info/rfc4722) | candidate only; no direct source proven yet |
| RFC4733 | [RFC Editor](https://www.rfc-editor.org/info/rfc4733) | MS-DTMF |
| RFC4737 | [RFC Editor](https://www.rfc-editor.org/info/rfc4737) | MS-QoE |
| RFC5109 | [RFC Editor](https://www.rfc-editor.org/info/rfc5109) | MS-H264PF |
| RFC5139 | [RFC Editor](https://www.rfc-editor.org/info/rfc5139) | MS-E911WS, MS-SIPRE |
| RFC5234 | [RFC Editor](https://www.rfc-editor.org/info/rfc5234) | MS-CONMGMT, MS-SIPAE, MS-SIPCOMP |
| RFC5245 | [RFC Editor](https://www.rfc-editor.org/info/rfc5245) | candidate only; no direct source proven yet |
| RFC5246 | [RFC Editor](https://www.rfc-editor.org/info/rfc5246) | MS-CONMGMT, MS-SIPAE, MS-TURN |
| RFC5280 | [RFC Editor](https://www.rfc-editor.org/info/rfc5280) | candidate only; no direct source proven yet |
| RFC5285 | [RFC Editor](https://www.rfc-editor.org/info/rfc5285) | MS-RTP |
| RFC5389 | [RFC Editor](https://www.rfc-editor.org/info/rfc5389) | MS-ICE2 |
| RFC5506 | [RFC Editor](https://www.rfc-editor.org/info/rfc5506) | MS-RTP |
| RFC5626 | [RFC Editor](https://www.rfc-editor.org/info/rfc5626) | candidate only; no direct source proven yet |
| RFC5627 | [RFC Editor](https://www.rfc-editor.org/info/rfc5627) | candidate only; no direct source proven yet |
| RFC5761 | [RFC Editor](https://www.rfc-editor.org/info/rfc5761) | MS-SDPEXT, MS-ICE2 |
| RFC5766 | [RFC Editor](https://www.rfc-editor.org/info/rfc5766) | MS-ICE2 |
| RFC6026 | [RFC Editor](https://www.rfc-editor.org/info/rfc6026) | candidate only; no direct source proven yet |
| RFC6125 | [RFC Editor](https://www.rfc-editor.org/info/rfc6125) | candidate only; no direct source proven yet |
| RFC6156 | [RFC Editor](https://www.rfc-editor.org/info/rfc6156) | MS-TURN |
| RFC6184 | [RFC Editor](https://www.rfc-editor.org/info/rfc6184) | MS-H264PF |
| RFC6190 | [RFC Editor](https://www.rfc-editor.org/info/rfc6190) | MS-H264PF |
| RFC6335 | [RFC Editor](https://www.rfc-editor.org/info/rfc6335) | candidate only; no direct source proven yet |
| RFC6442 | [RFC Editor](https://www.rfc-editor.org/info/rfc6442) | MS-SIPRE |
| RFC6665 | [RFC Editor](https://www.rfc-editor.org/info/rfc6665) | candidate only; no direct source proven yet |
| RFC8174 | [RFC Editor](https://www.rfc-editor.org/info/rfc8174) | candidate only; no direct source proven yet |
| RFC8445 | [RFC Editor](https://www.rfc-editor.org/info/rfc8445) | candidate only; no direct source proven yet |
| RFC8446 | [RFC Editor](https://www.rfc-editor.org/info/rfc8446) | candidate only; no direct source proven yet |
| RFC8489 | [RFC Editor](https://www.rfc-editor.org/info/rfc8489) | candidate only; no direct source proven yet |
| RFC8553 | [RFC Editor](https://www.rfc-editor.org/info/rfc8553) | candidate only; no direct source proven yet |
| RFC8656 | [RFC Editor](https://www.rfc-editor.org/info/rfc8656) | candidate only; no direct source proven yet |
| RFC8839 | [RFC Editor](https://www.rfc-editor.org/info/rfc8839) | candidate only; no direct source proven yet |
| RFC8866 | [RFC Editor](https://www.rfc-editor.org/info/rfc8866) | candidate only; no direct source proven yet |
| RFC9525 | [RFC Editor](https://www.rfc-editor.org/info/rfc9525) | candidate only; no direct source proven yet |

RFC update, errata, obsolescence and normative-dependency closure is not complete. These editions must not be silently upgraded: RFC 3265→RFC 6665; RFC 5245→RFC 8445/8839; RFC 4566→RFC 8866; RFC 3548→RFC 4648. The user and Microsoft versions may intentionally differ.

## Protocols and international standards outside overview lists

Exact MS references **MS-H26XPF, MS-KILE, MS-NLMP, MS-OAUTH2EX, MS-OXWUMS, MS-RDPBCGR, MS-RDPEMC, MS-SDP, MS-SIP, MS-WCCE, MS-WSPOL, MS-WSTEP** appear in [source-linked normative graph](normative-reference-ledger.json); other draft/ISO, ITU, W3C and OASIS nodes are likewise listed there, with edition exactness and paywall information in [phase1-audit.md](phase1-audit.md). Treat optional / server-only exclusions only after verifying the Windows Full Client build and service policy. **All twelve direct out-of-overview Microsoft target IDs now have official latest published revision/date landing metadata, captured in [transitive-microsoft-inventory.json](transitive-microsoft-inventory.json). Exact client-applicable historical versions and all their normative reference lists remain OPEN. No document not acquired must be marked fully analyzed.**

**No new protocol code until source acquisition and all subsequent analysis gates are complete.**

## 2026-10-23 follow-on: new source graph and RFC edition metadata

- [Second-tier official Microsoft normative reference analysis](phase1-transitive-research-2026-10-23.md) and [direct source-bound graph](transitive-reference-ledger.json): 12 examined original HTML/search reference sections, **246** new *second-ring* edges to **183** targets (11 full HTML, MS-RDPEMC partial). **No dated PDF-revision comparison yet**.
- [New unresolved target frontier](transitive-frontier.json): **146** IDs not present as targets in the existing **460 first-tier** normative graph; 52 MS/MC, 51 RFC, 43 external/drafts. First two rings have **345 unique target identifiers combined**, but recursive closure is still **OPEN**.
- [Official RFC Editor relation checks](rfc-version-relations-reviewed.json): 10 fixed editions with Update/Obsoletes metadata verified; global RFC normative references/errata not closed.

Source IDs/versions, official HTML subsection URLs and older released-PDF edition warnings are stored per row in the machine-readable ledgers; do not turn these metadata findings into universal client implementation requirements.

## 2026-10-24 P1-01 current-edition PDF section reconciliation

[Three-source original section audit](published-three-ms-pdf-page-audit-2026-10-24.md): MS-EUMSDP rev 9.0 (§1.2.1 p.6), MS-TURN rev 21.0 (§1.2.1 pp.9–10), MS-RTP rev 17.0 (§1.2.1 pp.9–10), each published 2025-05-20; official PDF text and page screenshots verified, full-file SHA-256 **not** available. Exactly 24 pre-existing normative edges matched. Historic counts elsewhere in this file reflect earlier snapshots. Current overview 19/46 PDF sections and 27 HTML-only, phase 1 OPEN.

## 2026-10-24 – Additional current original-PDF bibliography audit

[Four-source P1-01 edition audit](published-four-ms-pdf-section-audit-2026-10-24.md): MS-SRTP rev 14.0 (§1.2.1 p.7), MS-SDPEXT rev 16.0 (§1.2.1 pp.12–13), MS-H264PF rev 16.0 (§1.2.1 pp.7–8), MS-SIPCOMP rev 10.0 (§1.2.1 pp.6–7), each published 2025-05-20. 49 pre-existing normative edges verified on original web PDF page screenshots, **no full-file binary hashes** or editions-diff closures. Original earlier figures in the detailed historic narrative refer to previous snapshots; latest 23/46 with current-PDF sections and 23 HTML-only. Phase 1 OPEN.

## 2026-10-24 Original ZIP source evidence

[OfficeProto original ZIP source and SHA audit](officeproto-zip-source-audit-2026-10-24.md) replaces the previously reported **23 HTML-only** first-ring source lists with original full PDF section/hash evidence. 46/46 first-ring PDF pages now have edition evidence; 43 first-ring current revisions were present in this ZIP, three are older than previously validated current-edition page evidence. **MS-E911WS** gained four omitted W3C normative refs (new count 464, target union 199 unchanged). Source rights and private archival, individual normalized target-set semantic auditing, full version diffs and transitive graph remain open.

## Windows_Protocols.tar.xz — 2026-10-24 published original research extension

[Windows source evidence](windows-originals-2026-10-24.md) / [machine summary](windows-originals-2026-10-24.json): 451 PDF originals, 424 MS plus 27 MC, archival checksum `52f87efc17c9854f70b0b14650c877a572b1a99c7ed9df4f6c9a96387720c652`. The previous **6/12** second-tier PDF section count is now **12/12**, 145 existing normative referrer→target links reconciled, no new edges (246), 52 newly discovered MS/MC third-ring source *files* available. Counts elsewhere in earlier historical paragraphs refer only to their recorded snapshots. Public PDF redistribution/archiving, normative recursive closure, version-history changes and client applicability remain OPEN; 0/12 formal gates.
