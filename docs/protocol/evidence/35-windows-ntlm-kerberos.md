# Evidence: Windows dependencies of sign-in – MS-NLMP (NTLM), MS-KILE (Kerberos), MS-SPNG (SPNEGO)

Sources (Windows package): [MS-NLMP] "NT LAN Manager (NTLM) Authentication Protocol", 99 pp.; [MS-KILE] "Kerberos Protocol Extensions", 91 pp.; [MS-SPNG] "SPNEGO Extension", 33 pp. Use in SIP sign-in: MS-SIPAE (file 03, AE-24/25/29/30); in HTTP services: Negotiate (RFC 4559) for DLX/E911WS/OCAUTHWS (files 09, 28, 29).

## NTLM for SIP (MS-NLMP)

| ID | Requirement | Evidence |
|---|---|---|
| NL-01 | **Connectionless NTLM** (datagram): the server starts with the CHALLENGE_MESSAGE, the client answers with the AUTHENTICATE_MESSAGE (flags only here); fits SIPAE (first `gssapi-data` empty) | NLMP §1.3.1.2, p. 16; §1.7, p. 17; §3.1.5.2–3.1.5.2.1, pp. 48–49 |
| NL-02 | NTLMv2: NTOWFv2 = HMAC_MD5(MD4(UTF-16LE(password)), UTF-16LE(uppercase(user) ‖ domain)); temp = 01 01 Z(6) Time ClientChallenge Z(4) AvPairs Z(4); NTProofStr = HMAC_MD5(key, ServerChallenge ‖ temp); SessionBaseKey = HMAC_MD5(key, NTProofStr); with TargetInfo no LM response; timestamp from MsvAvTimestamp; MIC = HMAC_MD5(ExportedSessionKey, CHALLENGE ‖ AUTHENTICATE) | §3.3.2, pp. 60–61; §3.1.5.2.1, pp. 48–49 |
| NL-03 | Session keys: KXKEY (v2 = SessionBaseKey), SIGNKEY = MD5(ExportedSessionKey ‖ "session key to client-to-server signing key magic constant\0"), SEALKEY analogously ("…sealing key magic constant\0", length by the 128/56/40 flag) | §3.4.5.1–3.4.5.3, pp. 66–68 |
| NL-04 | Signature with extended session security: version 1 ‖ RC4(HMAC_MD5(SignKey, SeqNum ‖ message)[0..7]) ‖ SeqNum; without ESS: version ‖ RandomPad ‖ RC4(CRC32) ‖ RC4(0)⊕SeqNum | §3.4.4.1–3.4.4.2, pp. 64–65; structure §2.2.2.9, pp. 38–39 |
| NL-05 | **Connectionless: SeqNum from the application** (SIP: fixed **100**, file 03 AE-30) and RC4 re-keyed per message: SealingKey' = MD5(SealingKey ‖ SeqNum) | §3.4, pp. 61–62; §3.4.4, p. 63 |
| NL-06 | Channel bindings: MsvAvChannelBindings = MD5 of a `gss_channel_bindings_struct` (RFC 2744 §3.11) serialised per RFC 4121 §4.1.1.2 (integer and length fields 4 octets little-endian, zero lengths included); Z(16) = no bindings; the client SHOULD send the AV pair when the CHALLENGE has TargetInfo. For TLS: `tls-server-end-point` (RFC 5929 §4.1: hash of the server certificate with the signature algorithm's hash, MD5/SHA-1 → SHA-256; undefined for no or several hashes), application data = prefix ‖ ":" ‖ hash (RFC 5056 §2.1). [MS-SIPAE] names no channel bindings for SIP | NLMP §2.2.2.1, p. 31; §3.1.5.1.2, pp. 47–48; RFC 4121 §4.1.1.2; RFC 5929 §4.1 |

### Recomputed (own implementation of MD4/RC4)
Test vectors §4.2.4 (pp. 80–83): NTOWFv2 `0c868a40…2e3f` ✓, NTProofStr `68cd0ab8…6a1c` ✓, SessionBaseKey `8de40cca…5ca3` ✓, LMv2 `86c35097…aaaa` ✓, encrypted session key `c5dad254…d03e` ✓, SealKey `59f60097…4c58` ✓, SignKey `4788dc86…2d39` ✓, sealed data `54e50165…fb5f` ✓, checksum `70352851f2564309` ✓, signature `01000000 7fb38ec5c55d4976 00000000` ✓. Finding: the "magic constants" are hashed **including the terminating zero byte** (only then do the values match). These vectors become automated tests in M0.

## Kerberos / SPNEGO

| ID | Requirement | Evidence |
|---|---|---|
| KE-01 | GSS_GetMIC/VerifyMIC per session key type per RFC 1964/3961 (DES), RFC 4757 (RC4-HMAC), RFC 4121 (AES128/256-CTS-HMAC-SHA1-96) | KILE §3.4.5.6–3.4.5.7, pp. 70–71 |
| KE-02 | Supported encryption types/bit flags, authenticator checksum flags | KILE §1.7.2, p. 19; §2.2.7, p. 22; §3.2.5.2, p. 36 |
| KE-03 | Test vectors GSS_WrapEx AES128 and RC4, AES key generation | KILE §4.3–4.5, pp. 74–78 |
| SN-01 | SPNEGO (for HTTP Negotiate): NegTokenInit2, mechListMIC processing, NTLM RC4 key state for mechListMIC | SPNG §2.2.1, p. 14; §3.1.5.1, p. 17; §3.2.5.1, p. 21 |

## Notes / open points
- O-NL-1: Which NTLM variant (with/without ESS, with key exchange) SfB actually negotiates follows from the NegotiateFlags of the server CHALLENGE; the SIPAE examples (`response="01000000…"`) cannot be recomputed without key material. Notably, in MS-OCER/MS-CONFBAS examples the last 4 bytes of the client signatures are identical for different cnum (`42032bdb`), which fits NL-05 (fixed SeqNum 100, RC4 re-keyed per message) – inference.
- On Linux, RFC-conformant Kerberos libraries (GSS-API) can be used (decision D-03: behind an adapter crate).

## Gap
- L-18: RFC 4120/4121/3961/4757/2743/4559 were not in the Microsoft package.

## Later findings (files 38, 39)
- L-18 **closed**: RFC 4120/4121/3961/4757/2743/4559 obtained.
- O-NL-1 **documented for OCS 2007 R2** (OB-11): a capture from the SIPE test suite was recomputed with our own code. It uses ESS and KEY_EXCH. The signatures match only with SealKey per message = MD5(SealKey ‖ 100). This confirms NL-05; the identical last 4 bytes (`…64000000` = SeqNum 100) are explained.
- O-NL-1 **closed** (OB-43, normative): connectionless NTLM lets the client choose from the server's offer (MS-NLMP §2.2.1.3, p. 29) and every combination is specified; CommSCraft requires NTLMv2 + ESS and fails closed otherwise (design).

## Implementation (2026-10-31, channel bindings, issue #42)
`ntlm::channel_binding`: `tls-server-end-point` from the server certificate of the TLS connection (bounded DER walk to the signature algorithm; SHA-224/256/384/512 per RSA, ECDSA and DSA OIDs; MD5/SHA-1 → SHA-256) and the RFC 4121 MD5 (NL-06); NTLM over HTTP (MS-NTHT, web services) sends it, SIP sends Z(16). Recomputed against an independent computation (Python `hashlib`) for four signature algorithms. Inference: for RSASSA-PSS and Ed25519 certificates RFC 5929 defines no binding, so Z(16) is sent; a server that requires channel bindings will then refuse – open until an interop run shows otherwise (IT-27). Status: tested-offline.

