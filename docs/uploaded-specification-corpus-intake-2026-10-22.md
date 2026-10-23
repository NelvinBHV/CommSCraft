# Uploaded authoritative corpus intake and research status — 2026-10-22

**Scope:** inventory / integrity verification only. The user supplied `catalog.zip` and `assessments.zip`; these remain external conversation attachments, **not copied into GitHub**. No protocol implementation changes, real-server tests, merge, or release.

## Source artifact integrity (local verification)

| Artifact | SHA-256 | Contents / checks |
|---|---|---|
| catalog.zip | `29b5a638107e80a2c3bf4fbfadf3f856d0667c80338b87d91e2c5aadc0bbcad7` | 352 archive entries, 151 PDFs, 151 corresponding .txt extractions, 54 primary entries in inventory.json. ZIP CRC test passed. |
| assessments.zip | `37b3504ba00e39f3db1a32fc4244c453c9bab4e0359daa2ee65cdffef435c76b` | 113 entries, 112 .md research assessments, ZIP CRC test passed. |

All 54 primary inventory PDF files were present and **all 54 calculated SHA-256 hashes matched** `catalog/inventory.json`; all 151 PDF files have same-name TXT extracts. The inventory declares **145 distinct RFC identifiers and 145 distinct MS protocol references** across its 54 primary entries (reference mentions, NOT claims of downloaded or read documents).

**Critical distinction:** 151 available PDFs vs 54 hash-indexed primary documents. The other 97 PDFs are available in the ZIP, but were not independently reconciled against inventory.json SHA records in this intake. The 112 assessment files are substantial **user-supplied prior analyses**; their contents and asserted clause coverage have not been independently reviewed in full.

## Critical authoritative source availability

| Document | Revision | Published | Inventory SHA-256 verification | Relevant assessment |
|---|---|---|---|---|
| MS-OCSPROT | 9.0 | 2025-05-20 | PASS | system-workflows.md |
| MS-SIPAE | 11.0 | 2025-05-20 | PASS | microsoft-sip-authentication.md |
| MS-SIPREGE | 17.1 | 2025-08-19 | PASS | microsoft-sip-registration-provisioning.md |
| MS-CONMGMT | 10.0 | 2025-05-20 | PASS | connection-management.md |
| MS-PRES | 13.0 | 2025-05-20 | PASS | presence-and-roaming-state.md |

The SHA-256 values and official source URLs for each are inside `catalog/inventory.json` (uploaded, not repo-stored). The complete PDFs and TXT copies resolve earlier inability to access Microsoft PDF downloads through the browser. In particular MS-SIPAE current-revision §3.2.4.1 (client signing) and §3.2.5.5 (Digest) can now be analyzed directly from the supplied PDF rather than guessed from historical HTML subsections.

## Assessment presence, not blanket endorsement

The supplied assessments report, among other things, that MS-SIPAE rev 11.0 §§1–7, five examples and 27 product notes were assessed; that MS-SIPREGE rev 17.1 §§1–8, 12 provisioning groups, examples, three figures and 176 product notes were assessed; and that MS-OCSPROT rev 9.0 contains 21 use cases assessed. These are **claims made by the assessment authors**, not independent verification performed during this intake. Each exact section and normative claim must be checked against the corresponding original PDF and product-notes applicability before incorporation as verified system requirements.

## New research procedure

1. Parse source inventory into authoritative source/version/sha graph and extend it to all 151 PDFs, annotating missing revision data; map each 112 assessment file to its exact underlying source(s).
2. Independently spot-check and then systematically validate quoted/claimed MUST/SHOULD rules, algorithms, state transitions, data schemas, timers and exceptions against PDF text and figures. Correct prior HTML-only research where revision 2025 differs.
3. Build reference closure with applicable RFC update/obsolescence and client-vs-server roles; distinguish optional historical features, incompatible deployment profiles and current-classic Windows Full Client.
4. Write traceable SIP/registration/security/provisioning, presence/contacts/messaging, conference/call/media and session recovery models, including negative and policy-denied states.
5. Publish evidence status per requirement: PDF clause independently checked / inferred from cross-doc synthesis / assessment-only / missing.
6. Research must precede *any* implementation planning or changes. Real-server interoperability checks belong solely to the owner.

**Status:** authoritative corpus locally available and integrity-checked; comprehensive clause-by-clause review and cross-spec synthesis **NOT COMPLETE**. This intake is a substantial prerequisite but does not close the research gate.
