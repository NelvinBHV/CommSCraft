# MS-ABS rev10.1 / MS-DLX rev13 — address book snapshots, delta binary format, HTTPS distribution-list and directory search

**Research 2026-10-23. NO protocol implementation, downloaded client data or network tests.** Source PDFs from archive:
- Microsoft **[MS-ABS] Address Book File Structure rev10.1 v20250819 (2025-08-19)**, SHA-256 `e1c0e5f791bbb803d70d1de587773d02ae8e28921e98502340beb9379132b1e4`, official https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-ABS/%5BMS-ABS%5D.pdf . Original audited §§1.3–1.3.4,2.1,2.2.1–2.2.4,2.2.7–2.2.16,3.1–3.2 selected; original PDF **p18** rendered to check 12-byte compressed block header.
- Microsoft **[MS-DLX] Distribution List Expansion Protocol rev13.0 v20250520 (2025-05-20)**, SHA-256 `3c97fe84bd16c124451e1d94cd2c73fd44c20e8e9dc21aad11362ea2a99990fa`, official https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-DLX/%5BMS-DLX%5D.pdf . Original audited §§1.3,2.1,2.2.1–2.2.9,3.1.4.1–3.1.4.4 and selected §6 WSDL; PDF pp10/12 checked. Microsoft Learn: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-dlx/fb015a9b-c8bb-4d36-a9a4-fae6927ca8dc and https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abs/a87e47f6-6f1c-44b3-b7c3-1eded5f117d9 .
- Explicit IETF historical dependencies [RFC2818](https://www.rfc-editor.org/rfc/rfc2818) (HTTPS, 2000; superseded by [RFC9110](https://www.rfc-editor.org/rfc/rfc9110) HTTP semantics and [RFC9112](https://www.rfc-editor.org/rfc/rfc9112) HTTP/1.1 messaging for newer HTTP profiles), [RFC2616](https://www.rfc-editor.org/rfc/rfc2616) (HTTP/1.1 1999, superseded by later HTTP RFCs), [RFC3261](https://www.rfc-editor.org/rfc/rfc3261) (SIP 2002; provisioning endpoint discovery via SIP). **Do not automatically apply modern HTTP/profile conformance to historical Microsoft text.** P = direct first-party clause, S = source-supported synthesis, U = unverified version/role. **Downloaded ABS files are *not* SIP self-/roaming-contacts responses.**

## 1. AB server directory snapshots and transport: MS-ABS §1.3

| ID | Original | Source-exact fields and behavior |
|---|---|---|
| ABS-01 | MS-ABS §1.3 | Address Book Server produces **daily** snapshots of AD DS users, contacts and groups; client may schedule a **daily download** via **HTTP GET**, constructing a filename from date and **base URL provisioned in-band**. This is not a batch presence `SUBSCRIBE` or contact-list SERVICE. |
| ABS-02 | §1.3 | `F-XXXX.lsabs` full and `D-XXXX-YYYY.lsabs` delta files, optional `C-XXXX-YYYY.lsabs` compact deltas, and `F-XXXX.dabs`/`D-XXXX-YYYY.dabs` device-specific reduced files. XXXX/ YYYY are 4-hex-digit **0-based days since 2001-01-01 00:00 UTC**, not Gregorian calendar dates themselves. |
| ABS-03 | §1.3 | **Full** includes complete address-book objects, **delta** inserts/changes/deletes across snapshot dates; **compact delta** encodes changed attributes rather than full changed objects. `.dabs` intentionally omits some attributes and group objects, e.g. for constrained SIP phones; **do not equate device files to desktop `.lsabs`**. |
| ABS-04 | §§1.3.1–1.3.2 | ABS distribution includes server-configured `ABF_ATTRIBUTES`, normalization rules and optional pre-normalized dial strings (E.164), which influence client phone-number lookup. All files consist of 1+ **`COMPRESSED_BLOCK`** records; decompressed blocks concatenated form one logical file. |
| ABS-05 | §1.3.4, §2.2 | **Actual file record byte order is little-endian**, although spec diagrams show big-endian bit layout. All strings in structures must be **UTF-8**. Confusing diagram numbering with file endian would corrupt offsets and counts. |
| ABS-06 | §2.2.1–2.2.2 | Each block prefix `COMPRESSED_BLOCK_HEADER` is **12 bytes**: **CRC32 (4)** of original uncompressed bytes, **CompressedSize (4)** and **DecompressedSize (4)**. CompressedSize **MUST <=0x10000**, decompressed size **MUST >=compressed size**; equality permits stored uncompressed data. CRC calculation defined in **§5**, not an invented generic polynomial assumption. |
| ABS-07 | §2.2.3 | `ABF_FULL_HEADER` contains 16-byte **FileType GUID**, **CreationDate 16-bit**, **NumberOfAttributes 16-bit**, **MaximumAttributeId 16-bit**, **UseNormalizationRules 16-bit**, and **SourceStream 128 bytes** (SHOULD all zero; client does not validate). FileType full GUID literal is defined in §2.2.3. |
| ABS-08 | §2.2.4 | `ABF_DELTA_HEADER` has a **different FileType GUID**, **CreationDate** and **BaseCreationDate** (16 bits each), plus source/config info and attribute table. **A delta cannot be applied to arbitrary local base date**; source specifies base snapshot epoch. |
| ABS-09 | §2.2.7 | Every `ABF_ATTRIBUTE` stores **Length 16-bit, Id 16-bit, Flags 32-bit** and **NUL-terminated UTF-8 directory attribute name**; fields drive mapping/normalization. Unknown flags/attribute IDs require schema analysis rather than raw string rendering. |
| ABS-10 | §§2.2.8–2.2.10 | `ABF_CONTACT` has **16-byte objectGUID Id**, own length and attribute count; all-zero GUID is **sentinel** terminating contact list with **NumberOfAttributes=0 and Length=0x12**; sentinel **excluded from declared count**. This is not contact's SIP URI or MS-SIP groupID. |
| ABS-11 | §§2.2.15–2.2.16 | Compact delta path uses `ABF_CONTACTS_CHANGES` and `ABF_CONTACT_CHANGES` in place of ordinary `ABF_CONTACTS` and `ABF_CONTACT`. This is not equivalent to MS-SIP XML `contactDelta` for user's personal roster. |
| ABS-12 | §§1.3,3.1,3.2 | The original document defines **file structure and local parsing/compression**, but is not a full HTTPS server authentication/provisioning/remote-cache freshness protocol. A linked base URL from in-band provisioning is not proof TLS-certificate identity unless corresponding transport/endpoint policy verified. |

## 2. MS-DLX SOAP/WSDL on HTTPS, distinct directory request/response messages

| ID | Source | Exact operation / failure or trust scope |
|---|---|---|
| DLX-01 | §§1.3,2.1 | **SOAP/WSDL** service over **HTTPS/TCP**, negotiated SSL/TLS, address learned **through separate channel such as SIP**. No local .lsabs compressed block parsing occurs on this SOAP request. |
| DLX-02 | §3.1.4.1 | `ExpandDistributionListSoapIn` with `ExpandDistributionList` element requests **one list's membership**; after search, service sends `ExpandDistributionListSoapOut` with `ExpandDistributionListResponse` or server error code. |
| DLX-03 | §3.1.4.2 | `SearchAbEntrySoapIn` /`SearchAbEntry` gives search string, search fields and desired result attributes; server queries **users/distribution lists** and returns `SearchAbEntryResponse`. This is live server-side search, not local ABS full snapshot lookup. |
| DLX-04 | §3.1.4.3 | `SearchSkypeDirectory` endpoint asks **Skype public directory/graph** using `SkypeDirectorySearchRequest`; server MAY / SHOULD query corresponding service and returns `SearchSkypeDirectoryResponse`. This branch is **product/deployment conditional**, not proof all Windows Full Client releases can query public Skype. |
| DLX-05 | §3.1.4.4 | `ProvideSkypeSearchFeedback` posts feedback on previous public directory results; this is a fourth **WSDL operation**, not a contact roster mutation, a QoE report or IM delivery confirmation. MS-DLX §1.3 conceptual summary groups operations differently; use full WSDL operations as source of truth. |
| DLX-06 | §2.2.5.1 | `SearchResponseState` enumeration includes **Succeeded, NoEntryFound, InternalError, InvalidArgumentError, DatabaseError, CorruptionEntryError**. `NoEntryFound` is a successful network exchange but no directory match, **not** necessarily an authentication failure or proof contact removed. |
| DLX-07 | §§3.1.4.1–.4 / §2.1 | SOAP error response is carried in HTTPS response and service XML fault/status; **not a SIP 409 contact delta**, MS-OCER diagnostic code or SIP REGISTER challenge. TLS certificate validation, HTTPS user auth and server authorization are independent of authenticated SIP realm/SA. |
| DLX-08 | §§1.3,2.2,3.1.4 | Querying directory can reveal membership or personal attributes; privacy/search authorization is policy scoped. The existence of a WSDL schema does **not guarantee** all fields, public directory feature or group visibility are available for every deployment/client build. |

## 3. Source-to-client boundary diagram (S: reconstructed, no live test)

~~~mermaid
flowchart TD
 A[Registered client obtains in-band provisioning] --> B{Directory operation}
 B -->|Offline lookup| C[ABS provisioned base URL + date-named full/delta lsabs]
 C --> D[HTTP GET file and check file identity/date]
 D --> E[Decompress blocks: length bounds and original CRC]
 E --> F[Parse little endian file attributes and AD objectGUID]
 F --> G[Local address-book search/cache or reverse phone lookup]
 B -->|Live lookup/expand| H[DLX SOAP HTTPS service URL]
 H --> I{SOAP operation}
 I -->|ExpandDistributionList| J[Membership response/status]
 I -->|SearchAbEntry| K[User and distribution group search response/status]
 I -->|Public Skype directory| L[Conditional public search/feedback]
 A --> M[Separate SIP MS-SIP roaming personal contact and group roster]
 M --> N[Different deltaNum and presence subscriptions]
~~~

**State separation:** `DirectorySnapshotVersion (date/ABF GUID)`, `PersonalRosterDeltaNum`, `PresenceCategoryVersion`, `SubscriptionExpiry`, `SearchResultState` and `UserPermission` are separate. A sync failure in one does not prove all presence or login is lost.

## 4. Unresolved required work

1. **ABS-X01:** Full 79-page [MS-ABS] §2.1 flag/normalization and §5 decompression + CRC32 bit-exact algorithm still require clause-by-clause verification. No decompression code or tests were written. Need source-length overflow, malformed UTF-8 and decompression memory safeguards as requirements, not made-up profile values.
2. **ABS-X02:** HTTP vs HTTPS for ABS: §1.3 states **HTTP GET** but does not by itself require an HTTPS URL; specific in-band provisioning and deployed server config must be checked. Do not assume plaintext is safe or that server cannot supply HTTPS; certificate/auth boundary remains open.
3. **ABS-X03:** Filename four-hex-day wrap and local cache delta ordering, missing base dates, integrity/authenticated distribution and stale AD changes require explicit original clauses and versioned code policy, not guesses.
4. **DLX-X01:** Full ~70-page WSDL types, array bounds, response metadata, search filters/pagination, authentication and platform applicability not exhaustively audited; four operation names now catalogued but deep field matrices remain OPEN.
5. **DLX-X02:** Some source clauses cite **RFC2616** / **RFC2818** historic editions; modern HTTP RFCs and TLS SAN rules need deployment/historical applicability proof, not silent substitution.
6. **DLX-X03:** Local ABS search returning old user record and live DLX returning new/no result is not a specification contradiction; they have distinct freshness and authorization properties. User interface reconciliation needs evidence.
7. **DLX-X04:** Neither address-book download nor DLX HTTPS search implies personal contact `setContact` mutation or publisher's MS-PRES privacy policy, even if selected entry's SIP URI is later subscribed to.

**Research NOT COMPLETE; no protocol implementation, real server access or assistant testing, no merge or release.**
