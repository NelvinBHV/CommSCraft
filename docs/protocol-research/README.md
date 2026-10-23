# CommSCraft: Windows Full Client protocol research (DRAFT)

**Research snapshot:** 2026-10-23. **Phase 1 check: NOT COMPLETE (46/46 MS overview metadata; 45/46 direct §1.2.1 reference lists; 12 indexed current PDF reference-page sources; 455 edges, 113 RFC seeds; 0 full released binaries/hashes; transitive closure OPEN).** **Scope:** Skype for Business 2019 classic Windows desktop Full Client, SfB Server 2019 and Subscription Edition. **Status: INCOMPLETE. No implementation authorization.**

This is a source-driven research track, **not** a protocol conformance claim, a test report, or an assertion that the client can sign in. Existing `FEATURE_PARITY.json` (122 requirements), `WINDOWS_REFERENCE_CASES.json` (93 cases), `docs/122-feature-rfc-ms-reaudit.md` and immutable history remain the baseline inventory. Nothing here supersedes those artifacts without a separately reviewed reconciliation.

## Evidence levels and mandatory recording format

- **FACT** = an identified document, dated revision and section explicitly states the claim; record verbatim requirement keyword only where needed, paraphrase the rest. A web subsection without verified release attachment is **not revision-verified normative evidence**.
- **INFERENCE** = deduction from cited facts, with premises and falsification question; never present it as a wire requirement.
- **OPEN** = incomplete access, unresolved cross-reference, version delta, implementation-dependent option, ambiguous product applicability or missing behavior. No guessed wire values.
- **HISTORICAL** = an older version useful for navigation or history, not automatically normative for Skype for Business 2019/SE.

Suggested requirement record: `ID | role | trigger | wire shape | MUST/SHOULD/MAY | state/preconditions | timer/retry | failure | trust boundary | [document] revision/date § | evidence status | open question | linked parity ID`. No requirement may be marked `VERIFIED` without reviewing its exact cited edition.

## Source priority

1. Official Microsoft's **published version** of each [MS-*] Open Specification, download/date/revision as cited by its Microsoft Learn landing page; subsection and normative reference lists must be reconciled against the **same release PDF/DOCX**, not just loosely versioned live subsection pages.
2. Official [MS-OCSPROT] overview for dependency **discovery** and client scenarios, not a replacement for protocol processing rules.
3. RFC Editor canonical RFCs, including **Updates**, **Updated by**, **Obsoletes** and **Obsoleted by** relationships. Keep Microsoft-specified historic RFC editions alongside current IETF replacements when behavior differs.
4. Microsoft Windows Open Specifications for SSPI, KILE, NTLM, certificates and RDP; Microsoft support/lifecycle material for product applicability; repositories and fixture logs only as **secondary evidence**.

## Work products (all provisional)

- [source-catalog.md](source-catalog.md) — central versioned **46-MS / 113-RFC** primary-source index, access status and normative closure notes.
- [ms-ocsprot-inventory.json](ms-ocsprot-inventory.json) / [ms-ocsprot-inventory.md](ms-ocsprot-inventory.md) — all 46 identifiers from the six official MS-OCSPROT system protocol tables.
- [normative-reference-ledger.json](normative-reference-ledger.json) — 455 source-linked normative edges from 45 §1.2.1 lists (PDF-indexed and HTML); published-edition comparison OPEN.
- [rfc-inventory.json](rfc-inventory.json) — fixed-identifier IETF seed inventory with provenance and update-chain status.
- [phase1-audit.md](phase1-audit.md) — measurable Phase-1 coverage and explicit inaccessible / unreviewed sources.
- [dependency-matrix.md](dependency-matrix.md) — layered adjacency/edge types, normative-reference expansion backlog.
- [sipae-authentication-clause-review.md](sipae-authentication-clause-review.md) — MS-SIPAE §§3.2.5.1–3.2.5.2, authentication variants, MIC canonicalization, replay and failures (edition caveat).
- [rfc3261-rfc2782-clause-review.md](rfc3261-rfc2782-clause-review.md) — original-edition RFC clause review and timers.
- [full-122x93-traceability.md](full-122x93-traceability.md) — every existing Windows parity feature linked to the 93 baseline scenarios.
- [discovery-registration-analysis.md](discovery-registration-analysis.md) — analyzed clauses, wire/decision boundaries and explicit security gaps.
- [flows-and-states.md](flows-and-states.md) — high-level Mermaid diagrams; deliberately marks **unknown transitions**.
- [full-client-coverage.md](full-client-coverage.md) — traceability of broad Windows Full Client areas and outstanding research.
- [research-log.md](research-log.md) — verified progress, blockers, next examination procedure.

## Stop condition / release lock

**No new protocol implementation or implementation plan may be executed while documentation and analysis are incomplete.** The implementation plan requested for Phase 4 will be **compiled from validated requirement IDs only after full source review**, with an explicit graph of dependencies and acceptance criteria. Do not merge this draft to `main`, open a release, connect to real SfB infrastructure or run independent interoperability tests. The owner alone executes live tests. Historical offline successes must never be described as server interoperability.

## Completion checklist (currently all NOT satisfied)

- [ ] Entire MS-OCSPROT protocol inventory and every client-relevant cross-reference reconciled, including product applicability notes.
- [ ] All relevant [MS-*] published release PDFs/DOCXs and their normative reference sections individually read and pinned.
- [ ] All relevant RFCs and RFC update/obsolescence relationships reviewed in full applicable sections.
- [ ] All messages, header grammar, authorization, states, timers, errors, security boundaries and retry paths documented per feature.
- [ ] Each of the 122 feature requirements and 93 reference cases linked to source and a complete negative/policy-denied/recovery path.
- [ ] All contradictions and open questions addressed, or bounded and explicitly dispositioned with evidence.
- [ ] Full Windows-client build / supported behavior inventory and deployment profiles reconciled by owner evidence.
- [ ] Source-review signoff before implementation planning; interoperability evidence remains owner-provided.

## Navigation sources

- Microsoft overview [MS-OCSPROT] revision 9.0, 2025-05-20: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c
- MS-OCSPROT §§2.2.2.1, 2.2.2.2, 2.2.3.1, 2.2.3.2:
  https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/bdcd1606-11c6-4457-a708-3d1c296259a0 ;
  https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/bf570f65-7429-4df3-8f81-751956ff314f ;
  https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/14c988fa-2da1-444f-84b9-1eab89c8264c ;
  https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/b3382ff5-585c-4cf4-ad1f-c962a49d6ad0
- Historical **not current** [MS-OCSPROT] PDF v20150330, revision 4.4, 73 pages (seen via PDF and page-render): https://officeprotocoldoc.z19.web.core.windows.net/files/MS-OCSPROT/%5BMS-OCSPROT%5D-150330.pdf

**Access note:** The current PDF link on the Microsoft Learn landing page redirected to a host that was not accessible through the research browser on 2026-10-23. The existence of a working historical PDF **does not substitute** for a complete review of current published documents. Several Learn pages display an authorization banner despite rendering subsection text.
