# Evidence: MS-ICE2 and MS-ICE – connectivity establishment for media

Sources:
- [MS-ICE2] "Interactive Connectivity Establishment (ICE) Extensions 2.0", edition 20250520, 42 pp. – based on **draft-ietf-mmusic-ice-19**, ice-tcp-07, STUN-02, RFC 5389, RFC 4571, RFC 5761, RFC 5766 (§1.2.1, p. 10). From Lync 2010 (OCSPROT <10>/<11>).
- [MS-ICE] "Interactive Connectivity Establishment (ICE) Extensions", edition 20250520, 31 pp. – based on **draft-ietf-mmusic-ice-06**, draft-tcpcice-00, STUN-02. For OCS 2007 peers; SfB still offers it in the multipart SDP (MS-SIPRE §3.15).
The underlying IETF drafts were obtained from the IETF archive (file 38).

## MS-ICE2 (main procedure)

| ID | Requirement | Evidence |
|---|---|---|
| IC-01 | Full ICE implementation, **regular nomination** of the peer only; a lite peer is treated like non-ICE; **no ICE restart**; independent per media stream; exactly 2 components (RTP=1, RTCP=2), not necessarily adjacent ports; over TCP RTP/RTCP multiplexed on the same port; RTCP mux over UDP per RFC 5761 possible | §1.6, p. 15 |
| IC-02 | At most 1500 bytes per message | §2.1, p. 17 |
| IC-03 | STUN format STUN-02 and (from SfB 2016) RFC 5389, chosen by the peer's IMPLEMENTATION-VERSION (≥3 ⇒ RFC 5389; missing ⇒ RFC 5389; <3 ⇒ STUN-02); until detected send both formats in parallel; TCP framing RFC 4571; XOR-MAPPED-ADDRESS type **0x0020** | §2.2.2, p. 17; §3.1.5.2, p. 28; <2> p. 39 |
| IC-04 | Extra attributes: CANDIDATE-IDENTIFIER 0x8054 (foundation of the local candidate, NUL-padded to 4 bytes, requests only); IMPLEMENTATION-VERSION 0x8070 (in all requests/responses; Lync 2010/2013 = 2); APP-ID/SECURE-TAG not for SfB ≤2016 | §2.2.2.1–2.2.2.4, pp. 17–19; <4>–<6> p. 39 |
| IC-05 | Keep-alive = STUN Binding Request with MESSAGE-INTEGRITY only (0 or valid), no other attributes; ≤19 s without media; only the RTP component of the active pair | §2.2.3, p. 19; §3.1, p. 20; §3.1.6.3, pp. 30–31 |
| IC-06 | Timers: gathering 10 s; check phase max. 10 s, after the first request+response of the peer 5 s; USE-CANDIDATE 10 s; consent freshness 30 s (requests every 5 s, RFC 5389 format, from SfB 2016) | §3.1, p. 20; §3.1.6.2–3.1.6.5, pp. 30–31; <18> p. 39 |
| IC-07 | Flow: initial offer (default candidate UDP; TCP-only mode: default TCP) → (provisional) answer → checks → **final offer** of the controlling agent (caller) only with the chosen local+remote candidate (even if = default) → answer; then TURN Set Active Destination and release the other candidates | §3.1.4.1–3.1.4.7, pp. 21–22 |
| IC-08 | At most 20 provisional answers (forking) | §3.1.4.3, p. 21 |
| IC-09 | Peer without ICE/lite: only Binding Requests from the default candidate to open the paths, media over the default | §3.1.4.4.1, p. 22 |
| IC-10 | Candidate types exactly: UDP host/srflx/relay, TCP host active/passive, TCP srflx active, TCP relay active/passive; no simultaneous-open; no null/multicast/broadcast/link-local; ports ≥1024; IPv4 before IPv6; **max. 40 candidates** (Lync 2010: 20), **max. 80 pairs** | §3.1.4.8.1–3.1.4.8.1.1, p. 23; §3.1.4.8.2.1, p. 27; <9>, <14> p. 39 |
| IC-11 | srflx/relay only for the best interface towards the TURN server; TCP-only mode: active+passive host per interface; regular mode without TCP TURN: an artificial active TCP srflx candidate with the IP of a UDP srflx or host | §3.1.4.8.1.2–3.1.4.8.1.3, pp. 23–25 |
| IC-12 | Foundation/priority per ice-19 §4.1.1.3/§4.1.2 or ice-tcp-07 §3.2; pair priority/check list/states per ice-19 §5.7 | §3.1.4.8.1.4, p. 25; §3.1.4.8.2.2–3.1.4.8.2.3, p. 27 |
| IC-13 | Binding Request: USERNAME (ice-19 §7.1.1, new format) + MESSAGE-INTEGRITY (short-term, but **computed with the old STUN format**) + FINGERPRINT + CANDIDATE-IDENTIFIER | §3.1.4.8.2.4, p. 27 |
| IC-14 | Additionally a duplicate per check with FINGERPRINT over a printed CRC table; receiving: first the normal fingerprint check, then the table; stop duplicates as soon as the peer sends IMPLEMENTATION-VERSION | §3.1.4.8.2, pp. 25–26 |
| IC-15 | Checking incoming requests: drop without USERNAME/FINGERPRINT; USERNAME "localUfrag:…"; without MI ⇒ 401; wrong MI ⇒ 431; response only XOR-MAPPED-ADDRESS, USERNAME, MESSAGE-INTEGRITY, IMPLEMENTATION-VERSION | §3.1.5.2.2–3.1.5.2.3, pp. 28–29 |
| IC-16 | Drop responses without FINGERPRINT/MI/XOR-MAPPED or with 0.0.0.0/broadcast/multicast; errors 401/430/431/432/500 ⇒ retry, otherwise pair "Failed" | §3.1.5.3.1–3.1.5.3.3, pp. 29–30 |
| IC-17 | End: no valid pair ⇒ the call fails; controlling ⇒ regular nomination on the best pair; media first over the default, then the first successful pair, finally over the chosen one | §3.1.4.8.2.6, pp. 27–28; §3.1.4.8.3, p. 28 |
| IC-18 | TURN: MS-TURN; additionally RFC 5766 TURN only on Windows 10+ | §3.1.5.1, p. 28; <15> p. 39 |

