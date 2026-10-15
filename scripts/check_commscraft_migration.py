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
assert 'CommSCraft' in (ROOT/'apps/commscraft/src/main.rs').read_text()
import tomllib
app = tomllib.loads((ROOT/'apps/commscraft/Cargo.toml').read_text())
assert app['package']['name'] == 'commscraft'
assert app['bin'][0]['name'] == 'CommSCraft'
assert 'apps/commscraft' in tomllib.loads((ROOT/'Cargo.toml').read_text())['workspace']['members']
assert not (ROOT/'apps/sfb-desktop').exists()
for old in ('crates/sfb-core', 'crates/sfb-router', 'crates/sfb-transport', 'apps/sfb-inspect', 'apps/sfb-probe', 'xtask'):
    assert not (ROOT/old).exists(), old
for new in ('crates/commscraft-core', 'crates/commscraft-router', 'crates/commscraft-transport', 'apps/commscraft-inspect', 'apps/commscraft-probe', 'tools/commscraft-xtask'):
    assert (ROOT/new/'Cargo.toml').is_file(), new
print('CommSCraft migration source hashes verified (not a build/interop proof)')
