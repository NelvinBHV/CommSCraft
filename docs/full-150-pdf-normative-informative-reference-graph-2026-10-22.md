# Full 150-PDF cross-document reference extraction — normative, informative, overview and archival gaps

**Research audit: 2026-10-22 · evidence status: PARTIAL · implementation gate: CLOSED.**

## Purpose and source

This is the **second-level, archive-wide reference audit** on the owner-supplied local original `catalog.zip`. The previous [54-primary metadata closure audit](manifest-reference-closure-version-and-feature-gates-2026-10-22.md) is still accurate **in its original, narrow scope**, but does **not** establish transitive or archive-wide reference closure. The [150-original PDF archive edition audit](complete-150-microsoft-pdf-archive-version-audit-2026-10-22.md) established physical availability/date integrity. This work now explicitly parses **§1.2.1 Normative References, §1.2.2 Informative References**, or, for overview protocol documents, their **unsplit §1.2/§1.3/§1.4 References**.

**Original evidence:** All 150 Microsoft PDFs and companion full extracted text; each original's embedded footer `[MS-...]-vYYYYMMDD` and PDF SHA-256 computed. 54 documented URLs/revisions from `catalog/inventory.json`; the **96 unindexed PDFs' official exact revision URLs are not verified**. Names normalised case-insensitively (e.g. **MS-QoE → MS-QOE**) and self-document footer `[MS-X] - vYYYYMMDD` filtered to prevent false reference edges. The **historical edition `MS-SIPRE-2015.pdf` is treated as an additional physical edition of MS-SIPRE**, not a distinct modern protocol. Its footer date **20150904 is now extracted**, superseding the prior 'footer date unknown' statement based on a too-strict filename-to-ID matcher.

## Results, counted carefully

| Metric | Result | Meaning |
|---|---:|---|
| Microsoft original PDFs examined | **150** | Includes an additional *historical edition* of MS-SIPRE, i.e. not 150 independent feature requirements |
| §1.2.1 plus §1.2.2 parsable | **138** | Normative and informative **source listings** separated |
| Overview with unsplit references | **7** | **No normative or informative label inferred** |
| Pure AD schema reference structure absent | **5** | MS-ADA1, MS-ADA2, MS-ADA3, MS-ADLS, MS-ADSC; status **unclassified/not zero dependencies** |
| Distinct extracted (source, reference, source-section-category) listings | **3,060** | Not 3,060 client MUSTs or 3,060 distinct normative specs |
| From normative sections | **1,816** | **735 MS; 644 RFC; 42 Internet-Draft aliases; 395 other** |
| From informative sections | **804** | **193 MS; 79 RFC; 5 Internet-Draft aliases; 527 other** |
| From unsplit overview references | **440** | **302 MS; 93 RFC; 2 Internet-Draft aliases; 43 other** |
| Distinct RFC numbers **mentioned** anywhere across 150 references | **251** | Includes informative/overview citations. Does not prove these are all relevant/current, locally downloaded or client-required |
| Distinct Internet-Draft aliases mentioned | **23** | Must map to exact name/date/revision, including resolved historic ICE/TURN examples |
| Distinct Microsoft IDs referenced but whose PDF is NOT among the 150 | **191** | Includes normative/informative/overview from **all** AD/SMB/RDP/Windows system-level documents |
| Distinct missing Microsoft PDFs cited in a **normative** reference section | **68** | Across broader Windows infrastructure, not automatically Skype Windows client requirements |
| **31 selected direct Skype Windows full-client protocol source documents** | **0 direct MS-PDF omissions** | In this **selected immediate reference section** only. Not proof of recursive closure or of complete implementation behavior |
| **MS-OCSPROT 9.0** own unsplit source list | **49 MS IDs + 8 RFC IDs** | All directly listed MS PDFs in owner's corpus; **not a normative/informative classification** |

**Important:** The counts describe bibliography rows, **not references proven to be used normatively in a specific implementation clause**. A URL for an unindexed document has not been verified just because the PDF body has a publication date. There has been **no** automated traversal of every section's **inline citations**, normative update/obsolescence graph, or entire off-archive recursive graph. Actual Windows-client functional applicability still needs per-clause/manual proof.

