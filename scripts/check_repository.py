#!/usr/bin/env python3
"""Check bootstrap repository metadata. Not a Rust or server interoperability test."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
required = [
    'README.md', 'ROADMAP.md', 'AGENTS.md', 'CONTRIBUTING.md',
    'SECURITY.md', 'LICENSE', 'NOTICE', 'ATTRIBUTION.md',
    'docs/architecture.md', 'docs/spec-conformance.md',
    'docs/windows-parity.md', 'docs/source-import.md',
]
missing = [p for p in required if not (root / p).is_file()]
if missing:
    sys.exit('Missing mandatory files: ' + ', '.join(missing))
if (root / 'Cargo.toml').exists():
    print('Rust workspace present; compile and test separately')
else:
    print('Governance bootstrap valid; full Rust source import is STILL MISSING')
print('Not proof of authentication, Microsoft protocol conformity or v1 parity')
