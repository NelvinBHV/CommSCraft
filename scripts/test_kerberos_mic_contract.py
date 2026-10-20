"""Source-level checks. Not a Rust build, SSPI runtime or interoperability test."""
from pathlib import Path
import unittest
from rust_source_contract import code_without_layout

ROOT = Path(__file__).resolve().parent.parent


def source(name):
    return (ROOT / 'crates/commscraft-core/src' / name).read_text(encoding='utf-8')


class KerberosMicSourceContract(unittest.TestCase):
    def test_windows_ffi_declares_required_native_apis(self):
        win = source('windows_sspi.rs')
        for api in ('QueryContextAttributesW', 'MakeSignature', 'VerifySignature',
                    'SEC_E_OK', 'SECBUFFER_DATA', 'SECBUFFER_TOKEN',
                    'SECPKG_ATTR_SIZES'):
            with self.subTest(api=api):
                self.assertIn(api, win)

    def test_native_mic_bounded_and_context_gated(self):
        win = source('windows_sspi.rs')
        for condition in ('!self.context_complete()', 'sizes.max_signature == 0',
                          'sizes.max_signature as usize > MAX_MIC_TOKEN',
                          'message.len() > MAX_MIC_INPUT', 'token.len() > MAX_MIC_TOKEN',
                          'if status != SEC_E_OK || qop != 0',
                          'if status != SEC_E_OK || buffers[1].size == 0'):
            with self.subTest(condition=condition):
                self.assertIn(code_without_layout(condition), code_without_layout(win))

    def test_sspi_signing_does_not_use_ntlm_or_fallback(self):
        win = source('windows_sspi.rs')
        self.assertIn('"Kerberos\\0"', win)
        self.assertNotIn('"Negotiate\\0"', win)
        self.assertNotIn('"NTLM\\0"', win)
        self.assertIn('impl KerberosMicBackend for WindowsKerberosProvider', win)

    def test_sa_identity_bound_before_native_operations(self):
        bridge = source('kerberos_mic.rs')
        self.assertIn('backend.target_spn() != Some(sa.targetname.as_str())', bridge)
        self.assertIn('sa.scheme != "Kerberos"', bridge)
        self.assertIn('!backend.complete()', bridge)
        self.assertIn('RefCell<B>', bridge)

    def test_backend_not_production_fake(self):
        bridge = source('kerberos_mic.rs')
        self.assertIn('#[cfg(test)]\nmod tests {', bridge)
        self.assertEqual(bridge.count('struct FakeBackend'), 1)
        self.assertNotIn('pub struct FakeBackend', bridge)

    def test_mic_before_replay_and_no_state_on_bad_mic(self):
        sec = source('security.rs')
        fn = sec.split('pub fn verify_server_message', 1)[1].split('/// Hold SAs separately', 1)[0]
        self.assertLess(fn.index('verifier.verify(&signed, &info.signature)'),
                        fn.index('self.incoming.check(info.snum)'))
        self.assertLess(fn.index('self.incoming.check(info.snum)'),
                        fn.index('self.incoming.record(info.snum)'))

    def test_existing_security_fixture_tracks_gssapi_field(self):
        sec = source('security.rs')
        self.assertIn('sts_uri:None,gssapi_data:None', code_without_layout(sec))

    def test_reliably_no_live_transport_enablement(self):
        bridge = source('kerberos_mic.rs')
        self.assertNotIn('TcpStream', bridge)
        self.assertNotIn('connect(', bridge)
        self.assertNotIn('REGISTER sip:', bridge)


if __name__ == '__main__':
    unittest.main()
