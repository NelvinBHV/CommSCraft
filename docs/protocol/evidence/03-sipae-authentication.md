# Evidence: MS-SIPAE – SIP authentication and message signing

Source: [MS-SIPAE] "Session Initiation Protocol (SIP) Authentication Extensions", edition 20250520, rev. 11.0, 59 pp.
Dependencies: MS-NLMP (NTLM), RFC 4120 + MS-KILE (Kerberos), RFC 4121 (MIC), RFC 2246/4346/5246 (TLS), RFC 2104 (HMAC), RFC 3548 (Base16/64), RFC 3325/3323 (identity/privacy), RFC 3892 (Referred-By), RFC 4028 (session timer) – §1.2.1, §1.4, pp. 10–11.
Applies up to SfB 2021 / LTSC 2024 (§6, p. 53).

## Principle

| ID | Statement | Evidence |
|---|---|---|
| AE-01 | Two phases: (1) establish a security association (SA), (2) **sign and verify all further messages in both directions** with the SA | §3.1, p. 17 |
| AE-02 | Methods: NTLM (3 round trips), Kerberos (2), TLS-DSK (4); TLS-DSK from Lync 2010 (not OCS 2007/R2) | §3.1, pp. 17–18; <1> p. 53 |
| AE-03 | NTLM is meant for clients without DC reachability; Kerberos needs a KDC | §3.1, p. 17 |
| AE-04 | TLS-DSK uses a client certificate obtained out of band from an STS (MS-OCAUTHWS); the server SHOULD supply `sts-uri` in the challenge | §3.1, p. 17; §3.3.5.1, p. 33 |
| AE-05 | Replay protection: sliding window of 256 sequence numbers per SA | §3.1, p. 18 |
| AE-06 | Protocol versions: `version` parameter from v3; v4 = signature already in the final authentication request (crand/cnum/response) | §1.7, p. 12; §3.1, pp. 17–18; §3.2.5.1 steps 6.7–6.8, p. 26 |

## Header syntax (§2.2, pp. 13–16)

| ID | Element | Evidence |
|---|---|---|
| AE-10 | Challenge: `NTLM`/`Kerberos`/`TLS-DSK` with `realm`, `opaque`, `targetname`, `gssapi-data`, `version`, `sts-uri`; Kerberos target as `"sip/<fqdn>"` | §2.2.1, pp. 13–14 |
| AE-11 | `Authentication-Info` / new `Proxy-Authentication-Info` with `snum`, `srand` (8 hex), `rspauth`, `realm`, `targetname`, `opaque`, `version`; unlike RFC 3261 allowed in every response and in ACK/CANCEL | §2.2.2, p. 14 |
| AE-12 | `Authorization`/`Proxy-Authorization` with `qop`, `realm`, `opaque`, `version`, `targetname`, `gssapi-data`, `crand` (8 hex), `cnum`, `response`; also allowed in CANCEL | §2.2.3, p. 15 |
| AE-13 | Endpoint identifiers `epid` (From/To), `+sip.instance` (Contact), GRUU (Contact) from MS-SIPRE | §2.2.4, p. 15 |
| AE-14 | `Referred-By` with `ms-referee-uri` (client); other ms-identity parameters only server-to-server | §2.2.5, p. 16 |
| AE-15 | `p-session-on-behalf-of` (delegation) | §2.2.6, p. 16 |

## Client state model of the SA (§3.2.1, pp. 19–20)

States: **establishing → established → expired**. Data per SA: (realm, target), server version, opaque, proxy/UAS, expiry time, outgoing counter `cnum`, `SnumHigh`, window of received `snum`, GSS context (for TLS-DSK: hash function + client/server keys). Table of SAs indexed by server identity.

## Client procedures

