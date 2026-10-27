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
| Authenticated before trusted | A server message counts only after its MS-SIPAE signature verifies for a matching SA and its `snum` is fresh (window of 256). A parsed `200 OK` alone means nothing | `sipae::SecurityAssociation::verify` |
| No downgrade | MS-SIPAE versions below 3 are refused by default; they sign neither the To URI nor P-Asserted-Identity. NTLM: only NTLMv2 with extended session security (OB-43). Never fall back to plaintext or unvalidated TLS | `sipae::Policy` (versions); NTLM and TLS rules for the next crates |
| Constant-time comparison | Signatures and MACs are compared with `subtle` | `sipae::NtlmMic::verify_mic` |
| Secrets wiped, never printed | Key material is zeroized on drop (`zeroize`) and `Debug` prints `<redacted>`. Passwords live only in the system keyring (Secret Service) and in memory as long as needed | `sipae::NtlmMic`; keyring adapter planned |
| Randomness from the OS | Protocol crates take random values (`crand`, ICE credentials, SRTP keys) as parameters. Only the adapter layer draws them from the OS CSPRNG, so tests stay deterministic | `sipae::SecurityAssociation::sign` |
| No panics | `unsafe` forbidden; `unwrap`, `expect`, `panic!`, indexing and slicing are denied by workspace lints. Errors are `Result` | `Cargo.toml` `[workspace.lints]`, CI clippy |
| Hostile-input tests | Every parser gets tests with malformed input and a deterministic mutation test (thousands of corrupted messages; must not panic) | `sip-msg` `mutated_inputs_never_panic` |
| Minimal, reviewed dependencies | Pure Rust, mostly RustCrypto (D-03); new dependencies need a reason in the PR. No dependency on code from the specification package or from GPL projects | `Cargo.toml`, `Cargo.lock` (CI uses `--locked`) |
| No real servers in development | Work happens offline against specification examples and own fixtures (D-08); interop runs only by the owner | [`interop-tests.md`](interop-tests.md) |

## Rules for the next building blocks

- **TLS:** system trust store or the user's configured CA, host-name check against the SIP domain or
  the discovered server, TLS 1.2 or later, no option to switch validation off.
- **Discovery:** Autodiscover redirects and DNS SRV targets are only followed inside the user's SIP
  domain (or a domain the user confirms) before any credential is sent.
- **XML:** no DTDs, no external entities, depth and size limits.
- **Kerberos/TLS-DSK:** behind adapter crates (D-03); tickets and private keys never leave them.
- **Media:** SRTP only (`AES_CM_128_HMAC_SHA1_80`, K-SR-1); no unencrypted RTP fallback.
- **Logging:** headers that carry secrets (`Authorization`, `Proxy-Authorization`, `gssapi-data`,
  web tickets, SDP `a=crypto`) are redacted before anything is logged.
- **Planned CI additions (free runners only, D-09):** a dependency advisory check and longer
  fuzzing runs as a scheduled job.
