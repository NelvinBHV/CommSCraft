# OfficeProto.zip – complete original-PDF acquisition, edition and normative-source audit (2026-10-24)

**Evidence class:** FACT about *user-supplied* PDF bytes, not an assertion that a GitHub copy exists or that every document is the latest client-applicable revision. Original files remain outside the **public** Git repository pending copyright/redistribution disposition and an appropriate private binary upload path. This is research documentation only, with no protocol code or live-server testing.

## File acquisition and provenance

- Input upload: `OfficeProto.zip`, ZIP SHA-256 `27f31dc1020d284159a44cecef5233aee5856814271cbf20f4f7ee1b8dd452f8`.
- **74** valid original PDF files extracted; **180,967,975 bytes** total across the decompressed PDFs, each independently SHA-256 hashed; original PDF page counts and footer editions inspected.
- All **46** MS-OCSPROT overview source-specification IDs are present. Of these, **43** ZIP editions are the latest revision reported by the 2026-10-24 research inventory, and **three are historic**: **MS-ABS 10.0** (latest known 10.1), **MS-SIPREGE 17.0** (latest known 17.1), **MS-SSRTP 11.0** (latest known 11.1).
- **23** formerly HTML-only first-tier §1.2.1 source lists are now checked in exact current original PDF binaries; edition header, original PDF §1.2.1 and full-file SHA-256 inspected. All 46 first-tier normative sections now have *some current-edition original PDF section evidence* when these sources are combined with the previously recorded source-page/original evidence; this **does not** mean the files are archived in GitHub.
- Eight previously identified cross-library second-tier Microsoft source PDFs (MS-H26XPF, MS-OAUTH2EX, MS-OXWUMS, MS-RDPBCGR, MS-SDP, MS-WCCE, MS-WSPOL, MS-WSTEP) are still missing from this ZIP. Twenty-eight ZIP documents are outside the 46 first-tier MS-OCSPROT table identities, and do not automatically become product requirements.
- The embedded Microsoft *Intellectual Property Rights Notice* permits copies for developing implementations but expressly mentions distribution of **portions** in implementation documentation, without a blanket authorization to publish complete PDFs. The CommSCraft repository is public. Do **not** publish the complete set there absent adequate redistribution rights; use a separate access-restricted source archive where permitted. This is a license/repository-publication **OPEN** item, not evidence of a completed public upload.

## Original full-file SHA-256 for the 23 previously HTML-only first-tier sources

All of these PDFs match their cataloged *current* publication revision in the source document's revision history. PDF §1.2.1 reference **counts** agree with the previous research ledger except for the independently documented MS-E911WS correction in a previously PDF-checked source; complete target semantic canonicalization still requires review.

| ID | Rev | Full original PDF SHA-256 (user ZIP) |
|---|---|---|
| MS-DLX | 13.0 | `ff889cbc01f4638b80215394c1344e5a1d4e96f3e0a810904a74815d8811a0e2` |
| MS-EUMR | 9.0 | `cc3b8e598c30c0466875f9fa2cfe132ec668c718704c2808f4d51a7ca0764013` |
| MS-OCEXUM | 11.0 | `b68c2386989a4997afb456d07ea7399a7223f9394cb7fedbe4d494cdd3581d77` |
| MS-OCPSTN | 11.0 | `862611884cf77413e23a3a9ac4fa4cb7adc71a95389dfeb3d19e369eff1fae05` |
| MS-OCER | 11.0 | `2684ac14a7260b10dbb9786530ac754fe92c5b33a149bb4a9f25c00576419098` |
| MS-QoE | 12.0 | `abbf6db1fb140b0452699c76ac8ce6412feec1e2fe337d7669498efd4a936690` |
| MS-RGSWS | 9.0 | `766d904618beec230eac7bcecc4236016b8873bf40f8e557eb0998749e6e130e` |
| MS-SIPAPP | 8.0 | `8e037aa6522e4ae5f94627abf2a90de430c64ba7b515b285f63237c64b612773` |
| MS-XCCOSIP | 7.0 | `6310f8c2b718ca4ba3b91fd31e6afe57bfff3251b38867ad46a34f9981000e9e` |
| MS-OCGCWEB | 3.0 | `cad12266be9e021ddd669b109f07091dc829a86b44e35d22efc81b9100c8f219` |
| MS-CONFPRO | 13.0 | `109d712c67003869ecd6d0012d2ca88edbc1e65c1408cbfc9e267410e04f65ed` |
| MS-CONFIM | 11.0 | `8d6662af6740130f1c8d19f652c9c589c4058ab7a94d54127508900b3e91eb82` |
| MS-CONFAV | 11.0 | `4db0c425151492d696e554a4481df42f9c960f8cb7572e8975dad6f24f0bddec` |
| MS-CONFAS | 12.0 | `9e228401500d522fb7d95c082fbe7b1d519fa6a4766b80566a1190f1dfa52db9` |
| MS-XMLMC | 10.0 | `7549ab1685dc96bfb2227e44ffe59bfb3a632a26a378433debd8d727ccd2f954` |
| MS-OCSMP | 7.0 | `42db4ba26bc94a858d7cb9a5bac1fdec226eb9b86475bd7c8820b1f8f9d30f6a` |
| MS-OCDISCWS | 10.0 | `9009666086ea0798675a0df2b21f42b9aa14a7fc14ad717906726f72439a136d` |
| MS-CVWREST | 7.0 | `f512a00d00666dd6074f3c8a91fce19926d2c8a7f98d0ed19a8290648a7d6a99` |
| MS-DTMF | 15.0 | `9daf8d6464abd00ffa277137f9e032b9920297262953ab6c3d5e30ae6171c363` |
| MS-ICE | 11.0 | `0afe5f2bed2b2728688a11c9a40cb89878cb66da4cd4ca659fe6d105c6ba39ad` |
| MS-ICE2 | 19.0 | `5f0023f87f0f94e2db71517c1287374022ea93eaeb48c9ed227d97df7dfcf197` |
| MS-ICE2BWM | 9.0 | `925570b9d13c54b1f9158a247abdb6f6d61f2b69dce8e53daef671702342e706` |
| MS-TURNBWM | 9.0 | `ee2638dd0dd8a842f88bda739c32d94193f0c2b70ad6c543a3400dc416d3dc77` |