| ID | Requirement | Strength | Evidence |
|---|---|---|---|
| AE-20 | Send the first request without Authorization, preferably REGISTER; never ACK/CANCEL; the request MUST carry an endpoint identifier (epid, +sip.instance or GRUU) | SHOULD/MUST | §3.2.3, p. 20 |
| AE-21 | On 401/407: check the `Date` header; on clock skew > Kerberos tolerance (5 min) and failure, tell the user to synchronize the time | MUST/behaviour | §3.2.5.1 step 1, p. 24 |
| AE-22 | Digest challenge ⇒ anonymous conference join (see AE-40) | SHOULD | §3.2.5.1, p. 24 |
| AE-23 | SA already established + unsigned request ⇒ resend signed; a signed request is challenged anyway ⇒ discard the SA, re-establish | MUST | §3.2.5.1 step 2, p. 24 |
| AE-24 | NTLM: GSS_Init_sec_context with flags Datagram, Identify, Integrity; the first token is **empty** (connectionless NTLM), hence `gssapi-data=""`; the answer to the CHALLENGE_MESSAGE is the AUTHENTICATE_MESSAGE | MUST | §3.2.5.1 steps 3–6, pp. 24–26 |
| AE-25 | Kerberos: get a ticket for `targetname` (sip/<fqdn>), context with Integrity, Identify; AP_REQ as `gssapi-data` | MUST | §3.2.5.1 step 5, p. 25; §3.1, p. 18 |
| AE-26 | TLS-DSK: TLS handshake tunnelled in `gssapi-data` (Base64): client_hello → (server_hello, certificate, server_key_exchange, certificate_request, server_hello_done) → (certificate, client_key_exchange, certificate_verify, change_cipher_spec, finished) → (change_cipher_spec, finished); verify the server certificate | MUST | §3.1, p. 18; §3.2.5.1 step 3, pp. 24–25 |
| AE-27 | TLS-DSK keys: PRF(master_secret, "client EAP encryption", client_random‖server_random) to 128 bytes; client signing key = bytes 64–95, server signing key = bytes 96–127 (each truncated to the hash length) | MUST | §3.2.5.1, p. 25 |
| AE-28 | Authorization header after a challenge: scheme, realm, targetname, opaque (if present), `qop="auth"`, gssapi-data, version 3 or 4 according to the server offer; for v4 additionally cnum/crand/response | MUST | §3.2.5.1 step 6, pp. 25–26 |
| AE-29 | **Signing outgoing messages**: cnum++, crand random 32 bits; buffer of `<…>` fields in exactly this order: scheme, crand, cnum, realm, targetname, Call-ID, CSeq number, CSeq method, From URI, From tag, [v≥3: To URI], To tag, [v≥3: PAI/PPI sip URI, tel URI], Expires, [response: status code]; missing fields as `<>` | MUST | §3.2.4.1, pp. 21–22 |
| AE-30 | Signature: NTLM GSS_GetMIC with a **fixed sequence number 100**; Kerberos GSS_GetMIC (RFC 2743); TLS-DSK HMAC with the client key; result lower-case Base16 | MUST/SHOULD | §3.2.4.1 steps 3–4, p. 22 |
| AE-31 | With an SA in state "establishing", defer sending; each established SA belonging to the target produces its own Authorization header | SHOULD/MUST | §3.2.4.1, p. 21 |
| AE-32 | **Verifying incoming messages**: find the SA by scheme/realm/target, otherwise drop; buffer with srand/snum analogously; check `rspauth` (NTLM with seq 100); failure ⇒ drop; snum inside the window, no repetition | MUST | §3.2.5.2, pp. 26–28 |
| AE-33 | First validly signed message ⇒ SA "established", delete other SAs with the same realm/target; 403 during setup ⇒ delete the SA, ask for other credentials | MUST | §3.2.5.2 step 7, p. 28 |
| AE-34 | Authenticated identity: from v3 from `P-Asserted-Identity`, otherwise From (request) or To (response) | MUST | §3.2.5.3, p. 28 |
| AE-35 | SA expiry: NTLM 8 h; Kerberos min(ticket expiry, 8 h); TLS-DSK min(certificate expiry, 8 h); each minus a buffer ≥ 5 min; on expiry a new SA via REGISTER without Authorization; keep the old SA up to 32 s (timer B/F) | MUST/SHOULD | §3.2.2, p. 20; §3.2.6, p. 29 |
| AE-36 | Alternative identity: `P-Preferred-Identity` (sip and/or tel), `Privacy: id` or `none` | SHOULD | §3.2.4.2, pp. 22–23 |
| AE-37 | REFER after redirection: `ms-referee-uri` in Referred-By | SHOULD | §3.2.4.4, p. 23 |
| AE-38 | Delegation: `p-session-on-behalf-of` in INVITE or 2xx | SHOULD | §3.2.4.5, p. 23 |
| AE-40 | Anonymous conference join: From `<random>@anonymous.invalid`, only to the conference GRUU; digest MD5-sess or SHA256-sess, password = conference key (from MS-CONFBAS); afterwards Authorization pre-emptively | MUST/SHOULD | §3.2.4.3, p. 23; §3.2.5.5–3.2.5.6, p. 28; example §4.5, pp. 50–51 |

