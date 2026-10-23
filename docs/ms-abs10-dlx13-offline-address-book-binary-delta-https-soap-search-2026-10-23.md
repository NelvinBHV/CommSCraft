# MS-ABS rev10.1 + MS-DLX rev13 — offline ABF binary index vs HTTPS SOAP GAL expansion

**Source-only research tranche 2026-10-23; NOT a completed full-client implementation specification.** Microsoft first-party original PDFs (bytes and hashes independently validated against archived source manifest):
- **[MS-ABS] Address Book File Structure 10.1**, v20250819, **2025-08-19**, SHA-256 `e1c0e5f791bbb803d70d1de587773d02ae8e28921e98502340beb9379132b1e4`, https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-ABS/%5BMS-ABS%5D.pdf — §§1.3,2.2.1–2.2.16,3.1,§§5–6, §8 (selected). Original PDF pp9,18,19 rendered and inspected, page 79 total.
- **[MS-DLX] Distribution List Expansion Protocol 13.0**, v20250520, **2025-05-20**, SHA-256 `3c97fe84bd16c124451e1d94cd2c73fd44c20e8e9dc21aad11362ea2a99990fa`, https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-DLX/%5BMS-DLX%5D.pdf — §§1.3,2.1,2.2.5.1,3.1.4.1–3.1.4.3,3.1.5 plus selected §6 WSDL; PDF pp10,12,13 rendered and inspected, page 70 total.
- IETF reference edition: [RFC2818](https://www.rfc-editor.org/rfc/rfc2818) 2000 HTTPS; [RFC2616](https://www.rfc-editor.org/rfc/rfc2616) historical HTTP/1.1; Microsoft [MS-SIPREGE] provisioning of HTTP endpoints, [MS-OCSPROT] 9.0 §§2.5.7–2.5.8, [MS-SIP] 8.0 roaming contacts, [MS-PRES] 13.0. Later HTTP/TLS RFCs do **not silently replace** a dated 2025 specification. Local provisioning and URLs are deployment data; no network calls made.

**P** direct source requirement, **S** inter-spec interpretation, **U** optional/version-dependent, unresolved or incomplete section.

## 1. Binary address book snapshot, delta, compact delta and integrity

| ID | Section | Source rule |
|---|---|---|
| DIR-01 | MS-ABS §1.3 | **Server produces daily AD DS-derived files**; client can locally search user/contact/group and reverse-lookup phone. Client fetches **file by HTTP GET using base URL obtained via in-band provisioning**; not a SIP SUBSCRIBE to its own contact group list. |
| DIR-02 | §1.3 | **`F-XXXX.lsabs`** full, **`D-XXXX-YYYY.lsabs`** delta, **`C-XXXX-YYYY.lsabs`** compact delta, **`F-XXXX.dabs` / `D-XXXX-YYYY.dabs`** reduced device format. XXXX/ YYYY are **4-hex-digit days since 2001-01-01T00:00:00Z (zero-based)**. Delta references base YYYY/XXXX and cannot be applied blindly to unrelated snapshot. |
| DIR-03 | §1.3 | **`.lsabs`** contains full end-user-appropriate attributes, including user/contact/group objects; **`.dabs`** is reduced memory SIP phone profile and **omits group objects**. The classic Windows Full Client must not be assumed to use `.dabs` merely because it is valid MS-ABS. |
| DIR-04 | §§2.2.1–2.2.2 | A **COMPRESSED_BLOCK_HEADER** is 12 bytes: **CRC 4, CompressedSize 4, DecompressedSize 4**. `CompressedSize` **MUST <=0x10000**; `DecompressedSize MUST >= CompressedSize`; equality indicates block is not compressed. Validate CRC against **original uncompressed data**, algorithm in §5, not compressed bytes. |
| DIR-05 | §2.2.3 | `ABF_FULL_HEADER`: **FileType 16-byte GUID** `76 6c e1 44 fd 0a a9 40 8b 63 5f e9 b0 81 73 8f`; **CreationDate** 2 bytes, **NumberOfAttributes** 2 bytes, **MaximumAttributeId** 2 bytes, **UseNormalizationRules** 2 bytes and **SourceStream 128 bytes** (SHOULD zero; client does not validate). Fixed GUID distinguishes full vs delta even if filename wrong. |
| DIR-06 | §2.2.4 | `ABF_DELTA_HEADER`: **FileType** GUID for full delta `16 c1 4b b5 08 90 c7 47 b9 bd f3 bb 1a 0a b6 eb`, for compact delta `34 17 7d f7 87 ae 2b 4d 09 a0 8e e9 ba 89 4a 04`. **BaseCreationDate 2 bytes and CreationDate 2 bytes** are base/target XXXX/YYYY in filename; do not confuse them with local system clock or HTTP Last-Modified. |
| DIR-07 | §§2.2.5–2.2.11 | ABF normalization, attribute table, contacts, deleted contacts, per-contact attributes are **binary structures**, not the roaming-contact XML schema. Attribute IDs, table size and contact IDs must be validated before building searchable indexes; generic address-book JSON is not normative parser. |
| DIR-08 | §§2.2.15–2.2.16 | **Compact delta** uses **ABF_CONTACTS_CHANGES / ABF_CONTACT_CHANGES** with changed and deleted attributes instead of unchanged whole contact. The source explicitly product-versions compact delta support via §8 product notes. |
| DIR-09 | §§2.2.2–2.2.4,§5 | A syntactically named `D-XXXX-YYYY` is **not sufficient evidence** it applies to locally cached F-XXXX; must validate base header, decompression lengths/CRC and full/delta/compact flags, and only then update cache atomically (last step is S safety requirement, not source MUST in sections cited). |
| DIR-10 | §8 notes <6>,<9>,<10> | **OCS 2007 original client** lacks later compact delta support; OCS 2007 R2 historical KB hotfix gates exist. Applicability must be attached to **target Windows SfB build**, not asserted universally across all product generations. |

## 2. On-demand server GAL / distribution list SOAP vs local ABS

| ID | Source section | Source-bound operation |
|---|---|---|
| DIR-11 | MS-DLX §§1.3,2.1 | DL expansion and global address book search are **HTTPS SOAP/WSDL request → response**, and protocol **MUST use HTTPS**. Endpoint address obtained out-of-band, e.g. SIP provisioning; **SIP registration alone does not authenticate a SOAP caller**. |
| DIR-12 | §3.1.4.1 | **`ExpandDistributionListSoapIn`** contains **ExpandDistributionList** element; server SHOULD query distribution list repository; **`ExpandDistributionListSoapOut`** contains ExpandDistributionListResponse with member details (ActiveDirectoryObjectInfo). The server MUST return a response with status/error information, even if HTTP transport succeeds. |
| DIR-13 | §3.1.4.2 | **`SearchAbEntrySoapIn`** includes **SearchAbEntry** SOAP element; server SHOULD query user and distribution list repository; **`SearchAbEntrySoapOut`** contains SearchAbEntryResponse with response state and results. HTTP 2xx is a transport response and **not** a guarantee search matches. |
| DIR-14 | §§1.3,3.1.4.3 | **`SearchSkypeDirectorySoapIn/Out`** is a separately defined public-directory search Web-service operation. Source includes service feedback as conceptual function, but this historical service must **not be assumed available in every on-prem Skype for Business installation or today**. |
| DIR-15 | §2.2.5.1 | SearchResponseState values: **`Succeeded`, `NoEntryFound`, `InternalError`, `InvalidArgumentError`, `DatabaseError`, `CorruptionEntryError`**. Server MUST use Succeeded for successful matches; NoEntryFound if none; InvalidArgumentError for invalid args; SHOULD use others as appropriate. Do not map HTTP status 200 to search success. |
| DIR-16 | §§2.1,3.1.4.1–3.1.4.3 | Client MUST negotiate secure transport as described in this edition and compose WSDL body in correct namespace. Server identity/trust, user authorization and search limits are **separate gates** from REGISTER, contact list subscription and local ABF CRC. |
| DIR-17 | §3.1.5 | MS-DLX specifies **no timer events** for its protocol server role; cannot invent a background 30-second address book query or retries. HTTP stack/proxy/application timeouts are different specifications. |

## 3. Full client search/contact subsystems: 3 independent data planes (S)

~~~mermaid
flowchart TD
 A[REGISTER and in-band provisioning] --> B[Discover ABS base file URL]
 A --> C[Discover DLX HTTPS service URL]
 A --> D[MS-SIP roaming contact SUBSCRIBE]
 B --> E[HTTP GET full F-XXXX.lsabs]
 E --> F[Validate GUID, binary structures and uncompressed CRC]
 F --> G[Locally indexed GAL/reverse caller ID cache]
 G --> H[Conditional delta D-XXXX-YYYY or compact C-XXXX-YYYY]
 H --> F
 C --> I[HTTPS SOAP SearchAbEntry or ExpandDistributionList]
 I --> J{SOAP ResponseState}
 J -->|Succeeded| K[Server-backed search/expanded membership]
 J -->|NoEntryFound| L[No matching entries]
 J -->|Other error| M[Service error; no local ABF mutation implied]
 D --> N[XML contactList deltaNum snapshot]
 N --> O[User favourites/groups and subscription preference]
 G --> P[Client UI directory view]
 K --> P
 O --> P
~~~

**Key boundary:** `contactList` is **user's persisted favorites/groups**; ABS file is a **local directory snapshot**; DLX is **online AD search/expansion**. All can affect the same UI contact card but they **do not share one version number**, transaction, HTTP session or authentication channel. ABS may be stale while roster contact list stays current; DLX may fail while full ABS cache remains useful. This does not license automatic cross-source overwriting.

## 4. Unresolved full-source/edition issues

- **DIR-X01:** MS-ABS all **§2.2 binary structs**, compression algorithm **§5** and hash/index algorithm §6 must be fully field-by-field audited (endianness, CRC polynomial, normalization regex, guid byte encoding and overflow bounds). Current table contains **selected fields only**.
- **DIR-X02:** MS-ABS §1.3 describes **HTTP GET**, not universally HTTPS; absolute TLS requirement cannot be invented for the file-transfer protocol from MS-DLX's HTTPS rule. But modern secure deployment policy/client transport protection and integrity validation must be separately sourced.
- **DIR-X03:** Date-to-4hex mapping and rollover, multiple-day delta chains, missing/corrupt delta, caching strategy, retry/backoff and fallback to new full book require source clauses/product applicability. No downloaded real ABS files tested.
- **DIR-X04:** Full WSDL/XSD request/response definitions for DLX, expansion nesting and server search ordering/limits (WSDL §6) have not been exhaustively audited. `SearchSkypeDirectory` deployment and availability remains U, rather than blanket implementation requirement.
- **DIR-X05:** MS-ABS 2025 and MS-DLX 2025 are not proofs client will offer every historical feature; MS-SIP roaming contacts **2017** and MS-PRES **2025** must be version-guarded against target Windows full client/server topology.
- **DIR-X06:** Source GL directory identity (SIP URI, display name, phone) and privacy containers/ACL metadata must not be mistaken for authorization to obtain restricted directory attributes; cross-forest/gateway and client cache retention/security remain open.

**Status: selected source clauses verified; entire Windows client research remains NOT COMPLETE. No protocol code, live-server testing or manual interop, main merge or release.**
