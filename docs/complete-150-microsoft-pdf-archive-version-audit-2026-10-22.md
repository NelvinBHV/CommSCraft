# Archive-wide original Microsoft PDF inventory audit and version evidence

**2026-10-22 research, not completion certification.** Scanned the *owner-supplied* `catalog.zip`, extracted original MS-* PDF byte digests and version strings from accompanying PDF text; compared primary `catalog/inventory.json`. This audit does **not** represent a deep technical reading of the extra PDFs. The associated complete 150-row CSV was generated as an attached conversation artifact; the repository currently stores this methodological report, not the complete CSV.

## Quantified coverage: original materials

| Measure | Observed | How verified |
|---|---:|---|
| Total PDFs | 151 | ZIP member listing; 150 MS-PDFs + FIPS197.pdf |
| Microsoft PDF IDs | 150 | Distinct `catalog/MS-*.pdf` members |
| Manifest-indexed primaries | 54 | Parsed `catalog/inventory.json`, has official URL, revision, date, sha256, references |
| Additional Microsoft original PDFs | 96 | Set difference between 150 IDs and 54 primary manifest IDs |
| Extra PDF edition date recoverable from its extracted footer | **95 / 96** | Regex anchored to `[MS-ID] - vYYYYMMDD` |
| Extra historical file lacking that exact footer pattern | **1** | `MS-SIPRE-2015.pdf` – do not assume its publication date or current applicability |
| Primary inventory versus PDF footer edition mismatches | **0 / 54** | Parsed both date fields; all 54 match |
| Referenced Microsoft IDs missing from locally provided PDF set | **0** | Only *direct MS links in the primary manifest*, not entire transitive references |
| Primarily indexed RFC references | **145 unique RFC IDs** | Metadata-reference count; no claim of 145 verified RFC originals |

For the **95 additional dated originals**: publication-year breakdown **2024:57, 2026:12, 2025:11, 2017:6, 2018:3, 2021:3, 2022:2, 2019:1**. These are **embedded extracted original footer date strings**, not independently verified historical online revisions. Within 2026 group: MS-ADTS 20260824; MS-DRSR 20260309; MS-NRPC 20260209; MS-PKCA 20260309; MS-RDPEGFX 20260511; MS-RDPERP 20260714; MS-SAMR 20260427; MS-SFU 20260330; MS-SMB 20260114; MS-SPNG 20260714; MS-SMB2 20260928; MS-WCCE 20260824.

## Verification mechanics

For each MS PDF, `sha256(archived_pdf_bytes)` computed locally; for primaries footer publication/date compared to manifest `date`; for extras extracted first matching `[ID] - vYYYYMMDD` from `.txt`. Generated CSV columns: `id, archive_pdf, pdf_sha256, embedded_version_date, manifest_revision, manifest_date, manifest_url, manifest_ms_references, manifest_rfc_references, inventory_status, deep_analysis_status`. All rows explicitly set `deep_analysis_status=NOT CERTIFIED`, as complete clause coverage is not established. **Presence/date/digest do not certify actual RFC normativity, a client's support, byte semantics or any code-level compliance.**

**Special caveat:** The additional `MS-SIPRE-2015` is not necessarily a different canonical specification ID. Track as *historical edition of MS-SIPRE*, then compare version and product profile before drawing wire-compatibility conclusions. Do not inflate number of unique Microsoft protocols by treating an edition suffix as a protocol. The historical PDF date remains unresolved under this strict footer regex; inspect rendered front matter.

## Outstanding whole-system requirements

1. Cross-walk all 96 extras, determine functional applicability to classic Windows Full Client vs server administration/AD/RDP infrastructure and attach official revision URLs and update history.
2. Follow references from **each of 96 extra PDFs** and original sections of all 150 PDFs; 54-entry manifest direct closure is **not** transitive closure.
3. Audit all RFCs (including RFC 3261, 3264, 3265/6665, 3263, 3711, 4028, 4568, 5245/8445, 5389, 5922, 6026), their update/obsolescence graphs and client-era support.
4. Analyze WSDL/XSD and wire examples and trust boundary failure states, not just section titles or summaries. Record precise version/section-level proven facts and classify inferred and unknown.
5. Complete Full Client functional requirement matrix (startup, TLS/PKI, all SA schemes, registration, contacts, IM, conference, media, PSTN/emergency, sharing, signout, network recovery).
6. Only after all research proof obligations close, draft an *evidence-linked implementation plan*. **No code now.**

**Research status: NOT COMPLETE.** No new protocol code, real-server/interop tests, merges to main, releases.