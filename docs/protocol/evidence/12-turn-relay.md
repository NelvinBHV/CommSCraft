# Evidence: MS-TURN – Microsoft TURN extensions (media relay through the A/V Edge)

Source: [MS-TURN] "Traversal Using Relay NAT (TURN) Extensions", edition 20250520, 61 pp. Basis: **draft-rosenberg-midcom-turn-08** ([IETFDRAFT-TURN-08]) plus parts of turn-01/-05 and **draft-ietf-behave-rfc3489bis-02 (STUN-02)** – §1.2, §1.3, pp. 10–13. The drafts were obtained from the IETF archive (file 38, IE-01/IE-02).

Important: this is **not** RFC 5766 TURN. Magic cookie as an attribute, own message types, own framing.

## Transport

| ID | Requirement | Evidence |
|---|---|---|
| TU-01 | UDP (default 3478 per AVEDGEA) or TCP 443; IPv6 only from Lync 2013 | §2.1, p. 16; AVEDGEA §1.5 p. 9; <1> p. 54 |
| TU-02 | TCP variant: first **pseudo-TLS**: ClientHello record with fixed bytes (0x16 0x03 0x01 0x00 0x2D, HS type 0x01, length 0x000029, version 0x0301, 4 bytes time, 28 bytes random, session ID length 0, cipher list 0x0002/0x0018, compression 0x01/0x00); the server answers ServerHello+ServerHelloDone in one record (length 0x4E); only then TURN | §2.1.1, pp. 16–19 |
| TU-03 | Alternatively real TLS or HTTPS encapsulation on 443 (MAY) | §2.1.2–2.1.3, p. 19 |
| TU-04 | TCP framing per message: 1 byte type (0x02 TURN control, 0x03 data), 1 byte 0, 2 bytes length; not for pseudo-TLS | §2.1.4, p. 19 |
| TU-05 | UDP without framing | §2.1.5, p. 19 |

## Message format

| ID | Element | Evidence |
|---|---|---|
| TU-10 | 20-byte header (type 16 bits, top 2 bits 0; length; **128-bit transaction ID**), followed by TLV attributes; **magic cookie 0x72c64bc6 as the first attribute** (type 0x000F); MESSAGE-INTEGRITY always the last attribute | §2.2.1–2.2.2, pp. 20–22; §2.2.2.8, p. 27; §3.1.8, §3.1.10, pp. 38–39 |
| TU-11 | Types: 0x0003 Allocate, 0x0103 Allocate response, 0x0113 Allocate error, 0x0004 Send, 0x0115 Data Indication, 0x0006/0x0106/0x0116 Set Active Destination; no Send response, no Shared Secret | §2.2.1, pp. 20–21 |
| TU-12 | Mandatory attributes 0x0001 Mapped-Address, 0x0006 Username, 0x0008 Message-Integrity, 0x0009 Error-Code, 0x000A Unknown-Attributes, 0x000D Lifetime, 0x000E Alternate-Server, 0x000F Magic-Cookie, 0x0010 Bandwidth, 0x0011 Destination-Address, 0x0012 Remote-Address, 0x0013 Data, 0x0014 Nonce, 0x0015 Realm, 0x0017 Requested-Address-Family | §2.2.2, pp. 21–22 |
| TU-13 | Optional attributes 0x8008 MS-Version, 0x8020 XOR-Mapped-Address (XOR with the transaction ID: port with the top 16 bits, IPv4 with the top 32 bits, IPv6 with all 128 bits), 0x8032 MS-Alternate-Host-Name, 0x8037 APP-ID, 0x8039 SECURE-TAG, 0x8050 MS-Sequence-Number (20-byte connection ID + 32-bit counter), 0x8055 MS-Service-Quality (stream 1 audio/2 video/3 supplemental video/4 data; 0 best effort/1 reliable), 0x8090 MS-Alternate-Mapped-Address (IPv6), 0x8095 Multiplexed-Session-ID | §2.2.2.16–2.2.2.24, pp. 31–37 |
| TU-14 | **MESSAGE-INTEGRITY**: text = message up to before MI, padded with zeros to a multiple of 64 bytes. MS-Version <3 ⇒ HMAC-SHA1 with key = MD5(user ":" realm ":" password); both ≥3 ⇒ HMAC-SHA256 with K = HMAC-SHA256(Nonce, password), key = HMAC-SHA256(K, 0x01‖"TURN"‖0x00‖Username‖Realm‖0x00000100) | §2.2.2.3, pp. 23–25 |
| TU-15 | MS-Version: 1 = MS-ICE, 2 = MS-ICE2, 3 = +SHA-256, 4 = +IPv6, 5/6 = multiplexed TURN (UDP / UDP+TCP) | §2.2.2.17, pp. 32–33 |