## Server behaviour the client has to expect

| ID | Statement | Evidence |
|---|---|---|
| AE-50 | Server SA expires after 8 h; idle timer = Expires of the REGISTER 2xx, otherwise Session-Expires, otherwise 900 s | §3.3.2, pp. 30–31 |
| AE-51 | The server offers one challenge header per supported method; realm typically "SIP Communications Service", targetname = FQDN (Kerberos `sip/FQDN`); `Date` header always | §3.3.3, p. 31; §3.3.5.1, pp. 33–34; example §4.4, p. 47 |
| AE-52 | v4 server: without a signature in the authentication request only tolerated for REGISTER (Expires>0), INVITE to a conference GRUU or SUBSCRIBE `vnd-microsoft-provisioning-v2`; then "waiting for signature": the next server request to the client is rejected with 500 until a signed message arrives | §3.3.5.2 steps 7–8, p. 36; §3.3.4.1 step 1, p. 31 |
| AE-53 | Unauthorized AOR ⇒ 403 (signed), SA is discarded | §3.3.5.2 step 9, p. 36 |
| AE-54 | The server checks consistency epid ↔ +sip.instance ↔ GRUU | §3.3.5.2 step 1, p. 34 |

## Inconsistencies in the document (open, do not guess)

- K-AE-1: Example §4.4 step 11 (p. 50) shows for TLS-DSK `targetname="sip/server.contoso.com"` and **no** `version` parameter, although §3.3.4.1 step 5 (p. 33) requires `version` and §3.3.3 (p. 31) provides the `sip/` prefix only for Kerberos. Section 3 is normative; the example is wrong.
- K-AE-2: §3.2.5.3 (p. 28) refers to "[MS-NLMP] section 3.2.5.2" and "[MS-NLMP] section 6" for SA verification/server version – obviously MS-SIPAE §3.2.5.2 and the server version from the challenge are meant. Inference, not documented.
- K-AE-3: The length of the TLS-DSK signing keys ("truncating to the correct length") is not stated explicitly (inference: output length of the hash function, e.g. 32 bytes for SHA-256) – p. 25.
- K-AE-4 (found 2026-10-27): The version 4 example §4.3 (pp. 45–46) contradicts §3.2.4.1/§3.2.5.2: both buffers lack the `cnum`/`snum` field, the From/To URIs appear without `sip:`, the client buffer contains a To tag that the REGISTER does not have, and the server buffer lacks the status code `<200>`. The rspauth values of the Kerberos examples (`602306092`, pp. 44 and 46) have 9 hex digits and cannot be Base16-decoded. Section 3 is normative; the buffers of the version 3 examples §4.1 and §4.2 (pp. 43–44) follow it exactly and are the test vectors (`crates/sipae`).

## Later finding (file 39)
- OB-11: a real OCS 2007 R2 exchange (from the SIPE test suite) was recomputed: client and server signatures match only with a per-message sealing key MD5(SealKey ‖ 100) and sequence number 100 (confirms AE-30 and NL-05).
- Found 2026-10-27 (implementation, inference from §3.2.4.1 step 2 / §3.2.5.2 step 2): the signature buffer covers Call-ID, CSeq, From/To URI and tag, P-Asserted-/P-Preferred-Identity, Expires and the status code, but **not** Contact (incl. `gruu` and `expires` parameters), Via, Content-Type or the body. Their integrity rests on TLS alone. The client therefore takes the registration expiry from the signed Expires header (`crates/register`, test `expiry_comes_from_the_signed_header`).
- Correction (2026-10-27): §3.2.5.1 step 6.8 (p. 26) – a client that implements version 4 and gets a challenge with version ≥ 4 MUST answer `version=4` and already sign the request that carries the final authentication token (`cnum`, `crand`, `response` per §3.2.4.1 steps 1–3). AE-52 describes what a server tolerates from clients that do not do this; it is not a licence for the client. Implemented in `crates/sipae` (`handshake_signature`) and `crates/register`; versions above 4 are answered with 4.
