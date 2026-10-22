# MS-SIPAE 11.0: independently checked client signing and anonymous Digest

**Date:** 2026-10-22. **Evidence grade:** direct full published PDF review for the clauses listed, not a complete MS-SIPAE or Windows Full Client audit. Only research; no implementation or server tests.

## Version and source provenance

Official: [MS-SIPAE Session Initiation Protocol Authentication Extensions](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd), **revision 11.0, released May 20, 2025**. The user-supplied original file is `catalog/MS-SIPAE.pdf` inside `catalog.zip`; independently inspected matching `catalog/MS-SIPAE.txt`. PDF source digest according to the corpus inventory: `6f0337a593233162401f0265e5425ad38d839174d1f78b8031405c5c7de64836`, independently hash-verified at corpus intake. The full PDF itself is not committed; this analysis cites its normative numbered sections, pages, version and official link.

## 3.2.4.1 Sending Messages to the SIP Server — directly checked against 2025 PDF, pages 21–22/59

| Rule ID | Condition and normative result | Source |
|---|---|---|
| AE11-C01 | Client MUST check for established SAs linked to server/proxy before sending; if any linked SA remains `establishing`, SHOULD defer sending. Once all linked SAs are established, MUST create an authorization header **for each applicable SA**. | §3.2.4.1 preamble |
| AE11-C02 | Per-SA outgoing `cnum` increments **each time this procedure runs** and is encoded decimal; client generates fresh 32-bit `crand`, represented as eight hexadecimal digits. | §3.2.4.1 step 1 |
| AE11-C03 | Construct UTF-8 signed buffer with exact case and syntactic spelling of header values; ordered angle-bracket fields are scheme, crand, cnum, realm, targetname, Call-ID, CSeq number, CSeq method, From URI, From tag, optional To URI if version >=3, To tag, optional asserted/preferred SIP identity if version >=3, optional asserted/preferred tel identity if version >=3, Expires, and response status code only for a response. | §3.2.4.1 step 2 |
| AE11-C04 | For applicable but missing fields append empty angle brackets; for fields gated by version or request/response condition that **does not apply**, append nothing; if condition applies but field absent, append empty brackets. | §3.2.4.1 step 2 |
| AE11-C05 | NTLM: MS-NLMP §3.1.4 GSS_GetMIC; Kerberos: RFC 2743 §2.3.1 GSS_GetMIC. For NTLM SSPI a fixed message sequence number **100** is provided. TLS-DSK: RFC 2104 HMAC over the buffer, using negotiated hash and **client authentication key**, only after the final TLS-DSK server handshake message. | §3.2.4.1 step 3 |
| AE11-C06 | Encode signature token using Base16 (RFC 3548 §6). Replacing A–F with lowercase **SHOULD**, not MUST. | §3.2.4.1 step 4 |
| AE11-C07 | Established SA challenge from WWW-Authenticate → Authorization; challenge from Proxy-Authenticate → Proxy-Authorization. Include scheme, copied realm/targetname/opaque, qop=`auth`, generated cnum/crand and encoded response. | §3.2.4.1 step 5 |

### Exact signed-value position ledger

1 scheme; 2 crand; 3 cnum; 4 realm; 5 targetname; 6 Call-ID; 7 CSeq number; 8 CSeq method; 9 From URI; 10 From tag; 11 To URI (server protocol version >=3); 12 To tag; 13 P-Asserted-Identity/P-Preferred-Identity SIP URI (>=3); 14 same tel URI (>=3); 15 Expires; 16 response code (only responses).

The **position numbers are positional slots**, not always sixteen serialized entries. Version-dependent and request-vs-response optional slots are not serialized when conditions are false. The source explicitly requires syntax and case be preserved; general SIP header case-insensitivity does not authorize normalization of authenticated values.

## 3.2.5.5 and 3.2.5.6 — directly checked against 2025 PDF, page 28/59

§3.2.5.5 specifies the **anonymous conference** challenge case: a 401 Digest challenge whose client From URI has `<username>@anonymous.invalid` and whose destination is a conference GRUU defined by MS-SIPRE. Such client SHOULD check `algorithm` is MD5-sess or SHA256-sess and SHOULD use the **conference-key** as Digest password. RFC 3261 §22.4 and NIST FIPS 180-4 are referenced, while the conference-key is obtained per MS-CONFBAS (example §4.1). This is **not** blanket evidence that ordinary client REGISTER authentication follows this Digest flow.

§3.2.5.6 says a client with an established anonymous SA SHOULD construct Authorization preemptively on further messages to reduce challenges. This is a specific anonymous session continuation, not necessarily the general signed REGISTER policy.

## Bidirectional integrity linkage

- **Client → server:** §3.2.4.1 generation above, then server receives under §3.3.5.3; `cnum/crand/response`, client directional key.
- **Server → client:** §3.2.5.2 uses `snum/srand/rspauth`, server directional key, own replay window.
- **Server verification vs client signing:** do not substitute §3.3.5.3 alone when the **actual client signing clause is now directly available**. Check whether MS-SIPAE defines identical field treatment for response-derived client messages before assuming equivalence.
- **Multiple SAs:** §3.2.4.1 requires **each established SA** linked to relevant server/proxy to supply an authorization header. An architecture assuming a single global SA per transport is not supported by this clause.
- **Lifetime:** signature counters and replay states are SA-specific, not merely tied to a SIP transaction; transaction identity and registration-binding lifetime remain separate.

## Amendment to previously unresolved analysis

The earlier research files left §3.2.4.1 and §3.2.5.5 marked `not retrieved`. **Their full normative texts HAVE NOW BEEN READ from the owner's original 2025 PDF**; those earlier gaps are closed for these two sections. This does not close the complete MS-SIPAE analysis, cross-referenced MS-NLMP/MS-KILE requirements or version/applicability review.

## Still-open analysis

Confirm all 2025 MS-SIPAE sections and product-specific notes, especially §§2.2.3, 3.2.5.1, 3.2.5.2, 3.3.5.1–3.3.5.3; independently cross-check MS-SIPREGE revision 17.1 full REGISTER/provisioning/redirect flows, MS-CONFBAS conference-key definition, original MS-NLMP and RFC 2743 per-message APIs, and RFC 3548 historic encoding vs successor RFC 4648 applicability. Full Windows Full Client feature parity and document-reference closure remain UNFINISHED.

No generated test vector, interoperability claim, code modification, real server connection, merge or release.
