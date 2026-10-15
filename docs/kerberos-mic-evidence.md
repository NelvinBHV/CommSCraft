# Windows Kerberos per-message MIC — v0.10.0 evidence ledger

**Status: implementation + source-contract proof only. Uncompiled/unverified Windows native path. No SfB sign-in.**

The bridge converts MS-SIPAE's ordered, case-preserving SIP MIC buffer into an
opaque native Kerberos MIC token using the *same* `WindowsKerberosProvider`
context that was established for the pinned service principal. It is not a
replacement for Microsoft SIPAE authentication, Kerberos exchange, registration,
service-discovery trust, or the full Windows desktop client.

## Normative requirement → code → check

| ID | Primary normative source | Implementation and security property | Local check | Independent Windows/interop evidence |
|---|---|---|---|---|
| MIC-01 | [MS-SIPAE §3.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/71b7e07d-505f-4ca6-9577-0a27bd12b0e9) steps 4–6; [RFC 2743 §2.2](https://www.rfc-editor.org/rfc/rfc2743) | `windows_sspi.rs::step` requires pinned `sip/<FQDN>` and does not claim signed-in on `CONTINUE_NEEDED` | Existing `kerberos.rs` tests / `test_windows_sspi_contract.py` | **PENDING** (AD Kerberos realm) |
| MIC-02 | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) steps 1–3 | `security.rs::parse_authentication_info` matches scheme/realm/target/source, and `server_mic_buffer` builds MS's exact ordered fields without case folding | `security.rs` tests + `tests/fixtures/sip/signed-register-ok.sip` | **PENDING** (real signed response) |
| MIC-03 | [RFC 2743 §2.3.2](https://www.rfc-editor.org/rfc/rfc2743); [SSPI VerifySignature](https://learn.microsoft.com/en-us/windows/win32/api/sspi/nf-sspi-verifysignature) | `windows_sspi.rs::native_verify_mic` uses `SECBUFFER_DATA`+`SECBUFFER_TOKEN`, only with completed Kerberos context, rejects non-`SEC_E_OK` and abnormal QOP | `test_kerberos_mic_contract.py` + `kerberos_mic.rs` synthetic mock test | **PENDING** (native Windows SSPI test) |
| MIC-04 | [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) steps 4–6 | `security.rs::verify_server_message`: MIC before 257-entry replay window; bad MIC cannot establish SA or consume snum | `test_kerberos_mic_contract.py` + `security.rs` replay/tamper tests | **PENDING** (server traffic) |
| MIC-05 | [MS-SIPAE §3.2.4.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/8c2ae856-5438-42b5-8d08-5746b88e707f); [RFC 2743 §2.3.1](https://www.rfc-editor.org/rfc/rfc2743); [SSPI MakeSignature](https://learn.microsoft.com/en-us/windows/win32/api/sspi/nf-sspi-makesignature) | `security.rs::client_mic_buffer` prepares outgoing signed fields and nonce/counter; `windows_sspi.rs::native_get_mic` uses native package token. | `security.rs` exact-order tests + `kerberos_mic.rs` mock tests | **PENDING** (native token compatibility) |
| MIC-06 | [SSPI QueryContextAttributesW](https://learn.microsoft.com/en-us/windows/win32/api/sspi/nf-sspi-querycontextattributesw) / [SecPkgContext_Sizes](https://learn.microsoft.com/en-us/windows/win32/api/sspi/ns-sspi-secpkgcontext_sizes) | Native max-signature query; fail if missing/zero/over 512 bytes. Sign/verify input limited to 64 KiB. | `test_kerberos_mic_contract.py` | **PENDING** (Windows ABI compile/run) |
| MIC-07 | [MS-SIPAE §4.2 Kerberos example](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/b3989651-d013-4481-8bb0-c4fd1678300a); [RFC 3261 §17.1.3](https://www.rfc-editor.org/rfc/rfc3261) | Existing `kerberos.rs` REGISTER challenge transaction binding + `kerberos_mic.rs::bind` checks a complete context and pinned SA target; cannot switch to NTLM silently. | `kerberos.rs` tests, `kerberos_mic.rs` tests | **PENDING** (registration loop/Windows client) |

The SSPI per-message `MessageSeqNo` is set to `0` because this experimental
SIP adapter supplies no *additional* SSPI transport sequence counter. The
Microsoft `snum`/`cnum` fields are included in the MIC *payload* and checked
separately. This assumption requires **real Kerberos-to-SfB interop testing**;
we do not claim it is universally accepted by all servers.

## Evidence classes

- **Source inspection:** `python3 -m unittest discover -s scripts -p 'test_*.py'` validates the declared API, bounds, SA linkage, and fail-closed logic. This does **not** execute Rust.
- **Rust tests (authored; not executed here):** `cargo test -p commscraft-core kerberos_mic`; `cargo test --workspace`.
- **Native Windows build/test (not executed here):** GitHub Actions `windows-build` job and a domain-joined Windows integration environment.
- **Protocol interoperability (not executed):** compare real Kerberos signed REGISTER/authenticated 200, invalid MIC, replay, 401/407, time skew, failed KDC, proxy route, SA expiry/refresh with an authorized SfB Server SE lab and pinned Windows Full client. Capture sanitized traces with verified checksums and service identity.

## Critical non-claims

1. A native SSPI `MakeSignature` result does not authenticate the SIP peer, and a `VerifySignature` result does not necessarily validate the SIP transaction or TLS hostname. All those checks are separate.
2. An authenticated Kerberos context is distinct from a verified MS-SIPAE `Authentication-Info` header and from a REGISTER acceptance. A `200 OK` alone is insufficient.
3. Only the Windows Kerberos MIC primitive is added; no NTLM/TLS-DSK handshake, production SIP transport, subscriptions, calls, conference, sharing, or full Windows client parity is claimed.
4. Tests backed by a synthetic mock *are not cryptographic proof*. Native tests and server traces are still required.
