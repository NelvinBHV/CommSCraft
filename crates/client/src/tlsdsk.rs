//! TLS-DSK (function 6): where an account's user certificate is kept and how it is obtained from
//! the certificate provisioning service ([MS-OCAUTHWS] GetAndPublishCert, AW-10 … AW-15).
//!
//! Security by design: only a service host in the user's SIP domain or a trusted domain gets the
//! NTLM authentication; the issued certificate must carry the new key; key and certificate are
//! stored with modes 0700/0600 by `commscraft-tlsdsk`.

use crate::directory::HttpNtlm;
use crate::setup::Trust;
use commscraft_net::{OsRandom, TlsClient};
use std::path::PathBuf;
use std::time::Duration;

/// An account name for a directory, encoded one-to-one (`%XX` for anything but letters, digits,
/// `@` and `-`); `None` when empty or longer than 200 characters.
pub fn account_dir_name(v: &str) -> Option<String> {
    let mut out = String::new();
    for b in v.bytes() {
        if b.is_ascii_alphanumeric() || b == b'@' || b == b'-' {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    (!out.is_empty() && out.len() <= 200).then_some(out)
}

/// `$XDG_DATA_HOME/commscraft/tlsdsk/ACCOUNT` (directory 0700, files 0600).
pub fn identity_dir(aor: &str) -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(
        base.join("commscraft")
            .join("tlsdsk")
            .join(account_dir_name(aor.trim_start_matches("sip:"))?),
    )
}

fn hex_of(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Gets a user certificate: a new RSA key and PKCS#10 request, sent with NTLM to the service's
/// Windows-authentication binding at `sts_uri`; the device ID is the endpoint's `+sip.instance`
/// (AW-14). Stores it in `dir` and returns the days it stays valid.
#[allow(clippy::too_many_arguments)]
pub fn provision_certificate(
    tls: &TlsClient,
    trust: &Trust,
    sts_uri: &str,
    aor: &str,
    epid: &str,
    dir: &std::path::Path,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<u64, String> {
    use commscraft_sipae::base64;
    let url = commscraft_url::parse_https(sts_uri).ok_or("unusable certificate service URL")?;
    let domain = aor.rsplit('@').next().unwrap_or_default();
    if !commscraft_url::in_domain(&url.host, domain) && !trust.trusts(&url.host) {
        return Err(format!(
            "{} is outside your SIP domain and not trusted",
            url.host
        ));
    }
    let device = commscraft_sip_identity::instance_uuid_from_epid(epid)
        .map_err(|_| "no device ID".to_string())?;
    let mut id = [0u8; 16];
    commscraft_register::Random::fill(&mut OsRandom, &mut id);
    let part = |a: usize, b: usize| hex_of(id.get(a..b).unwrap_or(&[]));
    let request_id = format!(
        "{}-{}-{}-{}-{}",
        part(0, 4),
        part(4, 6),
        part(6, 8),
        part(8, 10),
        part(10, 16)
    );
    let entity = aor.trim_start_matches("sip:");
    log(&format!(
        "getting a user certificate for TLS-DSK (RSA {} bits) …",
        commscraft_tlsdsk::KEY_BITS
    ));
    let request = commscraft_tlsdsk::Request::generate(entity).map_err(|e| format!("{e:?}"))?;
    let body = commscraft_ocauthws::certificate_request(aor, &device, &request.csr, &request_id)
        .map_err(|e| format!("{e:?}"))?;
    log(&format!("  POST {sts_uri} (GetAndPublishCert)"));
    let negotiate =
        commscraft_ntlm::messages::negotiate_message(commscraft_ntlm::messages::HTTP_NEGOTIATE);
    let mut auth_err: Option<String> = None;
    let r = commscraft_net::http::post_with_auth(
        tls,
        &url,
        &[
            ("Content-Type", commscraft_ocauthws::CONTENT_TYPE),
            ("SOAPAction", commscraft_ocauthws::SOAP_ACTION),
        ],
        body.as_bytes(),
        user_agent,
        Duration::from_secs(30),
        &mut |challenge, cert| match challenge {
            None => Ok(format!("NTLM {}", base64::encode(&negotiate))),
            Some(ch) => {
                let token = ch
                    .strip_prefix("NTLM ")
                    .and_then(|b| base64::decode(b.trim()).ok());
                match token
                    .ok_or_else(|| "no NTLM challenge".to_string())
                    .and_then(|t| ntlm(&negotiate, &t, cert))
                {
                    Ok(a) => Ok(format!("NTLM {}", base64::encode(&a))),
                    Err(e) => {
                        auth_err = Some(e);
                        Err(commscraft_net::NetError::Io(std::io::Error::other("NTLM")))
                    }
                }
            }
        },
    )
    .map_err(|e| auth_err.take().unwrap_or_else(|| e.to_string()))?;
    log(&format!("  HTTP {} ({} bytes)", r.status, r.body.len()));
    for (name, value) in r.headers.iter().filter(|(n, _)| n.starts_with("x-ms")) {
        let v: String = value.chars().filter(|c| !c.is_control()).collect();
        log(&format!("    {name}: {v}"));
    }
    if r.status == 401 {
        return Err("the certificate service refused the credentials (401)".into());
    }
    let text = String::from_utf8_lossy(&r.body);
    let certificate = commscraft_ocauthws::parse_certificate_answer(&text, aor)
        .map_err(|e| format!("certificate service: {e:?} (HTTP {})", r.status))?;
    let identity =
        commscraft_tlsdsk::Identity::new(request, certificate).map_err(|e| format!("{e:?}"))?;
    identity.store(dir).map_err(|e| format!("{e:?}"))?;
    Ok(identity.remaining().unwrap_or(0) / 86_400)
}

#[cfg(test)]
mod tests {
    use super::account_dir_name;

    #[test]
    fn account_names_are_one_to_one() {
        assert_eq!(
            account_dir_name("alice@beispiel.invalid").as_deref(),
            Some("alice@beispiel%2Einvalid")
        );
        assert_eq!(account_dir_name("a/b").as_deref(), Some("a%2Fb"));
        assert_eq!(account_dir_name(""), None);
    }
}
