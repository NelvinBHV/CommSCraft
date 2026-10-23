# Transitive Microsoft Open Specifications beyond MS-OCSPROT six tables

**Snapshot:** 2026-10-23. **All 12 direct non-overview MS destination IDs have an official published-version identity.** This is **metadata only**, not the *recursive* §1.2.1 reference extraction, not a full published-PDF read, and not a settled classic Windows Full Client mandatory-dependency set. Some newer Windows specs were released in **2026**, after the May 2025 Office/SfB overview; older editions must be evaluated for client compatibility.

| Direct normative target | Latest revision at research date | Published | Official Microsoft source | Referenced by (examples) | Remaining |
| --- | --- | --- | --- | --- | --- |
| MS-H26XPF | 1.3 | 2017-06-01 | [official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-h26xpf/ebea5d4a-a184-4912-83a6-df29606aaa78) | MS-RTP | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-KILE | 49.0 | 2026-09-28 | [official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-kile/) | MS-SIPAE | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-NLMP | 37.0 | 2026-03-30 | [official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-nlmp/b38c36ed-2804-4868-a9ff-8dd3182128e4) | MS-SIPAE | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-OAUTH2EX | 8.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/sharepoint_protocols/ms-oauth2ex/56105233-7ca7-4b4b-821d-84bb55bfc2cf) | MS-OCAUTHWS | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-OXWUMS | 16.0 | 2025-05-20 | [official](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxwums/79ace092-b793-4fa5-82b0-dd30d12c14c2) | MS-OCEXUM | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-RDPBCGR | 62.0 | 2026-03-09 | [official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/5073f4ed-1e93-45e1-b039-6e30c385867c) | MS-RTASPF | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-RDPEMC | 16.0 | 2024-04-23 | [official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpemc/1c867b2b-40b8-459a-9af6-906b6e0096fc) | MS-RTASPF, MS-CONFAS | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-SDP | 1.0 | 2017-06-01 | [official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-sdp/697845ff-5357-4eb7-8bcb-162a0bc84deb) | MS-H264PF | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-SIP | 8.0 | 2017-06-01 | [official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/MS-SIP/0d72fb55-12ba-49f1-850d-f0bf3110bb1f) | MS-SIPREGE, MS-PRES, MS-CONFBAS | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-WCCE | 55.0 | 2026-08-24 | [official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wcce/) | MS-OCAUTHWS | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-WSPOL | 8.0 | 2019-03-13 | [official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wspol/94d73e99-3a43-4133-a310-1c38be24b8ce) | MS-OCAUTHWS | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |
| MS-WSTEP | 15.0 | 2024-04-23 | [official](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wstep/) | MS-OCAUTHWS | Full current PDF, release-specific §1.2.1 and normative transitive chain OPEN |

**Important library boundaries:** MS-OAUTH2EX comes from SharePoint; MS-OXWUMS from Exchange; MS-SIP, MS-SDP, MS-KILE, MS-NLMP, MS-RDPBCGR, MS-RDPEMC, MS-H26XPF, MS-WCCE, MS-WSPOL and MS-WSTEP from Windows Protocols. This is why stopping at the MS-OCSPROT overview list would miss security and client functionality dependencies.

**Version-specific warning:** MS-KILE 49.0 (2026-09-28), MS-NLMP 37.0 (2026-03-30), MS-WCCE 55.0 (2026-08-24), MS-RDPBCGR 62.0 (2026-03-09) were published after the 2025 Skype overview. Documenting latest revisions does **not** imply Windows Full Client 2019 negotiates or conforms to all their 2026 semantics. Verify explicitly applicable historical release profiles and product behavior appendices.

**Excluded by none yet.** No normative section here was read, no definitive applicability/exclusion decision was made, and no protocol implementation may be planned until the entire chain is resolved. The entire original first-tier graph appears in [normative-reference-ledger.json](normative-reference-ledger.json); audit in [phase1-audit.md](phase1-audit.md).
