# Historical desktop-client edition comparison: PSOM 2014 and XCCOSIP 2013 vs modern Open Specifications

**Research 2026-10-23. Scope: focused original version spot check, NOT a claim 2015 SfB binaries comply with every 2014/2013 requirement.** Official archived Microsoft original snapshots:
- [MS-PSOM] **v20141019**, release **2014-10-30**, revision **3.2** (source front matter), public original https://officeprotocoldoc.z19.web.core.windows.net/files/MS-PSOM/%5BMS-PSOM%5D-141030.pdf (web-extracted original p1 and §§3.2.3.1.1–.4 pp51–54).
- [MS-XCCOSIP] **v20131118**, release **2013-11-18**, revision **2.0** (source front matter), public original https://officeprotocoldoc.z19.web.core.windows.net/files/MS-XCCOSIP/%5BMS-XCCOSIP%5D-131118.pdf (web-extracted §2.2.2.2 pp37–40 and original §3.1 TOC, 94 pages).
- In existing primary 150-PDF archive: **MS-PSOM rev11.0 2025-05-20** SHA `6eee9f7f42014a0806d887448bb5837a68eb56a2a5048ebfa9fd9a4814b6e049`; **MS-XCCOSIP rev7.0 2024-08-20** SHA `dedf341e7f1031453bab3bc5a196a8563ad7381895e0458eb51bf870e270077e`. Compare current studies [PSOM packet and auth](ms-psom11-tls-auth-token-version-frames-whiteboard-2026-10-23.md) and [XCCOSIP inner statuses and timer](ms-xccosip7-persistentchat-sip-info-xccos-ack-timers-filetoken-2026-10-23.md). Old archived PDFs were **text-inspected via official Microsoft hosting**, but no independent cryptographic SHA copy stored in this tranche. Formal historical byte-diff/errata pass still OPEN.

## 1. Exact legacy PSOM join and hash exchange exist at least as early as October 2014

| ID | 2014 source | 2025 comparison | Evidence class / limit |
|---|---|---|---|
| HIST-P1 | 2014 §3.2.3.1.1.1 p51 requires C3P `sAuthId` and optional `fwdtls proxyHeader` from AddUser | 2025 §3.2.3.1.1.1 p53 has same conditional identifiers | **P both** for these specific clauses; not proof all distributed-object interfaces identical |
| HIST-P2 | 2014 §3.2.3.1.1.1 p51 uses TLS direct FQDN/port from `pwrpc.pwsURI` and conditional alternativeName, or provisioned proxy choices | Same 2025 source clause | **P both**, but no general crypto TLS profile downgrade permission |
| HIST-P3 | 2014 §§3.2.3.1.1.1–.2 p51–52 sends **0x70773200**, auth version zero, uint32 BE token length, sAuthId, expects **0x70773200** ACK | 2025 §§3.2.3.1.1.1–2 p53–54 describes same fields | **P both**, a historically stable *specific join format*, not complete protocol binary equivalence |
| HIST-P4 | 2014 §3.2.3.1.2 pp52–53 `SetChannel=0`, `version`, `addProtocol`, `doneProtocols`, hash mismatch terminates, then application Channel2 | Same 2025 §§3.2.3.1.2–4 pp54–56 | **P both**, target client's supported interface hash list may differ; cannot assume 2025 list in 2015 build |
| HIST-P5 | 2014 ConnMgr `[Hash=8322047979521208965L]` listed in §3.2.3.1.3 p53 | Same numerical example in 2025 §3.2.3.1.3 | **P both** for this ConnMgr interface hash only |

**Security implication (S):** A 2015-era client can plausibly speak this original PSOM token/ConnMgr handshake, but it cannot be claimed supported on actual installed client without its feature/profile and DataMCU capability evidence. The historical protocol requires cryptographic TLS; neither the historical `alternativeName` behavior nor modern PDF’s `tls` label justifies accepting invalid certificates.

## 2. File-token naming ambiguity predates the newer API prose

| ID | 2013 source | 2024 source | Result |
|---|---|---|---|
| HIST-X1 | §2.2.2.2.2 p38 explicitly lists **`cmd:getfutok` = upload**, **`cmd:getfdtok` = download** | §2.2.2.2.2 p37 defines identical separate command identifiers | **Historically established distinction** in command grammar. This is not new in 2024 |
| HIST-X2 | 2013 reply id table §2.2.2.2.4 p40 lists **`rpl:getftok`** for both upload and download | 2024 §2.2.2.2.4 also lists **`rpl:getftok`** | A **shared reply ID** does not imply the request commands are identical. Response correlation must consult command-specific matching rules |
| HIST-X3 | 2013 §3.1 TOC has **no numbered §3.1.19 token procedure** in that edition (last listed client procedure §3.1.18, “Retrieving Legacy User Preferences”) | 2024 adds §3.1.19, which says **`cmd:getfutok` for both upload and download** contrary to its own §2.2.2.2.2 | **Cross-edition typo candidate remains unresolved**: old command table favors `getfdtok` for downloads, but no official erratum or actual client packet evidence has established the correct 2024 `MUST` interpretation |
| HIST-X4 | 2013 §2.2.2.2.1 pp36–37 has `xccos ver=1`, `envid` and embedded `seqid` correlation, `grpchat` separate from commands | Same 2024 §2.2.2.2.1–3 | Stable envelope model, **not** proof all permission/room commands or File Repository implementation identical |

## 3. Normative classification and unresolved client profile

Microsoft Open Specifications published revision histories explicitly show **PSOM rev4.0 March 2015**, rev10.0 August 2024, rev11.0 May 2025, and **XCCOSIP rev2.0 Nov 2013**, rev7.0 Aug 2024. The Skype for Business Windows Full Client commonly overlaps releases across these dates, but the exact executable version, server product, Edge/DataMCU and feature deployment must be independently identified.

**No automatic historical substitution:** A 2025 original's prescriptive MUST could be newly introduced after a 2015 client build. The 2014 and 2013 sources show only the **specific clauses compared here**; remaining historical schema sections, TLS versions, role/error/timer changes and Microsoft product notes still need full edition-by-edition review.

**No protocol implementations, real servers, assistant interoperability tests, merge or release. Research INCOMPLETE.**
