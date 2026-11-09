//! The system's Kerberos for the MS-SIPAE Kerberos sign-in (function 5): a [`KerberosProvider`]
//! for the registrar over GSSAPI (crate `libgssapi`, linking `libgssapi_krb5`; reviewed system
//! library behind an adapter crate, so the protocol crates stay pure Rust and free of `unsafe`).
//!
//! Evidence: `docs/protocol/evidence/03-sipae-authentication.md` (AE-25, AE-30, AE-35). Sources:
//! [MS-SIPAE] 20250520 §3.2.5.1 step 5 (p. 25: "the protocol client first requests a ticket to a
//! service named in the targetname parameter value. It then creates the context by a call to
//! GSS_Init_sec_context and requests … Integrity, Identify"), §3.2.4.1 step 3 (GSS_GetMIC,
//! RFC 2743 §2.3.1), §3.2.5.2 step 3 (GSS_VerifyMIC, RFC 2743 §2.3.2), examples §4.2–§4.3.
//!
//! Credentials: the user's ticket cache (after `kinit` or a desktop login), or – when there is no
//! usable ticket – the password typed for the sign-in (`gss_acquire_cred_with_password`). The KDC
//! and realm come from the system's Kerberos configuration (`/etc/krb5.conf`, `KRB5_CONFIG`).
//!
//! Security by design: the service principal is the challenge's `targetname` only if it is
//! `sip/` plus a DNS host name; the context asks for integrity (no delegation, no mutual
//! authentication token is expected, §4.2); the password is wiped after use; tokens and keys are
//! never printed; every GSSAPI failure is an error the registrar turns into NTLM.

use commscraft_register::{KerberosContext, KerberosProvider};
use commscraft_sipae::{Error as SipaeError, Mic};
use libgssapi::context::{ClientCtx, CtxFlags, SecurityContext};
use libgssapi::credential::{Cred, CredUsage};
use libgssapi::name::Name;
use libgssapi::oid::{OidSet, GSS_MECH_KRB5, GSS_NT_KRB5_PRINCIPAL, GSS_NT_USER_NAME};
use zeroize::Zeroizing;

/// Where the Kerberos credentials come from.
pub enum Credentials {
    /// The default ticket cache.
    Cache,
    /// The ticket cache if it holds a usable ticket for `principal`, otherwise `principal` and
    /// its password.
    Password {
        principal: String,
        password: Zeroizing<String>,
    },
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Credentials::Cache => f.write_str("Cache"),
            Credentials::Password { principal, .. } => {
                write!(
                    f,
                    "Password {{ principal: {principal:?}, password: <redacted> }}"
                )
            }
        }
    }
}

/// The registrar's Kerberos provider.
pub struct Kerberos {
    credentials: Credentials,
    cred: Option<Cred>,
}

impl Kerberos {
    pub fn new(credentials: Credentials) -> Self {
        Kerberos {
            credentials,
            cred: None,
        }
    }

    fn mechs() -> Result<OidSet, String> {
        let mut s = OidSet::new();
        s.add(GSS_MECH_KRB5).map_err(|e| e.to_string())?;
        Ok(s)
    }

    /// The initiator credentials: from the cache, else with the password.
    fn credential(&mut self) -> Result<Cred, String> {
        if let Some(c) = &self.cred {
            return Ok(c.clone());
        }
        let mechs = Self::mechs()?;
        let cred = match &self.credentials {
            Credentials::Cache => Cred::acquire(None, None, CredUsage::Initiate, Some(&mechs))
                .map_err(|e| format!("no Kerberos ticket ({e})"))?,
            Credentials::Password {
                principal,
                password,
            } => {
                let name = Name::new(principal.as_bytes(), Some(GSS_NT_USER_NAME))
                    .map_err(|e| e.to_string())?;
                let cached = Cred::acquire(Some(&name), None, CredUsage::Initiate, Some(&mechs))
                    .ok()
                    .filter(|c| c.lifetime().is_ok_and(|l| l.as_secs() > 60));
                match cached {
                    Some(c) => c,
                    None => Cred::acquire_with_password(
                        Some(&name),
                        password,
                        None,
                        CredUsage::Initiate,
                        Some(&mechs),
                    )
                    .map_err(|e| format!("Kerberos refused the credentials ({e})"))?,
                }
            }
        };
        if let Credentials::Password { password, .. } = &mut self.credentials {
            // Not needed again: later contexts use the credential.
            password.clear();
        }
        self.cred = Some(cred.clone());
        Ok(cred)
    }
}

