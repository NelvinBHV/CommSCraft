//! Contact photos (function 15) for the command-line tool and the desktop client: the photo the
//! address book names (DL-06, fetched from the provisioned address book base URL, [MS-ABS] §1.3)
//! and – where the photo policy allows photos beyond the directory (RG-52) – first the photo from
//! Exchange (EX-06, [MS-OXWSPHOTO] §3.1.5.1.1, p. 9) through the user's Exchange web services
//! (Autodiscover GetUserSettings, EX-08, [MS-OXWSADISC] §1.5, pp. 11–12).
//!
//! Security by design: only hosts in the user's SIP domain or a trusted domain get the NTLM
//! authentication; TLS is verified; downloads are bounded by the server's photo limit (at most
//! [`PHOTO_MAX`]); only JPEG and PNG files are kept; the cache lives in a directory per account
//! and photo server with one-to-one names (issues #129, #145), directories 0700, files 0600.

use crate::directory::{ntlm_post, HttpNtlm};
use crate::setup::Trust;
use commscraft_net::TlsClient;
use commscraft_provisioning::{PhotoUsage, ServerConfiguration};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Largest photo downloaded when the server sets no limit (design).
pub const PHOTO_MAX: usize = 1024 * 1024;
/// Most Autodiscover redirects followed (design).
const EWS_MAX_REDIRECTS: usize = 3;

/// A GET with NTLM per MS-NTHT on one connection.
pub fn ntlm_get(
    tls: &TlsClient,
    url: &commscraft_url::HttpsUrl,
    headers: &[(&str, &str)],
    user_agent: &str,
    timeout: Duration,
    limit: usize,
    ntlm: &mut HttpNtlm<'_>,
) -> Result<commscraft_net::http::HttpResponse, String> {
    use commscraft_sipae::base64;
    let negotiate =
        commscraft_ntlm::messages::negotiate_message(commscraft_ntlm::messages::HTTP_NEGOTIATE);
    let mut auth_err: Option<String> = None;
    commscraft_net::http::get_with_auth(
        tls,
        url,
        headers,
        user_agent,
        timeout,
        limit,
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

/// The image type of a photo file by its first bytes; anything else is refused.
pub fn photo_kind(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("jpg")
    } else if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else {
        None
    }
}

/// A cache file name part, one-to-one (issue #129): ASCII letters, digits, `@` and `-` stay,
/// every other octet becomes `%XX` – `.`, `_` and `%` too, so two different inputs never give
/// the same name and no name is `.` or `..`. `None` when the name would exceed 200 octets.
pub fn cache_name(v: &str) -> Option<String> {
    let mut out = String::new();
    for b in v.bytes() {
        if b.is_ascii_alphanumeric() || b == b'@' || b == b'-' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    (!out.is_empty() && out.len() <= 200).then_some(out)
}

/// The account directory name of the earlier layout (lossy; only to remove it).
pub fn legacy_cache_name(v: &str) -> String {
    v.to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || ".-_@".contains(c) {
                c
            } else {
                '_'
            }
        })
        .take(120)
        .collect()
}

/// The photo cache directory of an account and photo server, and a contact's file name stem
/// there – all one-to-one (issue #145); `None` when a name would be too long.
pub fn photo_cache(base: &Path, aor: &str, host: &str, email: &str) -> Option<(PathBuf, String)> {
    let account = cache_name(&aor.trim_start_matches("sip:").to_ascii_lowercase())?;
    let server = cache_name(&host.to_ascii_lowercase())?;
    let key = cache_name(&email.to_ascii_lowercase())?;
    Some((base.join(account).join(server), key))
}

/// `$XDG_CACHE_HOME/commscraft/photos` (else `~/.cache/…`).
pub fn photos_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .map(|b| b.join("commscraft").join("photos"))
}

fn write_private(path: &Path, data: &[u8]) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    if let Some(dir) = path.parent() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|e| e.to_string())?;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .and_then(|mut f| f.write_all(data))
        .map_err(|e| e.to_string())
}

