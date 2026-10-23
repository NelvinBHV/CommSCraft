# MS-AVEDGEA rev 12.0 — verified original-source acquisition and reference audit

**Date of examination:** 2026-10-23. **This closes the missing first-tier MS-OCSPROT §2.2.3.2 source reference-list entry. It does NOT close Phase 1's transitive normative graph.** Source: user-provided *official published MS-AVEDGEA PDF*, title **Audio Video Edge Authentication Protocol**, footer `[MS-AVEDGEA] - v20250520`, release **2025-05-20**, **revision 12.0** per revision table PDF p.3. Official [Microsoft Published Version landing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-avedgea/56860f6a-2fcf-4cad-8ee5-466d8f382a45).

## File provenance / exact release identification

| Field | Verified |
| --- | --- |
| Source kind | **User-supplied exact-edition PDF binary** in this research conversation, filename `[MS-AVEDGEA].pdf`; PDF bytes are NOT committed to GitHub |
| SHA-256 of received bytes | `baa3cddc1e1bfb4b0c2b0a998c71b802f80d4e5e6bb30adc0d8d42ebbbb0c8ff` |
| Size | **1,562,486 bytes** |
| PDF pages | **40**, unencrypted, PDF 1.4 |
| PDF original edition marker | `v20250520`, document footer and copyright 2025 |
| Published revision/date | **12.0 / 2025-05-20**, revision table p.3; cover/footer |
| Local original available on later independent research runs? | **NOT GUARANTEED**; only checksum and findings preserved in repository; upload bytes must be supplied again or acquired from Microsoft to reverify |
| Source review | Complete §1.2.1 normative reference list visually checked against page **7**; §1.2.2 informative refs p.8; version history pp.2–3; scope/applicability pp.8–10; product behavior pp.36–37; change tracking p.38. Selected later clauses reviewed for dependency identification; exhaustive Phase-2 technical clause audit remains separate |

## Exactly five normative references — §1.2.1, PDF page 7

The five records below are the **entire explicitly enumerated normative-reference subsection** in this exact PDF release (not inferred from a general protocol description). Their original cited editions matter.

| Exact key as printed | Target / edition | Original URL printed in PDF | Acquisition status |
| --- | --- | --- | --- |
| `[IETFDRAFT-SIPSOAP-00]` | Deason, *SIP and SOAP*, **draft-deason-sip-soap-00**, **2000-06-30** | http://www.softarmor.com/wgdb/docs/draft-deason-sip-soap-00.txt | Historic draft original and reference chain NOT acquired/reviewed |
| `[RFC2119]` | RFC **2119**, **1997-03**, BCP14 normative keywords | https://www.rfc-editor.org/info/rfc2119 | RFC source catalog present; full original normative chain and RFC 8174 implications NOT closed |
| `[XMLNS]` | W3C *Namespaces in XML 1.0 (Third Edition)*, **2009-12** | https://www.w3.org/TR/2009/REC-xml-names-20091208/ | Exact cited W3C document linked, substantive reference review OPEN |
| `[XMLSCHEMA1/2]` | W3C *XML Schema Part 1: Structures (Second Edition)*, **2004-10** | https://www.w3.org/TR/2004/REC-xmlschema-1-20041028/ | Exact cited document linked, review OPEN |
| `[XMLSCHEMA2/2]` | W3C *XML Schema Part 2: Datatypes (Second Edition)*, **2004-10** | https://www.w3.org/TR/2004/REC-xmlschema-2-20041028/ | Exact cited document linked, review OPEN |

**Normative distinction:** MS-ICE, MS-ICE2, MS-SIPRE and MS-TURN are NOT named among these five normative references. They are separately classified as **informative** in §1.2.2, notwithstanding strong operational/protocol dependency in §§1.3–1.4. Do not falsely add normative reference edges for them from MS-AVEDGEA §1.2.1.

## Exactly five informative references — §1.2.2, PDF page 8

| Target key | Reason to follow for client architecture | Classification |
| --- | --- | --- |
| MS-ICE2 | ICE 2.0 integration | **Informative** (and contextual operational dependency) |
| MS-ICE | ICE traversal | **Informative** |
| MS-SIPRE | Receiving SIP requests and sending responses (§1.4) | **Informative** |
| MS-TURN | TURN token use and relay services (§§1.3,1.4) | **Informative** |
| XML10 | XML 1.0 (Third Edition), W3C **2004-02** | **Informative** |

This classification is important: informative cross-spec dependency can still be essential to functionality, but should not be misreported as a normative §1.2.1 target.

## Applicability and publication changes

- **§1.6 p.9:** A/V Edge authentication supplies security tokens for access to a TURN server. **§1.4 p.9:** SIP SERVICE transport / MS-SIPRE routing and TURN integration.
- **Appendix B, p.36:** Explicitly lists **Microsoft Skype for Business 2019**, **Microsoft Skype for Business Server 2019**, and **Microsoft Skype for Business Server Subscription Edition**, among other product versions. This confirms named *document applicability*, **not** that a given deployment always uses edge traversal or every optional token feature.
- **Revision Summary, p.3:** rev **11.0** published 2024-08-20, rev **12.0** published 2025-05-20; both marked Major. **§8 Change Tracking, p.38** specifically lists `Appendix B: Product Behavior — Updated list of supported products` as the change since prior release. This reconciles the reason for the **Major** label without asserting that the wire formats are byte-identical to the previous edition; true diff to v11.0 PDF has not been performed.
- **§1.7 p.10:** Requested protocol version is in XML request, unsupported version yields `Version Mismatch` (specific processing in §3.1.5). Product/client version vs protocol version **are different axes**.
- **§2.1 p.11:** SIP SERVICE messages MUST be on a TLS-secured connection. **§2.2 p.11:** XML content-type `application/msrtc-media-relay-auth+xml`, namespace `http://schemas.microsoft.com/2006/09/sip/mrasp`. These are **selected Phase-2 preview facts**, not a complete analysis here.
- **§2.2.2.1.2 p.11:** `credentialsRequest` schema bounds 1 to **100** and requires `requestID`, `version`, `to`, `from` (with default route `loadbalanced`). **§3.1.5.1.1 p.21** distinguishes `Request Too Large` from malformed XML. Client options and all state/timeout guards require Phase 2.
- **§5.1 pp.29:** HMAC SHA-1/SHA-256, TLS secrecy of exposed credentials, and explicit requirement to authenticate/authorize clients before distributing TURN tokens. No keys or live server traffic were used.

## Document classification and closure

- **Original source existence**: VERIFIED with file size/SHA-256, pages, correct revision, product list, document change-tracking and direct normative/informative lists.
- **MS-OCSPROT six-table first-tier coverage after addition**: **46/46** *reference lists directly inspected* (some other 33 are HTML-only and lack PDF edition binding).
- **Recursive closure**: **NOT VERIFIED**. This newly verified PDF introduces **5 normative source edges** and **5 separately recorded informative links**; older Microsoft architecture and W3C/IETF draft references still require complete source acquisition, edition and dependency scrutiny.
- **Published product release changes / client build**: document's §8 tracked; client and historical release matrix as a whole still OPEN.
- **Phase 1 / Phase 2 / implementations**: Phase 1 **NOT COMPLETE** under original definition; MS-AVEDGEA Phase-2 message/state/security extraction still OPEN. No new protocol implementation, server connection, interoperable test, merge or release.
