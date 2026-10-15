"""CommSCraft desktop identity regression; independent of runtime interop."""
import hashlib
from pathlib import Path
import tomllib
import unittest
from check_import_integrity import ROOT, verify


class CommSCraftIdentity(unittest.TestCase):
    def test_cargo_package_binary_and_workspace(self):
        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        app = tomllib.loads((ROOT / "apps/commscraft/Cargo.toml").read_text(encoding="utf-8"))
        self.assertIn("apps/commscraft", workspace["workspace"]["members"])
        self.assertNotIn("apps/sfb-desktop", workspace["workspace"]["members"])
        self.assertEqual(app["package"]["name"], "commscraft")
        self.assertEqual(app["bin"][0]["name"], "CommSCraft")
        self.assertEqual(app["bin"][0]["path"], "src/main.rs")
        self.assertFalse((ROOT / "apps/sfb-desktop").exists())

    def test_user_facing_title_and_start_command(self):
        source = (ROOT / "apps/commscraft/src/main.rs").read_text(encoding="utf-8")
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        self.assertIn("CommSCraft – Community Prototype", source)
        self.assertIn("cargo run -p commscraft", readme)
        self.assertNotIn("cargo run -p sfb-desktop", readme)

    def test_pristine_app_manifest_and_source_provenance(self):
        import json
        ledger = json.loads((ROOT / "docs/source-import-manifest.json").read_text())
        historic = {x["path"]: x["sha256"] for x in ledger["files"]}
        for old in ("apps/sfb-desktop/Cargo.toml", "apps/sfb-desktop/src/main.rs"):
            src = ROOT / "docs/history/import-v0.14.0" / old
            self.assertTrue(src.is_file(), old)
            self.assertEqual(hashlib.sha256(src.read_bytes()).hexdigest(), historic[old])
        self.assertEqual(verify(), 100)


if __name__ == "__main__":
    unittest.main()