## Source-verified correction: MS-E911WS rev 9.0

**FACT:** The complete `[MS-E911WS].pdf` (2025-05-20, 38 pages, **1,271,558 bytes**, SHA-256 `ef6ee906db885d9f7e637456ae69144066293a48d5cea64ac9d6fa2b3af63e95`) has twelve explicitly normative `§1.2.1` references across **PDF pp.8–9**. Page text and rendered screenshots were inspected.

The earlier first-ring ledger incorrectly recorded only eight, omitting four individually listed source identifiers on PDF p.9:

- **[WSDL]**: W3C Note, March 2001 (`WSDL1.1-2001`);
- **[XMLNS]**: Namespaces in XML 1.0 Third Edition, December 2009 (`XMLNS-ThirdEdition-2009`);
- **[XMLSCHEMA1/2]**: XML Schema Part 1 Second Edition, October 2004 (`XMLSCHEMA1-SecondEdition-2004`);
- **[XMLSCHEMA2/2]**: XML Schema Part 2 Second Edition, October 2004 (`XMLSCHEMA2-SecondEdition-2004`).

These four reference *targets* already occur elsewhere in the first-tier graph. They are **four new source-specific directed normative edges**, **zero new unique target identifiers**. The existing `WSA1.0-WSDL-2006` edge is valid and separate; it must not be removed or conflated with `WSDL1.1-2001`. Separately, `§1.2.2` on p.9 lists MS-NLMP, MS-OCAUTHWS, RFC2818, RFC4559 **informatively**, which are not counted as normative.

**Recalculated first-level directed edges: 460 → 464, external edges 83 → 87, distinct targets remain 199.** Second-tier directed edges remain 246; RFC seed remains 113. These new graph totals do not establish full recursive normative closure, client requirements, or release readiness.

## Provenance ambiguities that must not be hidden

The source ZIP contains different SHA-256 digests from historically recorded hashes **even where the edition footer and byte length agree**, including:
- **MS-AVEDGEA 12.0:** ZIP `cd6995b4dae1fd9b29f63fe92f2fc0cde779f1b134b8e085b5f295110fb3ba73` versus previous `baa3cddc1e1bfb4b0c2b0a998c71b802f80d4e5e6bb30adc0d8d42ebbbb0c8ff`; 1,562,486 bytes.
- **MS-SIPAE 11.0:** ZIP `b33ebb273291fc0c471de269a3bb1c55d5010c1e87970aa331f91548678df2d9` versus previous `6f0337a593233162401f0265e5425ad38d839174d1f78b8031405c5c7de64836`; 1,875,611 bytes.
- **MS-OCSPROT 9.0:** ZIP `08d5d71a7c182ef08c0dc7a912ad29461cd782657310cc30bf79830fb21b054e` versus previous `b0380ba5606072eb071bd67ffa7561e12cd7e55ec9dbd23a6553558438524ff2`; 4,927,193 bytes.

**OPEN:** Original previous binaries are not present for a byte-level diff; do not overwrite historical provenance, infer identical binaries from matching length/edition, or conclude document-content divergence without a text/binary comparison.

## Remaining gates and archiving rule

- First-level current §1.2.1 release-source sections available: **46/46** in some exact original-PDF evidence; 23 newly supplied and hashed full original files. **Remaining first-tier HTML-only release-section entries: 0**.
- Source-specific reference **counts** now match all 46 after correcting MS-E911WS; full canonical target-set/manual semantic verification and full previous-version diffs remain **OPEN**.
- Missing second-tier current-release originals: **8**; RFC normative references/update/errata and external ISO/ITU/W3C editions remain open; exact Windows 2019 client/server/policy scope remains unpinned.
- The 74 original full PDFs were available and SHA-256 checked **locally**, but **NOT uploaded to public GitHub**. The research branch contains provenance and audit metadata only. Private archival with the rights to store/distribute is required before claiming "PDFs in repo". No protocol implementation or live interoperability test was performed.
