# CommSCraft — GitHub Actions cost policy

**Only standard GitHub-hosted runners** are permitted for CommSCraft CI.

- Allowed `runs-on` labels: `ubuntu-latest` and `windows-latest`.
- Allowed matrix selection: `${{ matrix.os }}` with the `os` matrix restricted to those labels.
- Forbidden: GitHub larger runners, self-hosted runners, custom runner groups, paid third-party CI/build services, dynamic runner expressions or external reusable job workflows that bypass the policy.
- All CI, Rust tests, RFC/MS evidence checks and release checks must use this same standard-runner restriction.
- `python3 scripts/check_standard_runners.py` checks repository workflows and fails when the permitted runner configuration changes. Governance CI runs this check for pull requests.
- Keep the GitHub repository **public** if relying on the public-repository standard-runner billing policy. If visibility, GitHub billing rules, or runner settings change, **review costs before triggering more builds**.
- Do not enable any paid features or purchase usage for CommSCraft without explicit user approval.

This is a runner-configuration safeguard, not a live billing meter; GitHub account billing must still be reviewed separately. Existing RFC 3261, MS-SIPAE and MS-SIPREGE tests and evidence requirements remain mandatory.
