# Evidence: MS-CONMGMT – server discovery, TLS, keep-alive

Source: [MS-CONMGMT] "Connection Management Protocol", edition 20250520, rev. 10.0, 27 pp.
According to §6 (p. 23) it applies up to and including Skype for Business 2021 and LTSC 2024.

## Transport

| ID | Requirement | Strength | Evidence |
|---|---|---|---|
| CON-01 | All SIP traffic over TCP; TLS on top is optional | MUST | §2.1, p. 10 |
| CON-02 | The client must verify the server identity via TLS (RFC 5246 §7) using the certificate | MUST | §3.2, p. 15 |
| CON-03 | The server certificate carries the outbound proxy FQDN as subject name | MUST (server) | §3.3, p. 16 |
| CON-04 | The client needs a trusted root CA for verification | precondition | §1.5, p. 9 |

## Automatic discovery of the outbound proxy (§3.1, pp. 13–15)

| ID | Requirement | Strength | Evidence |
|---|---|---|---|
| CON-10 | Domain = host part of the address of record (`sip:alice@contoso.com` → `contoso.com`) | MUST | §3.1.4, p. 13 |
| CON-11 | Query SRV in parallel: `_sipinternaltls._tcp`, `_sipinternal._tcp`, `_sip._tls`, `_sip._tcp` | MUST | §3.1.5, pp. 13–14 |
| CON-12 | Sort each query by SRV priority | MUST | §3.1.5, p. 14 |
| CON-13 | DHCP INFORM to 255.255.255.255 with option 120 (RFC 3361), option 60 = `MS-UC-Client`; ignore returned IP addresses; 5 s timer; ports 5061 (TLS) / 5060 (TCP) | SHOULD (from OCS 2007 R2, footnotes <1>–<3>) | §3.1.2, §3.1.5, §3.1.6, pp. 13–15 |
| CON-14 | Result order: `_sipinternaltls` → DHCP/TLS → `_sipinternal` → DHCP/TCP → `_sip._tls` → `_sip._tcp` | MUST | §3.1.5, p. 14 |
| CON-15 | Rules for still-pending queries (when `_sip._tcp` is awaited) | MUST/SHOULD | §3.1.5, p. 14 |
| CON-16 | Ignore TLS SRV targets whose host is not in `<domain>` or a subdomain | SHOULD | §3.1.5, p. 14 |
| CON-17 | Connect TLS entries via TLS, TCP entries via TCP | MUST | §3.1.5, p. 14 |
| CON-18 | Append fallback hosts: `sipinternal.<d>:443` (TLS), `sipinternal.<d>` (TCP), `sip.<d>:443`, `sip.<d>`, `sipexternal.<d>:443`, `sipexternal.<d>` | SHOULD | §3.1.5, pp. 14–15 |
| CON-19 | Entries one after another; a new A lookup before each; IPs one after another; on A failure or all TCP failures next entry; other errors or list exhausted ⇒ discovery failed | SHOULD/MUST | §3.1.5, p. 15 |
| CON-20 | Discovery is skipped when the client is manually configured with server/port/transport | applicability | §1.6, p. 9 |

## Keep-alive (§2.2.1, §2.2.2, §3.4, pp. 10–19)

| ID | Requirement | Strength | Evidence |
|---|---|---|---|
| CON-30 | Header `ms-keep-alive: UAC;hop-hop=yes` in a SIP request of the client (example REGISTER) | MUST | §3.4.5.1, p. 17; example §4.1, p. 21 |
| CON-31 | ABNF: role UAC/UAS, capabilities `hop-hop`/`end-end`/`tcp`=yes/no, `timeout=` seconds; header at most once | MUST | §2.2.1, p. 11 |
| CON-32 | Only the client initiates; negotiation right after transport and compression setup | MUST | §3.4.4, §3.4.5, p. 17 |
| CON-33 | Error response, missing header, multiple headers or empty intersection ⇒ negotiation failed, send no keep-alives | MUST | §3.4.5.3, p. 18 |
| CON-34 | Success: adopt `timeout`, refresh timer = 2/3 of the timeout; any data sent resets the timer | SHOULD/MUST | §3.4.2, p. 17; §3.4.5.3, p. 18 |
| CON-35 | Keep-alive message = CRLFCRLF on the connection | MUST | §2.2.2, p. 12; §3.4.5.4, p. 19 |
| CON-36 | Server recommends 300 s; grace 32 s (timer B/F); missing ⇒ registration removed, connection closed, **without** NOTIFY | server behaviour | §3.4.2, §3.4.6, pp. 17–19; <4>, <5> p. 23 |

## Server-side time limits the client must respect

| ID | Statement | Evidence |
|---|---|---|
| CON-40 | A connection without a successful transaction/authentication is closed after 32 s | §3.5.2, §3.5.6, pp. 19–20 |
| CON-41 | Idle 15 min 32 s ⇒ server closes the connection | §3.5.2, p. 19 |
| CON-42 | A new authenticated connection of the same endpoint ⇒ server closes the old one together with its security association | §3.5.5, p. 20 |

## Conflicts
- K-CON-1 (found 2026-10-27): §3.4.2 (p. 17) sets the client refresh timer to two thirds of the negotiated timeout, but in the next sentence says it "is reset to the time-out value" when the client sends data. Taken literally, the second rule lets the connection idle for the full timeout, which the server's expiry timer (timeout plus a grace period) only just tolerates. Decision: always two thirds (`crates/register` `keepalive`). The response example §4.2 (p. 21) also has typing errors (`To: <sip: alice@…>`, `Call-ID … CSeq: 1` on one line).

## Consequence for the Linux client
One connection = one TCP/TLS session to the outbound proxy carrying all SIP dialogs; reconnecting creates a new security association (see MS-SIPAE).

## Later finding (file 39)
- The Skype for Business client tries Lync Autodiscover (`lyncdiscoverinternal`, `lyncdiscover`) **before** the SRV records above; see OB-06 in file 39 and evidence file 08.
- Implementation (2026-10-27): keep-alive negotiation on the REGISTER (CON-30 … CON-35) in `crates/register` (`keepalive`), with the server timeout bounded to 30 … 3600 s (design). TLS adapter `crates/net` (rustls, TLS 1.2/1.3 only, system trust store plus optional enterprise CA, certificate and host name always verified).
- Research (2026-10-27, [SFB-TLS], official): Skype for Business Server supports TLS-1.2-only operation from **2015 CU9** (6.0.9319.548) and **2019 CU1** (17.0.2046.123); Lync Server 2013 front ends are out of scope (Windows Fabric 1.0 has no TLS 1.2; only the Edge role is supported). The supported cipher suite order starts with `TLS_ECDHE_ECDSA/RSA_WITH_AES_256/128_GCM_SHA384/256`, which rustls offers; the CBC and static-RSA suites of the list are not offered by rustls. Consequence: CommSCraft, which does not offer TLS 1.0/1.1, needs servers at these update levels with TLS 1.2 enabled; Lync Server 2013 (best effort in D-06) only through an updated Edge. Interop test IT-18.