## Client flow

| ID | Requirement | Evidence |
|---|---|---|
| TU-20 | Credentials (long-term username/password) from MS-AVEDGEA | §1.5, p. 14 |
| TU-21 | Allocate 1: without MESSAGE-INTEGRITY, with MS-Version and MS-Service-Quality ⇒ 401 with Nonce/Realm (+ Alternate-Server = direct server address; with UDP the next Allocate goes there) | §3.1.12, p. 39; §3.2.4.1, p. 40; §2.2.2.7, p. 26 |
| TU-22 | Allocate 2 (after 401/431/432/434/435/438): Username, Realm, Nonce (from the error response), MS-Version, MS-Service-Quality, Requested-Address-Family if needed (missing ⇒ IPv4+IPv6), MESSAGE-INTEGRITY | §3.2.5.2, pp. 42–43 |
| TU-23 | Allocate response: check authenticity; XOR-Mapped-Address (server-reflexive address), Mapped-Address (relay address IPv4 or IPv6), MS-Alternate-Mapped-Address (IPv6 relay), MS-Sequence-Number (afterwards echo connection ID + counter++ in **all** authenticated requests) | §3.2.5.1, pp. 41–42; §2.2.2.21, pp. 34–35 |
| TU-24 | Data to a peer: a Send request (Destination-Address + Data, MS-Sequence-Number, MI) sets the permission; answers arrive as Data Indication (Remote-Address + Data) | §1.3, p. 12; §3.2.4.2, p. 40; §3.2.5.5, p. 44 |
| TU-25 | Set Active Destination ⇒ afterwards raw data without TURN encapsulation (TCP: framing type 0x03) | §3.2.4.3, pp. 40–41; §3.2.4.5, §3.2.5.6, pp. 41, 44 |
| TU-26 | Teardown: Allocate with Lifetime 0 (authenticated, with sequence number) | §2.2.2.6, p. 26; §3.2.4.4, p. 41 |
| TU-27 | Retransmit Allocate/Set Active Destination every 650 ms, at most 9 times | §3.2, p. 39; §3.2.5.6, p. 44 |
| TU-28 | Ignore malformed messages; with TCP close the connection | §3.1.10, pp. 38–39 |
| TU-29 | Multiplexed TURN (channel 0xFF10) and APP-ID/SECURE-TAG: according to product behaviour **not supported by any SfB client version** (Windows components only) – not required for an SfB-compatible client | <19>, <20>, <23>–<25>, <30>, <33> pp. 55–56 |

## Gaps
- L-01 (**closed**, file 38 IE-01/IE-02): draft-rosenberg-midcom-turn-08, draft-ietf-behave-turn-01, -turn-05, draft-ietf-behave-rfc3489bis-02 obtained from the IETF archive.
- O-TU-1: According to the Wireshark dissector an attribute **0x8006** appears in MS-TURN captures that is not described in the specifications (OB-38, secondary). Meaning unknown; the client must be able to ignore it as an unknown optional attribute (≥ 0x8000). Interop capture needed.

## Later findings (files 38, 39)
- Comparison with turn-08 confirms message types, attributes and MAGIC-COOKIE (IE-01); error codes 434–440 from turn-08 §9.2.10.
- MS-TURN specifics (no padding, NONCE/REALM swapped) confirmed by the Wireshark dissector (OB-38).
