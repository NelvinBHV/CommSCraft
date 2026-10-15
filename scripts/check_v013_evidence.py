#!/usr/bin/env python3
"""Source SHA-256 and frozen MS/RFC mapping checks; NOT Rust/interop test execution."""
from __future__ import annotations
import hashlib
import json
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / 'docs/evidence-v0.13.0.json'


def verify():
    ledger = json.loads(LEDGER.read_text(encoding='utf8'))
    assert ledger['version'] == '0.13.0'
    assert ledger['rust_compiled'] is False and ledger['rust_tests_executed'] == 0
    assert ledger['server_interoperability_tested'] is False
    assert ledger['v1_release_ready'] is False
    assert len(ledger['requirements']) >= 8
    for name, expected in ledger['sha256'].items():
        original = {'Cargo.toml': 'docs/history/v0.13.0/Cargo.toml',
                    'scripts/check_v013_evidence.py': 'docs/history/v0.13.0/check_v013_evidence.py',
                    'crates/sfb-router/src/lib.rs': 'docs/history/import-v0.14.0/crates/commscraft-router/src/lib.rs',
                    'crates/sfb-core/src/transaction.rs': 'docs/history/import-v0.14.0/crates/commscraft-core/src/transaction.rs'}
        archived = ROOT / 'docs/history/import-v0.14.0' / name
        file = (ROOT/original.get(name, str(archived.relative_to(ROOT)) if archived.is_file() else name)).resolve()
        assert file.is_relative_to(ROOT.resolve()) and file.is_file(), name
        assert hashlib.sha256(file.read_bytes()).hexdigest() == expected, name
    manifest = tomllib.loads((ROOT/'Cargo.toml').read_text())
    assert 'crates/commscraft-router' in manifest['workspace']['members']
    assert tomllib.loads((ROOT/'crates/commscraft-router/Cargo.toml').read_text())['package']['version']=='0.13.0'
    assert 'pub fn from_message' in (ROOT/'crates/commscraft-core/src/transaction.rs').read_text()
    core = (ROOT/'crates/commscraft-core/src/transaction.rs').read_text()
    assert 'pub transport: String' in core and 'version.to_ascii_uppercase()' in core
    router = (ROOT/'crates/commscraft-router/src/lib.rs').read_text()
    for token in ('MAX_IN_FLIGHT: usize = 64', 'MAX_RECENTLY_CLOSED: usize = 128',
                  'RouterError::DuplicateIdentity', 'RouterError::DuplicateOrLateResponse',
                  'self.remember(', 'TransactionEvent::Final', 'pub fn disconnect('):
        assert token in router, token
    assert 'TcpStream' not in router and 'TlsStream' not in router
    assert 'pub fn send(' not in router and 'pub fn register(' not in router
    tests = (ROOT/'crates/commscraft-router/tests/offline_wire.rs').read_text()
    for test in ('framed_tls_like_stream_routes_two_replies_without_interleaving',
                 'duplicate_final_from_coalesced_stream_is_not_delivered_twice',
                 'malformed_response_does_not_consume_tracked_request'):
        assert f'fn {test}' in tests
    return {'hashed_files': len(ledger['sha256']), 'requirements': len(ledger['requirements'])}

if __name__ == '__main__':
    print('v0.13 source evidence (NOT Rust runtime/interop):', verify())
