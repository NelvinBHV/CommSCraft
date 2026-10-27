# Security by design

> **Last reviewed:** 2026-10-27 · **Last updated:** 2026-10-27 · **Change:** major (first version)

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
| Strict HTTP | Autodiscover GET over the verified TLS client, `Connection: close`, bounded response, Content-Length and chunked parsed strictly and never both, no cookies, no credentials; redirects decided by `discovery` | `net::http` |
| Verified TLS | rustls (ring), TLS 1.2/1.3 only, forward-secret AEAD suites; system trust store plus an optional enterprise CA file; host name from discovery, never an IP; no option to switch verification off; timeouts on connect/read/write | `net` |
| OS randomness, fail closed | `OsRandom` checks the OS CSPRNG at start; a later failure aborts the process instead of continuing with predictable values | `net` |
| Owner-only network use | `commscraft-cli` refuses to connect without `--authorized`; password read without echo; verbose output redacts authentication headers and bodies; EPID file mode 0600 | `commscraft-cli` |
| Hostile-input tests | Every parser gets tests with malformed input and a deterministic mutation test (thousands of corrupted messages; must not panic) | `sip-msg` `mutated_inputs_never_panic` |
| Minimal, reviewed dependencies | Pure Rust, mostly RustCrypto (D-03); new dependencies need a reason in the PR. No dependency on code from the specification package or from GPL projects | `Cargo.toml`, `Cargo.lock` (CI uses `--locked`) |
| No real servers in development | Work happens offline against specification examples and own fixtures (D-08); interop runs only by the owner | [`interop-tests.md`](interop-tests.md) |

## Rules for the next building blocks

- **Kerberos/TLS-DSK:** behind adapter crates (D-03); tickets and private keys never leave them.
- **Media:** SRTP only (`AES_CM_128_HMAC_SHA1_80`, K-SR-1); no unencrypted RTP fallback.
- **Logging:** headers that carry secrets (`Authorization`, `Proxy-Authorization`, `gssapi-data`,
  web tickets, SDP `a=crypto`) are redacted before anything is logged.
- **Planned CI additions (free runners only, D-09):** a dependency advisory check and longer
  fuzzing runs as a scheduled job.
