# Security blocking discrepancy: SRTP SHA-1/256 across 2015–2025 Microsoft editions and RFC 3711/4568

**Research timestamp:** 2026-10-22. **Severity:** interoperability and cryptographic-selection blocker; **not resolved**. **All original excerpts must be interpreted by precise version and role**. User provided local primary PDFs from `catalog.zip`; cross-checked visually rendered actual 2025 PDF pages **MS-SRTP 12/19, MS-SDPEXT 20/82, MS-SSRTP 14/24**, not only OCR or secondhand assessments. RFCs checked against official RFC Editor HTML. This explicitly supplements [the existing media security/profile analysis](ms-sdpext16-srtp14-avedgea12-security-negotiation-conflict-2026-10-22.md).

## A. Primary-source and edition comparison

| Evidence ID | Original publication/edition, primary clause | Precisely documented statement | Relevance |
|---|---|---|---|
| CRYPT-E01 | [RFC 3711](https://www.rfc-editor.org/rfc/rfc3711), March 2004, §§4.2.1, 5.2 | RFC SRTP default message authentication is **HMAC-SHA1**; default `n_a=160` and `n_tag=80`, with specific SRTCP constraints. | Defines baseline only; extensions can explicitly define alternative algorithms. |
| CRYPT-E02 | [RFC 4568](https://www.rfc-editor.org/rfc/rfc4568), July 2006, §6.2.1 | Negotiated SDES `AES_CM_128_HMAC_SHA1_80` explicitly means AES-128 Counter Mode with **HMAC-SHA1** and **80-bit tag**. Base64 key + salt is **30 decoded bytes** (128-bit master key + 112-bit salt). | Crypto-suite identity fixes hash algorithm, not just tag width. |
| CRYPT-E03 | Microsoft [MS-SRTP] **2015-09-04**, v20150904, PDF pp11–13, §3.1.3.3.2 | Historical Microsoft source specifies **HMAC-SHA-1**, `n_a=160`, `n_tag=80`. Archived official [historical PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-SRTP/%5BMS-SRTP%5D-150904.pdf) and revision summary identify **rev5.0**. | **Historical evidence only**, not current behavior proof. |
| CRYPT-E04 | **MS-SRTP rev14.0 v20250520**, §3.1.3.3.2, original PDF **p12/19**, SHA256 **1e6e706604da2f1925568c27fedfeca9d2283383685979e33d7116f994cb363d** | Current original explicitly says **HMAC-SHA-256** with `n_a=160`, `n_tag=80`, *while pointing to RFC3711 §4.2*. Microsoft Learn [same clause](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-srtp/f38137ac-7394-4907-b80f-8c4f290cffac) agrees with local original. | Normative current 2025 wording contradicts RFC's default description and advertised SHA1 suite name. |
| CRYPT-E05 | **MS-SDPEXT rev16.0 v20250520**, §3.1.5.1, PDF **p19–20/82**, SHA256 **83a151015e74ccadd7e14e856cd0967fcb64eb3c30c167f3638fa0d50edfcdb2** | SDP `a=crypto` suite **MUST** be `AES_CM_128_HMAC_SHA1_80`; only inline key method allowed; `lifetime=2^31`; **optional** 1-byte MKI; the protocol can also use `a=cryptoscale`. | Direct conflict **within May 2025 specs** when standard SDP suite identity is assumed authoritative. |
| CRYPT-E06 | **MS-SSRTP rev11.1 v20250916**, §§3.1.3.2–3.1.3.3.2, PDF **p14–15/24**, SHA256 **a9d20e69afe73ba3bb85ed4073ecb78c42b3eb0a5ddc16c1fc86ef37acb979a3** | SSRTP **MUST use HMAC-SHA1**, `n_a=160`, `n_tag=80`. It has **different authenticated fields, ESN and packet format** (its §2.2.1.2), and says by default to inherit other MS-SRTP parameters unless overridden. | SSRTP explicitly overrides SHA1 itself; it does **not** remove the direct MS-SRTP/SDP discrepancy for non-scaled SRTP. |
| CRYPT-E07 | MS-SRTP 14.0 §7, MS-SDPEXT 16.0 §7 (May 2025) | Their **most recent** change-tracking row mentions a product-behavior appendix update, not this cryptographic rule. | It does **not** prove *when* SHA256 appeared, whether it was intentional or a publication defect; do not infer change date from §7 summary. |

**First-party URLs:** original 2025 PDFs from the 54-entry ZIP manifest for MS-SRTP, MS-SDPEXT, MS-SSRTP. Historical 2015 URL above is **an additional verified edition source**; its URL must be tracked by separate edition, not overwrite the 2025 manifest record. The 2015 PDF was inspected as indexed official PDF text; direct renderer access through web screenshot was unavailable, so do not assign a newly calculated archival SHA256 to 2015. The 2025 originals were visually checked.

## B. Why this is a real profile conflict, not an innocuous display difference

- A wire SDP suite such as `AES_CM_128_HMAC_SHA1_80` specifies the keyed hash type **SHA-1** in RFC4568 §6.2.1. Both SHA-1 and SHA-256 variants might truncate a tag to 80 bits; the common tag size `80` and auth-key length `160` **do not make their authenticators interchangeable**.
- RFC3711 §4.2 and §5.2 describe HMAC-SHA1; MS-SRTP 2025 §3.1.3.3.2 explicitly writes HMAC-SHA256 while citing those RFC clauses. These are **distinct algorithm names** even if a deployment could support negotiated alternatives.
- MS-SSRTP §3.1.3.3.2 directly selects SHA1 for scaled-specific packet authentication. That **does not** prove which authenticated fields or transform an ordinary SRTP packet uses, since SSRTP has an explicit distinct layout/ESN and role-specific `a=cryptoscale`.
- Do **not** assert an editorial typo, hidden SHA256 override of the advertised suite, universal interoperability, or a specific actual Skype for Business binary's hash selection from these texts alone.

## C. Proven constraints surrounding the unresolved hash

| Constraint | Verified source | Consequence for evidence model |
|---|---|---|
| Plaintext inline key in signaling | MS-SDPEXT §5.1, RFC4568 §8 | SIP TLS must provide adequate confidentiality for SDP; credential secrecy through every intermediary and end-to-end identity need separately proved. |
| MKI vs media tags | MS-SDPEXT §3.1.5.1 | 1-byte MKI when present and unique SDP tag constraints independent from hash type. |
| Negotiation via re-INVITE | MS-SDPEXT §3.1.5.2.2 | Re-INVITE **MUST NOT** renegotiate SRTP vs SSRTP or `a=crypto` parameters; does not resolve initial offer/final-answer fork changes under §3.1.5.12. |
| Per-direction crypto state | MS-SRTP §§3.1.3.1–2 | Two cryptographic contexts (sent/received), and each SSRC has independent rollover and replay; hash identity remains per negotiated transform. |
| Replay | MS-SRTP §§3.1.5.1–2 | 64-entry replay state and SRTCP shared index by direction; replay rejection is distinct from HMAC verification and ICE consent. |
| SSRTP packet layout | MS-SSRTP §§2.2.1.1–2,3.1.3.3.2 | Different authenticated portion from RFC3711; the SHA1 result cannot be transplanted as evidence of ordinary SRTP layout. |

## D. Research gates that must close before algorithm selection (no implementation plan)

1. Retrieve exact **MS-SRTP intermediate archived editions** after 2015 and up to 2025; compare versions, paragraph changes, version-history changelog, and Microsoft errata. Until then the transition point is **UNKNOWN**.
2. Confirm with first-party errata/product behavior or authoritative interoperability notes whether **MS-SRTP rev14.0 `HMAC-SHA-256`** is intentional, profile-gated, or a documentation defect. Do not infer an answer from SHA-256 being generally stronger.
3. Original [MS-SDPEXT] §§3.1.5.1–2, [MS-SSRTP] §§2.2.1.2 and 3.1.3.3, [MS-SRTP] §3.1.3.3.2 need **crossed per-role/offer-answer matrices**, since `a=cryptoscale client/server` changes packet direction algorithms.
4. Confirm full RFC4568 §6.1, §6.2.1 and RFC3711 §4.2/§4.3 normative bytes and any RFC updates, including re-keying/lifetime conditions, without guessing packet test vectors.
5. Build historic Skype for Business **specific release/capability profile** including media gateway/MCU/peer differences; no authenticated live probes by the assistant.

## E. Negative-case / security test requirements for later research only

Without performing tests: if a packet authenticates only under a *different* hash than explicitly negotiated, it **cannot** be treated as validated for the agreed `AES_CM_128_HMAC_SHA1_80` transform merely because AES-CM decrypts it or the tag length matches. RTP and SRTCP use distinct packet/auth fields. Failed media-authentication must not be conflated with a SIPAE MIC failure, TURN 401 challenge, ICE consent expiry or SIP 430 route failure.

**Conclusion:** This is a **confirmed textual contradiction** in selected Microsoft 2025 specifications and RFC4568; the **real deployed behavior and explanation remain OPEN**. Research is NOT COMPLETE. No protocol implementation, server traffic, interoperability test, main merge or release.
