# Primary source catalog — 2026-10-23 (OPEN inventory)

**Method:** "Landing verified" means the official Microsoft Learn *Published Version* row was directly inspected; it **does not** mean that version's complete contents or references have been analyzed. "Overview enumerated" means the identifier was identified in MS-OCSPROT §§2.2.2–2.2.3 or Microsoft Office technical document index; versions are intentionally recorded as `NOT CHECKED`. New candidate documents must be added rather than silently excluded.

## Published releases confirmed on Microsoft landing pages

| Doc ID | Scope | Published revision | Published date | Comparison/known prior | Status and official source |
| --- | --- | --- | --- | --- | --- |
| MS-OCSPROT | Skype for Business system overview | **9.0** | **2025-05-20** | 8.0 2021-08-17 | Landing verified; full current PDF inaccessible; [Microsoft](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c) |
| MS-CONMGMT | Discovery, long-lived connections, keep-alive, TLS cert requirements | **10.0** | **2025-05-20** | 9.0 2024-08-20 | Landing verified; selected live §3.1.5 reviewed, **release binding unconfirmed**; [Microsoft](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/9f6ebbf0-9000-4dc8-86cd-ebfe22467cbf) |
| MS-SIPAE | SIP authentication extensions | **11.0** | **2025-05-20** | 10.0 2024-08-20 | Landing verified; detail not fully reviewed; [Microsoft](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd) |
| MS-SIPREGE | SIP registration and provisioning | **17.1** | **2025-08-19** | 17.0 2025-05-20 | Landing verified; selected live §3.1.2.5.1 reviewed, **release binding unconfirmed**; [Microsoft](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d) |
| MS-SIPRE | Routing / endpoint identity / GRUU | **13.0** | **2025-05-20** | 12.0 2024-08-20 | Landing verified; detail open; [Microsoft](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipre/ab4ab249-3796-4ed1-8cec-f496d81a1a83) |
| MS-PRES | Presence / enhanced presence | **13.0** | **2025-05-20** | 12.0 2024-08-20 | Landing verified; detail open; [Microsoft](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/741d682d-590b-4600-916e-8deeee426a5c) |
| MS-CONFBAS | C3P conference control | **15.0** | **2025-05-20** | 14.0 2024-08-20 | Landing verified; detail open; [Microsoft](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confbas/6cb739fe-3a84-4266-8d52-0af777f6f1fa) |
| MS-SDPEXT | SDP media extensions | **16.0** | **2025-05-20** | 15.0 2024-08-20 | Landing verified; detail open; [Microsoft](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sdpext/cd17a549-b948-42a6-aa6b-fa707710faac) |

**Important:** MS-SIPREGE was updated **after** 2025-05-20. A catalog pinned indiscriminately to the MS-OCSPROT release date would miss that newer registration document. The version diff (17.0 → 17.1) has **not** been examined; no compatibility conclusion is made.

## Additional Microsoft specifications — identified, edition NOT CHECKED

The following are **candidates**, not a declaration that every item is mandatory for every deployment. Final relevance must be settled by the feature matrix, normative reference chains and product behavior notes. Canonical lookup entry point: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-offprotlp/939e9850-c48d-464d-860e-78f8b281e7bd .

| Group | Identifiers (each one: version/date, full sections, normative closure STILL OPEN) |
| --- | --- |
| Directory/contacts | MS-ABS, MS-DLX; also investigate server web-service address book discovery and distribution lists |
| SIP/control/web services | MS-EUMR, MS-OCEXUM, MS-OCPSTN, MS-OCER, MS-QoE, MS-SIPCOMP, MS-RGSWS, MS-PSOM, MS-SIPAPP, MS-OCAUTHWS, MS-E911WS |
| Conference | MS-CONFPRO, MS-CONFIM, MS-CONFAV, MS-CONFAS, MS-XMLMC |
| Media/payload | MS-EUMSDP, MS-DTMF, MS-H264PF, MS-RTP, MS-RTPRADEX, MS-RTASPF, MS-RTVPF, MS-SRTP, MS-SSRTP |
| ICE/edge | MS-AVEDGEA, MS-ICE, MS-ICE2, MS-ICE2BWM, MS-TURN, MS-TURNBWM |
| Other linked Microsoft technologies | MS-RDPBCGR (application-sharing graphics), MS-KILE (Kerberos extensions), MS-NLMP (NTLM), MS-SIP (cross-spec reference observed in MS-SIPREGE §1.2.1); applicable Windows trust/SSPI/certificate specifications |
| Cross-cutting discovery | Microsoft Office technical documents index and MS-OCSPROT §§2.2.1, 2.2.2.3 (directory/HTTP details not yet collected in this pass) |

