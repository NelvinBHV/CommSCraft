# Official RFC / Microsoft Protocol Source Catalog — Phase 1 snapshot

**As of 2026-10-23. Status: PHASE 1 NOT COMPLETE.** This index supersedes the older initial eight-source-only seed inventory. A Microsoft Learn **Published Version** row is a verified *revision/date metadata claim*, **not** proof that the corresponding immutable PDF was downloaded or every normative link read. Source versions, content access, reference coverage, and product relevance are separate fields.

## Central source-of-truth files

| Dataset | Contents | Current verifiable completeness |
| --- | --- | --- |
| [MS-OCSPROT primary inventory](ms-ocsprot-inventory.json) and [human table](ms-ocsprot-inventory.md) | **46** named documents across **six** overview tables; each with latest revision/date, direct official landing and overview § | Six named overview categories inventoried; **0** current-release PDFs retrieved/hashed; detailed versions/change histories open |
| [Normative-reference ledger](normative-reference-ledger.json) | **71** precisely source-linked §1.2.1 edges extracted from **four** Microsoft documents; target type/draft editions; Microsoft Windows cross-library documents | **4/46** first-tier lists checked as *live HTML*, **42/46** unchecked; full transitive closure open |
| [RFC inventory](rfc-inventory.json) | **71** candidate/explicitly normative RFCs, persistent RFC Editor entry point per RFC; selected obsolete/update relations | Candidate seed **not** exhaustive, only **25** exact RFCs named by the four audited §1.2.1 pages; normative/update chains incomplete |
| [Phase 1 audit and unreviewed documents](phase1-audit.md) | Detailed coverage counts, access failures, release-version drift, paywalls and exact exit criteria | **OPEN**; no completed claim |
| [Research log](research-log.md) | Branch checkpoint, completed primary sections, unknowns and next steps | Documentation only; no implementation authorization |

**Overview:** [MS-OCSPROT] revision **9.0**, published **2025-05-20**, official [Microsoft published-version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c). It explicitly states that it describes the system as an overview; processing rules are in individual specifications.

## Exact version and primary entry point for all 46 MS-OCSPROT listed [MS-*] documents

