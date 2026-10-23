# CommSCraft: manifest-derived transitive reference coverage and edition register

Research tranche 2026-10-22. **Corpus scope, not claim of full protocol analysis.** Primary material: user-supplied `catalog.zip` (`catalog/inventory.json`, original PDFs and .txt versions) and `assessments.zip` (112 unverified secondary reviews). This is a data lineage / gap analysis based on locally enumerating ZIP member names and the primary manifest; no code changes and no live-server tests.

## Reproducible inventory counts and limitations

| Observable fact | Value | Meaning |
|---|---:|---|
| Manifest-indexed Microsoft PDF documents | **54** | Each has explicit document ID, PDF URL, revision, date, SHA-256 and reference lists. All 54 matched SHA-256 at earlier intake. |
| Distinct MS-*.pdf archive members | **150** | This counts MS-prefixed PDFs only. One further PDF makes 151 PDFs overall. |
| Additional MS PDFs not indexed by manifest | **96** | Locally present, but absent manifest-level edition/date/SHA and analysis status. Do not report them as unavailable. |
| Referenced Microsoft MS-ID not among 150 MS PDFs | **0** | **Direct, manifest-recorded Microsoft reference closure for the 54 indexed documents is locally available.** This does not establish transitive closure of the 96 extra PDFs or all client features. |
| Unique RFC references in indexed corpus | **145** | RFC *identifiers referenced*, not 145 downloaded RFCs or checked modern updates. |
| Microsoft secondary assessment Markdown files | **112** | Presence does not mean original PDF assertions have independently passed review. |

### Explicit methodological caveat

Manifest `ms` and `rfc` arrays are derived metadata supplied in the ZIP. They may include informative as well as normative citations, and can omit references missed by the extraction script. **No unverified reference should be labeled normative solely because it is in the manifest.** Normativity, section linkage, RFC update/obsolescence and profile relevance require inspection of each original document's §1.2 references and exact usage.

## Edition register: primary documents grouped by subsystem

These are **manifest-declared editions and dates** (YYYY-MM-DD), not proof that each source has been fully read. Original URL and SHA-256 for each record are in `catalog/inventory.json`.

| Area | Document = revision (publication) |
|---|---|
| System/connection | MS-OCSPROT=9.0 (2025-05-20); MS-CONMGMT=10.0 (2025-05-20); MS-SIP=8.0 (2017-06-01); MS-SIPCOMP=10.0 (2025-05-20) |
| SIP/auth/identity | MS-SIPAE=11.0 (2025-05-20); MS-SIPRE=13.0 (2025-05-20); MS-SIPREGE=17.1 (2025-08-19); MS-NLMP=37.0 (2026-03-30); MS-KILE=49.0 (2026-09-28); MS-OCAUTHWS=11.0 (2025-05-20) |
| Presence/directories | MS-PRES=13.0 (2025-05-20); MS-ABS=10.1 (2025-08-19); MS-DLX=13.0 (2025-05-20); MS-XCCOSIP=7.0 (2024-08-20) |
| Conferences | MS-CONFBAS=15.0; MS-CONFPRO=13.0; MS-CONFIM=11.0; MS-CONFAV=11.0; MS-CONFAS=12.0; MS-PSOM=11.0; MS-XMLMC=10.0 (all 2025-05-20) |
| Media | MS-ICE=11.0; MS-ICE2=19.0; MS-ICE2BWM=9.0; MS-SDPEXT=16.0; MS-SRTP=14.0; MS-TURN=21.0; MS-TURNBWM=9.0; MS-RTP=17.0; MS-RTPRADEX=13.0; MS-RTVPF=11.0; MS-RTASPF=10.0; MS-DTMF=15.0; MS-H264PF=16.0; MS-AVEDGEA=12.0; MS-QoE=12.0 (all 2025-05-20); MS-SSRTP=11.1 (2025-09-16) |
| PSTN and UM | MS-OCPSTN=11.0; MS-OCEXUM=11.0; MS-EUMR=9.0; MS-EUMSDP=9.0; MS-E911WS=9.0; MS-RGSWS=9.0 (all 2025-05-20) |
| Web/app/sharing | MS-CVWREST=7.0; MS-ECREST=8.0; MS-OCDISCWS=10.0; MS-OCSMP=7.0; MS-SIPAPP=8.0; MS-OCER=11.0 (all 2025-05-20); MS-WOPI=15.1 (2025-08-19); MS-OCGCWEB=2.0 (2016-09-14); MS-RDPBCGR=62.0 (2026-03-09) |
| Background, deployment | MS-ADOD=9.0 (2021-10-26); MS-MQSD=13.0 (2017-06-01) |

The above grouping is for **triage**. Some documents (AD internals, RDP, queue management, web clients) may be server-side or optional, not necessarily mandatory on the classic Windows Full Client wire.

## High-priority directly declared Microsoft-to-Microsoft edges

