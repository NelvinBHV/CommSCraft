# Windows Full Client v1.0: stricter parity contract

This repository is **not** a Windows-compatible Skype for Business application yet. The
`v0.6.2` update hardens the acceptance contract and evidence validation; `v0.6.1` expands the *acceptance criteria*, not the implemented client features.

## Baseline and scope

* Reference platform: the last applicable, supported **full Skype for Business
  Windows client** connected to Skype for Business Server 2019 / Subscription Edition.
  The installed executable build and exact server build must be pinned **before**
  final acceptance. Microsoft documents a 2015/2016/2019 desktop feature matrix;
  that matrix is a **minimum** and must be supplemented by a direct behavior audit
  of the actual Windows build.
* `WINDOWS_REFERENCE_CASES.json`: 93 independent functional scenarios corresponding
  to capability groups in Microsoft's desktop feature comparison. Some scenarios
  share implementation modules but **cannot** share a blanket acceptance verdict.
* `FEATURE_PARITY.json`: 122 client requirements; each new detailed requirement is
  represented separately. Many requirements (such as security and transport)
  are necessary for parity though they are not directly in the Microsoft feature table.
* Both disabled-by-policy and enabled-by-policy behaviors must be correct.
  A missing server role, licensing restriction or incompatible deployment is not
  evidence that a required Windows-client function may be omitted.
* Inherent Windows integrations (Outlook/Exchange, call-control devices, accessibility,
  packaging, remote-control APIs) are mandatory for the Windows version. Other
  platforms may use equivalent OS integrations, but cannot be used as substitutes
  in the Windows 1.0 acceptance tests.

## Evidence required for each Windows scenario

The strict gate `python3 scripts/check_release.py --release` checks:

1. Every Windows reference scenario from the frozen list is present and maps to
   features in `FEATURE_PARITY.json`.
2. Every mapped feature is `verified`, with both unit-test output and actual
   server/client interoperability evidence files under `release/evidence/`.
3. Every individual reference scenario has a dedicated JSON result at
   `release/evidence/windows-reference/<case-id>.json`. This must describe a
   **real-server-cross-client** test, use the pinned client/server version, have
   `result: "pass"`, an execution date and a reviewer.
4. `release/interop-signoff.json` records the reference Windows client build,
   SfB server version, environment, security review and explicit approval. It
   must also attest that the **build-specific Windows feature inventory** was
   independently reviewed and parity discrepancies resolved.
5. The frozen `reference_windows_build` value in
   `WINDOWS_REFERENCE_CASES.json` matches the approved Windows client build.

All of this is necessary but not sufficient: the CI gate checks file
consistency, not that a person truly performed an interop test. Independent
review of raw captures, test logs, screenshots, server configuration and
security findings is essential. **Do not fabricate evidence files or mark
requirements verified based on mocks or test UI.**

## Feature sources

- [Microsoft: Desktop client feature comparison for Skype for Business Server 2019](https://learn.microsoft.com/en-us/skypeforbusiness/plan/feature-comparison), applies to Server 2019 and Subscription Edition; the publicly available page identifies a January 2023 content update. This is an official baseline, not an exhaustive statement of current Click-to-Run client UI behavior.
- [Microsoft: Plan for clients and devices](https://learn.microsoft.com/en-us/skypeforbusiness/plan-your-deployment/clients-and-devices/clients-and-devices) distinguishes the Full and Basic clients and discusses deployment variants.
- [MS-OCSPROT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c) provides the system-wide protocol map, to be combined with the more detailed MS- documents and normative IETF RFCs.

**Technical completeness and functional equivalence are different.** A compliant SIP
message parser does not establish Outlook interoperability, a usable remote desktop
session, or acceptable audio quality. Real human workflows must pass too.

## Additional v0.6.2 release contract

The normative `V1_RELEASE_CONTRACT.json` prevents a feature-reduced 1.0 interpretation.
It explicitly requires all visible Full Windows desktop behaviors, including integration
with Outlook/Exchange, accessibility, keyboard workflows, enterprise policy, and failure
paths. The **supported Server Subscription Edition** is the primary compatibility target;
Server 2019 can be tested as a legacy compatibility profile, not as a substitute for SE.

The executable build must be a real four-part client version, and each acceptance
artifact must match the recorded SHA-256 of the reference client executable and the
Windows OS build. A reference case requires actual files from the authorized lab and
recorded passes for Windows→Rust, Rust→Windows, policy enabled/disabled, and failure
recovery. Passing these machine checks never replaces independent human verification.
For UI-only or unidirectional workflows, document a meaningful equivalent in each
direction or explicitly record the limitation; the 1.0 release remains blocked until
all functionality parity claims are justified and reviewed. Never fabricate evidence.

**Honest status:** No real Windows client, SfB Server SE lab or Rust toolchain was
available during v0.6.2 authoring. Only the Python release-contract tests were run;
Rust code and production interoperability were not validated in this iteration.