## Audit protocol and reproducibility

1. Iterate the owner ZIP's `catalog/MS-*.txt`, pair it to the same physical `MS-*.pdf` and calculate SHA-256.
2. Use **anchored section headings**, ignoring table-of-contents entries, to identify `1.2.1`, `1.2.2` and next genuine `1.3` section; do **not** scan entire PDF indiscriminately (would count repeated `[RFC]` body citations as bibliography entries).
3. Overview `MS-OCSPROT` and analogous system overviews have a **single references list**, stop at their subsequent Functional Architecture/Overview heading; classify `overview_unclassified`.
4. Extract bracketed identifiers **only at the beginning of reference-entry lines** and deduplicate per (source document, section classification, reference ID). Explicitly drop footer line `[MS-X] - vYYYYMMDD`, which can otherwise generate a spurious self-dependency in every document.
5. Normalize the ID by uppercase **only for comparison** to locally available PDF names; preserve source-stated reference alias in the record. Mark IETF Internet-Drafts by **alias**, not an invented draft publication.
6. Mark `MS` targets **LOCAL_PDF_PRESENT / NOT_LOCAL**; `RFC` and third-party material **not yet original-verified** by bibliography listing alone.
7. Each record requires the source original **edition date** and reference section status; original PDFs' SHA-256 means byte identity, not correctness or completeness of the prose extraction. A second audit must manually validate unusual tables/broken PDF line wrapping.

**Complete machine-readable exports (available as conversation artifacts, *not yet committed as full CSV blobs to GitHub*):** `reference_nodes_150.csv` (150 PDFs including version/date/SHA256/54 URL provenance/reference-category counts) and `reference_edges_150.csv` (all 3,060 classified source-reference edges with missing status). To avoid claiming archival closure, only this independently reviewable summary is committed in the branch at present.

## Scope guard: client roots vs wider Windows environment

**31 immediate Skype for Business/Windows Full Client roots checked for missing MS-PDF references:** MS-OCSPROT, MS-CONMGMT, MS-SIPREGE, MS-SIPAE, MS-SIPRE, MS-SIP, MS-PRES, MS-ABS, MS-DLX, MS-CONFBAS, MS-CONFPRO, MS-CONFIM, MS-CONFAV, MS-CONFAS, MS-SDPEXT, MS-SRTP, MS-SSRTP, MS-AVEDGEA, MS-TURN, MS-ICE, MS-ICE2, MS-QoE, MS-OCPSTN, MS-EUMR, MS-EUMSDP, MS-OCEXUM, MS-E911WS, MS-RGSWS, MS-RTP, MS-DTMF, MS-RTCP.

**Finding: no direct missing MS target in these 31 sources.** This statement does **not** prove they cover every required feature, the actual Windows client uses all of them, their referenced RFCs are present, or that dependencies of their dependencies are closed. The broader **Windows infrastructure** sources such as MS-SMB2, MS-GPOL, MS-CIFS, MS-RPCE, MS-FSSHTTP, MS-OXWUMS and AD/Kerberos support produce the off-archive references below. For example, **MS-SMB2** has 9, **MS-GPOL** 7, **MS-CIFS** and **MS-RPCE** 6 each off-archive normative *Microsoft document* IDs in this extracted snapshot. These are **not automatically required SIP client behaviors**.

**All 68 unique Microsoft IDs cited normatively in wider archive but not locally present:**

```text
MS-BRWS MS-BRWSA MS-CMRP MS-CRTD MS-CSRA MS-CTA MS-DCOM
MS-DFSC MS-DFSNM MS-DLTM MS-DPSP MS-DSDIFFGRAM MS-DTCO
MS-EERR MS-EFSR MS-EMFPLUS MS-EVEN MS-FPSE MS-FRS1
MS-FSA MS-FSCC MS-FSSHTTPB MS-GKDI MS-GPCAP MS-GPFR
MS-GPIPSEC MS-GPREG MS-GPSCR MS-GPSI MS-ICPR MS-LISTSWS
MS-MQCN MS-MQMQ MS-MQQB MS-MQQP MS-MQRR MS-MSRP MS-NBTE
MS-NMFTB MS-NNS MS-ODATA MS-OXDSCLI MS-OXWSCDATA
MS-OXWSCORE MS-OXWSCVTID MS-OXWSSRCH MS-PAN MS-PCCRC
MS-RA MS-RAP MS-RCMP MS-RDPEAR MS-RPCH MS-RPCL
MS-RPRN MS-RSVD MS-RTPME MS-SCMR MS-SHDACCWS MS-SMBD
MS-SRVS MS-SWN MS-TLSP MS-VERSS MS-W32T MS-WMF MS-WMI MS-XCA
```