| Identifier | Group | Published rev | Published date | Microsoft official landing | Full published contents read? |
| --- | --- | --- | --- | --- | --- |
| MS-ABS | directory (§2.2.1) | 10.1 | 2025-08-19 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abs/ddc59abf-1f42-4935-9b72-5314665dbcff) | **NO** |
| MS-DLX | directory (§2.2.1) | 13.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-dlx/fa0e41cd-2b34-4081-b3cf-59ecdad7a5da) | **NO** |
| MS-CONMGMT | sip (§2.2.2.1) | 10.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/9f6ebbf0-9000-4dc8-86cd-ebfe22467cbf) | **NO** |
| MS-EUMR | sip (§2.2.2.1) | 9.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-eumr/3ef7d3c4-2b15-4d79-a8e4-8eca927540ac) | **NO** |
| MS-OCEXUM | sip (§2.2.2.1) | 11.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocexum/d2018f70-8c96-481b-859e-d3b741618fa2) | **NO** |
| MS-OCPSTN | sip (§2.2.2.1) | 11.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocpstn/6e69d8d4-ba59-484f-9f2f-61d8ee4ed0d9) | **NO** |
| MS-OCER | sip (§2.2.2.1) | 11.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocer/8b2b5ee5-6051-4f66-b51e-71102ef7782f) | **NO** |
| MS-PRES | sip (§2.2.2.1) | 13.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/741d682d-590b-4600-916e-8deeee426a5c) | **NO** |
| MS-QoE | sip (§2.2.2.1) | 12.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-qoe/c1050431-f4b2-44d4-a425-63d75e99f5df) | **NO** |
| MS-SIPCOMP | sip (§2.2.2.1) | 10.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipcomp/eaf7bc25-7128-454a-841e-539dfca2207d) | **NO** |
| MS-SIPAE | sip (§2.2.2.1) | 11.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd) | **NO** |
| MS-SIPRE | sip (§2.2.2.1) | 13.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipre/ab4ab249-3796-4ed1-8cec-f496d81a1a83) | **NO** |
| MS-SIPREGE | sip (§2.2.2.1) | 17.1 | 2025-08-19 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d) | **NO** |
| MS-RGSWS | sip (§2.2.2.1) | 9.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rgsws/762316e0-3c6c-42c2-b3e8-41a9901e1b7c) | **NO** |
| MS-PSOM | sip (§2.2.2.1) | 11.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-psom/0b9a26ba-b92e-4a71-a1ea-205c5a0c250c) | **NO** |
| MS-SIPAPP | sip (§2.2.2.1) | 8.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipapp/2cc5f99e-7190-4672-bb24-6dc9c6347019) | **NO** |
| MS-OCAUTHWS | sip (§2.2.2.1) | 11.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocauthws/63749828-ab8d-4b24-941c-b11985d854c5) | **NO** |
| MS-E911WS | sip (§2.2.2.1) | 9.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-e911ws/ab5d7449-2c15-434b-bf65-fdf38b8ffabd) | **NO** |
| MS-XCCOSIP | sip (§2.2.2.1) | 7.0 | 2024-08-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-xccosip/e1afc2e0-8b2d-4fa4-a162-788349202345) | **NO** |
| MS-OCGCWEB | sip (§2.2.2.1) | 3.0 | 2018-04-27 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocgcweb/7bab1c71-de7b-424c-afaa-7678c3cc54ed) | **NO** |
| MS-CONFBAS | conference (§2.2.2.2) | 15.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confbas/6cb739fe-3a84-4266-8d52-0af777f6f1fa) | **NO** |
| MS-CONFPRO | conference (§2.2.2.2) | 13.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confpro/ec4a3626-66f9-4270-8adc-5f81f7fdad8a) | **NO** |
| MS-CONFIM | conference (§2.2.2.2) | 11.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confim/6f14d1ec-c854-4360-bdda-a6c61a655a60) | **NO** |
| MS-CONFAV | conference (§2.2.2.2) | 11.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confav/8e51c8b8-9869-4bff-b67e-56dbffefaa77) | **NO** |
| MS-CONFAS | conference (§2.2.2.2) | 12.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confas/bd7cc980-49d4-4ccb-a9c6-f3fda247ec8f) | **NO** |
| MS-XMLMC | conference (§2.2.2.2) | 10.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-xmlmc/5b0ef2c2-f3f0-4223-8c40-aeb02d5d74d7) | **NO** |
| MS-OCSMP | http (§2.2.2.3) | 7.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsmp/53c3507c-c987-47a9-9bc8-bb2baa18e1b9) | **NO** |
| MS-OCDISCWS | http (§2.2.2.3) | 10.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocdiscws/95554ed7-530a-4a27-beee-b353ea9f7e95) | **NO** |
| MS-CVWREST | http (§2.2.2.3) | 7.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-cvwrest/62436ec9-e9c9-4e72-9a3f-3081a8f92f67) | **NO** |
| MS-ECREST | http (§2.2.2.3) | 8.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ecrest/4454be95-159a-4c99-a75a-5c260609343a) | **NO** |
| MS-EUMSDP | media (§2.2.3.1) | 9.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-eumsdp/c01cded2-02a3-467a-a959-5ebdc91e542b) | **NO** |
| MS-DTMF | media (§2.2.3.1) | 15.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-dtmf/7a2c80c0-5fa7-4b8c-9216-c33804f0e0d4) | **NO** |
| MS-H264PF | media (§2.2.3.1) | 16.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-h264pf/f3024858-350e-49b0-9866-99c4fd2b912d) | **NO** |
| MS-RTP | media (§2.2.3.1) | 17.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtp/26dda764-6e96-456b-810b-9ae0275af564) | **NO** |
| MS-RTPRADEX | media (§2.2.3.1) | 13.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtpradex/c28f4661-9e7c-4319-b6e5-58d0dba6b823) | **NO** |
| MS-RTASPF | media (§2.2.3.1) | 10.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtaspf/42824cea-064e-45b3-a81b-24584449d2eb) | **NO** |
| MS-RTVPF | media (§2.2.3.1) | 11.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtvpf/3f37858b-c991-41c4-bbbd-df10fcffc89e) | **NO** |
| MS-SDPEXT | media (§2.2.3.1) | 16.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/cd17a549-b948-42a6-aa6b-fa707710faac) | **NO** |
| MS-SRTP | media (§2.2.3.1) | 14.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-srtp/d9641c95-b152-4cc7-8311-d178f3241f1f) | **NO** |
| MS-SSRTP | media (§2.2.3.1) | 11.1 | 2025-09-16 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ssrtp/980fc2b6-ba83-4313-9f5a-cad50449e98e) | **NO** |
| MS-AVEDGEA | ice (§2.2.3.2) | 12.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-avedgea/56860f6a-2fcf-4cad-8ee5-466d8f382a45) | **NO** |
| MS-ICE | ice (§2.2.3.2) | 11.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice/4feb9bf6-dbe8-4713-a228-e7e697c57eae) | **NO** |
| MS-ICE2 | ice (§2.2.3.2) | 19.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice2/8cd2fe91-1aa6-49e1-b482-f92b5c40b7e2) | **NO** |
| MS-ICE2BWM | ice (§2.2.3.2) | 9.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ice2bwm/9da023f3-da10-430d-825b-1d9d7b2ac5ca) | **NO** |
| MS-TURN | ice (§2.2.3.2) | 21.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-turn/9e434b27-eb13-4249-b031-2d15c3835c8b) | **NO** |
| MS-TURNBWM | ice (§2.2.3.2) | 9.0 | 2025-05-20 | [Official](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-turnbwm/7fd11bbc-3e10-476c-80be-9d62b1b35b1a) | **NO** |

