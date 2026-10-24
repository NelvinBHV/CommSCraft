# Eight Microsoft published original PDFs — source evidence (2026-10-24)

**FACT:** The requester uploaded eight original Microsoft PDF binaries. Full-file SHA-256, size, number of pages, `[MS-*] - vYYYYMMDD` footer edition, revision history and exact published §1.2.1 reference sections have been checked locally; the full PDF binaries are **not stored in GitHub**. Earlier MS-AVEDGEA SHA-256 is historical and was not rechecked in this session. Editions of MS-KILE/NLMP dated 2026 are **not automatically applicable** to Skype for Business 2019.

| File | Revision / date | Pages | Bytes | Full SHA-256 | Reference evidence |
|---|---|---:|---:|---|---|
| MS-OCSPROT | 9.0, 2025-05-20 | 73 | 4927193 | `b0380ba5606072eb071bd67ffa7561e12cd7e55ec9dbd23a6553558438524ff2` | Overview; no standalone §1.2.1 normative reference list |
| MS-SIPAE | 11.0, 2025-05-20 | 59 | 1875611 | `6f0337a593233162401f0265e5425ad38d839174d1f78b8031405c5c7de64836` | §1.2.1 starts PDF page 10; 24 normative / 1 informative references |
| MS-SIPREGE | 17.1, 2025-08-19 | 134 | 3301629 | `597a3192b46a713d23c59ccfdaaa62b2d1a2a54341eb7f4c5a5299222d699608` | §1.2.1 starts PDF page 12; 17 normative / 1 informative references |
| MS-ABS | 10.1, 2025-08-19 | 79 | 3010562 | `e1c0e5f791bbb803d70d1de587773d02ae8e28921e98502340beb9379132b1e4` | §1.2.1 starts PDF page 9; 3 normative / 2 informative references |
| MS-SIP | 8.0, 2017-06-01 | 93 | 2709399 | `6ee1c16c922b7c154373572d5a3341139aaf6d91922a35468581a209f68e9e52` | §1.2.1 starts PDF page 10; 16 normative / 4 informative references |
| MS-KILE | 49.0, 2026-09-28 | 91 | 2892199 | `8d172e6fb7b760812fba2e41b1bb591dc80fcfce441419153cc93622456e7a01` | §1.2.1 starts PDF page 12; 43 normative / 18 informative references |
| MS-NLMP | 37.0, 2026-03-30 | 99 | 3032203 | `00fe14a058134aabf6a6db1bcc1c47b8d45f0656ef7de610c03712906a945a27` | §1.2.1 starts PDF page 12; 15 normative / 9 informative references |
| MS-RDPEMC | 16.0, 2024-04-23 | 41 | 1698642 | `17658ee28cef61d0ba11beb69be0e1fbe9e0b0f6aa7c146d5bc4100f64c0985b` | §1.2.1 starts PDF page 9; 4 normative / 0 informative references |

## Source-specific §1.2.1 verification

- **MS-ABS 10.1**: three normative references (E.164 February 2005, MC-RegEx, RFC2119) and two separately informative references (UASDC, Unicode) on PDF page 9. The exact 10.1 original is now available, but this does not prove a complete 10.0→10.1 content diff.
- **MS-SIPAE 11.0**: 24 normative references on PDF pages 10–11, including MS-KILE, MS-NLMP and RFC3261; one informative RFC3324. No synthetic MIC fixture is evidence of live authentication.
- **MS-SIPREGE 17.1**: 17 normative references on PDF pages 12–13, including historic `draft-ietf-sip-gruu-10`; RFC5234 is listed as informative. A later RFC must not silently replace the explicitly cited draft.
- **MS-SIP 8.0**: 16 normative references on PDF pages 10–11 and four informative references.
- **MS-KILE 49.0**: 43 normative references on PDF pages 12–14 and 18 informative references. PDF page footer `[MS-KILE] - v20260928` is **not a self-reference**.
- **MS-NLMP 37.0**: 15 normative references on PDF pages 12–13 and nine informative references.
- **MS-RDPEMC 16.0**: four normative references in PDF page 9 (**MS-ERREF, MS-RDPBCGR, MS-RDPEPS, RFC2119**), zero informative. The previously *partial indexed* reference list is now verified against exact original.
- **MS-OCSPROT 9.0**: the 73-page original of the system overview has been fully acquired and SHA-256 verified. This is an overview for the six protocol families, not a 47th source §1.2.1.

**Reconciliation:** The normalized normative target sets in these seven source bibliographies agree with the existing first-/second-level registers: 3+24+17+16+43+15+4 = **122 source-specific edges; zero detected discrepancies**. The existing 460 and 246 Microsoft normative edge totals **must not grow** merely because source binaries were acquired.

**Current evidence count (not a completion claim):** overview exact-edition §1.2.1 PDFs **16/46** (previously 13); HTML-only **30** (previously 33). Second-tier exact edition PDFs **4/12** (previously 0); remaining HTML-only **8** (previously 12, including the formerly partial MS-RDPEMC). Eight of the 58 immediate Microsoft specifications now have a full-file hash evidenced (seven new plus historically verified MS-AVEDGEA); **MS-OCSPROT overview** is an additional, separate new full-file hash. Microsoft normative graph still **460 + 246**, 146 further discovered nodes remain open. No previous/current version full-text diffs or RFC recursive closure established. Exact Windows-client build, Server role/policy, security applicability, and external historical standards remain OPEN.

**Phase 1 INCOMPLETE; 0/12 formal gates PASS. No implementation, live sign-in/interoperability, main merge, tag or release.**