Other missing IDs occur in **informative or unsplit overviews**. Counting the union across categories yields 191, not their sum because some IDs are cited in more than one category. **Do not call all 191 normative.**

## Immediate Windows-client dependency sample (exact source/section classes)

| Source PDF and version | Bibliographic relationship | Source classification |
|---|---|---|
| MS-OCSPROT 9.0 (20250520) | 49 Microsoft spec IDs (e.g. MS-SIPAE, MS-SIPREGE, MS-SIPRE, MS-CONFBAS, MS-ICE2, MS-TURN, MS-PRES, MS-ABS, MS-DLX, MS-QoE), 8 RFC IDs | **Overview/unsplit**, not automatically normative |
| MS-CONMGMT 10.0 (20250520) | MS-SIPAE, MS-SIPCOMP, MS-SIPREGE | §1.2.1 **normative** |
| MS-SIPAE 11.0 (20250520) | MS-CONFBAS, MS-KILE, MS-NLMP, MS-OCAUTHWS, MS-PRES, MS-SIPRE | §1.2.1 **normative** |
| MS-SIPREGE 17.1 (20250819) | MS-ABS, MS-AVEDGEA, MS-CONMGMT, MS-DLX, MS-E911WS, MS-OCER, MS-PRES, MS-QoE, MS-RGSWS, MS-SIPAE, MS-SIPRE, MS-SIP | §1.2.1 **normative** |
| MS-ICE2 19.0 (20250520) | Aliases `IETFDRAFT-ICENAT-19`, `IETFDRAFT-ICETCP-07`, `IETFDRAFT-STUN-02` | §1.2.1 **normative** Internet-Drafts |
| MS-TURN 21.0 (20250520) | `IETFDRAFT-TURN-08` resolves **specifically to draft-rosenberg-midcom-turn-08 (2005)** | §1.2.1 **normative** alias; archive identified separately |

**Exact historic draft resolution:** see [TURN/STUN 2005 source study](ms-turn21-ietf-2005-original-draft-reference-closure-2026-10-22.md) and [ICE/ICE2 historical draft variants](ms-ice11-vs-ice2-19-historic-drafts-client-state-matrix-2026-10-22.md). A listed alias does not establish that a later RFC's behavior was supported in the old Windows client.

## Still blocking completion — explicit follow-up obligations

1. **96 additional official URLs/revisions and publication history**: check one by one against Microsoft Open Specifications; verify current revisions and product-version notes instead of using a PDF footer as full revision proof.
2. **151 physical PDF files** include 150 MS originals + FIPS197 PDF; five AD schema documents lack §1.2 reference lists and need a different dependency extraction method.
3. **All 3,060 entries** need second-pass inspection of broken/duplicate source lines, archive integrity and official external reference endpoints; `RFC` and `Internet-Draft` original presence still not certified by local ZIP.
4. Follow *transitive* normative and actual inline section references from the **specific Windows client feature roots**; classify server-only, client, gateway, conference MCU, AD, media edge and optional deployment branches.
5. RFC update/replacement chains (notably 3265→6665→7621, ICE/SDP, TLS, SRTP, SIP INFO) must be tied to actual Microsoft **client-era profiles**, and textual contradictions (SRTP SHA-1/256) remain **BLOCKED**.
6. Generate a strictly evidence-linked requirement-to-protocol-to-state/error/privacy mapping for every Full Client workflow; only then authorize a future implementation plan.

**This is an inventory and source-reference audit, NOT complete normative clause analysis and NOT functional proof of the whole client. NO protocol code, live-server/interoperability tests, main merge or release.**
