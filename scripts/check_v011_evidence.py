#!/usr/bin/env python3
"""Tamper-evident, local-only verification of the v0.11 source evidence ledger.

This checks source files and declarations, NOT Rust compilation or SfB interop.
"""
from __future__ import annotations
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / 'docs' / 'evidence-v0.11.0.json'


def verify() -> dict:
    ledger = json.loads(LEDGER.read_text(encoding='utf-8'))
    assert ledger['version'] == '0.11.0'
    assert ledger['native_rust_compilation'] == 'not_run'
    assert ledger['windows_sspi_runtime'] == 'not_run'
    assert ledger['sfb_server_interoperability'] == 'not_run'
    assert ledger['v1_release_ready'] is False
    required_specs = {'RFC3261-17.1.2', 'RFC3261-10.2.4', 'MS-SIPAE-3.2.5.2', 'MS-SIPREGE-BASIC-REG', 'MS-SIPREGE-BASIC-UNREG'}
    assert required_specs.issubset(set(ledger['sources']))
    for name, digest in ledger['sha256'].items():
        # Original immutable v0.11 files remain archived when v0.12 edits live documentation/CI.
        archive = {'README.md': 'docs/history/v0.11.0/README.md',
                   'crates/sfb-core/src/registration.rs': 'docs/history/import-v0.14.0/crates/commscraft-core/src/registration.rs',
                   '.github/workflows/rust.yml': 'docs/history/v0.11.0/rust.yml'}
        archived = ROOT / 'docs/history/import-v0.14.0' / name
        file = ROOT / archive.get(name, str(archived.relative_to(ROOT)) if archived.is_file() else name)
        assert file.is_file() and file.resolve().is_relative_to(ROOT.resolve()), f'missing/unsafe source: {name}'
        assert hashlib.sha256(file.read_bytes()).hexdigest() == digest, f'changed file: {name}'
    code = (ROOT / 'crates/commscraft-core/src/registration.rs').read_text(encoding='utf-8')
    start = code.index('pub fn observe_mic_checked_200<')
    end = code.index('/// Returns a NEW independent transaction', start)
    block = code[start:end]
    assert block.index('candidate.receive_response') < block.index('matching_binding')
    assert block.index('matching_binding') < block.index('sa.verify_server_message')
    assert block.index('sa.verify_server_message') < block.index('self.transaction = candidate')
    assert 'untrusted_gruu' in block and 'RegistrarReportedBinding' in block
    return {'verified_files': len(ledger['sha256']), 'declared_specifications': len(ledger['sources'])}

if __name__ == '__main__':
    result = verify()
    print(f"Source ledger passed: {result['verified_files']} files; {result['declared_specifications']} spec references; NO Rust/runtime/interop claim")
