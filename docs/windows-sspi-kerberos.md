# Windows Kerberos SSPI adapter — v0.10.0

**Status: EXPERIMENTAL, LOCAL CREDENTIAL/TOKEN HANDOFF ONLY. NOT a working login.**

This module adds a native Windows `Secur32.dll` adapter in
`crates/commscraft-core/src/windows_sspi.rs`. It has **no network access** and
is compiled only on Windows (`#[cfg(windows)]`). There are no hardcoded
passwords, password prompts, delegation requests or Negotiate/NTLM fallback.
It uses the current Windows logon session's default Kerberos credential.

## Protocol / API traceability

| Required behavior | Primary reference | Local implementation | Verification still needed |
|---|---|---|---|
| Obtain Kerberos outbound user credentials | Windows `AcquireCredentialsHandleW` (Kerberos) | `WindowsKerberosProvider::new` with package `Kerberos`, `SECPKG_CRED_OUTBOUND`, null auth-data | Windows CI compilation and logon-session integration |
| Initialize Kerberos ticket to pinned `sip/<FQDN>` | [MS-SIPAE] §3.2.5.1 step 5; RFC 2743; Windows `InitializeSecurityContextW` (Kerberos) | `step` uses the explicit target, not untrusted response target | AD test realm and canonical service SPN |
| Request Identify/Integrity | [MS-SIPAE] §3.2.5.1 | `ISC_REQ_IDENTIFY | ISC_REQ_INTEGRITY`, additionally requests mutual/replay/sequence | Verify returned context attributes on domain-joined Windows |
| GSS continuation (when needed) | Windows Kerberos `InitializeSecurityContext` documentation; RFC 2743 | `step` accepts a bounded token only after a previous context; returns `complete=false` on `SEC_I_CONTINUE_NEEDED` | Authenticated server token provenance and actual multi-leg wire integration |
| Encode initial token in SIP `gssapi-data` | [MS-SIPAE] §3.2.5.1 step 6, §4.2 | `KerberosNegotiation::respond` now permits a nonempty intermediate GSS token | End-to-end REGISTER/transaction binding |
| Prevent unauthenticated acceptance | [MS-SIPAE] §3.2.5.2, RFC 3261 | `KerberosPhase::AwaitingContextContinuation`; no signed-in state is exposed | Native SSPI `VerifySignature`/GSS_VerifyMIC and replay integration |
| Dispose sensitive native handles | Windows `DeleteSecurityContext` and `FreeCredentialsHandle` | `Drop` in `WindowsKerberosProvider` | Windows instrumentation/ASAN equivalent and lifecycle tests |

References:

- https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9
- https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/b3989651-d013-4481-8bb0-c4fd1678300a
- https://learn.microsoft.com/en-us/windows/win32/secauthn/acquirecredentialshandle--kerberos
- https://learn.microsoft.com/en-us/windows/win32/secauthn/initializesecuritycontext--kerberos
- https://www.rfc-editor.org/rfc/rfc2743
- https://www.rfc-editor.org/rfc/rfc4121

## Offline-local self-check

```powershell
cargo run -p commscraft-inspect -- --sspi-check
```

On Windows this **only acquires and releases a Kerberos credential handle**.
It sends **no** network messages, service tickets, SIP/REGISTER or authorization
tokens. No target SPN is used at this stage. On non-Windows systems it prints
an unsupported-platform message. If the OS has no usable credentials the
self-check reports unavailable rather than switching to NTLM.

## Security / release constraints

1. A real application must resolve its home pool and trusted SIP SPN through
   **trusted configuration** and validate TLS peer identity before consuming
   a server challenge. No untrusted challenge may override the configured SPN.
2. `SEC_I_CONTINUE_NEEDED` is **not** an authenticated context. In this state
   a REGISTER draft may carry an initial token, but no session identity or
   server approval is asserted.
3. The Windows SSPI adapter has not been compiled or exercised here; Windows
   GitHub Actions is configured to build the module, but that is not a claim
   that CI ran successfully.
4. Server `Authentication-Info` must be verified with the same native GSS
   context. A synthetically supplied MIC/HMAC verifier is insufficient.
5. The code handles only Kerberos from the logged-in Windows identity.
   TLS-DSK, NTLM, validated production MIC integration, token-provenance binding, live REGISTER,
   presence, media, and full Windows-client parity remain **open**.
6. Never enable live network auth based solely on successful self-check,
   `KerberosStep.complete` or a received unsigned `200 OK`.

**Interoperability requirement:** Test against a permitted Skype for Business
Server Subscription Edition environment and a pinned Windows Full-client build.
No such testing has happened here.

## Added in v0.10.0

`WindowsKerberosProvider` implements the `KerberosMicBackend` trait for
completed contexts using native SSPI `QueryContextAttributesW`,
`MakeSignature`, and `VerifySignature`. The `KerberosMicBridge` adapter binds
the context to an MS-SIPAE SA key; `SecurityAssociation` still performs the
MIC-first, replay-second checks. See
[implementation and proof ledger](kerberos-mic-evidence.md).

**Correction to the v0.9.0 limitations:** per-message FFI is now authored,
but **neither its Rust/Windows compilation nor its actual operation** is
verified. An authorization-ready or signed-in state remains unimplemented.