/// `sip/` plus a DNS host name (AE-10, §2.2.1: "sip/" followed by the server's FQDN).
pub fn service_principal(target: &str) -> Option<&str> {
    let host = target.strip_prefix("sip/")?;
    let ok = !host.is_empty()
        && host.len() <= 253
        && host.contains('.')
        && host.split('.').all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    ok.then_some(target)
}

impl KerberosProvider for Kerberos {
    fn context(&mut self, target: &str) -> Result<KerberosContext, String> {
        let spn = service_principal(target).ok_or("unusable Kerberos target name")?;
        let cred = self.credential()?;
        let name =
            Name::new(spn.as_bytes(), Some(GSS_NT_KRB5_PRINCIPAL)).map_err(|e| e.to_string())?;
        let mut ctx = ClientCtx::new(
            Some(cred),
            name,
            CtxFlags::GSS_C_INTEG_FLAG,
            Some(GSS_MECH_KRB5),
        );
        let token = ctx
            .step(None, None)
            .map_err(|e| format!("no Kerberos ticket for {spn} ({e})"))?
            .map(|b| b.to_vec())
            .ok_or("Kerberos gave no token")?;
        if !ctx.is_complete() {
            // A further round trip is not part of the SIP Kerberos handshake (§4.2).
            return Err("the Kerberos context needs more than one token".into());
        }
        let lifetime = ctx.lifetime().ok().map(|d| d.as_secs());
        Ok(KerberosContext {
            token,
            mic: Box::new(KerberosMic(ctx)),
            lifetime,
        })
    }
}

/// GSS_GetMIC / GSS_VerifyMIC of an established Kerberos context (AE-30).
pub struct KerberosMic(ClientCtx);

impl std::fmt::Debug for KerberosMic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KerberosMic { context: <redacted> }")
    }
}

impl Mic for KerberosMic {
    fn get_mic(&mut self, buffer: &[u8]) -> Result<Vec<u8>, SipaeError> {
        self.0
            .get_mic(buffer)
            .map(|b| b.to_vec())
            .map_err(|_| SipaeError::WrongState)
    }

    fn verify_mic(&mut self, buffer: &[u8], token: &[u8]) -> bool {
        self.0.verify_mic(buffer, token).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only `sip/` plus a DNS host name is used as service principal.
    #[test]
    fn service_principal_names() {
        assert_eq!(
            service_principal("sip/pool.contoso.com"),
            Some("sip/pool.contoso.com")
        );
        for bad in [
            "pool.contoso.com",
            "sip/",
            "sip/localhost",
            "sip/a..b",
            "sip/pool.contoso.com@EVIL",
            "sip/pool contoso.com",
            "HTTP/pool.contoso.com",
        ] {
            assert_eq!(service_principal(bad), None, "{bad}");
        }
    }

    /// Without a ticket cache and without a reachable KDC the provider fails with a reason (the
    /// registrar then uses NTLM) instead of hanging or panicking.
    #[test]
    fn no_credentials_is_an_error() {
        // An empty cache and a realm no KDC serves (the system configuration is not used).
        let dir = std::env::temp_dir().join(format!("commscraft-gss-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let conf = dir.join("krb5.conf");
        std::fs::write(
            &conf,
            "[libdefaults]\n default_realm = INVALID.TEST\n dns_lookup_kdc = false\n dns_lookup_realm = false\n[realms]\n INVALID.TEST = {\n  kdc = 127.0.0.1:9\n }\n",
        )
        .unwrap();
        std::env::set_var("KRB5_CONFIG", &conf);
        std::env::set_var("KRB5CCNAME", format!("FILE:{}", dir.join("cc").display()));
        let mut k = Kerberos::new(Credentials::Cache);
        assert!(k.context("sip/pool.contoso.com").is_err());
        assert!(k.context("not-a-principal").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
