#!/usr/bin/env python3
"""Source-hash and documented-contract checks only: NOT a compiler or interoperability test."""
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / 'docs/evidence-v0.12.0.json'

def verify():
    data = json.loads(LEDGER.read_text(encoding='utf-8'))
    assert data['version'] == '0.12.0'
    assert data['rust_compiled'] is False and data['rust_tests_executed'] == 0
    assert data['live_tls_handshake_tested'] is False
    assert data['sfb_interoperability_tested'] is False and data['v1_release_ready'] is False
    assert len(data['requirements']) >= 7
    for name, digest in data['sha256'].items():
        p = ROOT / ("docs/history/import-v0.14.0/.github/workflows/rust.yml" if name == ".github/workflows/rust.yml" else "docs/history/import-v0.14.0/scripts/check_v012_evidence.py" if name == "scripts/check_v012_evidence.py" else name)
        if not p.is_file():
            p = ROOT / 'docs/history/import-v0.14.0' / name
        assert p.is_file() and p.resolve().is_relative_to(ROOT.resolve()), name
        assert hashlib.sha256(p.read_bytes()).hexdigest() == digest, name
    inbound = (ROOT/'crates/commscraft-core/src/inbound.rs').read_text()
    tls = (ROOT/'crates/commscraft-transport/src/lib.rs').read_text()
    cli = (ROOT/'apps/commscraft-probe/src/main.rs').read_text()
    assert 'SipStreamDecoder' in inbound and 'self.decoder.push' in inbound
    assert 'ReceiveError::Truncated' in inbound and 'ReceiveError::Malformed' in inbound
    assert 'TlsConnector::new()' in tls and 'tls.connect(peer.host(), tcp)' in tls
    assert 'native_tls' in tls and 'TlsSipReceiver::connect' in cli
    assert 'pub fn send(' not in tls and 'pub fn register(' not in tls
    assert 'pub mod inbound;' in (ROOT/'crates/commscraft-core/src/lib.rs').read_text()
    assert '"crates/commscraft-transport"' in (ROOT/'Cargo.toml').read_text()
    return {'hashed_files':len(data['sha256']), 'spec_requirements':len(data['requirements'])}

if __name__ == '__main__':
    print('v0.12 source evidence (NOT compiled):', verify())
