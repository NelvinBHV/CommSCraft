# Re-Audit: 122 offline inventory entries versus RFC and Microsoft evidence

Date: 2026-10-18. Branch: `integration/v030-sip-san-six-prs` (PR #35).

## Audit decision

**Inventory coverage is 122/122, but normative requirement verification is 0 and independently verified Microsoft Skype for Business Full Windows Client interoperability is 0/122.** This is not a release acceptance.

The immutable baseline `FEATURE_PARITY.json` still contains 7 historical `offline_subset` records. The development overlay `release/offline-feature-development.json` contributes 115 additional entries. Their merge covers all 122 named catalog IDs; it does not convert metadata/UI previews into fully working product capabilities.

The ledger `release/offline-feature-spec-audit.json` has 122 records, reports 121 exact fixture mappings, one historic `windows-ui` exception, and explicitly reports `normative_requirement_verified: 0` and `live_interoperability_verified: 0`. This is a ledger-level inspection, **not** independent execution of all Rust tests. The existing audit unit test checks uniqueness, expected IDs, source paths and fixture symbol presence. It does not compare every normative statement in a pinned Microsoft PDF to code and test outcomes.

## Primary-source evidence checked during this re-audit

- RFC 2782, February 2000, §3: DNS SRV priorities and weights. https://www.rfc-editor.org/rfc/rfc2782#section-3 . Discovery records must be tested against the actual priority/weight procedure and error cases before a normative pass is granted.
- RFC 8445, July 2018, §5.1.2: positive unique candidate priority range 1–2^31−1 and prioritization rules. https://www.rfc-editor.org/rfc/rfc8445#section-5.1.2 . The `ice` UI preview merely stages a nonzero `u32` and does **not** implement candidate gathering, exchange, STUN connectivity checks, or all RFC priority constraints.
- [MS-SIPAE], revision 11.0, published 2025-05-20, catalog: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd . The `certificate-auth` display placeholder does **not** authenticate, validate certificate paths, or establish a session.
- [MS-CONFBAS], revision 15.0, published 2025-05-20, catalog: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confbas/6cb739fe-3a84-4266-8d52-0af777f6f1fa . Local participant/meeting models do **not** demonstrate conferencing wire signaling.
- [MS-PRES], revision 13.0, published 2025-05-20, catalog: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/741d682d-590b-4600-916e-8deeee426a5c . Local presence drafts do **not** establish presence-server interoperability.

The three MS links establish published revision identities; **their protocol clauses have not been exhaustively reviewed here**. For other Microsoft reference families in the ledger, especially those with `edition: UNVERIFIED` or `section: not-applicable-local-only`, revision and section are not validated normative evidence. Do not turn a contextual family link into a specification-conformance claim.

## Remaining evidence gates

1. Pin exact public edition/date/URL and clause for each protocol-relevant feature; extract individual MUST/SHOULD requirements and any pertinent security/edge-case constraints.
2. Create a feature-by-requirement map to source and *positive, negative, policy-denied and recovery* fixture IDs; verify that test assertions implement the clause rather than merely mention the feature name.
3. Record executed offline tests for the exact commit on Ubuntu/Windows, with hashed job evidence and no credentials or private captures.
4. Where permitted, perform separate authorized real Windows Full Client versus server interoperability tests and preserve sanitized evidence. Neither a passed local Rust test nor an RFC citation proves this.
5. Complete the historical Windows UI test exception and independent security/release approvals before any parity or release claims.

**Do not change** historical evidence hashes, security gates, `main`, or the v0.30.0 release decision. This report corrects interpretation of the inventory but does not claim source-code remediation, complete clause audits, or interoperability.
