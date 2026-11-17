//! The user's own Exchange data for the interfaces: one EWS call (`ews_call`) and the
//! appointments and automatic replies (function 16: GetUserAvailability, [MS-OXWAVLS]
//! 20250520; GetUserOofSettings, [MS-OXWOOF] 20250520), as the server's calendar settings allow
//! ([MS-SIPREGE] §2.2.2.5.6, pp. 50–52: `DisableCalendarPresence` – no calendar data is loaded,
//! `DisablePresenceNote` – the out-of-office message is not loaded).
//!
//! Security by design: the EWS host gets the NTLM authentication only in the user's SIP domain or
//! a trusted domain; TLS is verified; the answers are read by the bounded parsers of `ews`.

use crate::directory::{ntlm_post, HttpNtlm};
use crate::setup::Trust;
use commscraft_ews::calendar::{self as cal, CalendarEvent, Oof};
use commscraft_net::TlsClient;
use commscraft_provisioning::CalendarPolicy;

/// One EWS call at the user's EWS URL; the answer's text (a SOAP fault comes with 500).
#[allow(clippy::too_many_arguments)]
pub fn ews_call(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    settings: &commscraft_ews::UserSettings,
    (name, action): (&str, &str),
    body: &str,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<String, String> {
    let base = settings
        .https_url(&["InternalEwsUrl", "ExternalEwsUrl"])
        .ok_or("Exchange named no web services URL")?;
    let url = commscraft_url::parse_https(base).ok_or("unusable EWS URL")?;
    let own = aor.rsplit('@').next().unwrap_or_default();
    if !commscraft_url::in_domain(&url.host, own) && !trust.trusts(&url.host) {
        return Err(format!(
            "{} is outside your SIP domain and not trusted",
            url.host
        ));
    }
    log(&format!("  POST {base} ({name})"));
    let r = ntlm_post(
        tls,
        &url,
        &[
            ("Content-Type", commscraft_ews::CONTENT_TYPE),
            ("SOAPAction", action),
        ],
        body,
        user_agent,
        ntlm,
    )?;
    match r.status {
        401 => Err("not authorized at Exchange (401)".into()),
        200 | 500 => Ok(String::from_utf8_lossy(&r.body).to_string()),
        s => Err(format!("HTTP {s}")),
    }
}

/// The user's appointments from the start of today for four days, and the automatic replies.
/// Each part may be switched off by the server or fail on its own; both failing is an error.
#[allow(clippy::too_many_arguments)]
pub fn appointments(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    settings: &commscraft_ews::UserSettings,
    policy: &CalendarPolicy,
    now: u64,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<(Option<Vec<CalendarEvent>>, Option<Oof>), String> {
    let mailbox = aor.trim_start_matches("sip:").to_ascii_lowercase();
    let (start, end) = cal::window(now);
    let events = if policy.disable_calendar {
        None
    } else {
        let request =
            cal::availability_request(&mailbox, start, end).map_err(|e| format!("{e:?}"))?;
        Some(
            ews_call(
                tls,
                trust,
                aor,
                settings,
                ("GetUserAvailability", cal::GET_USER_AVAILABILITY_ACTION),
                &request,
                user_agent,
                ntlm,
                log,
            )
            .and_then(|x| cal::parse_availability(&x, start).map_err(|e| format!("{e:?}")))
            .map(|a| a.events),
        )
    };
    let oof = if policy.disable_oof_note {
        None
    } else {
        let request = cal::oof_request(&mailbox).map_err(|e| format!("{e:?}"))?;
        Some(
            ews_call(
                tls,
                trust,
                aor,
                settings,
                ("GetUserOofSettings", cal::GET_USER_OOF_SETTINGS_ACTION),
                &request,
                user_agent,
                ntlm,
                log,
            )
            .and_then(|x| cal::parse_oof(&x).map_err(|e| format!("{e:?}"))),
        )
    };
    match (events, oof) {
        (Some(Err(a)), Some(Err(o))) => Err(format!("{a}; {o}")),
        (Some(Err(a)), None) | (None, Some(Err(a))) => Err(a),
        (e, o) => Ok((e.and_then(Result::ok), o.and_then(Result::ok))),
    }
}
