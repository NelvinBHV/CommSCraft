# Offline transport / REGISTER / security handoff

**Scope:** synthetic offline fixtures only. No live REGISTER, Kerberos/SSPI, TLS provenance, or authenticated sign-in.

## Invariants and test mapping

- Session ownership: the handoff holds an opaque, non-cloneable fixture handle from `OfflineTransportSessionOwner`; `rejects_reconnect_disconnect_and_foreign_owner` checks foreign owner, reconnection and disconnect.
- Transaction scope: a single sanitized SIP Via branch is retained; `rejects_missing_security_wrong_transaction_and_replay` rejects a different branch, a missing fixture security observation, and replay.
- Hostile input: `rejects_invalid_branch_at_creation` rejects CRLF injection; only bounded ASCII branch characters are accepted.
- No sign-in: `authenticated_sign_in` always returns false. A fixture MIC boolean is not evidence of a verified Kerberos MIC.

Implementation: `crates/commscraft-transport/src/offline_register_security_handoff.rs`. The API intentionally does not accept peer certificate fingerprints or claim to bind to a real socket. The current owner model uses synthetic owner identifiers and saturating generations: neither guarantees unique production identity or unforgeable provenance. An actual adapter must derive identity from the validated live TLS stream and securely bind verified SPN/MIC and REGISTER state before any sign-in claim.

## References

RFC 3261 (June 2002) §17.1.3, https://www.rfc-editor.org/rfc/rfc3261#section-17.1.3 ; RFC 5922 (June 2010) §§7.1–7.3, https://www.rfc-editor.org/rfc/rfc5922#section-7.1 ; Microsoft [MS-SIPAE] §3.2.5.2, https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ . These are design references, not interoperability certification. Publication revision must be checked before production implementation.

**Residual blockers:** no cryptographic MIC verification, no production session identity, no server interoperability. Issue #18 remains open; draft PR only; no network tests or merge.
