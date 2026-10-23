# CommSCraft: Windows Full Client protocol research (DRAFT)

**Research snapshot:** 2026-10-23. **Phase 1 check: NOT COMPLETE (46/46 MS overview first-tier normative §1.2.1 lists inventoried; 12 current PDF reference page excerpts + 1 fully uploaded SHA-256-verified MS-AVEDGEA PDF; 460 normative edges, 113 RFC seeds; recursive closure and published-release edition checks OPEN).** **Scope:** Skype for Business 2019 classic Windows desktop Full Client, SfB Server 2019 and Subscription Edition. **Status: INCOMPLETE. No implementation authorization.**

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

- [rfc-version-relations-reviewed.json](rfc-version-relations-reviewed.json) — 10 official RFC Editor header Update/Obsolete relation reviews; complete RFC reference closure OPEN.
- [phase1-transitive-research-2026-10-23.md](phase1-transitive-research-2026-10-23.md) — 12 additional Microsoft source §1.2.1 reference audits, 246 second-tier edges, unresolved edition and scope risks.
- [transitive-reference-ledger.json](transitive-reference-ledger.json) — separate second-tier normative edge register; 11 official HTML sources and 1 search-only partial source.
- [transitive-frontier.json](transitive-frontier.json) — 146 new reference target IDs to assess at the next graph depth.
**Für die Wiederaufnahme zuerst lesen:** [CURRENT_STATUS.md](CURRENT_STATUS.md) (verbindlicher Forschungsstand, offene Nachweise und Reihenfolge) und [research-checkpoint.json](research-checkpoint.json) (maschinenlesbare Kennzahlen, Quell-Blob-SHAs und Arbeitsgrenzen). Ältere Zahlen im chronologischen Log sind historische Zwischenstände.

- [CURRENT_STATUS.md](CURRENT_STATUS.md) — current handoff: 46/46 direct overview reference lists, 460 normative edges, 199 targets, still Phase 1 incomplete.
- [research-checkpoint.json](research-checkpoint.json) — machine-readable snapshot with verified source hashes and explicit blockers.
- [source-catalog.md](source-catalog.md) — central versioned **46-MS / 113-RFC** primary-source index, access status and normative closure notes.
- [ms-ocsprot-inventory.json](ms-ocsprot-inventory.json) / [ms-ocsprot-inventory.md](ms-ocsprot-inventory.md) — all 46 identifiers from the six official MS-OCSPROT system protocol tables.
- [ms-avedgea-12-source-audit.md](ms-avedgea-12-source-audit.md) — uploaded current full PDF, SHA-256, exact five normative and five informative references, product and revision details.
- [transitive-microsoft-inventory.json](transitive-microsoft-inventory.json) / [human-readable table](transitive-microsoft-inventory.md) — 12 direct non-overview Microsoft references with latest official publication version/date; recursive references OPEN.
- [normative-reference-ledger.json](normative-reference-ledger.json) — **460** source-linked normative edges across **46** first-tier §1.2.1 lists; 12 PDF index sections, one SHA-256-verified uploaded full edition, remaining HTML release comparison OPEN.
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

- [RFC first-hop original bibliography audit](rfc-first-hop-audit-2026-10-23.md) / [JSON register](rfc-first-hop-reference-ledger.json): RFC 3261 §29 (26 normative) and RFC 2782 References (9 *unclassified*), 23 new RFC candidates outside unchanged 113 seeds; recursion/client applicability OPEN.

- [Formeller Phase-1-Abschlusscheck](phase1-closure-audit-2026-10-23.md) / [Gate-Ledger (JSON)](phase1-closure-gates-2026-10-23.json): 58 Microsoft-Einzelnachweise und 12 notwendige Bedingungen, derzeit **alle offen**. Kein Phase-1-Abschluss behauptet.

- [MS-ABS edition-specific PDF audit](ms-abs-edition-audit-2026-10-23.md) / [JSON evidence](ms-abs-edition-evidence-2026-10-23.json): rev 9.0 vs 10.0 original §1.2.1, §8 and §9 inspected; current 10.1 edition remains OPEN, no additional Phase-1 gate passes.
