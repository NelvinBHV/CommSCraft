# Evidence: MS-SIPCOMP – SIP compression (optional)

Source: [MS-SIPCOMP] "Session Initiation Protocol (SIP) Compression Protocol", edition 20250520, 24 pp. Applies up to SfB LTSC 2024 (§6, p. 21). Dependency: RFC 2118 (MPPC).

**Optional** for a client (OCSPROT §2.5.1 step 4, p. 28: "can request compression … if enabled based on group policy"). Without compression the connection stays uncompressed.

| ID | Requirement | Strength | Evidence |
|---|---|---|---|
| CP-01 | Only over TLS; NEGOTIATE is the **first** SIP packet after the TLS handshake | MUST | §1.5, p. 8; §2.1, p. 9; §3.1.4.1, p. 11 |
| CP-02 | `NEGOTIATE sip:<IP of the proxy>:<port>` with CSeq, Via, Call-ID, From, To, `Compression: LZ77-8K`, `Max-Forwards: 0`, no body | MUST/SHOULD | §2.2.1, p. 9; §3.1, p. 11; example §4.1, p. 19 |
| CP-03 | Timer F = 5 s instead of 32 s; expiry or non-200 ⇒ continue uncompressed; 200 without `LZ77-8K` ⇒ close TCP | SHOULD/MUST | §3.1, p. 11; §3.1.5.3, p. 12 |
| CP-04 | The client compresses only **after** the server has sent its first compressed packet | MUST NOT/SHOULD | §3.2.5, p. 13 |
| CP-05 | Every data packet with a 6-byte header: 4 bits flags (0x8 FLUSHED, 0x4 AT_FRONT, 0x2 COMPRESSED; 0x1 never), 4 bits type = 0, 3 bytes reserved = 0, 2 bytes uncompressed size | MUST | §2.2.4, p. 10 |
| CP-06 | Per direction its own 8 KiB history buffer + HistoryOffset (start 0); if data does not fit at the end ⇒ offset 0 and AT_FRONT; expansion ⇒ uncompressed with FLUSHED | MUST | §3.2, p. 13; §3.2.5.1.1, p. 15 |
| CP-07 | MPPC coding: offset <64 `1111`+6 bits, 64–319 `1110`+8 bits, 320–8191 `110`+13 bits; length 3 = `0`, 4–7 `10`+2 bits, 8–15 `110`+3 bits … 4096–8191 `111111111110`+12 bits; literals per RFC 2118 §4.1 | MUST | §3.2.5.1, pp. 14–15 |
| CP-08 | Decompression: FLUSHED ⇒ clear the buffer; AT_FRONT ⇒ offset 0; FLUSHED+COMPRESSED ⇒ close TCP | MUST | §3.2.5.2, pp. 16–17 |

Consistency check: the example tuple <16,15> → `1111010000` (offset 16, class <64) + `110111` (length 15, class 8–15) = `1111010000110111` – matches the table (p. 15).

## Open
- O-CP-1: The byte order of the 16-bit "Uncompressed size" field and the bit position of flags/type in the first byte are not stated explicitly in the text (bit diagram only, p. 10). Interop test required.
- O-CP-2: Literal coding is adopted only by reference to RFC 2118 §4.1.

## Later findings (file 39)
- O-CP-2 **closed** (OB-01, RFC 2118 §4.1: literal < 0x80 as 8 bits, ≥ 0x80 as `10` + low 7 bits).
- O-CP-1 **partly** (OB-02): flags in the high nibble is an inference from the diagram convention; the byte order of the length stays open. No public implementation or archived older edition found (OB-30).
