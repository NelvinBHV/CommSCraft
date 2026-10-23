# MS-ABS historical-edition source audit — 2026-10-23

**P1-01/P1-02/P1-07 — PARTIAL; not a current-release attestation. Phase 1 remains INCOMPLETE.** This work examines official PDF-rendered pages for published **historical editions**, not full downloaded/hashed original binaries. All protocol implementation, real-server testing, merge and release gates remain closed.

## Primary-source provenance and levels

| Edition | Official version metadata | Inspected official PDF | Inspected pages | Evidence level |
| --- | --- | --- | --- | --- |
| MS-ABS rev **9.0**, 2024-08-20 (`v20240820`) | [Microsoft published / previous-version table](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abs/ddc59abf-1f42-4935-9b72-5314665dbcff) | [2024 PDF](https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-ABS/%5BMS-ABS%5D-240820.pdf) | p.9 §1.2.1/§1.2.2; p.74 §8; p.76 §9 | `FACT: official PDF text AND rendered page inspected`; **NO local binary SHA-256** |
| MS-ABS rev **10.0**, 2025-05-20 (`v20250520`) | [Microsoft previous-version table](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abs/ddc59abf-1f42-4935-9b72-5314665dbcff) | [2025 PDF](https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-ABS/%5BMS-ABS%5D-250520.pdf) | p.7 scope of normative sections; p.9 §1.2.1/§1.2.2; p.74 §8; p.76 §9 | `FACT: official PDF text AND rendered page inspected`; **NO local binary SHA-256** |
| MS-ABS rev **10.1**, 2025-08-19 (currently published) | [Microsoft Published Version](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abs/ddc59abf-1f42-4935-9b72-5314665dbcff) | Current PDF link redirects to a host not accessible in this research session | **No current-edition page available** | `OPEN: release-original / §1.2.1 / binary hash / changes vs 10.0` |

The official Microsoft table categorizes 9.0 as Major (2024-08-20), 10.0 as Major (2025-05-20), and 10.1 as Minor (2025-08-19). **Neither a revision class nor identical historic references establishes 10.1's text**. The official latest direct PDF/DOCX delivery was not fetched or hashed.

## FACT — edition-specific reference sections

Both inspected exact historical PDFs show the following **three** §1.2.1 *normative reference identifiers* on printed page 9:

| Reference identifier | Source citation in both PDFs | Existing first-tier ledger target |
| --- | --- | --- |
| `[E164]` | ITU-T Recommendation E.164, **February 2005**, paid original | `E164-2005` |
| `[MC-RegEx]` | Microsoft “Regular Expression Language Elements”, historical MSDN URL `az24scfc(VS.80).aspx` | `MC-RegEx` |
| `[RFC2119]` | RFC 2119, **March 1997**, BCP 14 | `RFC2119` |

Both versions separately list **two INFORMATIVE §1.2.2 references**: `[UASDC]` (Ziv/Lempel, May 1977) and `[UNICODE]` (Unicode Consortium home page). They are **not** new normative graph edges.

**Reconciliation:** The existing three `MS-ABS → (RFC2119, E164-2005, MC-RegEx)` records in `normative-reference-ledger.json` are consistent with these two *historical* published editions. That ledger intentionally describes the current MS-ABS release **10.1**, so its `exact_release_pdf_correspondence = UNVERIFIED` MUST remain unchanged. Do not increment **460** first-tier edges; do not promote the first-tier **13/46 current-edition PDF section proofs** to 14/46.

## FACT — normative body versus cited normative documents

MS-ABS 10.0, printed p.7 §1 introduction, states **§§1.7 and 2** are the normative sections of this file-format specification; other sections and examples are informative. The **reference classification** in §1.2.1 is a different question from the **normative applicability of a body clause**. Do not turn informative §1.3 prose or product examples into independent MUST requirements for the Windows client.

MS-ABS 10.0, printed p.74 **§8 Appendix D**, explicitly includes **Skype for Business 2019**, **Skype for Business Server 2019**, and **Server Subscription Edition** among products the specification covers; the actual Windows client build and server role/policy profile remain **OPEN**. A product being enumerated does not make every optional feature mandatory for that product.

## FACT — bounded historical change tracking

- The **2024 rev 9.0** §9 table, p.76, flags **§8 supported-product list** as Major-changed relative to its previous release.
- The **2025 rev 10.0** §9 table, p.76, flags a **§1.1 glossary / cryptographic hash function** entry as Minor and the **§8 supported-product list** as Major.
- The displayed 2024 and 2025 **§1.2.1** lists are identical in identifier and cited edition. This is **only a verified *section-level* comparison** of these two historical PDFs, **not** a binary/textual full-document diff and **not** a 10.0→10.1 diff.

## OPEN — required next evidence

1. Obtain the **latest MS-ABS 10.1 (2025-08-19) official PDF/DOCX** through an authorized working delivery path, record the exact official URL, byte size, cryptographic SHA-256, section and PDF page; reconcile §1.2.1 against 10.0 and existing HTML dated 2023-08-03.
2. Inspect 10.1 §9 and compare all substantive historical revisions, especially the §1.1 and §8 product/version notes. No historic PDF page equals the 10.1 released edition.
3. Resolve the original **E.164 February 2005** paywalled edition, the historical MC-RegEx source, and client 2019 build applicability without inventing or substituting a modern revision.
4. Maintain first-/second-tier and RFC registers, including the **12 formal Phase-1 gates**, unchanged until exact-release proof exists.

**Scope:** documentation research only; no protocol code, test fixtures, genuine server connections, interoperability claims, main merge or release.
