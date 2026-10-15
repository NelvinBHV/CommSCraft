"""Source-level safety contract checks (NOT compilation or Windows tests)."""
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parent.parent


class WindowsSspiSourceContract(unittest.TestCase):
    def test_windows_module_gated(self):
        library = (ROOT / 'crates/commscraft-core/src/lib.rs').read_text()
        self.assertIn('#[cfg(windows)]\npub mod windows_sspi;', library)

    def test_explicit_kerberos_not_negotiate(self):
        source = (ROOT / 'crates/commscraft-core/src/windows_sspi.rs').read_text()
        self.assertIn('"Kerberos\\0"', source)
        self.assertIn('AcquireCredentialsHandleW', source)
        self.assertIn('InitializeSecurityContextW', source)
        self.assertIn('DeleteSecurityContext', source)
        self.assertIn('FreeCredentialsHandle', source)
        self.assertIn('SEC_I_CONTINUE_NEEDED', source)
        self.assertNotIn('"Negotiate\\0"', source)
        self.assertNotIn('"NTLM\\0"', source)

    def test_context_not_authentication(self):
        core = (ROOT / 'crates/commscraft-core/src/kerberos.rs').read_text()
        self.assertIn('AwaitingContextContinuation', core)
        self.assertIn('else { KerberosPhase::AwaitingContextContinuation }', core)
        self.assertIn('fn incomplete_gss_context_produces_initial_token_without_claiming_authentication', core)


if __name__ == '__main__':
    unittest.main()
