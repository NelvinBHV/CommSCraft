# CommSCraft

**Independent, clean-room Rust client project targeting Skype for Business Windows Full Client feature parity.**

Inspired by the modular project structure, agent-friendly development approach, and open-source workflows of [storytold/wordcraft](https://github.com/storytold/wordcraft). CommSCraft is not a fork of WordCraft and is not affiliated with Microsoft.

> **Experimental v0.14.0 prototype.** The existing Rust workspace is prepared for import and has NOT yet been fully published in this GitHub repository. No live authenticated sign-in, media calling, interoperable messaging or Windows-client parity has been demonstrated.

## Tour

- [Architecture](docs/architecture.md): apps, crates and security boundaries.
- [Protocol traceability](docs/spec-conformance.md): Microsoft Open Specifications and IETF RFC references with evidence standards.
- [Windows Full Client parity](docs/windows-parity.md): mandatory v1.0 acceptance conditions.
- [Roadmap](ROADMAP.md): source import, security, communications and integration milestones.
- [Source import](docs/source-import.md): preserve all historical Rust files and immutable evidence.
- [Contributing](CONTRIBUTING.md), [Agent rules](AGENTS.md), [Security](SECURITY.md), [Attribution](ATTRIBUTION.md).

## Status

| Area | Reality |
| --- | --- |
| SIP parsing, routing, REGISTER and SSPI-adapter source | Local v0.14.0 prototype; GitHub import pending |
| Real Skype for Business sign-in and presence | Not interoperably implemented |
| Real chat, calling, media, conferencing and sharing | Not interoperably implemented |
| Windows Full Client parity | Not verified; v1.0 must remain blocked |

## Developer quick start (AFTER source import)

    git clone https://github.com/NelvinBHV/CommSCraft.git
    cd CommSCraft
    cargo test --workspace --all-targets
    cargo run -p sfb-desktop
    python3 scripts/check_release.py --release

These commands are not yet runnable from GitHub until the complete Rust workspace is imported. A v1 release must fail unless the final release check succeeds with real-server and Windows-reference evidence.

## Specification basis

- [RFC 3261: SIP](https://www.rfc-editor.org/rfc/rfc3261)
- [RFC 3263: SIP server discovery](https://www.rfc-editor.org/rfc/rfc3263)
- [MS-OCSPROT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/)
- [MS-SIPAE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/)
- [MS-SIPREGE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/)

See [LICENSE](LICENSE) and [NOTICE](NOTICE).