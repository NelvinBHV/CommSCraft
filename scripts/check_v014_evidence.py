#!/usr/bin/env python3
"""Tamper-evident source/spec references only; never a Rust or SfB runtime claim."""
from __future__ import annotations
import hashlib
import json
import subprocess
import sys
import tomllib
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / 'docs' / 'evidence-v0.14.0.json'


def verify():
    data = json.loads(LEDGER.read_text(encoding='utf8'))
    assert data['version'] == '0.14.0'
    assert data['rust_compiled'] is False
    assert data['rust_tests_executed'] == 0
    assert data['windows_sspi_runtime_tested'] is False
    assert data['real_sfb_interoperability_tested'] is False
    assert data['v1_release_ready'] is False
    assert len(data['requirements']) == 8
    for path, digest in data['sha256'].items():
        original = {'README.md': 'docs/history/v0.14.0/README.md',
                    'apps/sfb-desktop/Cargo.toml': 'docs/history/import-v0.14.0/apps/sfb-desktop/Cargo.toml',
                    'crates/sfb-core/src/session.rs': 'docs/history/import-v0.14.0/crates/sfb-core/src/session.rs',
                    'scripts/check_v014_evidence.py': 'docs/history/v0.14.0/check_v014_evidence.py'}
        archived = ROOT / 'docs/history/import-v0.14.0' / path
        src = (ROOT / original.get(path, str(archived.relative_to(ROOT)) if archived.is_file() else path)).resolve()
        assert src.is_relative_to(ROOT.resolve()) and src.is_file(), path
        assert hashlib.sha256(src.read_bytes()).hexdigest() == digest, path
    source = (ROOT/'crates/commscraft-core/src/session.rs').read_text(encoding='utf8')
    assert 'SessionState::Registered' not in source
    assert '(S::Registering, E::RegistrationAccepted) => S::AcceptanceUnverified' in source
    assert 'GateState::MicCheckedNotSignedIn' in source
    check = source.index('self.flow.observe_mic_checked_200(')
    commit = source.index('self.state = match result', check)
    assert check < commit
    assert 'self.state = GateState::Rejected;' in source
    assert 'pub fn disconnect(&mut self)' in source
    assert source.count('#[test]') >= 11
    core = tomllib.loads((ROOT/'crates/commscraft-core/Cargo.toml').read_text())
    assert core['package']['version'] == '0.14.0'
    assert (ROOT/'.github/workflows/registration-gate.yml').is_file()
    # Immutable historical source ledgers MUST remain verifiable.
    for name in ('check_v011_evidence.py', 'check_v012_evidence.py', 'check_v013_evidence.py'):
        subprocess.run([sys.executable, str(ROOT/'scripts'/name)], check=True,
                       capture_output=True, text=True)
    return {'file_hashes': len(data['sha256']), 'requirements': len(data['requirements']),
            'authored_rust_tests': source.count('#[test]')}

if __name__ == '__main__':
    print('v0.14 SOURCE evidence (NOT compiled, NOT interop):', verify())
