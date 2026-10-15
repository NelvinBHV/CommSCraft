# Outgoing MS-SIPAE signature drafting (v0.7.0, OFFLINE ONLY)

**This module is not a login provider and must never be used with live traffic
without a separately audited mutual authentication implementation.**

## Normative sources

- [MS-SIPAE 3.2.4.1 — sending authenticated client messages](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9), and [3.3.5.3 — the SIP server verifying client messages](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/47b1baf7-ceae-40bc-b1bf-8c26a1ec885a): ordered angle-bracket MIC input, `cnum`/`crand` and Authorization / Proxy-Authorization.
- [MS-SIPAE 3.2.5.1 — challenge processing](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9): GSS context and TLS-DSK key derivation; not implemented.
- [MS-SIPAE 3.2.5.2 — response MIC validation](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ec772ab7-96e7-4b54-ac9b-84cd0de2a6c3) and [RFC 2104](https://www.rfc-editor.org/rfc/rfc2104): HMAC and server response authentication.
- [RFC 3261](https://www.rfc-editor.org/rfc/rfc3261) §§8, 10 and 17: SIP request syntax, REGISTER processing and transaction correlation.

As of October 2026, consult the [published MS-SIPAE version 11.0](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd). Examples are not evidence of real-world server support or version negotiation.

## Implemented and testable

- For an **already-established** offline `SecurityAssociation`, construct an
  outgoing client MIC input from exact-case SIP `Call-ID`, `CSeq`, `From`, `To`,
  optional SIP/tel identity and `Expires`, plus the selected SA metadata.
- Use OS random bytes for every `crand` (32 bits, eight lowercase hex digits).
  Increment `cnum` per SA only when the complete signed draft is valid. Caller
  cannot supply a fixed production nonce; deterministic injection is test-only.
- `ClientMicSigner` is a separated provider interface. `TlsDskClientHmacSha256`
  signs only with an **externally supplied client authentication key**; this
  key must not be the server authentication key. It is not a key derivation
  or certificate-validation implementation. Actual GSS signers are absent.
- Create `Authorization` for a registrar or `Proxy-Authorization` for a proxy.
  Reject pre-existing authorization headers, malformed CSeq/method pairs,
  invalid identity format, unsafe realm/target quoting and oversized MICs.
- Verify synthetically signed 200 OK with the *server* test key, then build
  a signed outgoing REGISTER draft with a **different** client test key.
  Code does not transmit or reuse the draft as an authenticated session.
- Include an independent Python HMAC-SHA256 vector and Rust tests for exact
  field order, signature bytes, state-machine gating, replay and failure paths.

## Mandatory missing work before network authentication

1. Proven server identity and certificate path/service-origin policy validation.
2. Real MS-SIPAE v3/v4 version negotiation, token-bearing 401/407 flows and
   Kerberos/NTLM GSS contexts or complete **inner** TLS-DSK handshake/PRF.
3. Binding the correct **client** and **server** MIC contexts to a mutually
   authenticated SA; fail closed on any untrusted key/identity.
4. Challenge retransmission with fresh branch, CSeq and correct dialog context,
   proper authentication, signed server response and route-set processing.
5. Full ABNF coverage, clock/replay handling, policy differences, fuzzing and
   interop tests against an authorized Skype for Business Server test setup.
6. Complete Windows Full client parity contract (122 requirements, 93 minimum
   reference scenarios). No feature is currently independently verified.

## How to reproduce

```sh
cargo test -p commscraft-core
cargo run -p commscraft-inspect -- --signing-demo
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/check_release.py --release  # EXPECTED to fail
```

The synthetic vectors (`tests/vectors/client-mic-v4.json`) are locally generated
fixtures, **not Microsoft-provided cryptographic test vectors**. Run the Rust
commands on a machine with Rust and dependencies; none were executed in the
artifact-authoring environment because no Rust toolchain is available.
