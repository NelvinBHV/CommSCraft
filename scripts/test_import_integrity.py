import unittest
from pathlib import Path
import tempfile
import shutil
import json
import hashlib
from check_import_integrity import verify, ROOT

class ImportEvidence(unittest.TestCase):
    def test_all_historical_files(self):
        self.assertEqual(verify(), 100)

    def test_modified_file_fails_closed(self):
        with tempfile.TemporaryDirectory() as temp:
            dst = Path(temp)
            (dst/"docs").mkdir()
            (dst/"docs/history/import-v0.14.0").mkdir(parents=True)
            (dst/"docs/history/import-v0.14.0/Cargo.toml").write_text("wrong")
            ledger = {"file_count": 100, "files": [{"path": "Cargo.toml", "sha256": "0"*64} for _ in range(100)]}
            (dst/"docs/source-import-manifest.json").write_text(json.dumps(ledger))
            with self.assertRaises(AssertionError):
                verify(dst)
