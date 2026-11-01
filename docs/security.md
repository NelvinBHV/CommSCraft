# Security by design

> **Last reviewed:** 2026-11-01 · **Last updated:** 2026-11-01 · **Change:** minor (static audit, issue #67: emergency routing order and call limit, on-behalf-of identity, transfers that ask, bounded 409 retries, quoted verbose output)

CommSCraft handles passwords, Kerberos tickets, NTLM session keys and SRTP keys. It talks to
servers and peers it cannot trust. Security is part of every design decision, not a later review.
This page lists the rules every crate follows and how each is enforced. The general rules in
[`AGENTS.md`](../AGENTS.md) (Security, Never crash) still apply.

## Threat model (short)

| Attacker | Can | We assume |
|---|---|---|
| Network attacker | Read, change, inject and replay traffic | TLS with certificate validation protects signalling; SRTP protects media |
| Hostile or compromised server or proxy | Send any SIP, XML, HTTP, STUN or RTP content, including malformed and huge messages | Nothing it sends can crash the client, exhaust memory, or inject into our own messages |
| Hostile peer (federated user, conference member) | Send any message that the server relays | Same as above; identities come only from server-signed headers (AE-34) |
| Local user of another account | Read files and logs | No secrets in logs, debug output or plain files |

## Rules and where they are enforced

| Rule | How | Where today |
|---|---|---|
| Bounded input | Every parser has explicit limits (message size, header count, line length, parameter count, value length); buffers never grow without a check | `sip-msg::Limits`, `sipae` (32 parameters, hex ≤ 1024), `abs` (block size), `psom` (lengths) |
| Fail closed | Malformed, ambiguous or unexpected input is an error, never a guess. Examples: several Content-Length fields, duplicate From/To/Call-ID/CSeq, duplicate auth parameters, two Authentication-Info for one SA | `sip-msg`, `sipae` |
| Poison on stream errors | After a framing error the decoder refuses all further data; the connection must be closed | `sip-msg::StreamDecoder` |
| No injection | Serializers refuse CR, LF and control characters. Values copied from the network into our headers or signature buffers must not contain quotes, backslashes or angle brackets | `sip-msg::Message::to_bytes`, `sipae::checked`, `sipae::signature_buffer` |
| Authenticated before trusted | A server message counts only after its MS-SIPAE signature verifies for a matching SA and its `snum` is fresh (window of 256). A parsed `200 OK` alone means nothing; an unauthenticated 200 to a REGISTER is refused, a badly signed one dropped | `sipae::SecurityAssociation::verify`, `register` |
| Know what the signature covers | MS-SIPAE signs Call-ID, CSeq, From, To, P-Asserted-Identity, Expires and the status code, but not Contact (GRUU), Via, Event, Content-Type or the body (e.g. the reason in a deregister NOTIFY, the provisioning document); those rely on TLS. Where a value exists in both places the signed one wins (registration expiry) | `register::Registrar` |
| Transactions are bound | Responses must match Call-ID, CSeq, From tag and Via branch of the request in flight; every wait has a timeout (32 s) | `register::Registrar::belongs_to_us` |
| No downgrade | MS-SIPAE versions below 3 are refused by default; they sign neither the To URI nor P-Asserted-Identity. NTLM: only NTLMv2 with extended session security, 128-bit keys and key exchange (OB-43); the client claims no Windows version. Never fall back to plaintext or unvalidated TLS | `sipae::Policy`, `ntlm::messages::choose_flags`; TLS rules for the next crates |
| Constant-time comparison | Signatures and MACs are compared with `subtle` | `sipae::NtlmMic::verify_mic` |
| Secrets wiped, never printed | Key material is zeroized on drop (`zeroize`) and `Debug` prints `<redacted>`. Passwords live only in the system keyring (Secret Service) and in memory as long as needed | `sipae::NtlmMic`; keyring adapter planned |
| Randomness from the OS | Protocol crates take random values (`crand`, ICE credentials, SRTP keys) as parameters. Only the adapter layer draws them from the OS CSPRNG, so tests stay deterministic | `sipae::SecurityAssociation::sign` |
| No panics | `unsafe` forbidden; `unwrap`, `expect`, `panic!`, indexing and slicing are denied by workspace lints. Errors are `Result` | `Cargo.toml` `[workspace.lints]`, CI clippy |
| Bounded XML | No DTD (so no external entities, no entity expansion), only predefined entities and character references, limits on size, depth, attributes and nodes, tree built without recursion | `xml` |
| Provisioned values are input too | Web service URLs from provisioning must be `https://` with a DNS host (no IP literal, no user info); a host outside the user's SIP domain is marked so that the user is asked before credentials go there; SIP URIs, numbers, port ranges (≥ 1024) and enumerations are checked; invalid values are dropped and listed | `provisioning::ServerConfiguration` |
| Trust across domains only by the user | A SIP server outside the user's SIP domain is used only if Autodiscover (verified HTTPS, started at the user's domain) named it **and** the user trusts its domain (interactive question or `--trust-domain`, remembered with mode 0600) – the model of `TrustModelData` in the Windows client; never for DNS SRV results | `commscraft-cli` |
| Discovery without secrets | Autodiscover over HTTPS only (D-10 proposal), unauthenticated Domain resource, at most 10 redirects, no loops; every URL and host checked; DNS SRV targets outside the user's domain dropped; Autodiscover targets, links and registrar redirects outside the domain marked so the user is asked; TLS only, no plain TCP or DHCP targets | `discovery`, `url`, `register` (redirect) |
| DNS as a hint only | SRV lookups with a random ID, answers only from the queried server, same question, bounded parsing (backward compression pointers only), TCP retry on truncation; results are filtered to the user's domain and the TLS certificate decides | `net::dns` |
| Strict HTTP | Autodiscover GET over the verified TLS client, `Connection: close`, bounded response, Content-Length and chunked parsed strictly and never both, a response ends exactly where its framing says (last chunk, trailer section and final CRLF read; surplus octets refused, so a kept-alive connection never starts mid-stream – issue #42), no cookies, no credentials; redirects decided by `discovery` | `net::http` |
| Verified TLS | rustls (ring), TLS 1.2/1.3 only, forward-secret AEAD suites; system trust store plus an optional enterprise CA file; host name from discovery, never an IP; no option to switch verification off; timeouts on connect/read/write | `net` |
| OS randomness, fail closed | `OsRandom` checks the OS CSPRNG at start; a later failure aborts the process instead of continuing with predictable values | `net` |
| Owner-only network use | `commscraft-cli` refuses to connect without `--authorized`; password read without echo, echo restored on every return path and on panic (a signal still leaves it off: `stty echo` repairs it; issue #42); verbose output redacts authentication headers and bodies; EPID file mode 0600 | `commscraft-cli` |
| Display data is neutralised | Names, notes and activities from presence documents and contact lists are stripped of control characters (C0/C1, bidirectional overrides) and bounded before they reach a terminal or UI; every line the CLI prints passes `term::clean` (C0/C1 incl. ESC, DEL and bidirectional formatting characters shown as `\u{…}` – issue #42); contact URIs are checked before they go into our own XML | `presence::printable`, `presence::checked_sip_uri` |
| Plain-text messaging | IM sessions offer and accept only `text/plain`; HTML/RTF are refused (415), never rendered; incoming text is UTF-8 checked, bounded and stripped of control characters; the peer's SDP must describe a message session | `im` |
| Reversible actions on real accounts | The CLI sets the own presence only on request and with `expireType="endpoint"`, so it disappears when the session ends | `commscraft-cli presence --set-status` |
| Credentials only to the user's servers over HTTPS | Address book search uses only provisioned HTTPS URLs in the user's SIP domain or a trusted domain; NTLM over HTTP stays on one verified TLS connection and is bound to it with `tls-server-end-point` channel bindings (issue #42), the password never leaves the registrar; the search text is bounded and escaped, the SOAP answer parsed without DTD, SIP URIs checked, display text stripped of control characters | `dlx`, `net`, `register` |
| Conferences only on the user's command | A conference is created or joined only by a user command; invitations are reported, never auto-joined; focus/MCU URIs and roster entries are checked SIP URIs; the roster is bounded (250 participants, 16 endpoints each); delivery notifications only match our own Message-Ids and time out after 10 s | `conf`, `session` |
| Audio devices behind one adapter | Only `audio-alsa` links a system audio library (D-03, D-11); device name checked; only mono 16-bit at the codec rates; the negotiated rate must equal the requested one; bounded buffers; nothing from the network is interpreted there | `audio-alsa` |
| Media authenticated before use | SRTP/SRTCP: tag checked in constant time before decryption and before any state change; 64-entry replay window per SSRC and for SRTCP; wrong MKI refused; at most 32 streams per context; failures counted, never logged with content; keys zeroised. RTP/RTCP parsers bounded (1500 octets, 16 compound parts, 10 report blocks) | `srtp`, `rtp` |
| No silent plaintext media | SDP: only AES_CM_128_HMAC_SHA1_80 with inline 30-octet keys and 1-byte MKI; an encrypted offer without usable key is refused; a plaintext answer or offer is accepted only by explicit policy (`Encryption::Off` / `allow_plaintext`); a peer reusing our key is refused; keys never printed, zeroised; SDP parsing bounded | `sdp` |
| STUN integrity first | STUN: ≤ 1500 octets, ≤ 32 attributes, lengths checked; MESSAGE-INTEGRITY compared in constant time and accepted only as the last attribute (FINGERPRINT may follow) | `stun` |
| Devices only during a call | CLI: microphone and speaker are opened only for an answered call and closed when it ends; queues between media and device threads are bounded (frames dropped, never piled up); the relay password is passed to the media thread once | `commscraft-cli` |
| Media sockets accept only known peers | Call media: non-blocking sockets, receives bounded to 1500 octets; relay sockets accept only the TURN server; RTP/RTCP only from the ICE-selected/usable peer or the direct address; ICE credentials and SRTP keys from the OS CSPRNG (abort if unavailable) | `call-media` |
| No traffic to this host or the local link | ICE candidates, peer-reflexive addresses, mapped addresses and the default addresses of a peer without ICE must not be loopback or link-local (unless we use such an address ourselves, i.e. a local test), unspecified, multicast, broadcast or port 0 (issue #42) | `ice::usable_peer_address`, `ice-agent`, `call-media` |
| Bounded TCP relay buffer | One way into the TCP relay's write buffer (64 KiB): media that does not fit is dropped like a lost datagram, a TURN control message that does not fit gives the relay up (issue #42) | `call-media` |
| Media authenticated before decoding | Media core: SRTP authentication before any parsing of the payload; only the peer's first SSRC and the negotiated payload types; jitter buffer bounded (50 packets) and windowed; packets > 1500 octets dropped; playout never blocks | `media` |
| Calls only by the user | Calls: an incoming call only rings (60 s, then 480) until the user accepts; every SDP passes the bounded parser and the encryption policy (a plaintext answer to a required offer ends the call with BYE); keys are never renegotiated in a re-INVITE (SD-13); at most 4 calls | `session` |
| Emergency calls are never blocked, never half-done | An emergency call may exceed the call limit by one; the time-bound e911active routing is published (before the INVITE, MS-SIPRE §3.17.5.1, without waiting) only once the emergency call exists, so forwarding is never suspended without a call that ends the suspension; a call that cannot start is reported loudly (issue #67) |
| Identity claims only where the routing shows them | A call counts as meant for someone else ("for X", answered with p-session-on-behalf-of) only when it was addressed to X (`To`) and Referred-By names the same X – a transferred call (Referred-By = the transferor, addressed to us) is ours (issue #67) |
| Transfers that may cost ask first | A REFER is followed automatically only to a SIP address in the user's own domain; phone numbers and other domains wait for `/follow` or `/decline` (NOTIFY 603) (issue #67) |
| Bounded retries | 409 version conflicts of publications and delegate changes: at most three retries in a row until one succeeds (issue #67) |
| Untrusted text cannot pose as CLI output | Multi-line server content in verbose output (Autodiscover bodies) is printed as a quoted block (`  | ` per line) after control and bidi escaping (issue #67) |
| Connectivity checks authenticated | ICE: requests need our ufrag, FINGERPRINT and valid MI (401/431 otherwise); responses matched by transaction ID and exact address pair, verified with the peer's password, with a usable XOR-MAPPED-ADDRESS checked before the transaction is consumed (a forged answer cannot cancel the genuine one); duplicate transaction IDs redrawn; candidates and pairs bounded; every phase bounded by timers | `ice-agent` |
| Relay credentials stay inside | MRAS answer parsed bounded (no DTD), request ID and status checked, username/password limited to the base64 alphabet, relay hosts must be DNS names or IP addresses; credentials kept in the engine, never in events or logs, zeroised | `avedgea`, `session` |
| Relay responses authenticated | TURN: only messages with the MS magic cookie first; success responses must carry valid MESSAGE-INTEGRITY; unauthenticated challenges limited to three; SHA-256 never downgraded; data only from peers we sent to; bounded retransmission and TCP frames (1500 octets); password zeroised | `turn` |
| Every request answered or dropped | The session engine drops unsigned requests without an answer, answers every signed one (unknown dialog 481, unsupported 405/488/501), bounds conversations (64) and subscriptions (128), and times out every client transaction | `session` |
| Hostile-input tests | Every parser gets tests with malformed input and a deterministic mutation test (thousands of corrupted messages; must not panic) | `sip-msg` `mutated_inputs_never_panic` |
| Minimal, reviewed dependencies | Pure Rust, mostly RustCrypto (D-03); new dependencies need a reason in the PR. No dependency on code from the specification package or from GPL projects | `Cargo.toml`, `Cargo.lock` (CI uses `--locked`) |
| No real servers in development | Work happens offline against specification examples and own fixtures (D-08); besides the owner's runs, only the owner's isolated lab is used, and nothing about it is published | [`interop-tests.md`](interop-tests.md) |

## Rules for the next building blocks

- **Kerberos/TLS-DSK:** behind adapter crates (D-03); tickets and private keys never leave them.
- **Media:** SRTP only (`AES_CM_128_HMAC_SHA1_80`, K-SR-1); no unencrypted RTP fallback.
- **Logging:** headers that carry secrets (`Authorization`, `Proxy-Authorization`, `gssapi-data`,
  web tickets, SDP `a=crypto`) are redacted before anything is logged.
- **Planned CI additions (free runners only, D-09):** a dependency advisory check and longer
  fuzzing runs as a scheduled job.
