# Historical source import

The existing v0.14.0 CommSCraft Rust workspace is available as a separate local ZIP artifact and is NOT YET present in this GitHub repository.

Before claiming a working project, import apps/, crates/, xtask/, Cargo.toml, scripts/, tests/, all feature matrices and original docs/evidence files. Exclude build outputs, Python caches, secrets, enterprise traffic and debug logs.

Preserve historical source hashes: original renamed bytes belong in immutable docs/history paths with explicit migration mapping; never rewrite original evidence digests.

Reconcile this README and other bootstrap files with the original, richer source documentation. Internal sfb-* Cargo package names can remain while the public project name is CommSCraft.

Run migration source tests and Rust Linux/Windows CI after import. Real Skype for Business server interoperability requires separate authorized tests.