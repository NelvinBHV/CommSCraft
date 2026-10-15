#!/usr/bin/env python3
"""Verify preserved legacy hashes after public-project rebranding. Not an interop test."""
from pathlib import Path
import hashlib, json
ROOT=Path(__file__).resolve().parent.parent
for version, source, archived in [
    ('0.13.0', 'Cargo.toml', 'docs/history/v0.13.0/Cargo.toml'),
    ('0.14.0', 'README.md', 'docs/history/v0.14.0/README.md'),
]:
    ledger=json.loads((ROOT/f'docs/evidence-v{version}.json').read_text())
    got=hashlib.sha256((ROOT/archived).read_bytes()).hexdigest()
    assert got==ledger['sha256'][source], (version, source)
assert (ROOT/'README.md').read_text().startswith('# CommSCraft\n')
assert 'NelvinBHV/CommSCraft' in (ROOT/'Cargo.toml').read_text()
assert 'CommSCraft' in (ROOT/'apps/sfb-desktop/src/main.rs').read_text()
print('CommSCraft migration source hashes verified (not a build/interop proof)')
