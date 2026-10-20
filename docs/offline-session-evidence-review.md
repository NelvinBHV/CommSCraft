# Offline session evidence adapter security review

Status: design only, not live authentication. No network access or sign-in.

## Trust decisions

- A fixture generation counter is not an unforgeable connection identity. A future live adapter must own a handle tied to the same TLS stream and invalidate it on reconnect.
- A synthetic certificate fingerprint is not PKIX verification. The live transport must verify the real peer certificate on its validated stream against the originating SIP domain.
- A MIC boolean is not Kerberos server verification. A separately reviewed security-association adapter must validate the MIC and replay state.
- A Via branch correlates a SIP transaction, not authentication. A future transaction layer must create and own the REGISTER handle.
- No result of this fixture adapter authorizes sign-in. Issue #18 remains open.

## Evidence mapping

Source: `crates/commscraft-core/src/offline_session_adapter.rs`. Negative tests: `fixture_adapter_rejects_unverified_mic_replay_expiry_and_disconnect` and `fixture_adapter_rejects_mismatched_identity_and_transaction`. Exact-head CI pending.

Normative references: RFC 5922 (June 2010) sections 7.1-7.3, https://www.rfc-editor.org/rfc/rfc5922#section-7.1 ; RFC 3261 (June 2002) section 17.1.3, https://www.rfc-editor.org/rfc/rfc3261#section-17.1.3 ; Microsoft MS-SIPAE section 3.2.5.2, https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ ; Microsoft MS-CONMGMT section 3.1.5, https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/ . Verify Microsoft publication revision before production implementation.
