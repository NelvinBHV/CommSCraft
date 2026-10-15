# Security model (v0.9.0)

The desktop app is a **local demo**. `commscraft-probe` is separate, and makes live
DNS or TLS connections **only when explicitly invoked with `--dns` or
`--tls-probe`**. It never sends SIP requests or credentials.

## Implemented trust boundaries

1. SIP outbound frames use validated header names and values; they derive
   Content-Length from the actual byte body. No raw CRLF interpolation.
2. The stream parser bounds header/body size, requires Content-Length on a
   reliable stream, and rejects ambiguous framing instead of resyncing.
3. Offline non-INVITE SIP responses match Via-branch/sent-by, Call-ID and CSeq.
   A 401/407 is **terminal** for that transaction, never a successful login.
4. DNS SRV records are ordered by [RFC 2782](https://www.rfc-editor.org/rfc/rfc2782),
   with capped processing. The CLI performs all 4 Microsoft SRV lookups, but
   contacts **TLS endpoints only**; no TCP downgrade is possible.
5. The opt-in TLS diagnostic uses operating-system trust anchors and performs
   normal server certificate chain and hostname checks. A TLS failure stops
   discovery; it never falls back to plaintext or accepts invalid certificates.
6. Before TCP/TLS probes, hosts must equal the requested SIP domain or be within
   its DNS subtree. This conservative lab policy reduces DNS redirect risks;
   some real deployments require a separately audited, less restrictive policy.
7. [MS-SIPAE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/e60fa322-b73b-4e83-9ddb-4b20bd17a4a1)
   challenge parsing is read-only. Debug output redacts realm, target, opaque,
   URI and token values. The v0.8.0 Kerberos handoff can encode an externally
   generated GSS token in a fresh offline REGISTER draft, but it has no real
   native provider, credential store, actual token exchange, server MIC verification
   tied to Kerberos or sign-in.
8. All CLI/protocol fixtures are synthetic. `commscraft-inspect` shows sanitized
   results instead of printing raw SIP headers and user identities. `SipMessage`
   `Debug` intentionally omits all header values, request URIs and body contents.
9. The Kerberos handshake plan pins a trusted service principal independently
   of SIP challenges, rejects unexpected 401/407 identity and never downgrades
   silently to NTLM/Digest. The default GSS provider fails closed.

## Known gaps (important)

- `native-tls` checks the SRV target's certificate hostname but does not, by
  itself, establish the entire SIP-origin-to-target service identity binding
  specified in RFC 3263 and deployed SfB certificate policies. The probe
  **cannot** be used to assert a server is authorized to sign in an identity.
- DNS is not DNSSEC-validated by default. DHCP option 120, proxies, HTTPS
  autodiscover, VPN/network change handling, and full MS-CONMGMT sequencing
  have not yet been implemented.
- `commscraft-probe` is intentionally a blocking diagnostic, not an asynchronous
  resilient production transport. Timeout and size limits exist but a fully
  cancellable I/O layer is still required.
- RFC 3261 ABNF, Microsoft authentication security association (SA) lifecycle,
  signed SIP messages and replay protection must be completed and audited
  before processing credentials or attempting a real REGISTER.
- Never test an organization’s infrastructure without authorization. Use a
  lab environment for interoperability, first with credentials **disabled**.

## v0.4 REGISTER-specific guardrails

- The new `registration.rs` generates **offline-only** Microsoft-style requests, never writes a socket, and exposes no credential adapter in the REGISTER transport path. The separately opt-in Windows SSPI token source is not wired to live registration.
- `401/407` are **not retried with fabricated authentication**. MS-SIPAE SA establishment (GSS/TLS-DSK), signature checking and replay window are required first.
- A `200 OK` with matching `+sip.instance` yields `UnverifiedAcceptance`, **not** `Registered`. A returned GRUU is likewise untrusted absent SA verification.
- A `423` with `Min-Expires` outside the bounded range or not greater than the requested expiry is rejected. New offline transactions increase CSeq and require a different branch.
- The Contact parser is deliberately conservative and is not a complete SIP URI equality algorithm; interoperability against real MS Server Contact parameters remains to be tested.
- Caller-provided UUID, EPID, From tag, Call-ID and branch MUST be provisioned using a cryptographically strong entropy source in any future production adapter. Test fixtures have public, deterministic values and MUST NOT be used for live credentials.

## v0.5 offline authentication model

- `security.rs` implements **only incoming** MS-SIPAE signed-message validation
  against a caller-supplied verifier; the challenge target, protocol, realm,
  proxy/UAS and version must match. The canonical `<...>` buffer is generated
  conservatively; unsupported address grammar is an error.
- The HMAC-SHA256 verifier takes a **server** authentication key that only a
  correctly authenticated TLS-DSK handshake could derive. It does **not**
  perform the handshake, authenticate a server, or bind a SIP identity to a
  trusted transport. Public keys in fixtures prove only local test logic.
- `snum` is checked after MIC success and stored in a 257-number inclusive
  sliding window. Duplicates and too-old values fail closed. Failed MICs do
  not advance the replay window.
- SA expiration, 403 during establishment and association mismatches are
  handled without promoting a live user login. Outgoing request signing,
  actual GSS providers, TLS-DSK handshake, signed REGISTER response binding,
  key vaults and real integration remain OPEN.
- Inspectors do not print raw auth headers, MICs, tokens, realm/target/opaque
  or signed buffer bytes. A rustdoc / code-review pass is required before
  any future live credentials are enabled.

## 0.6.0 inbound message and feature-gate notes

- `presence.rs`, `im.rs`, and `sdp.rs` are **offline only**. The future
  networking adapter MUST authenticate the SIP server and verify relevant
  MS-SIPAE MICs before passing messages to the client state machines.
- The PIDF `NOTIFY` body is deliberately opaque; it is not XML-parsed,
  rendered as HTML, or used to publish visibility claims.
- The IM model only accepts UTF-8 `text/plain`; a SIP 200 is **not** a read
  receipt, an encrypted channel, or proof that a remote user displayed text.
- The SDP inspector only counts media descriptions; it never opens devices,
  sockets, or chooses codecs/ICE candidates from untrusted input.
- A release tag `v1.*` must pass the 122-requirement / 93-reference-scenario formal parity gate and
  manual signed interop evidence review against the authorized Windows reference.
  The automated checker cannot replace independent testing.

## v0.7.0 outgoing MIC trust boundary

- `TlsDskClientHmacSha256` is for *synthetic separately provided client keys*
  and must never be instantiated using a server key or unauthenticated key.
- `SecurityAssociation::sign_client_request` uses the OS CSPRNG; signing never
  opens a socket and is gated by established association state. Establishment
  in local fixtures does not prove a trusted remote Skype for Business server.
- `cnum` is consumed only on successful validated draft creation. A failed
  cryptographic signing operation does not mutate the original request.
- Request bodies are not independently protected by the selected-header MIC
  input; do not infer full message confidentiality or integrity without the
  actual authenticated transport/session architecture.
- Pending vulnerabilities: no real key exchange, service-origin binding,
  audited key lifetime/zeroization, body integrity policy or interop testing.

## v0.9.0 Windows-only SSPI integration boundary

The native Windows SSPI adapter is compile-time gated and explicitly selects
Kerberos. An initial context token, or local credential acquisition, is not a
server identity proof and is not equivalent to an authenticated SfB session.
No production code calls `verify_server_message` using an SSPI verifier yet;
therefore the client does not enable real registration. Refer to
[Windows SSPI design](windows-sspi-kerberos.md).