**Version updates later than overview 9.0 (2025-05-20)**: MS-ABS **10.1 (2025-08-19)**, MS-SIPREGE **17.1 (2025-08-19)** and MS-SSRTP **11.1 (2025-09-16)**. The respective 10.0→10.1, 17.0→17.1 and 11.0→11.1 **substantive release changes remain unread**, though their current and previous revision dates were checked in the published version tables. Do not assume a unified 2025-05-20 edition across all documents.

### Outside the 46 overview-list IDs, explicitly found in normative-reference chains

| Identifier / standard | Source family | Known edition/revision | Official/reference page | Content/edition status |
| --- | --- | --- | --- | --- |
| MS-SIP | Microsoft Windows Protocols | 8.0, **2017-06-01** | [Official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/MS-SIP/0d72fb55-12ba-49f1-850d-f0bf3110bb1f); cited by MS-SIPREGE §1.2.1 and MS-PRES §1.2.1 | Published metadata checked; full PDF and normative closure NOT checked |
| MS-H26XPF | Microsoft Windows Protocols | 1.3, **2017-06-01** | [Official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/MS-H26XPF/ebea5d4a-a184-4912-83a6-df29606aaa78); cited by MS-RTP §1.2.1 | Same |
| MS-KILE / MS-NLMP | Windows authentication | Version and publication UNVERIFIED | To resolve via Windows Open Specifications | Full specifications NOT examined |
| MS-RDPBCGR | Windows screen/application-sharing dependency | UNVERIFIED | To resolve via Windows Open Specifications | NOT examined |
| MS-ABCH | Address-book candidate, referenced by existing CommSCraft audit | UNVERIFIED | Office Open Specifications search/disambiguation pending | NOT examined |
| MSFT-H264UCConfig | Microsoft H.264 AVC/SVC encoder modes | V1.1 **2011**, URL directly cited in MS-RTP §1.2.1 | [Normative MS-RTP §1.2.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtp/9258c646-3fa5-4702-a9cf-86b30029ca7a) | Publisher PDF NOT acquired |
| ISO-3166-1 / ISO-8601 | International standards | **ISO 3166-1:2013; ISO 8601:2004** as cited by MS-PRES | [MS-PRES §1.2.1](https://learn.microsoft.com/fr-ca/openspecs/office_protocols/ms-pres/a25dbe23-7871-431a-a8a9-1ea95138a7ee) | Original cited editions require **paid access**, not acquired |
| SOAP1.1-Envelope | W3C XML protocol | SOAP 1.1, May 2001 reference on MS-PRES page | [MS-PRES §1.2.1](https://learn.microsoft.com/fr-ca/openspecs/office_protocols/ms-pres/a25dbe23-7871-431a-a8a9-1ea95138a7ee) | Original content not evaluated |
| IETF draft-ietf-sip-gruu-10 | Legacy GRUU semantics | **July 2006**, not RFC 5627 | [MS-SIPREGE §1.2.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/24cbca88-e618-4d70-979d-3c52d6ec5c04) | Original text not downloaded |
| draft-ietf-mmusic-ice-06 / -19; draft-ietf-mmusic-ice-tcp-07; draft-ietf-sipping-sip-offeranswer-08; draft-ietf-sip-199-01 | Legacy SDP/ICE SIP | Dated 2005–2008 per MS source | [MS-SDPEXT §1.2.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/be2f02ae-6687-444c-b21b-64553bb00ed5) | Original IETF drafts, exact revision comparison pending |

**Do not exclude cross-family protocols solely because they are absent from the six overview tables.** A decision of "not applicable" needs a documented Windows Full Client/product policy reason and cannot be inferred from the spec title.

## All currently enumerated RFC original-edition identifiers (seed, not closure)

All links go to the [RFC Editor](https://www.rfc-editor.org). An RFC number denotes a **fixed published edition**; published date, Updates/Obsoletes/errata and normative-reference closure must be separately checked for each row. **DIRECT** = named in at least one of the four specifically inspected MS normative §1.2.1 pages; **SEED** = identified as a plausible requirement from existing CommSCraft research and official overview/application context but not yet verified as a direct normative referral.

| Identifier | Primary RFC Editor entry | Evidence class |
| --- | --- | --- |
| RFC950 | [RFC Editor](https://www.rfc-editor.org/info/rfc950) | **DIRECT:** MS-SIPREGE |
| RFC1034 | [RFC Editor](https://www.rfc-editor.org/info/rfc1034) | **DIRECT:** MS-PRES |
| RFC1035 | [RFC Editor](https://www.rfc-editor.org/info/rfc1035) | SEED; normative source closure OPEN |
| RFC1341 | [RFC Editor](https://www.rfc-editor.org/info/rfc1341) | **DIRECT:** MS-PRES |
| RFC1521 | [RFC Editor](https://www.rfc-editor.org/info/rfc1521) | **DIRECT:** MS-SDPEXT |
| RFC2104 | [RFC Editor](https://www.rfc-editor.org/info/rfc2104) | SEED; normative source closure OPEN |
| RFC2119 | [RFC Editor](https://www.rfc-editor.org/info/rfc2119) | **DIRECT:** MS-SIPREGE, MS-SDPEXT, MS-RTP, MS-PRES |
| RFC2131 | [RFC Editor](https://www.rfc-editor.org/info/rfc2131) | SEED; normative source closure OPEN |
| RFC2132 | [RFC Editor](https://www.rfc-editor.org/info/rfc2132) | SEED; normative source closure OPEN |
| RFC2246 | [RFC Editor](https://www.rfc-editor.org/info/rfc2246) | SEED; normative source closure OPEN |
| RFC2716 | [RFC Editor](https://www.rfc-editor.org/info/rfc2716) | SEED; normative source closure OPEN |
| RFC2743 | [RFC Editor](https://www.rfc-editor.org/info/rfc2743) | SEED; normative source closure OPEN |
| RFC2782 | [RFC Editor](https://www.rfc-editor.org/info/rfc2782) | SEED; normative source closure OPEN |
| RFC3261 | [RFC Editor](https://www.rfc-editor.org/info/rfc3261) | **DIRECT:** MS-SIPREGE, MS-SDPEXT, MS-PRES |
| RFC3262 | [RFC Editor](https://www.rfc-editor.org/info/rfc3262) | **DIRECT:** MS-SDPEXT |
| RFC3263 | [RFC Editor](https://www.rfc-editor.org/info/rfc3263) | SEED; normative source closure OPEN |
| RFC3264 | [RFC Editor](https://www.rfc-editor.org/info/rfc3264) | **DIRECT:** MS-SDPEXT |
| RFC3265 | [RFC Editor](https://www.rfc-editor.org/info/rfc3265) | **DIRECT:** MS-SIPREGE, MS-PRES |
| RFC3311 | [RFC Editor](https://www.rfc-editor.org/info/rfc3311) | SEED; normative source closure OPEN |
| RFC3325 | [RFC Editor](https://www.rfc-editor.org/info/rfc3325) | SEED; normative source closure OPEN |
| RFC3361 | [RFC Editor](https://www.rfc-editor.org/info/rfc3361) | SEED; normative source closure OPEN |
| RFC3389 | [RFC Editor](https://www.rfc-editor.org/info/rfc3389) | **DIRECT:** MS-SDPEXT |
| RFC3428 | [RFC Editor](https://www.rfc-editor.org/info/rfc3428) | SEED; normative source closure OPEN |
| RFC3515 | [RFC Editor](https://www.rfc-editor.org/info/rfc3515) | SEED; normative source closure OPEN |
| RFC3548 | [RFC Editor](https://www.rfc-editor.org/info/rfc3548) | SEED; normative source closure OPEN |
| RFC3550 | [RFC Editor](https://www.rfc-editor.org/info/rfc3550) | **DIRECT:** MS-RTP |
| RFC3551 | [RFC Editor](https://www.rfc-editor.org/info/rfc3551) | **DIRECT:** MS-SDPEXT, MS-RTP |
| RFC3581 | [RFC Editor](https://www.rfc-editor.org/info/rfc3581) | SEED; normative source closure OPEN |
| RFC3605 | [RFC Editor](https://www.rfc-editor.org/info/rfc3605) | **DIRECT:** MS-SDPEXT |
| RFC3711 | [RFC Editor](https://www.rfc-editor.org/info/rfc3711) | SEED; normative source closure OPEN |
| RFC3856 | [RFC Editor](https://www.rfc-editor.org/info/rfc3856) | SEED; normative source closure OPEN |
| RFC3863 | [RFC Editor](https://www.rfc-editor.org/info/rfc3863) | **DIRECT:** MS-PRES |
| RFC3892 | [RFC Editor](https://www.rfc-editor.org/info/rfc3892) | SEED; normative source closure OPEN |
| RFC3903 | [RFC Editor](https://www.rfc-editor.org/info/rfc3903) | SEED; normative source closure OPEN |
| RFC4028 | [RFC Editor](https://www.rfc-editor.org/info/rfc4028) | SEED; normative source closure OPEN |
| RFC4120 | [RFC Editor](https://www.rfc-editor.org/info/rfc4120) | SEED; normative source closure OPEN |
| RFC4145 | [RFC Editor](https://www.rfc-editor.org/info/rfc4145) | **DIRECT:** MS-SDPEXT |
| RFC4235 | [RFC Editor](https://www.rfc-editor.org/info/rfc4235) | **DIRECT:** MS-SDPEXT |
| RFC4320 | [RFC Editor](https://www.rfc-editor.org/info/rfc4320) | SEED; normative source closure OPEN |
| RFC4480 | [RFC Editor](https://www.rfc-editor.org/info/rfc4480) | **DIRECT:** MS-PRES |
| RFC4566 | [RFC Editor](https://www.rfc-editor.org/info/rfc4566) | **DIRECT:** MS-SDPEXT |
| RFC4568 | [RFC Editor](https://www.rfc-editor.org/info/rfc4568) | **DIRECT:** MS-SDPEXT |
| RFC4571 | [RFC Editor](https://www.rfc-editor.org/info/rfc4571) | **DIRECT:** MS-SDPEXT |
| RFC4574 | [RFC Editor](https://www.rfc-editor.org/info/rfc4574) | **DIRECT:** MS-SDPEXT |
| RFC4585 | [RFC Editor](https://www.rfc-editor.org/info/rfc4585) | **DIRECT:** MS-SDPEXT, MS-RTP |
| RFC4648 | [RFC Editor](https://www.rfc-editor.org/info/rfc4648) | SEED; normative source closure OPEN |
| RFC4722 | [RFC Editor](https://www.rfc-editor.org/info/rfc4722) | SEED; normative source closure OPEN |
| RFC4733 | [RFC Editor](https://www.rfc-editor.org/info/rfc4733) | SEED; normative source closure OPEN |
| RFC5245 | [RFC Editor](https://www.rfc-editor.org/info/rfc5245) | SEED; normative source closure OPEN |
| RFC5246 | [RFC Editor](https://www.rfc-editor.org/info/rfc5246) | SEED; normative source closure OPEN |
| RFC5280 | [RFC Editor](https://www.rfc-editor.org/info/rfc5280) | SEED; normative source closure OPEN |
| RFC5285 | [RFC Editor](https://www.rfc-editor.org/info/rfc5285) | **DIRECT:** MS-RTP |
| RFC5389 | [RFC Editor](https://www.rfc-editor.org/info/rfc5389) | SEED; normative source closure OPEN |
| RFC5506 | [RFC Editor](https://www.rfc-editor.org/info/rfc5506) | **DIRECT:** MS-RTP |
| RFC5626 | [RFC Editor](https://www.rfc-editor.org/info/rfc5626) | SEED; normative source closure OPEN |
| RFC5627 | [RFC Editor](https://www.rfc-editor.org/info/rfc5627) | SEED; normative source closure OPEN |
| RFC5761 | [RFC Editor](https://www.rfc-editor.org/info/rfc5761) | **DIRECT:** MS-SDPEXT |
| RFC5766 | [RFC Editor](https://www.rfc-editor.org/info/rfc5766) | SEED; normative source closure OPEN |
| RFC6026 | [RFC Editor](https://www.rfc-editor.org/info/rfc6026) | SEED; normative source closure OPEN |
| RFC6125 | [RFC Editor](https://www.rfc-editor.org/info/rfc6125) | SEED; normative source closure OPEN |
| RFC6335 | [RFC Editor](https://www.rfc-editor.org/info/rfc6335) | SEED; normative source closure OPEN |
| RFC6665 | [RFC Editor](https://www.rfc-editor.org/info/rfc6665) | SEED; normative source closure OPEN |
| RFC8174 | [RFC Editor](https://www.rfc-editor.org/info/rfc8174) | SEED; normative source closure OPEN |
| RFC8445 | [RFC Editor](https://www.rfc-editor.org/info/rfc8445) | SEED; normative source closure OPEN |
| RFC8446 | [RFC Editor](https://www.rfc-editor.org/info/rfc8446) | SEED; normative source closure OPEN |
| RFC8489 | [RFC Editor](https://www.rfc-editor.org/info/rfc8489) | SEED; normative source closure OPEN |
| RFC8553 | [RFC Editor](https://www.rfc-editor.org/info/rfc8553) | SEED; normative source closure OPEN |
| RFC8656 | [RFC Editor](https://www.rfc-editor.org/info/rfc8656) | SEED; normative source closure OPEN |
| RFC8839 | [RFC Editor](https://www.rfc-editor.org/info/rfc8839) | SEED; normative source closure OPEN |
| RFC8866 | [RFC Editor](https://www.rfc-editor.org/info/rfc8866) | SEED; normative source closure OPEN |
| RFC9525 | [RFC Editor](https://www.rfc-editor.org/info/rfc9525) | SEED; normative source closure OPEN |

### Selected edition/update links verified from RFC Editor original pages

- [RFC 3265](https://www.rfc-editor.org/info/rfc3265) **obsoleted by RFC 6665**; RFC 6665 also updates RFC 3261 and RFC 4660. Microsoft MS-SIPREGE/MS-PRES normative references specifically name RFC 3265, **not** RFC 6665. Both belong in the compatibility profile.
- [RFC 5245](https://www.rfc-editor.org/info/rfc5245) is obsolete, with references to **RFC 8445** and **RFC 8839** as replacing texts; [RFC 8445](https://www.rfc-editor.org/info/rfc8445) says it obsoletes RFC 5245.
- [RFC 8866](https://www.rfc-editor.org/info/rfc8866) obsoletes **RFC 4566**, but **MS-SDPEXT §1.2.1** still directly cites RFC 4566.
- Update/obsolescence/errata chains for remaining RFCs have **not** been exhaustively traversed. Do not infer that modern replacement editions are wire-compatible with Microsoft extension profiles.

## Primary normative reference sections actually inspected, separate from full-document read

- **MS-SIPREGE 17.1**: [§1.2.1 live source](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/24cbca88-e618-4d70-979d-3c52d6ec5c04), last HTML update 2025-08-19; **full release revision mapping UNVERIFIED**
- **MS-SDPEXT 16.0**: [§1.2.1 live source](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/be2f02ae-6687-444c-b21b-64553bb00ed5), last HTML update 2024-04-16; **full release revision mapping UNVERIFIED**
- **MS-RTP 17.0**: [§1.2.1 live source](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtp/9258c646-3fa5-4702-a9cf-86b30029ca7a), last HTML update 2024-04-16; **full release revision mapping UNVERIFIED**
- **MS-PRES 13.0**: [§1.2.1 live source](https://learn.microsoft.com/fr-ca/openspecs/office_protocols/ms-pres/a25dbe23-7871-431a-a8a9-1ea95138a7ee), last HTML update 2025-05-20; **full release revision mapping UNVERIFIED**

No release-specific processing rule, source completeness or whole-client applicability can be certified from these four live index sections alone. The exact remaining 42 not yet inspected document IDs are in [Phase 1 audit](phase1-audit.md).

## Known gaps and next acquisition requirements

- Microsoft published PDFs/DOCXs were linked but the 2025 MS-OCSPROT download redirected to `officeprotocoldoc.z19.web.core.windows.net` and could not be retrieved by the research browser; **no current source SHA-256** available. PDFs for the other 46 were **not** acquired; do not pass historical revision 4.4 (2015) as revision 9.0 (2025).
- More **direct §1.2.1** lists, *all recursive dependencies*, dated prior-version changes, Microsoft applicability footnotes and official RFC update chains remain open.
- To close Phase 1, a reviewer must verify the full **published-version** sources and recorded normative transitive closure in [phase1-audit.md](phase1-audit.md); no stage or feature may be marked complete prematurely.

**Phase 2 and Phase 3 technical analyses already on this branch are preliminary and do not replace Phase 1 closure. No protocol implementation, server test, merge or release may result from this source catalog.**