/// The user's Exchange web services (EX-08): Autodiscover GetUserSettings at the endpoints of
/// the user's own mail domain, with NTLM like the other web services (inference, O-EX-5/OB-48).
/// Only hosts in the user's SIP domain or a trusted domain get credentials; redirects only to
/// HTTPS and at most three times.
pub fn ews_settings(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<commscraft_ews::UserSettings, String> {
    use commscraft_ews::{UserSettings as U, GET_USER_SETTINGS_ACTION, SETTINGS};
    let mut mailbox = aor.trim_start_matches("sip:").to_ascii_lowercase();
    let user_domain = mailbox
        .split_once('@')
        .map(|(_, d)| d.to_string())
        .ok_or("no mail domain")?;
    let mut urls: Vec<String> = commscraft_ews::autodiscover_urls(&user_domain)
        .ok_or("unusable mail domain")?
        .into();
    let mut last = String::from("no Exchange Autodiscover endpoint answered");
    let mut redirects = 0;
    while let Some(u) = (!urls.is_empty()).then(|| urls.remove(0)) {
        let Some(url) = commscraft_url::parse_https(&u) else {
            continue;
        };
        if !commscraft_url::in_domain(&url.host, &user_domain) && !trust.trusts(&url.host) {
            last = format!("{} is outside your SIP domain and not trusted", url.host);
            continue;
        }
        let body = commscraft_ews::user_settings_request(&mailbox, &SETTINGS, &u)
            .map_err(|e| format!("{e:?}"))?;
        log(&format!("  POST {u} (GetUserSettings)"));
        let r = match ntlm_post(
            tls,
            &url,
            &[
                ("Content-Type", commscraft_ews::CONTENT_TYPE),
                ("SOAPAction", GET_USER_SETTINGS_ACTION),
            ],
            &body,
            user_agent,
            ntlm,
        ) {
            Ok(r) => r,
            Err(e) => {
                last = e;
                continue;
            }
        };
        if r.status == 401 {
            return Err("not authorized at Exchange (401)".into());
        }
        let text = String::from_utf8_lossy(&r.body);
        match commscraft_ews::parse_user_settings(&text) {
            Ok(s @ U::Settings(_)) => return Ok(s),
            Ok(U::RedirectAddress(a)) if redirects < EWS_MAX_REDIRECTS => {
                redirects += 1;
                mailbox = a;
                urls.insert(0, u);
            }
            Ok(U::RedirectUrl(next)) if redirects < EWS_MAX_REDIRECTS => {
                redirects += 1;
                urls.insert(0, next);
            }
            Ok(_) => return Err("too many Autodiscover redirects".into()),
            Err(e) if r.status == 200 => last = format!("{e:?}"),
            Err(_) => last = format!("HTTP {}", r.status),
        }
    }
    Err(last)
}

/// A contact's photo from Exchange: GetUserPhoto at the photos URL, else the EWS URL, with NTLM.
/// Kept as `exchange-<address>.<ext>` in the cache directory of the account and photo server
/// with its ETag next to it; the ETag goes back in If-None-Match, and a 304 keeps the cached
/// photo (§3.1.5, pp. 8–9). `Ok(None)`: Exchange has no photo.
#[allow(clippy::too_many_arguments)]
pub fn exchange_photo(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    settings: &commscraft_ews::UserSettings,
    email: &str,
    limit: usize,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<Option<PathBuf>, String> {
    use commscraft_ews::PhotoAnswer as A;
    let base = settings
        .https_url(&[
            "InternalPhotosUrl",
            "ExternalPhotosUrl",
            "InternalEwsUrl",
            "ExternalEwsUrl",
        ])
        .ok_or("Exchange named no web services URL")?;
    let full = commscraft_ews::photo_url(base, email, commscraft_ews::PhotoSize::Hr240)
        .map_err(|e| format!("{e:?}"))?;
    let url = commscraft_url::parse_https(&full).ok_or("unusable photo URL")?;
    let own = aor.rsplit('@').next().unwrap_or_default();
    if !commscraft_url::in_domain(&url.host, own) && !trust.trusts(&url.host) {
        return Err(format!(
            "{} is outside your SIP domain and not trusted",
            url.host
        ));
    }
    let dir = photos_dir().ok_or("no HOME")?;
    let legacy = legacy_cache_name(email);
    for ext in ["jpg", "png", "etag"] {
        let _ = std::fs::remove_file(dir.join(format!("exchange-{legacy}.{ext}")));
    }
    let (dir, key) =
        photo_cache(&dir, aor, &url.host, email).ok_or("address too long for the photo cache")?;
    let etag_file = dir.join(format!("exchange-{key}.etag"));
    let cached = ["jpg", "png"]
        .iter()
        .map(|ext| dir.join(format!("exchange-{key}.{ext}")))
        .find(|p| p.is_file());
    let etag = cached
        .as_ref()
        .and_then(|_| std::fs::read_to_string(&etag_file).ok());
    let mut headers = vec![("Accept", "image/*")];
    if let Some(t) = etag.as_deref() {
        headers.push(("If-None-Match", t.trim()));
    }
    log(&format!(
        "  GET {full}{}",
        if etag.is_some() {
            " (If-None-Match)"
        } else {
            ""
        }
    ));
    let r = ntlm_get(
        tls,
        &url,
        &headers,
        user_agent,
        Duration::from_secs(15),
        limit,
        ntlm,
    )?;
    let header = |n: &str| r.header(n).ok().flatten();
    match commscraft_ews::photo_answer(r.status, header("Content-Type"), header("ETag")) {
        A::Image { etag, .. } => {
            let ext = photo_kind(&r.body).ok_or("the photo is not a JPEG or PNG image")?;
            let path = dir.join(format!("exchange-{key}.{ext}"));
            write_private(&path, &r.body)?;
            match etag {
                Some(t) => write_private(&etag_file, t.as_bytes())?,
                None => {
                    let _ = std::fs::remove_file(&etag_file);
                }
            }
            Ok(Some(path))
        }
        A::Unchanged => cached
            .map(Some)
            .ok_or_else(|| "304 without a cached photo".to_string()),
        A::NoPhoto => Ok(None),
        A::Unauthorized => Err("not authorized at Exchange (401)".into()),
        A::Failed(s) => Err(format!("HTTP {s}")),
    }
}

/// A contact's photo: the address book search names the file (DL-06: PhotoRelPath, PhotoHash,
/// PhotoSize); the photo policy of endpointConfiguration decides whether photos are shown and
/// how large they may be (RG-52). With `AllPhotos` the Exchange photo comes first. Directory
/// photos are kept named by their hash, so a known state is not fetched again. `settings`
/// keeps the Exchange settings between calls (Autodiscover runs once).
#[allow(clippy::too_many_arguments)]
pub fn fetch(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    services: Option<&ServerConfiguration>,
    uri: &str,
    user_agent: &str,
    settings: &mut Option<commscraft_ews::UserSettings>,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<PathBuf, String> {
    let c = services.ok_or("no provisioning yet (address book service unknown)")?;
    if c.photo_usage == Some(PhotoUsage::NoPhoto) {
        return Err("the server's policy shows no photos".into());
    }
    let uri = commscraft_presence::checked_sip_uri(uri).map_err(|_| "not a SIP URI")?;
    let found = crate::directory::search(tls, trust, services, &uri, user_agent, ntlm, log)?;
    let entry = found
        .into_iter()
        .find(|e| {
            e.sip_uri
                .as_deref()
                .is_some_and(|s| s.eq_ignore_ascii_case(&uri))
        })
        .ok_or("no such user in the address book")?;
    let limit = c.max_photo_kb.map_or(PHOTO_MAX, |kb| {
        (kb as usize).saturating_mul(1024).min(PHOTO_MAX)
    });
    if c.photo_usage == Some(PhotoUsage::AllPhotos) {
        let email = entry
            .mail
            .clone()
            .unwrap_or_else(|| uri.trim_start_matches("sip:").to_string());
        if settings.is_none() {
            *settings = ews_settings(tls, trust, aor, user_agent, ntlm, log).ok();
        }
        if let Some(s) = settings.as_ref() {
            match exchange_photo(tls, trust, aor, s, &email, limit, user_agent, ntlm, log) {
                Ok(Some(path)) => return Ok(path),
                Ok(None) => log(&format!("  Exchange has no photo of {email}")),
                Err(e) => log(&format!(
                    "  no photo from Exchange ({e}) – trying the directory"
                )),
            }
        }
    }
    let photo = entry.photo.ok_or("this user has no photo")?;
    if photo.size.is_some_and(|n| n as usize > limit) {
        return Err(format!(
            "the photo is larger than the {limit} bytes allowed"
        ));
    }
    let dir = photos_dir().ok_or("no HOME")?;
    let key = photo.hash.clone().unwrap_or_else(|| photo.rel_path.clone());
    let key = cache_name(&key).ok_or("photo name too long")?;
    if photo.hash.is_some() {
        for ext in ["jpg", "png"] {
            let cached = dir.join(format!("{key}.{ext}"));
            if cached.is_file() {
                return Ok(cached);
            }
        }
    }
    let mut last = String::from("no usable service");
    for b in [&c.abs_internal, &c.abs_external].into_iter().flatten() {
        if !b.in_user_domain && !trust.trusts(&b.host) {
            last = format!("{} is outside your SIP domain and not trusted", b.host);
            continue;
        }
        let full = format!("{}/{}", b.url.trim_end_matches('/'), photo.rel_path);
        let Some(url) = commscraft_url::parse_https(&full) else {
            continue;
        };
        log(&format!("  GET {full}"));
        let r = match ntlm_get(
            tls,
            &url,
            &[("Accept", "application/octet-stream")],
            user_agent,
            Duration::from_secs(15),
            limit,
            ntlm,
        ) {
            Ok(r) => r,
            Err(e) => {
                last = e;
                continue;
            }
        };
        if r.status != 200 {
            last = format!("HTTP {}", r.status);
            continue;
        }
        if photo.size.is_some_and(|n| n as usize != r.body.len()) {
            return Err("the photo does not have the announced size".into());
        }
        let ext = photo_kind(&r.body).ok_or("the photo is not a JPEG or PNG image")?;
        let path = dir.join(format!("{key}.{ext}"));
        write_private(&path, &r.body)?;
        return Ok(path);
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photo_kinds() {
        assert_eq!(photo_kind(&[0xff, 0xd8, 0xff, 0xe0]), Some("jpg"));
        assert_eq!(photo_kind(b"\x89PNG\r\n\x1a\n...."), Some("png"));
        assert_eq!(photo_kind(b"<svg onload=x>"), None);
        assert_eq!(photo_kind(b""), None);
    }

    #[test]
    fn cache_names_are_one_to_one() {
        assert_eq!(cache_name("a.b_c").as_deref(), Some("a%2Eb%5Fc"));
        assert_ne!(cache_name("a+b"), cache_name("a_b"));
        assert!(cache_name("..").is_some_and(|n| n != ".."));
        assert!(cache_name(&"x".repeat(300)).is_none());
        let base = Path::new("/c");
        let (d1, k1) =
            photo_cache(base, "sip:Alice@contoso.com", "Ex.contoso.com", "Bob@x").unwrap();
        let (d2, k2) =
            photo_cache(base, "sip:alice@contoso.com", "ex.contoso.com", "bob@x").unwrap();
        assert_eq!((d1, k1), (d2, k2));
        let (d3, _) =
            photo_cache(base, "sip:carol@contoso.com", "ex.contoso.com", "bob@x").unwrap();
        assert_ne!(d3, base.join("alice@contoso%2Ecom"));
    }
}
