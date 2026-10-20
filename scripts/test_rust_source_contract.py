"""Negative fixtures for layout-independent Rust source contract matching."""
import unittest

from rust_source_contract import code_without_layout


class SourceContractScannerTests(unittest.TestCase):
    def test_formatting_preserves_executable_guard(self):
        self.assertIn("ifstatus!=SEC_E_OK{returnErr(());}", code_without_layout(
            "if status != SEC_E_OK {\n return Err(());\n }"
        ))

    def test_comments_and_literals_cannot_satisfy_positive_contract(self):
        source = (
            "// if status != SEC_E_OK { return Err(()); }\n"
            "const NOTE: &str = \"if status != SEC_E_OK { return Err(()); }\";\n"
            "let raw = r#\"tls.peer_certificate()\"#;"
        )
        compact = code_without_layout(source)
        self.assertNotIn("ifstatus!=SEC_E_OK{returnErr(());}", compact)
        self.assertNotIn("tls.peer_certificate()", compact)

    def test_nested_comments_cannot_satisfy_contract(self):
        self.assertNotIn("verify_server_message()", code_without_layout(
            "/* a /* verify_server_message() */ b */ let x=1;"
        ))

    def test_unterminated_comment_and_literal_fail_closed(self):
        for source in ("/* missing", 'r##"missing', '"missing'):
            with self.subTest(source=source), self.assertRaises(ValueError):
                code_without_layout(source)


if __name__ == "__main__":
    unittest.main()
