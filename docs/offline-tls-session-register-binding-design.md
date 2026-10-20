# Offline TLS session / REGISTER evidence contract (design only)

**Status:** source-only design until exact-head CI passes. No live transport, Kerberos authentication, REGISTER send, network test, or sign-in authorization.

## Normative references

- RFC 5922, June 2010, §§7.1–7.3: https://www.rfc-editor.org/rfc/rfc5922#section-7.1 — validate SIP-domain identity from the actual verified TLS peer leaf; SRV targets do not replace the originating SIP domain.
- RFC 3261, June 2002, §17.1.3: https://www.rfc-editor.org/rfc/rfc3261#section-17.1.3 — Via branch correlates SIP transactions; the magic cookie is not proof of authentication.
- Microsoft [MS-SIPAE] §3.2.5.2: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c — server MIC verification and security-association replay handling remain independent and mandatory.
- Microsoft [MS-CONMGMT] §3.1.5: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3 — SIP discovery context.

## Trust boundary / integration prerequisites

1. The future transport must obtain the leaf certificate **from the same platform-validated TLS stream** and compare its RFC 5922 SIP identity with the original SIP URI domain. A caller-provided `verified` flag, certificate fingerprint, or synthetic report cannot prove this.
2. The TLS session must own an opaque, unforgeable connection identity and invalidate all associated evidence on reconnect, certificate change, expiry or disconnect. A counter supplied by a caller is insufficient in production.
3. A trusted security-association implementation must bind the expected Kerberos server SPN to that original domain and to the same live connection, and independently validate the server MIC and replay state. Never infer this from a parsed SIP 200 OK.
4. The REGISTER transaction identity must be created and tracked by the actual SIP transaction layer, not supplied by untrusted SIP headers or UI input. Any mismatch fails closed.
5. Only after independent live interoperability and security review could an authorization API be designed. **This PR adds no such API.**

## Offline-only source and tests

- `crates/commscraft-core/src/tls_boundary.rs::FixtureSessionCorrelation`: synthetic certificate SHA-256 byte array, connection generation, SPN and REGISTER branch correlation. This is a **fixture model**: a nonzero digest is not proof of certificate validation, a generation integer is not an unforgeable TLS session identifier, and a `mic_checked` boolean is not a verified server MIC.
- `fixture_session_rejects_certificate_spn_transaction_replay_and_disconnect`: certificate swap, wrong generation, wrong SPN, wrong REGISTER branch, MIC denial, replay, disconnect, and constant false sign-in.
- `fixture_session_rejects_missing_certificate_and_expiry`: zero digest fixture and expired report rejected.
- `authenticated_sign_in()` always returns `false`. The core module cannot access a socket, native TLS peer certificate, SSPI credential, or send REGISTER.

**Verification:** exact-head CI results to be recorded after GitHub Actions completes on standard `ubuntu-latest` and `windows-latest` runners. No claim of RFC 5922 live interoperability or v1.0 parity. Issue #18 remains open.