| Source document (manifest edition) | Direct reference edges enumerated in primary manifest | Cross-protocol concern |
|---|---|---|
| MS-OCSPROT 9.0 | **50** distinct MS references | Umbrella system map; verify exact Windows-client roles for each referenced protocol |
| MS-SIPREGE 17.1 | MS-ABS, MS-AVEDGEA, MS-CONMGMT, MS-DLX, MS-E911WS, MS-OCER, MS-PRES, MS-QoE, MS-RGSWS, MS-SIP, MS-SIPRE, MS-SIPAE (**12 total**) | Bootstrap/provisioning links user directory, location, presence, QoE and auth |
| MS-SIPAE 11.0 | MS-CONFBAS, MS-KILE, MS-NLMP, MS-OCAUTHWS, MS-PRES, MS-SIPRE | Auth must include Windows Kerberos/NTLM and SIP identity, not only response MIC |
| MS-CONFBAS 15.0 | MS-CONFAV, MS-CONFPRO, MS-CONMGMT, MS-OCER, MS-PSOM, MS-SIP, MS-SIPAE, MS-SIPRE, MS-SIPREGE | Conference provisioning/control/authentication |
| MS-CONFAV 11.0 | MS-CONFBAS, MS-CONFPRO, MS-ICE, MS-ICE2, MS-RTP, MS-RTPRADEX, MS-SDPEXT, MS-SIPRE | Conference state → SDP/media |
| MS-CONFAS 12.0 | MS-CONFBAS, MS-CONFPRO, MS-ICE, MS-ICE2, MS-RDPBCGR, MS-RDPEMC, MS-RTASPF, MS-RTP, MS-SDPEXT, MS-SIPRE | Desktop/app sharing and RDP media roles |
| MS-PRES 13.0 | MS-CONFBAS, MS-OCER, MS-SEARCH, MS-SIP, MS-SIPAE, MS-SIPRE, MS-SIPREGE, MS-XCCOSIP | Category privacy, subscription and identity |
| MS-RTP 17.0 | **13** MS references including MS-ICE/ICE2, MS-SDPEXT, MS-RTPRADEX, MS-RTVPF | Realtime media formats and transport |
| MS-SDPEXT 16.0 | **11** MS references, including MS-ICE/ICE2, MS-SIPRE, MS-SRTP | SDP interface across call, media and crypto |
| MS-KILE 49.0 | **24** MS references | Windows security stack, must not imply every server-side dependency is a client protocol |

Original manuscript references from `inventory.json`; the short edge lists above are extracted manually from that manifest, **not independently adjudicated as normative**. A complete machine-readable 54-row edge table still needs explicit export/version auditing.

## Available-but-unindexed material: concrete gap rather than “missing spec”

The 96 additional MS PDFs include MS-ADA1, MS-ADA2, MS-ADA3, MS-ADCAP, MS-ADDM, MS-ADSC, MS-ADTS, MS-APDS, MS-AUTHSOD, MS-CIFS, MS-CSSP, MS-DRSR, MS-DTYP, MS-ERREF, MS-FSSHTTP, MS-GPOL, MS-GPSB, MS-H26XPF, MS-KKDCP, MS-LCID, MS-NTHT, MS-OAUTH2EX, MS-OBPAS, MS-OBPRS, MS-OXWUMS, MS-PAC, MS-PKCA, and many others. The direct manifest graph is **file-complete for Microsoft identifiers**, but these unindexed PDFs must still have edition/date/sha added before the promised “complete source/version directory” can be called complete.

Likewise the RFCs from manifest metadata are **not verified locally present PDFs**. For example RFC 3261 (base SIP), 3264 (SDP offer/answer), 3265 (historical SIP events), 6665 (replacement), 3711 (SRTP), 4568 (SDP security descriptions), 5389 (STUN), 5245/8445 (ICE historical/current) have to be individually version-status audited and scoped by Microsoft edition. Only the ones actually listed by source count toward this manifest closure.

## Cross-spec feature gate / research plan (NOT implementation plan)

| Feature | Main original sources | In-progress state | Required evidence to close |
|---|---|---|---|
| Startup/TLS | MS-OCSPROT, MS-CONMGMT, RFC 3263, TLS, RFC 5922 | Partly clause-checked | Validated server identity, failure matrices, historical TLS profile |
| Authentication | MS-SIPAE 11, MS-KILE 49, MS-NLMP 37 | Client MIC §3.2.4.1 checked | Every scheme/context state and product-note applicability |
| REGISTER/provisioning | MS-SIPREGE 17.1, RFC 3261, RFC 3265/6665 | Key client/server provisioning rules checked | Complete errors, NOTIFY paths, expiry/recovery, payload schema |
| Contacts/presence | MS-PRES 13, MS-ABS, MS-DLX | Publication/privacy and BENOTIFY partly checked | XML categories, roaming contacts, multi-device and subscriptions |
| Direct IM | MS-SIPRE 13, MS-SIPAPP, MS-PRES | Protocol chain mapped | MESSAGE/dialog, multipart, receipts, feature guards |
| Conference IM | MS-CONFIM 11, MS-CONFBAS 15 | 202/IMDN 10s timer checked | IMDN schemas, MCU lifecycle, retries/reconnect |
| AV/calls | MS-CONFAV 11, MS-SDPEXT 16, MS-ICE2 19, MS-SRTP 14 | Key negotiation, crypto and ICE clauses checked | SDP full grammar, ICE nomination/restart, codec version matrix |
| Sharing | MS-CONFAS 12, MS-RDPBCGR 62 | Reference edges mapped | Complete app-sharing connection/media/state analysis |
| PSTN/emergency | MS-OCPSTN, MS-E911WS, MS-EUMR, MS-EUMSDP | Inventory only | Dialing, gateways, location/security, voicemail, DTMF |
| Signout/recovery | MS-SIPREGE, MS-PRES, MS-SIPAE | Unregister and cleanup cases partly checked | Cross-spec races, server migration, SA expiry and transport failure |

**Source status conventions:** present/PDF-hash verified ≠ edition inspected ≠ clause verified ≠ complete client profile. No later implementation planning until every needed source and cross-reference resolves, with explicit documentary gaps.

**Research gate:** NOT COMPLETE. No protocol code modifications, live-server/interoperability tests, merge into main, or release.
