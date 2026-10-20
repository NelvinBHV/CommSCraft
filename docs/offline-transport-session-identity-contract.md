# Offline transport-owned session identity contract

**Status:** offline fixture model only. No real TLS session identity, authentication, Kerberos, REGISTER transmission, or sign-in.

## Security invariants

1. Only the transport owner issues a handle. The handle is non-cloneable and its fields are private; this is API-level ownership, not cryptographic unforgeability.
2. Each fixture reconnect advances the owner's generation. Previous handles and handles from other owners fail closed.
3. Disconnect revokes the active fixture handle. No fixture authorizes sign-in.
4. A live implementation must bind an opaque transport-internal handle to the actual platform-verified TLS stream, peer certificate and original SIP domain; a caller-supplied counter or fingerprint cannot establish that provenance.
5. REGISTER transaction state and independently verified Kerberos SPN/MIC evidence must subsequently be tied to the same live connection by separately reviewed adapters. This PR does not add those adapters.

## Source, tests and references

Source: `crates/commscraft-transport/src/offline_session_identity.rs`.
Tests: `rejects_cross_owner_and_reconnect_stale_handles` (cross-owner, reconnect, disconnect) and `rejects_zero_owner_and_disconnected_session` (invalid owner and disconnected state). CI evidence to be recorded after exact-head Actions finish.

RFC 5922 (June 2010) sections 7.1-7.3: https://www.rfc-editor.org/rfc/rfc5922#section-7.1 . RFC 3261 (June 2002) section 17.1.3: https://www.rfc-editor.org/rfc/rfc3261#section-17.1.3 . Microsoft MS-SIPAE section 3.2.5.2: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ . Microsoft MS-CONMGMT section 3.1.5: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/ . Publication revision must be verified before production integration.

**Residual risk:** synthetic owner identifiers can collide and are not secure identities. Production identity provenance and authorized interoperability evidence remain blocked by Issue #18. Draft only; no merge.
