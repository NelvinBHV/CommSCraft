# Non-negotiable Windows Full Client parity contract

CommSCraft v1.0 must implement all applicable behaviors of an exact recorded Windows Skype for Business Full Client build against an explicitly documented supported on-premises server environment.

This includes contacts and presence; 1:1 and multi-party IM; files; devices; audio/video; Enterprise Voice; response groups and delegation; conferencing, roster, recording and Office content; screen/application sharing and remote control; Outlook/Office integration; policies, accessibility, errors and reconnection.

Acceptance requires both directions for applicable features (Windows to Rust, Rust to Windows), permissive and restrictive policy cases, real server evidence, captured/redacted outcome details, exact build identities and independent review.

The locally prepared FEATURE_PARITY.json (122 requirements), WINDOWS_REFERENCE_CASES.json (93 cases) and V1_RELEASE_CONTRACT.json must be imported intact and then expanded for uncovered cases. Counts do not indicate working functions.

Do not release based only on a parser, a GUI mock, successful Python source checks or synthetic Kerberos tokens.