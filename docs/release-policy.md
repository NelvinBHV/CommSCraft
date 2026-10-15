# Release policy

Version 1.0 is reserved for full, independently evidenced parity with the **Skype for Business Windows Full Client**. No exceptions are permitted for rare or policy-gated functions.

### Required gates

1. `FEATURE_PARITY.json`, `WINDOWS_REFERENCE_CASES.json` and `V1_RELEASE_CONTRACT.json` complete and up to date.
2. Linux/Windows `cargo fmt`, tests and strict clippy checks passing.
3. Windows Server SE test matrix, exact server/client build, bidirectional interop, authorization, policy-off and failure/recovery paths.
4. Authenticated Kerberos or TLS-DSK security association, trusted TLS channel, microphone/camera/screen-share security review.
5. Reviewed evidence and release artifacts with checksums; no secrets or personal data.
6. `python3 scripts/check_release.py --release` passes.

Until then, publish only version 0.x experimental source snapshots. Do not label demos, fixtures, or source-only tests 'interoperable'.
