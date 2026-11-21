//! Contact photos over UCWA (function 15, second path; evidence UW-01 … UW-10): the Skype for
//! Business server fetches the photo from Exchange itself and hands it out, so the client needs no
//! access of its own to the mailbox. Used when the directory and Exchange paths give nothing.
//!
//! Flow: Autodiscover's `OAuth` link → its `MsRtcOAuth` challenge → an OAuth token with the
//! Windows grant, the token request authenticated with NTLM over HTTP (no password in a form)
//! → the User resource's UCWA link → an application → `contactPhoto` links from the contact list,
//! from an expanded distribution group or from a search → the photo.
//!
//! Security by design: Autodiscover, token service and UCWA host must be in the user's SIP
//! domain or a domain the user trusts (they see credentials or the token); every link from a
//! UCWA answer must stay on the UCWA host (`commscraft_ucwa::resolve`); the token is never
//! logged; answers and photos are bounded; only JPEG and PNG are taken.

use crate::directory::HttpNtlm;
use crate::setup::Trust;
use commscraft_net::TlsClient;
use commscraft_ucwa as u;
use commscraft_url::HttpsUrl;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long one request may take to start answering.
const TIMEOUT: Duration = Duration::from_secs(45);
/// The photo size asked for (UW-07; the interface shows 96-pixel thumbnails).
pub const PHOTO_SIDE: u32 = 96;
/// Most photo links remembered (design).
const MAX_LINKS: usize = 20_000;

/// A UCWA application of the signed-in user.
pub struct Session {
    service: HttpsUrl,
    token: u::Token,
    renew_at: Instant,
    me_photo: Option<String>,
    my_contacts: Option<String>,
    search: Option<String>,
    contacts_loaded: bool,
    /// Lower-case SIP URI → photo link.
    photos: HashMap<String, String>,
    aor: String,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UCWA session at {}", self.service.host)
    }
}

fn allowed(host: &str, user_domain: &str, trust: &Trust) -> bool {
    commscraft_url::in_domain(host, user_domain) || trust.trusts(host)
}

fn text(e: u::Error) -> String {
    format!("{e:?}")
}

