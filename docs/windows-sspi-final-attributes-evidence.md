# CommSCraft Windows Kerberos SSPI final attributes — conformance evidence

**Scope:** Hardening of the existing Windows-only Kerberos SSPI token provider, not new productive sign-in support. Windows-specific unit tests require a compiled Windows GitHub Actions job. No real SIP server, domain credential, or Kerberos token is needed to execute these negative tests.

| ID | Requirement and normative source | Code symbol | Executable test |
| --- | --- | --- | --- |
| SSPI-FLAG-01 | Microsoft [InitializeSecurityContext (Kerberos)](https://learn.microsoft.com/en-us/windows/win32/secauthn/initializesecuritycontext--kerberos): security context attributes are **not definitive until a final successful return**; intermediate values can change | `validate_final_context_attributes` defers validation for `SEC_I_CONTINUE_NEEDED` and `SEC_I_COMPLETE_AND_CONTINUE` | `intermediate_sspi_legs_do_not_require_final_security_attributes` |
| SSPI-FLAG-02 | Microsoft [Context Requirements](https://learn.microsoft.com/en-us/windows/win32/secauthn/context-requirements): compare requested `ISC_REQ_*` services against the actual `ISC_RET_*` flags; reject missing security attributes | `validate_final_context_attributes` requires mutual auth, integrity, replay, sequence and identify on the final leg | `final_leg_requires_all_returned_security_attributes` |
| SSPI-FLAG-03 | [RFC 4121 §4.1.1.1](https://www.rfc-editor.org/rfc/rfc4121#section-4.1.1.1): GSS context flags include mutual auth, integrity, replay and sequence. Final returned attributes must support intended protection rather than relying on requested options alone | `WindowsKerberosProvider::step`, `REQUIRE` | `final_leg_requires_all_returned_security_attributes` |
| SSPI-FLAG-04 | Microsoft [InitializeSecurityContext (Kerberos)](https://learn.microsoft.com/en-us/windows/win32/secauthn/initializesecuritycontext--kerberos): failed SSPI status cannot establish a trusted context; incomplete context must be deleted on failure | `validate_final_context_attributes`, `WindowsKerberosProvider::step` and RAII `Drop` | `rejected_sspi_status_can_never_pass_attribute_gate` |
| SSPI-FLAG-05 | [MS-SIPAE §§3.2.5.1–3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c): server authentication and subsequent MIC/replay checks remain mandatory | `WindowsKerberosProvider` is token/MIC backend **only**, it does not authorize registration | Existing `RegisterVerificationGate` and `gated_presence` tests |

## Trust boundaries and limitations

- `ISC_RET_MUTUAL_AUTH` indicates the result of a security-context policy negotiation, **not** proof that a particular remote Skype for Business TLS endpoint, SIP transaction, or user session has been independently verified. Microsoft explicitly cautions against equating the requested flag with actual mutual authentication.
- The required `IDENTIFY` flag is a pre-existing conservative CommSCraft policy. The change requires it only at successful completion, together with the other requested protections. A server/context that cannot provide all these services fails closed.
- `REPLAY_DETECT` / `SEQUENCE_DETECT` are SSPI per-message services; they **do not replace** the MS-SIPAE signed-`snum` replay window.
- Native SSPI, Windows credential acquisition, SIP REGISTER token exchange and TLS identity binding have **not** been tested against a real server. All new tests are offline flag checks.
- The pre-change source is preserved at `docs/history/import-v0.14.0/crates/sfb-core/src/windows_sspi.rs`; its original SHA-256 remains in the existing `docs/source-import-manifest.json` and is checked without modifying historical ledgers.
- **CI required:** `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, historical SHA-256 checks, on standard GitHub-hosted `ubuntu-latest` and `windows-latest` only. Test names above are authored source, not a passed-test claim until GitHub Actions confirms.

The CommSCraft v1.0 full Windows Full Client parity release gate stays **blocked**.
