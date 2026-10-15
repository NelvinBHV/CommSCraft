# Security policy

CommSCraft is experimental research code, NOT production-ready. Do not use it with real work accounts or networks without explicit authorization.

Never publish access tokens, NTLM/Kerberos credentials, contact lists, private SIP captures, recordings or secrets in public issues.

Report security issues privately to the maintainer via GitHub security advisories if enabled.

Must fail closed on certificate or hostname mismatch, wrong server principal, invalid MIC, replay, malformed Content-Length, oversized messages and unexpected transport changes. No plaintext downgrade.

v1.0 requires independent security review and actual interoperable Windows Full Client behavior.