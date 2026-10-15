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

    def test_every_active_package_is_commscraft_branded(self):
        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        required = {
            "apps/commscraft": "commscraft",
            "apps/commscraft-inspect": "commscraft-inspect",
            "apps/commscraft-probe": "commscraft-probe",
            "crates/commscraft-core": "commscraft-core",
            "crates/commscraft-transport": "commscraft-transport",
            "crates/commscraft-router": "commscraft-router",
            "crates/commscraft-register-events": "commscraft-register-events",
            "tools/commscraft-xtask": "commscraft-xtask",
        }
        self.assertEqual(set(workspace["workspace"]["members"]), set(required))
        for folder, expected in required.items():
            package = tomllib.loads((ROOT / folder / "Cargo.toml").read_text(encoding="utf-8"))
            self.assertEqual(package["package"]["name"], expected)
        for former in ("apps/sfb-inspect", "apps/sfb-probe", "crates/sfb-core",
                       "crates/sfb-router", "crates/sfb-transport", "xtask"):
            self.assertFalse((ROOT / former).exists(), former)

    def test_readme_uses_new_commscraft_package_commands(self):
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        for command in (
            "cargo run -p commscraft",
            "cargo run -p commscraft-inspect",
            "cargo test -p commscraft-core",
            "cargo test -p commscraft-router",
            "cargo run -p commscraft-xtask",
        ):
            self.assertIn(command, readme)
        for legacy in ("cargo run -p sfb-inspect", "cargo test -p sfb-core",
                       "cargo test -p sfb-router", "cargo run -p xtask"):
            self.assertNotIn(legacy, readme)

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
