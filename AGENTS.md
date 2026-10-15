# Contributor and AI-agent requirements

CommSCraft is a clean-room interoperability project based on public Microsoft Open Specifications and IETF RFCs.

1. Cite exact document version, URL and section for every protocol change.
2. Map each normative requirement to source path, test fixture and executed test result.
3. Treat all SIP/SDP/network data as hostile; bound memory and fail closed.
4. Never disable TLS certificate validation, accept silent plaintext downgrade or log Kerberos/NTLM secrets.
5. An offline MIC, parsed 200 OK or acquired SSPI handle is not an authenticated sign-in.
6. Keep transport and OS-specific adapters separate from protocol state and user interface.
7. Run network tests only with explicit authorization and sanitized evidence.
8. Distinguish source-only, tested-offline and independently verified-interoperable status.
9. Preserve historical evidence-ledger bytes across renames; record changed hashes separately.
10. Never copy proprietary Microsoft or storytold WordCraft code, art or brand assets.
11. v1.0 requires verified parity for every Windows Full Client feature, including policy-denied cases and recovery.
12. GitHub Actions may run only on standard GitHub-hosted `ubuntu-latest` / `windows-latest` runners. Do not use paid larger, custom or self-hosted runners or paid external CI services. Enforce via `scripts/check_standard_runners.py`; obtain explicit approval before any potentially chargeable runner change.