Overview inventory source for SIP group: [MS-OCSPROT §2.2.2.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/bdcd1606-11c6-4457-a708-3d1c296259a0); conference: [§2.2.2.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/bf570f65-7429-4df3-8f81-751956ff314f); media: [§2.2.3.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/14c988fa-2da1-444f-84b9-1eab89c8264c); ICE: [§2.2.3.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/b3382ff5-585c-4cf4-ad1f-c962a49d6ad0).

## IETF RFC standards — candidate references and edition hazards

All RFC URLs use the official RFC Editor metadata/standard text; each `RFC` number is a **fixed published edition**, not an auto-updating standard. **Status for all rows: candidate recognized; complete relevant sections, errata, obsolescence/update chains and Microsoft edition compatibility NOT CHECKED** unless singled out elsewhere.

| Layer | RFCs to close and analyze (initial seed, NOT exhaustive) |
| --- | --- |
| Requirements, naming/discovery | [2119](https://www.rfc-editor.org/info/rfc2119), [8174](https://www.rfc-editor.org/info/rfc8174), [1034](https://www.rfc-editor.org/info/rfc1034), [1035](https://www.rfc-editor.org/info/rfc1035), [2782](https://www.rfc-editor.org/info/rfc2782), [3263](https://www.rfc-editor.org/info/rfc3263), [2131](https://www.rfc-editor.org/info/rfc2131), [2132](https://www.rfc-editor.org/info/rfc2132), [3361](https://www.rfc-editor.org/info/rfc3361) |
| SIP core, dialogs and extensions | [3261](https://www.rfc-editor.org/info/rfc3261), [3265](https://www.rfc-editor.org/info/rfc3265), [6665](https://www.rfc-editor.org/info/rfc6665), [3325](https://www.rfc-editor.org/info/rfc3325), [3892](https://www.rfc-editor.org/info/rfc3892), [3428](https://www.rfc-editor.org/info/rfc3428), [3515](https://www.rfc-editor.org/info/rfc3515), [5626](https://www.rfc-editor.org/info/rfc5626), [3581](https://www.rfc-editor.org/info/rfc3581), [3262](https://www.rfc-editor.org/info/rfc3262), [3311](https://www.rfc-editor.org/info/rfc3311), [4028](https://www.rfc-editor.org/info/rfc4028) |
| SDP, media and session control | [3550](https://www.rfc-editor.org/info/rfc3550), [3551](https://www.rfc-editor.org/info/rfc3551), [4566](https://www.rfc-editor.org/info/rfc4566), [8866](https://www.rfc-editor.org/info/rfc8866), [3264](https://www.rfc-editor.org/info/rfc3264), [3711](https://www.rfc-editor.org/info/rfc3711), [4733](https://www.rfc-editor.org/info/rfc4733), [4585](https://www.rfc-editor.org/info/rfc4585), [5245](https://www.rfc-editor.org/info/rfc5245), [8445](https://www.rfc-editor.org/info/rfc8445), [5389](https://www.rfc-editor.org/info/rfc5389), [8489](https://www.rfc-editor.org/info/rfc8489), [5766](https://www.rfc-editor.org/info/rfc5766), [8656](https://www.rfc-editor.org/info/rfc8656) |
| TLS, X.509 and GSS | [5246](https://www.rfc-editor.org/info/rfc5246), [8446](https://www.rfc-editor.org/info/rfc8446), [5280](https://www.rfc-editor.org/info/rfc5280), [6125](https://www.rfc-editor.org/info/rfc6125), [9525](https://www.rfc-editor.org/info/rfc9525), [2743](https://www.rfc-editor.org/info/rfc2743), [4120](https://www.rfc-editor.org/info/rfc4120), [4648](https://www.rfc-editor.org/info/rfc4648) |

**Version conflict register:** Microsoft [MS-SIPREGE §1.2.1](https://learn.microsoft.com/fr-fr/openspecs/office_protocols/ms-siprege/24cbca88-e618-4d70-979d-3c52d6ec5c04) currently lists RFC 3261, RFC 3265 and **draft-ietf-sip-gruu-10** (rather than simply assuming modern finalized GRUU rules). MS-OCSPROT §§2.2.2.1 and 2.2.3.1 reference RFC 3265, RFC 3550 and RFC 3551. Legacy Microsoft references must be compared with replacements rather than silently upgraded. Normative-reference closure for each spec remains **OPEN**.

## Unavailable / not yet evaluated

- Current 2025/2025-08 PDFs/DOCXs: **not acquired/reviewed** during this pass (tool redirect restriction). Historical MS-OCSPROT PDF 2015-03-30 revision 4.4 available and visually inspected only for structure; **not used to infer modern requirements**.
- The actual exhaustive list of `[MS-*]` documents linked from *every* referenced specification: **not yet traversed**.
- Version history, substantive changes and product behavior appendices for all candidates other than the eight landing rows: **not yet evaluated**.
- Exact target Windows Full Client build and deployed policy feature profile: **not provided / unresolved**.
- No claims of normative completeness are made.
