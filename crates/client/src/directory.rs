//! The address book web service (MS-DLX): search and distribution list expansion over HTTPS
//! with NTLM (MS-NTHT), for the command-line tool and the desktop client.
//!
//! Security by design: only services in the user's SIP domain or a trusted domain get the NTLM
//! authentication; TLS is verified; answers are parsed by the bounded `dlx` parser.

use crate::setup::Trust;
use commscraft_net::TlsClient;
use commscraft_provisioning::{ServerConfiguration, ServiceUrl};
use std::time::Duration;

/// The NTLM step of the session's registrar for HTTP: NEGOTIATE, CHALLENGE and the server's
/// certificate (channel binding) in, AUTHENTICATE out.
pub type HttpNtlm<'a> = dyn FnMut(&[u8], &[u8], Option<&[u8]>) -> Result<Vec<u8>, String> + 'a;

/// A POST with NTLM per MS-NTHT on one connection: NEGOTIATE in the first request,
/// AUTHENTICATE after the 401 with the CHALLENGE.
pub fn ntlm_post(
    tls: &TlsClient,
    url: &commscraft_url::HttpsUrl,
    headers: &[(&str, &str)],
    body: &str,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
) -> Result<commscraft_net::http::HttpResponse, String> {
    use commscraft_sipae::base64;
    let negotiate =
        commscraft_ntlm::messages::negotiate_message(commscraft_ntlm::messages::HTTP_NEGOTIATE);
    let mut auth_err: Option<String> = None;
    commscraft_net::http::post_with_auth(
        tls,
        url,
        headers,
        body.as_bytes(),
        user_agent,
        Duration::from_secs(15),
        &mut |challenge, cert| match challenge {
            None => Ok(format!("NTLM {}", base64::encode(&negotiate))),
            Some(ch) => {
                let token = ch
                    .strip_prefix("NTLM ")
                    .and_then(|b| base64::decode(b.trim()).ok());
                let answer = token
                    .ok_or_else(|| "no NTLM challenge".to_string())
                    .and_then(|t| ntlm(&negotiate, &t, cert));
                match answer {
                    Ok(a) => Ok(format!("NTLM {}", base64::encode(&a))),
                    Err(e) => {
                        auth_err = Some(e);
                        Err(commscraft_net::NetError::Io(std::io::Error::other("NTLM")))
                    }
                }
            }
        },
    )
    .map_err(|e| auth_err.take().unwrap_or_else(|| e.to_string()))
}

/// One SOAP call to the first usable address book service of provisioning; `log` gets the
/// URL used.
#[allow(clippy::too_many_arguments)]
pub fn dlx_call(
    tls: &TlsClient,
    trust: &Trust,
    services: Option<&ServerConfiguration>,
    action: &str,
    body: &str,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<(u16, String), String> {
    let c = services.ok_or("no provisioning yet (address book service unknown)")?;
    if c.dlx_enabled == Some(false) {
        return Err("the address book web service is disabled by the server".into());
    }
    let candidates: Vec<&ServiceUrl> = [
        &c.dlx_internal,
        &c.dlx_external,
        &c.abwq_internal,
        &c.abwq_external,
    ]
    .into_iter()
    .flatten()
    .collect();
    if candidates.is_empty() {
        return Err("the server announced no address book web service".into());
    }
    let mut last = String::from("no usable service");
    for s in candidates {
        if !s.in_user_domain && !trust.trusts(&s.host) {
            last = format!("{} is outside your SIP domain and not trusted", s.host);
            continue;
        }
        let Some(url) = commscraft_url::parse_https(&s.url) else {
            continue;
        };
        log(&format!("  POST {} ({action})", s.url));
        match ntlm_post(
            tls,
            &url,
            &[
                ("Content-Type", commscraft_dlx::CONTENT_TYPE),
                ("SOAPAction", action),
            ],
            body,
            user_agent,
            ntlm,
        ) {
            Ok(r) if r.status == 401 => return Err("not authorized (401)".into()),
            Ok(r) => return Ok((r.status, String::from_utf8_lossy(&r.body).to_string())),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// Address book search (MS-DLX SearchAbEntry, OB-44).
pub fn search(
    tls: &TlsClient,
    trust: &Trust,
    services: Option<&ServerConfiguration>,
    query: &str,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<Vec<commscraft_dlx::Entry>, String> {
    let body = commscraft_dlx::search_request(query).map_err(|e| format!("{e:?}"))?;
    let (status, text) = dlx_call(
        tls,
        trust,
        services,
        commscraft_dlx::SOAP_ACTION,
        &body,
        user_agent,
        ntlm,
        log,
    )?;
    // A SOAP fault comes with 500.
    match commscraft_dlx::parse_response(&text) {
        Ok(found) if status == 200 => Ok(found),
        Err(commscraft_dlx::Error::Fault(f)) => Err(format!("server fault: {f}")),
        Err(e) if status == 200 => Err(format!("{e:?}")),
        _ => Err(format!("HTTP {status}")),
    }
}

/// Distribution list expansion (MS-DLX ExpandDistributionList, DL-04).
pub fn expand(
    tls: &TlsClient,
    trust: &Trust,
    services: Option<&ServerConfiguration>,
    group: &str,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<commscraft_dlx::Expansion, String> {
    let body = commscraft_dlx::expand_request(group).map_err(|e| format!("{e:?}"))?;
    let (status, text) = dlx_call(
        tls,
        trust,
        services,
        commscraft_dlx::EXPAND_SOAP_ACTION,
        &body,
        user_agent,
        ntlm,
        log,
    )?;
    match commscraft_dlx::parse_expand_response(&text) {
        Ok(x) if status == 200 => Ok(x),
        Err(commscraft_dlx::Error::Fault(f)) => Err(format!("server fault: {f}")),
        Err(e) if status == 200 => Err(format!("{e:?}")),
        _ => Err(format!("HTTP {status}")),
    }
}
