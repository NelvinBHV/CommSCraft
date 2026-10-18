"""Fail closed on missing clause-linked offline DNS/REGISTER fixture names."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class DnsRegisterClauses(unittest.TestCase):
    def test_pinned_rfc_sections_and_fixture_symbols(self):
        data = json.loads((ROOT / "release/dns-register-clause-fixtures.json").read_text(encoding="utf-8"))
        rows = data["records"]
        self.assertEqual(len(rows), 8)
        self.assertEqual(len({r["id"] for r in rows}), len(rows))
        self.assertEqual(data["normative_full_feature_verified"], 0)
        self.assertEqual(data["microsoft_interoperability_verified"], 0)
        for row in rows:
            self.assertTrue(row["url"].startswith("https://www.rfc-editor.org/rfc/"))
            self.assertTrue(row["document"] and row["edition"] and row["section"] and row["ms_url"])
            self.assertTrue(row["ms_url"].startswith("https://learn.microsoft.com/"))
            self.assertTrue(row["scope"])
            source = ROOT / row["source"]
            self.assertTrue(source.is_file(), row["id"])
            text = source.read_text(encoding="utf-8")
            for field in ("positive", "negative"):
                self.assertIn("fn " + row[field] + "(", text, row["id"] + ":" + field)

if __name__ == "__main__":
    unittest.main()
