# Developer workflow

This project follows a clean-room, test-first model similar in organization to WordCraft, adapted for RFC- and MS-specific SIP semantics.

## Quick start

```shell
rustup toolchain install stable --component rustfmt clippy
python3 -m unittest discover -s scripts -p 'test*.py'
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

## Central CI driver

`cargo run -p xtask -- ci` runs Python specification/evidence tests, cargo formatting, unit tests and clippy, stopping at the first failure. Workflows check Linux and Windows separately. `cargo run -p xtask -- release-check` runs the hard v1.0 release gate, which must currently fail.

`cargo run -p xtask -- help` lists commands. Network test cases must be explicitly enabled in a controlled environment. Do not submit enterprise captures, secrets or session recordings. See `AGENTS.md` and `SECURITY.md`.

## Conformance versus interoperability

Every protocol change must identify a Microsoft Open Specifications revision, RFC, section and citation URL; map requirements to file/function names; add a positive and a malformed/adversarial testcase; publish outcomes as **source present**, **test executed**, or **interoperability verified** separately. A source-only check is not an interoperability test.