### Checked (CRC table, IC-14)
Machine comparison of the 256 entries (pp. 25–26) with the standard CRC-32 table (reflected polynomial 0xEDB88320): **255 identical, 1 misprint** – index 90 reads `8BBE8EA` in the document, correct is `8BBEB8EA`. ⇒ The "additional" fingerprint procedure uses the ordinary CRC-32 table.
- O-IC-1: Whether the STUN fingerprint XOR value (0x5354554E) is applied to this duplicate and what exactly differs from the "normal" fingerprint computation per ice-19 §7.1.1 is not stated in the document. Interop test needed.

## MS-ICE (version 1, compatibility)

| ID | Difference to ICE2 | Evidence |
|---|---|---|
| IC-30 | IPv4 only; STUN-02 only; based on ice-06/tcpcice-00 | §1.4, §2.1–2.2, pp. 12–15 |
| IC-31 | Candidates: UDP local/STUN/TURN, TCP STUN/TURN (actpass); max. 20 candidates, 40 pairs; candidate ID 32 characters + password 16 characters random, component 1/2; priority 0…1 | §3.1.4.8.1–3.1.4.8.1.4, pp. 18–19; §3.1.4.8.2.1, p. 20 |
| IC-32 | USERNAME = transport pair identifier from the peer's view; MI with the password of the remote candidate; keep-alive MI = 0 | §3.1.4.8.2.4, p. 20; §2.2.3, p. 15 |
| IC-33 | Provisional answer: the caller sends USERNAME-less requests from TURN candidates only to open permissions | §3.1.4.3, p. 17 |
| IC-34 | The final offer always comes from the initiator of the media session, also for streams added later by the other side | §3.1.4.5, p. 17 |
| IC-35 | Check phase after the first request+response shortened to **3 s** (ICE2: 5 s) | §3.1.6.2, p. 25 |

## Gap
- L-02: draft-ietf-mmusic-ice-19, ice-06, ice-tcp-07 and STUN-02 (basis of MS-ICE/MS-ICE2) were not in the Microsoft package.

## Later findings (files 38, 39)
- L-02 **closed**: ice-19, ice-06, ice-tcp-00/-07, STUN-02 and rfc3489bis-08 obtained.
- IC-12/IC-13 now with a primary source: priority formulas (IE-04, recomputed with SfB example values IE-05), MI in the old STUN format (IE-02).
- O-IC-1 narrowed (IE-03) and then **secondary evidence** (OB-09, libnice): the duplicate uses CRC-32 with the misprinted entry 0x08BBE8EA followed by XOR 0x5354554E; the misprinted variant is accepted only without IMPLEMENTATION-VERSION.
