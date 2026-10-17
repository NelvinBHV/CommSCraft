# v0.30.0 authorized interoperability acceptance procedure

**Current status:** NOT EXECUTED. An offline Rust test, a verified X.509 parsing fixture, an HTTP/SIP response, or a green CI workflow is **not** a successful authenticated Skype for Business sign-in. This is a human-operated acceptance procedure, not an instruction to contact production networks.

## Preconditions — actions for an authorized test owner

1. Provide an **isolated**, expressly authorized Skype for Business Server Subscription Edition environment and Windows test client; designate the authorized operator and independent security reviewer. Do not enter company production credentials or expose test addresses, contacts, or captures in public GitHub.
2. Record precise four-component server/client Windows builds, CommSCraft Git commit SHA, configuration/policy state, server FQDN, original SIP sign-in domain and TLS-SRV target in a **private** evidence inventory. Public evidence must be sanitized.
3. Create disposable test identities using only administrator-issued test accounts. Prepare test cases for correct/wrong/expired/revoked certificate, wrong SIP reference domain, incorrect SPN, MIC failure/replay and forced connection drop.
4. Have a human independently review the intended security behavior and the report before any milestone `passed` state is entered. Use the standard GitHub-hosted `ubuntu-latest` and `windows-latest` CI only.

## Offline implementation review (can be performed without a server)

At the exact integration SHA, run:

```bash
python3 scripts/check_standard_runners.py
python3 -m unittest discover -s scripts -p "test*.py"
python3 scripts/check_v030_milestone.py
python3 scripts/check_v030_milestone.py --release
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

The fourth command is **expected to fail closed** while milestone evidence is pending. The rustfmt check may be affected by tracked historical source-import formatting debt (Issue #9); do not silently mark it green. Record each actual exit code and report rather than replacing negative evidence.

## Authorized live protocol cases (NOT executed by this repository change)

| Case | Required observation | Fail-closed behavior |
| --- | --- | --- |
| Original SIP domain vs SRV target | TLS verification uses original domain and cert from the actual TLS peer, never a DNS-controlled SRV hostname identity | Certificate-domain mismatch aborts; no plaintext downgrade |
| Certificate trust chain/SAN | Platform PKIX/trust and exact SIP SAN requirements are both checked | Untrusted/expired/wrong-EKU/malformed/ambiguous SAN cannot authorize a session |
| SPN and authentication | Server service principal is pinned to this connection and verified in the authenticated REGISTER/SIPAE flow | Wrong principal, missing MIC, altered MIC and replay never produce authenticated state |
| Connection and transport changes | Security association and authenticated state are tied to the verified TLS connection | Drop/reconnect/redirect does not reuse stale authorization |
| Negative policies and recovery | Appropriate denial and reattempt/recovery are observed with reference-client parity | Policy denial cannot be mistaken for successful sign-in |

**Do not infer live results from local `TlsSipReceiver` tests:** the transport currently exposes a receive-only diagnostic, not complete Windows SSPI-to-TLS authenticated REGISTER integration. Implement and review that connection-scoped state machine before claiming the live cases above are met.

## Evidence review and completion

For each gate in `release/v030-milestone.json`, supply its exact Git commit SHA, concrete GitHub Actions job/run URL for CI, privately archived/sanitized reference evidence, dated human reviewer identity and decision. The independent reviewer should validate underlying artifacts and negative outcomes, not just the JSON syntax. If any test fails, remains unavailable or was not authorized, leave `outcome: pending` or `failed` and **do not tag v0.30.0**.

Public normative references: [RFC 3261 (June 2002), §§19.1.1/25.1](https://www.rfc-editor.org/rfc/rfc3261); [RFC 5922 (June 2010), §§7.1–7.3](https://www.rfc-editor.org/rfc/rfc5922); [RFC 5280 (May 2008), §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280); official Microsoft MS-SIPAE/MS-CONMGMT specifications must be pinned to the exact published revision before claiming interoperability. No network tests were run while writing this procedure.