impl Session {
    /// Opens a UCWA application for `aor`.
    pub fn open(
        tls: &TlsClient,
        trust: &Trust,
        aor: &str,
        user_agent: &str,
        ntlm: &mut HttpNtlm<'_>,
        log: &mut dyn FnMut(&str),
    ) -> Result<Session, String> {
        let user_domain = aor
            .trim_start_matches("sip:")
            .rsplit_once('@')
            .map(|(_, d)| d.to_ascii_lowercase())
            .ok_or("the SIP address has no domain")?;
        // UW-04: the OAuth link of the Autodiscover answer.
        let outcome = crate::setup::autodiscover(tls, aor, log, false)
            .ok_or("no Autodiscover answer (UCWA unknown)")?;
        let oauth = outcome
            .links
            .iter()
            .find(|l| l.token.eq_ignore_ascii_case("OAuth"))
            .ok_or("Autodiscover names no OAuth resource (no UCWA)")?
            .url
            .clone();
        if !allowed(&oauth.host, &user_domain, trust) {
            return Err(format!(
                "{} is outside your SIP domain and not trusted",
                oauth.host
            ));
        }
        // UW-01: without a token the OAuth resource answers 401 with the token service.
        log(&format!("  UCWA: GET {}{}", oauth.host, oauth.path));
        let first =
            commscraft_net::http::get(tls, &oauth, u::AUTODISCOVER_XML, user_agent, TIMEOUT)
                .map_err(|e| e.to_string())?;
        if first.status != 401 {
            return Err(format!(
                "OAuth resource: HTTP {} instead of a challenge",
                first.status
            ));
        }
        let challenge = u::oauth_challenge(
            first
                .headers
                .iter()
                .filter(|(n, _)| n == "www-authenticate")
                .map(|(_, v)| v.as_str()),
        )
        .ok_or("no MsRtcOAuth challenge")?;
        if !challenge.offers_windows() {
            return Err("the token service offers no Windows authentication".into());
        }
        if !allowed(&challenge.token_url.host, &user_domain, trust) {
            return Err(format!(
                "{} is outside your SIP domain and not trusted",
                challenge.token_url.host
            ));
        }
        // UW-02: the token, the request authenticated with NTLM.
        log(&format!(
            "  UCWA: POST {}{} (Windows grant)",
            challenge.token_url.host, challenge.token_url.path
        ));
        let r = crate::directory::ntlm_post(
            tls,
            &challenge.token_url,
            &[(
                "Content-Type",
                "application/x-www-form-urlencoded;charset=UTF-8",
            )],
            &u::windows_token_request(),
            user_agent,
            ntlm,
        )?;
        if r.status != 200 {
            return Err(format!("token service: HTTP {}", r.status));
        }
        let token = u::token(&r.body).map_err(text)?;
        let renew_at = Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(300));
        // UW-04: the User resource with the token names the UCWA service.
        let user = bearer_get(
            tls,
            &oauth,
            &token,
            u::AUTODISCOVER_XML,
            64 * 1024,
            user_agent,
        )?;
        if user.status != 200 {
            return Err(format!("User resource: HTTP {}", user.status));
        }
        let service = u::ucwa_links(&user.body)
            .map_err(text)?
            .into_iter()
            .find(|l| allowed(&l.host, &user_domain, trust))
            .ok_or("the User resource names no usable UCWA service")?;
        // UW-05: the application.
        let mut id = [0u8; 16];
        {
            use commscraft_register::Random;
            commscraft_net::OsRandom::new()
                .map_err(|e| e.to_string())?
                .fill(&mut id);
        }
        let endpoint: String = id.iter().map(|b| format!("{b:02x}")).collect();
        let body = u::application_request("en-US", &endpoint, user_agent)
            .ok_or("unusable application values")?;
        log(&format!("  UCWA: POST {}{}", service.host, service.path));
        let created = commscraft_net::http::post_with_auth(
            tls,
            &service,
            &[("Content-Type", u::XML), ("Accept", u::XML)],
            body.as_bytes(),
            user_agent,
            TIMEOUT,
            &mut |_, _| Ok(token.authorization()),
        )
        .map_err(|e| e.to_string())?;
        if created.status != 201 && created.status != 200 {
            return Err(format!("application: HTTP {}", created.status));
        }
        let app = u::resource(&created.body).map_err(text)?;
        let me_photo = app
            .find("me")
            .and_then(|m| m.link("photo"))
            .map(str::to_string);
        let people = app.find("people");
        Ok(Session {
            my_contacts: people
                .and_then(|p| p.link("myContacts"))
                .map(str::to_string),
            search: people.and_then(|p| p.link("search")).map(str::to_string),
            me_photo,
            service,
            token,
            renew_at,
            contacts_loaded: false,
            photos: HashMap::new(),
            aor: aor.trim().to_ascii_lowercase(),
        })
    }

    /// The token is near its end: open a new session.
    pub fn expired(&self) -> bool {
        Instant::now() >= self.renew_at
    }

    fn get(
        &self,
        tls: &TlsClient,
        href: &str,
        accept: &str,
        limit: usize,
        user_agent: &str,
    ) -> Result<commscraft_net::http::HttpResponse, String> {
        let url = u::resolve(&self.service, href).ok_or("a link outside the UCWA service")?;
        bearer_get(tls, &url, &self.token, accept, limit, user_agent)
    }

    fn remember(&mut self, links: Vec<u::PhotoLink>) -> usize {
        let n = links.len();
        for l in links {
            if self.photos.len() >= MAX_LINKS {
                break;
            }
            self.photos.insert(l.uri, l.href);
        }
        n
    }

    /// UW-07: the photo links of the whole contact list, once per session.
    fn load_contacts(&mut self, tls: &TlsClient, user_agent: &str, log: &mut dyn FnMut(&str)) {
        if self.contacts_loaded {
            return;
        }
        self.contacts_loaded = true;
        let Some(href) = self.my_contacts.clone() else {
            return;
        };
        match self.get(tls, &href, u::XML, u::MAX_DOCUMENT, user_agent) {
            Ok(r) if r.status == 200 => match u::resource(&r.body) {
                Ok(doc) => {
                    let n = self.remember(u::photo_links(&doc));
                    log(&format!("  UCWA: {n} contact(s) with a photo link"));
                }
                Err(e) => log(&format!("  UCWA contact list: {e:?}")),
            },
            Ok(r) => log(&format!("  UCWA contact list: HTTP {}", r.status)),
            Err(e) => log(&format!("  UCWA contact list: {e}")),
        }
    }

    /// UW-08: a search for an address; its contacts' photo links are remembered.
    fn search(
        &mut self,
        tls: &TlsClient,
        query: &str,
        user_agent: &str,
    ) -> Result<u::Resource, String> {
        let search = self.search.clone().ok_or("no UCWA search")?;
        let href = u::search_query(&search, query, 10).ok_or("unusable search query")?;
        let r = self.get(tls, &href, u::XML, u::MAX_DOCUMENT, user_agent)?;
        if r.status != 200 {
            return Err(format!("search: HTTP {}", r.status));
        }
        let doc = u::resource(&r.body).map_err(text)?;
        self.remember(u::photo_links(&doc));
        Ok(doc)
    }

    /// UW-09: a distribution group's members with their photo links, through the group found
    /// by its address and its `expandDistributionGroup` link. Returns how many members have one.
    pub fn expand_group(
        &mut self,
        tls: &TlsClient,
        address: &str,
        user_agent: &str,
        log: &mut dyn FnMut(&str),
    ) -> Result<usize, String> {
        let found = self.search(tls, address, user_agent)?;
        let href = u::group_expand_link(&found, address)
            .ok_or("the search does not find this distribution group")?;
        let r = self.get(tls, &href, u::XML, u::MAX_DOCUMENT, user_agent)?;
        match r.status {
            200 => {}
            403 => return Err("the server does not show this group's members (403)".into()),
            s => return Err(format!("expand: HTTP {s}")),
        }
        let doc = u::resource(&r.body).map_err(text)?;
        let n = self.remember(u::photo_links(&doc));
        log(&format!(
            "  UCWA: {n} member(s) of {address} with a photo link"
        ));
        Ok(n)
    }

    /// The photo of `uri` (JPEG or PNG, at most `limit` bytes): `Ok(None)` when the server has
    /// none for it.
    pub fn photo(
        &mut self,
        tls: &TlsClient,
        uri: &str,
        limit: usize,
        user_agent: &str,
        log: &mut dyn FnMut(&str),
    ) -> Result<Option<Vec<u8>>, String> {
        let key = uri.trim().to_ascii_lowercase();
        let link = if key == self.aor && self.me_photo.is_some() {
            self.me_photo.clone()
        } else {
            if !self.photos.contains_key(&key) {
                self.load_contacts(tls, user_agent, log);
            }
            if !self.photos.contains_key(&key) {
                if let Err(e) = self.search(tls, key.trim_start_matches("sip:"), user_agent) {
                    log(&format!("  UCWA search: {e}"));
                }
            }
            self.photos.get(&key).cloned()
        };
        let Some(link) = link else {
            return Ok(None);
        };
        let href = u::photo_query(&link, PHOTO_SIDE);
        log(&format!("  UCWA: GET photo of {key}"));
        let r = self.get(
            tls,
            &href,
            "image/jpeg,image/pjpeg,image/png,application/octet-stream",
            limit,
            user_agent,
        )?;
        log(&format!("    HTTP {}, {} bytes", r.status, r.body.len()));
        if r.status != 200 && std::env::var_os("COMMSCRAFT_VERBOSE").is_some() {
            log(
                &String::from_utf8_lossy(r.body.get(..r.body.len().min(400)).unwrap_or_default())
                    .replace(char::is_control, " "),
            );
        }
        match r.status {
            200 => {}
            404 => return Ok(None),
            403 => return Err("the server shows no photo of this person (403)".into()),
            s => return Err(format!("photo: HTTP {s}")),
        }
        crate::photos::photo_kind(&r.body).ok_or("the photo is not a JPEG or PNG image")?;
        Ok(Some(r.body))
    }
}

/// A GET with the OAuth token (UW-03).
fn bearer_get(
    tls: &TlsClient,
    url: &HttpsUrl,
    token: &u::Token,
    accept: &str,
    limit: usize,
    user_agent: &str,
) -> Result<commscraft_net::http::HttpResponse, String> {
    commscraft_net::http::get_with_auth(
        tls,
        url,
        &[("Accept", accept)],
        user_agent,
        TIMEOUT,
        limit,
        &mut |_, _| Ok(token.authorization()),
    )
    .map_err(|e| e.to_string())
}
