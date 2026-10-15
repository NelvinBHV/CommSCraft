# MS-SIPAE Kerberos REGISTER challenge planning (v0.9.0)

**Status:** Offline-only integration boundary. **Not a real login.** No **network-wired** GSS/SSPI
credential provider, network REGISTER transmission, validated user identity or
validated server MIC exists in this path.

## Normative mapping

| Requirement | Authority | Implementation / scope |
|---|---|---|
| 401 from registrar vs 407 from proxy and different authorization headers | [MS-SIPAE] §3.2.5.1 steps 1, 6 | `challenge::parse_challenges`; `kerberos::respond` |
| Reuse SA by authenticating server (realm and targetname) | [MS-SIPAE] §3.2.1 | `SecurityAssociation` (offline), `KerberosNegotiation` pins SPN |
| Obtain Kerberos ticket for `sip/<FQDN>` and call `GSS_Init_sec_context` with integrity and identify | [MS-SIPAE] §3.2.5.1 step 5; [MS-KILE]; RFC 2743 | `KerberosProvider` **trait only**, built-in provider rejects |
| Encode GSS token in `gssapi-data`, include `qop="auth"`, realm, targetname, opaque, version | [MS-SIPAE] §3.2.5.1 step 6 | offline `KerberosNegotiation::respond` (v3) |
| Reissue REGISTER with incremented CSeq / new branch | RFC 3261 §10.2, §17.1.2; [MS-SIPAE] §4.2 | `RegisterFlow::plan_authentication_retry` |
| Verify signed 200 response before reporting registered | [MS-SIPAE] §3.2.5.2; [MS-SIPREGE] | **Not wired**: `KerberosNegotiation` never grants authenticated state |

References:
- https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9
- https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/b3989651-d013-4481-8bb0-c4fd1678300a
- https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/82a3b226-f1e4-460f-b823-ca42cc251410
- https://www.rfc-editor.org/rfc/rfc3261
- https://www.rfc-editor.org/rfc/rfc2743

## Fail-closed decisions

1. The administrator/deployment policy supplies the approved Kerberos SPN in advance. The
   remote 401/407 **cannot** choose a new SPN or force NTLM/Digest fallback.
2. At most one Kerberos offer is accepted, and it must match that SPN exactly.
3. Challenge Call-ID, CSeq and Via branch must match the *already observed* REGISTER.
4. Retry keeps endpoint fields stable, increments CSeq and changes Via branch. Caller must
   generate the branch using a CSPRNG; this API validates syntax, not entropy.
5. This first phase supports only a **single-step completed** Kerberos GSS context and
   negotiates MS-SIPAE protocol version 3. Version 4 MIC on the first auth challenge,
   multi-round GSS continuation, NTLM and TLS-DSK token exchanges are NOT supported.
6. Tokens are bounded, routine `SipMessage` debug output is redacted, and temporary token buffers are overwritten on drop. Production tracing must separately avoid wire dumps and memory snapshots.
7. The default `NoKerberosProvider` refuses to produce tokens. Unit tests use a *fake
   offline provider* with public deterministic tokens solely to verify formatting.
8. After a syntactically correct response token, the state is only
   `AwaitingVerifiedServerResponse`, **never** `Authenticated`. A separate vetted
   GSS_VerifyMIC context and a checked REGISTER transaction are still required.

## Critical remaining dependencies

- Native Windows SSPI and cross-platform GSSAPI credential providers, integrity and
  context identity verification, plus tested principal canonicalization and clock-skew handling.
- Securely bind the security association to TLS peer identity, actual Microsoft account,
  user endpoint and protected transport; parse/sign protocol v4 as required.
- Authenticate received server responses using the negotiated GSS context, then hand off
  to REGISTER contact and GRUU validation; implement refresh, error states and provisioning.
- Perform fully authorized server/Windows client interoperability tests across SfB releases.

**No live SIP feature is enabled by this change.** `cargo run -p sfb-inspect -- --kerberos-plan-demo`
shows the intentional refusal when no provider exists.

## v0.9.0 status

A Windows-only native SSPI credential and context **token source** now exists; see
[windows-sspi-kerberos.md](windows-sspi-kerberos.md). Initial tokens with
`SEC_I_CONTINUE_NEEDED` are permitted in offline drafts, but they leave the
negotiation at `AwaitingContextContinuation` and do **not** constitute login.
There is still no native GSS MIC verification or authenticated SIP transport.
