# Contributing

Use small Pull Requests with: intended Windows reference behavior, affected Rust crate, exact RFC/MS Open Specifications citation, positive and negative tests, security impact, executed test results and limitations.

Tests against a real enterprise server must be authorized and opt-in; sanitize evidence and avoid publishing tokens and chat data.

After source import, run cargo fmt --all -- --check, cargo test --workspace --all-targets, cargo clippy --workspace --all-targets -- -D warnings, and python3 -m unittest discover -s scripts -p test*.py.

Read AGENTS.md and SECURITY.md before submitting a change.