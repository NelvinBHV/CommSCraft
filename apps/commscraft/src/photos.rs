//! Contact photos (function 15) and the user's appointments (function 16) for the desktop
//! client: a thread of its own fetches them with the shared `client::photos` and
//! `client::exchange`, so the session's SIP never waits for HTTP. The NTLM step needs the signed-in registrar, which the
//! session worker owns: the thread asks the worker for it over a channel.
//!
//! Security by design: the queue of addresses is bounded; downloads are bounded by the photo
//! policy; the picture is decoded here, in pure Rust (zune-jpeg, png), only up to
//! [`MAX_SIDE`] pixels a side, and handed on as a small thumbnail.

use commscraft_client::setup::Trust;
use commscraft_net::TlsClient;
use commscraft_provisioning::ServerConfiguration;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};

/// Largest photo side decoded (design: Exchange's largest size is 648).
pub const MAX_SIDE: usize = 1024;
/// The side of the thumbnail handed to the interface.
pub const THUMB: usize = 96;
/// Addresses waiting at most.
const QUEUE: usize = 1024;

/// What the thread asks of the worker, which holds the sign-in's credentials.
pub enum NtlmAsk {
    /// The NTLM step: NEGOTIATE, CHALLENGE, the server's certificate.
    Ntlm {
        negotiate: Vec<u8>,
        challenge: Vec<u8>,
        cert: Option<Vec<u8>>,
        reply: Sender<Result<Vec<u8>, String>>,
    },
    /// The body of a UCWA password grant (UW-12), in a wiped buffer.
    PasswordGrant {
        reply: Sender<Result<zeroize::Zeroizing<String>, String>>,
    },
}

/// A photo shown is kept this long before it is asked for again (design; the mobile clients
/// keep contact photos too and show them at once).
const FRESH: std::time::Duration = std::time::Duration::from_secs(24 * 3600);
/// A failed photo is asked for again after this time, doubled per failure (design).
const RETRY_FIRST: std::time::Duration = std::time::Duration::from_secs(120);
/// Longest wait before asking again, and most attempts (design).
const RETRY_MAX: std::time::Duration = std::time::Duration::from_secs(3600);
const MAX_ATTEMPTS: u32 = 8;

/// The file of a photo shown for `uri` (the account's own folder, names one-to-one).
fn shown_file(aor: &str, uri: &str) -> Option<std::path::PathBuf> {
    shown_file_in(&commscraft_client::photos::photos_dir()?, aor, uri)
}

fn shown_file_in(base: &std::path::Path, aor: &str, uri: &str) -> Option<std::path::PathBuf> {
    use commscraft_client::photos::cache_name;
    let account = cache_name(&aor.trim_start_matches("sip:").to_ascii_lowercase())?;
    let key = cache_name(&uri.trim_start_matches("sip:").to_ascii_lowercase())?;
    Some(base.join(account).join("shown").join(key))
}

/// A kept photo of `uri` and whether it is still fresh.
fn kept(aor: &str, uri: &str) -> Option<(Vec<u8>, bool)> {
    kept_at(&shown_file(aor, uri)?)
}

fn kept_at(path: &std::path::Path) -> Option<(Vec<u8>, bool)> {
    let path = path.to_path_buf();
    let meta = std::fs::metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > commscraft_client::photos::PHOTO_MAX as u64 {
        return None;
    }
    let bytes = std::fs::read(&path).ok()?;
    commscraft_client::photos::photo_kind(&bytes)?;
    let fresh = meta
        .modified()
        .ok()
        .and_then(|m| m.elapsed().ok())
        .is_some_and(|age| age < FRESH);
    Some((bytes, fresh))
}

/// The wait before asking again after `attempts` failures: doubled each time, bounded.
fn retry_wait(attempts: u32) -> std::time::Duration {
    RETRY_FIRST
        .saturating_mul(1u32 << attempts.saturating_sub(1).min(10))
        .min(RETRY_MAX)
}

/// What became of one photo request.
enum Outcome {
    Photo(Vec<u8>),
    /// The servers have none for this person.
    None,
    /// Not reachable now: asked for again later.
    Failed,
}

/// A thumbnail: side × side RGBA.
pub struct Thumb {
    pub uri: String,
    pub side: usize,
    pub rgba: Vec<u8>,
}

/// A job of the thread.
enum Job {
    Photo(String, Option<Box<ServerConfiguration>>),
    Calendar(commscraft_provisioning::CalendarPolicy),
    Expand(String, Option<Box<ServerConfiguration>>),
    /// The contact list kept in Exchange: read it, or change it and read it again.
    Contacts(Option<Box<ContactChange>>),
    /// The focus of a meeting link (join manager).
    MeetingLink(String),
    /// A finished conversation for the Conversation History folder.
    SaveHistory(Box<commscraft_ews::history::Conversation>),
    /// The entries of the Conversation History folder.
    HistoryList,
    /// One entry's text.
    HistoryBody(commscraft_ews::ucs::ItemId),
    /// The offline address book: the cached file brought up to date (delta or full file).
    AddressBook(Box<ServerConfiguration>),
}

/// A change of the contact list in Exchange with the list and identifiers it applies to.
pub type ContactChange = (
    commscraft_presence::ContactEdit,
    commscraft_presence::ContactList,
    commscraft_client::exchange::UcsIds,
);

/// What the thread found.
pub enum Done {
    Thumb(Thumb),
    /// The focus URI of a meeting link.
    MeetingFocus(Result<String, String>),
    /// A conversation saved in the Conversation History folder, or why not.
    HistorySaved(Result<(), String>),
    HistoryList(Result<Vec<commscraft_ews::history::Entry>, String>),
    /// The offline address book and how it was obtained, or why not.
    AddressBook(Result<(Box<commscraft_abs::book::AddressBook>, String), String>),
    /// One entry's text, by its item ID.
    HistoryBody(String, Result<String, String>),
    /// The contact list from Exchange (after a change, if one was asked for).
    Contacts(
        Result<
            (
                commscraft_presence::ContactList,
                commscraft_client::exchange::UcsIds,
            ),
            String,
        >,
    ),
    /// The members of a distribution group (MS-DLX ExpandDistributionList).
    Expanded(String, Result<commscraft_dlx::Expansion, String>),
    Calendar(
        Result<
            (
                Option<Vec<commscraft_ews::calendar::CalendarEvent>>,
                Option<commscraft_ews::calendar::Oof>,
            ),
            String,
        >,
    ),
}

pub struct Photos {
    tx: SyncSender<Job>,
}

impl Photos {
    /// Starts the thread; thumbnails come out of `done`, NTLM steps out of `ntlm`.
    pub fn start(
        tls: TlsClient,
        trust: Trust,
        aor: String,
        ntlm: Sender<NtlmAsk>,
        done: Sender<Done>,
    ) -> Self {
        let (tx, rx) = mpsc::sync_channel(QUEUE);
        std::thread::spawn(move || run(&tls, &trust, &aor, &rx, &ntlm, &done));
        Photos { tx }
    }

    /// Asks for a contact's photo; a full queue drops the request.
    pub fn want(&self, uri: &str, services: Option<&ServerConfiguration>) {
        let _ = self.tx.try_send(Job::Photo(
            uri.to_string(),
            services.map(|s| Box::new(s.clone())),
        ));
    }

    /// Asks for the members of a distribution group.
    pub fn expand(&self, email: &str, services: Option<&ServerConfiguration>) {
        let _ = self.tx.try_send(Job::Expand(
            email.to_string(),
            services.map(|s| Box::new(s.clone())),
        ));
    }

    /// Asks for the contact list kept in Exchange, after this change if one is given.
    pub fn contacts(&self, change: Option<ContactChange>) {
        let _ = self.tx.try_send(Job::Contacts(change.map(Box::new)));
    }

    /// Asks for the entries of the Conversation History folder.
    /// Brings the offline address book up to date.
    pub fn address_book(&self, services: &ServerConfiguration) {
        let _ = self
            .tx
            .try_send(Job::AddressBook(Box::new(services.clone())));
    }

    pub fn history_list(&self) {
        let _ = self.tx.try_send(Job::HistoryList);
    }

    /// Asks for one entry's text.
    pub fn history_body(&self, id: commscraft_ews::ucs::ItemId) {
        let _ = self.tx.try_send(Job::HistoryBody(id));
    }

    /// Saves a finished conversation in the Conversation History folder.
    pub fn save_history(&self, t: commscraft_ews::history::Conversation) {
        let _ = self.tx.try_send(Job::SaveHistory(Box::new(t)));
    }

    /// Asks the join manager for the focus of a meeting link.
    pub fn meeting_link(&self, link: &str) {
        let _ = self.tx.try_send(Job::MeetingLink(link.to_string()));
    }

    /// Asks for the user's appointments and automatic reply.
    pub fn calendar(&self, policy: &commscraft_provisioning::CalendarPolicy) {
        let _ = self.tx.try_send(Job::Calendar(policy.clone()));
    }
}

fn run(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    rx: &Receiver<Job>,
    ntlm_tx: &Sender<NtlmAsk>,
    done: &Sender<Done>,
) {
    let ua = format!("CommSCraft/{}", env!("CARGO_PKG_VERSION"));
    let mut settings = None;
    // Photos done with (shown fresh, or none on the servers); the failed ones are asked for
    // again later (`retry`: when, how often already, the services they were asked with).
    let mut seen = std::collections::BTreeSet::new();
    type Services = Option<Box<ServerConfiguration>>;
    let mut retry: std::collections::BTreeMap<String, (std::time::Instant, u32, Services)> =
        std::collections::BTreeMap::new();
    // The UCWA path (function 15, second path): opened when first needed, again after its token
    // ends; after a failure not tried again for a while.
    let mut ucwa: Option<commscraft_client::ucwa::Session> = None;
    let mut ucwa_retry_at: Option<std::time::Instant> = None;
    loop {
        // New requests first; else a failed photo whose time has come; else wait.
        let job = match rx.try_recv() {
            Ok(j) => j,
            Err(mpsc::TryRecvError::Disconnected) => return,
            Err(mpsc::TryRecvError::Empty) => {
                let now = std::time::Instant::now();
                let due = retry
                    .iter()
                    .find(|(_, (at, _, _))| *at <= now)
                    .map(|(k, (_, _, svc))| (k.clone(), svc.clone()));
                match due {
                    Some((uri, svc)) => Job::Photo(uri, svc),
                    None => {
                        let wait = retry
                            .values()
                            .map(|(at, _, _)| at.saturating_duration_since(now))
                            .min()
                            .unwrap_or(RETRY_MAX)
                            .max(std::time::Duration::from_secs(1));
                        match rx.recv_timeout(wait) {
                            Ok(j) => j,
                            Err(mpsc::RecvTimeoutError::Timeout) => continue,
                            Err(mpsc::RecvTimeoutError::Disconnected) => return,
                        }
                    }
                }
            }
        };
        let mut ntlm = |n: &[u8], c: &[u8], cert: Option<&[u8]>| {
            let (reply, answer) = mpsc::channel();
            ntlm_tx
                .send(NtlmAsk::Ntlm {
                    negotiate: n.to_vec(),
                    challenge: c.to_vec(),
                    cert: cert.map(<[u8]>::to_vec),
                    reply,
                })
                .map_err(|_| "signed out".to_string())?;
            answer.recv().map_err(|_| "signed out".to_string())?
        };
        let mut grant = || {
            let (reply, answer) = mpsc::channel();
            ntlm_tx
                .send(NtlmAsk::PasswordGrant { reply })
                .map_err(|_| "signed out".to_string())?;
            answer.recv().map_err(|_| "signed out".to_string())?
        };
        let mut log = |l: &str| {
            if std::env::var_os("COMMSCRAFT_VERBOSE").is_some() {
                eprintln!("photo: {l}");
            }
        };
        let (uri, services) = match job {
            Job::Photo(uri, services) => (uri, services),
            Job::Expand(email, services) => {
                // A distribution group's members have their own photo path in UCWA: the
                // expanded group names each member's photo link (UW-09), so the photos asked
                // for next need no search each.
                if photos_allowed(services.as_deref()) {
                    if let Some(s) = ucwa_session(
                        &mut ucwa,
                        &mut ucwa_retry_at,
                        tls,
                        trust,
                        aor,
                        &ua,
                        &mut ntlm,
                        &mut grant,
                        &mut log,
                    ) {
                        if let Err(e) = s.expand_group(tls, &email, &ua, &mut log) {
                            log(&format!("  UCWA group {email}: {e}"));
                        }
                    }
                }
                let found = commscraft_client::directory::expand(
                    tls,
                    trust,
                    services.as_deref(),
                    &email,
                    &ua,
                    &mut ntlm,
                    &mut log,
                );
                if done.send(Done::Expanded(email, found)).is_err() {
                    return;
                }
                continue;
            }
            Job::AddressBook(services) => {
                let today = commscraft_client::address_book::abs_today(now_secs());
                let r = match today {
                    None => Err("clock out of range".to_string()),
                    Some(today) => {
                        let mut get = |url: &commscraft_url::HttpsUrl| {
                            commscraft_client::photos::ntlm_get(
                                tls,
                                url,
                                &[("Accept", "application/octet-stream")],
                                &ua,
                                std::time::Duration::from_secs(60),
                                commscraft_client::address_book::ADDRESS_BOOK_MAX,
                                &mut ntlm,
                            )
                        };
                        commscraft_client::address_book::fetch(
                            &mut get,
                            aor,
                            trust,
                            Some(&services),
                            true,
                            today,
                            &mut log,
                        )
                        .map(|(b, how)| (Box::new(b), how))
                    }
                };
                if done.send(Done::AddressBook(r)).is_err() {
                    return;
                }
                continue;
            }
            Job::HistoryList | Job::HistoryBody(_) => {
                if settings.is_none() {
                    settings = Some(
                        commscraft_client::photos::ews_settings(
                            tls, trust, aor, &ua, &mut ntlm, &mut log,
                        )
                        .ok(),
                    );
                }
                let Some(Some(s)) = settings.as_ref() else {
                    let e: Result<(), String> = Err("Exchange is not reachable".into());
                    let d = match job_kind_body(&job) {
                        Some(id) => Done::HistoryBody(id, e.map(|_| String::new())),
                        None => Done::HistoryList(e.map(|_| Vec::new())),
                    };
                    if done.send(d).is_err() {
                        return;
                    }
                    continue;
                };
                let d = match job {
                    Job::HistoryBody(id) => Done::HistoryBody(
                        id.id.clone(),
                        commscraft_client::exchange::history_body(
                            tls, trust, aor, s, &id, &ua, &mut ntlm, &mut log,
                        ),
                    ),
                    _ => Done::HistoryList(commscraft_client::exchange::history_list(
                        tls, trust, aor, s, &ua, &mut ntlm, &mut log,
                    )),
                };
                if done.send(d).is_err() {
                    return;
                }
                continue;
            }
            Job::SaveHistory(t) => {
                if settings.is_none() {
                    settings = Some(
                        commscraft_client::photos::ews_settings(
                            tls, trust, aor, &ua, &mut ntlm, &mut log,
                        )
                        .ok(),
                    );
                }
                let saved = match settings.as_ref() {
                    Some(Some(s)) => commscraft_client::exchange::save_conversation(
                        tls, trust, aor, s, &t, &ua, &mut ntlm, &mut log,
                    ),
                    _ => Err("Exchange is not reachable".into()),
                };
                if done.send(Done::HistorySaved(saved)).is_err() {
                    return;
                }
                continue;
            }
            Job::MeetingLink(link) => {
                log(&format!("  GET meeting link ({} bytes)", link.len()));
                let found = commscraft_client::setup::meeting_focus(tls, &link, &ua);
                if done.send(Done::MeetingFocus(found)).is_err() {
                    return;
                }
                continue;
            }
            Job::Contacts(change) => {
                if settings.is_none() {
                    settings = Some(
                        commscraft_client::photos::ews_settings(
                            tls, trust, aor, &ua, &mut ntlm, &mut log,
                        )
                        .ok(),
                    );
                }
                let found = match (settings.as_ref(), change) {
                    (Some(Some(s)), None) => commscraft_client::exchange::contact_list(
                        tls, trust, aor, s, &ua, &mut ntlm, &mut log,
                    ),
                    (Some(Some(s)), Some(c)) => {
                        let (edit, list, ids) = *c;
                        commscraft_client::exchange::change_contact_list(
                            tls,
                            trust,
                            aor,
                            s,
                            &edit,
                            (&list, &ids),
                            &ua,
                            &mut ntlm,
                            &mut log,
                        )
                    }
                    _ => Err("Exchange is not reachable".into()),
                };
                if done.send(Done::Contacts(found)).is_err() {
                    return;
                }
                continue;
            }
            Job::Calendar(policy) => {
                if settings.is_none() {
                    settings = Some(
                        commscraft_client::photos::ews_settings(
                            tls, trust, aor, &ua, &mut ntlm, &mut log,
                        )
                        .ok(),
                    );
                }
                let found = match settings.as_ref() {
                    Some(Some(s)) => commscraft_client::exchange::appointments(
                        tls,
                        trust,
                        aor,
                        s,
                        &policy,
                        now_secs(),
                        &ua,
                        &mut ntlm,
                        &mut log,
                    ),
                    _ => Err("no Exchange web services".into()),
                };
                if done.send(Done::Calendar(found)).is_err() {
                    return;
                }
                continue;
            }
        };
        let key = uri.to_ascii_lowercase();
        if seen.contains(&key) {
            continue;
        }
        // A kept photo at once (as the mobile clients show theirs); a fresh one needs no request.
        let had = kept(aor, &uri);
        if let Some((bytes, fresh)) = &had {
            if send_thumb(done, &uri, bytes).is_err() {
                return;
            }
            if *fresh {
                seen.insert(key.clone());
                retry.remove(&key);
                continue;
            }
        }
        // Diagnosis: `COMMSCRAFT_PHOTOS_UCWA_FIRST=1` skips the directory and Exchange paths.
        let ucwa_first = std::env::var("COMMSCRAFT_PHOTOS_UCWA_FIRST").is_ok_and(|v| v == "1");
        let got = if ucwa_first {
            Err("directory and Exchange skipped (COMMSCRAFT_PHOTOS_UCWA_FIRST)".to_string())
        } else {
            commscraft_client::photos::fetch(
                tls,
                trust,
                aor,
                services.as_deref(),
                &uri,
                &ua,
                &mut settings,
                &mut ntlm,
                &mut log,
            )
        };
        let outcome = match got {
            Ok(path) => match std::fs::read(&path) {
                Ok(b) => Outcome::Photo(b),
                Err(_) => Outcome::Failed,
            },
            Err(e) => {
                log(&format!("  {e}"));
                if photos_allowed(services.as_deref()) {
                    let limit = services.as_deref().and_then(|c| c.max_photo_kb).map_or(
                        commscraft_client::photos::PHOTO_MAX,
                        |kb| {
                            (kb as usize)
                                .saturating_mul(1024)
                                .min(commscraft_client::photos::PHOTO_MAX)
                        },
                    );
                    // Second path: the photo through the Skype for Business server (UCWA); a
                    // refused token opens the session again once, with a new token.
                    let mut o = Outcome::Failed;
                    for _ in 0..2 {
                        let Some(s) = ucwa_session(
                            &mut ucwa,
                            &mut ucwa_retry_at,
                            tls,
                            trust,
                            aor,
                            &ua,
                            &mut ntlm,
                            &mut grant,
                            &mut log,
                        ) else {
                            break;
                        };
                        match s.photo(tls, &uri, limit, &ua, &mut log) {
                            Ok(Some(b)) => o = Outcome::Photo(b),
                            Ok(None) => o = Outcome::None,
                            Err(e) => {
                                log(&format!("  UCWA: {e}"));
                                if s.refused() {
                                    continue;
                                }
                            }
                        }
                        break;
                    }
                    o
                } else {
                    Outcome::None
                }
            }
        };
        match outcome {
            Outcome::Photo(bytes) if thumbnail(&bytes).is_some() => {
                // Kept for the next start and for times the servers cannot be reached.
                if let Some(path) = shown_file(aor, &uri) {
                    if let Err(e) = commscraft_client::photos::write_private(&path, &bytes) {
                        log(&format!("  photo not kept: {e}"));
                    }
                }
                let same = had.as_ref().is_some_and(|(b, _)| *b == bytes);
                if !same && send_thumb(done, &uri, &bytes).is_err() {
                    return;
                }
                seen.insert(key.clone());
                retry.remove(&key);
            }
            Outcome::Photo(_) | Outcome::None => {
                seen.insert(key.clone());
                retry.remove(&key);
            }
            Outcome::Failed => {
                let attempts = retry.get(&key).map_or(0, |(_, n, _)| *n) + 1;
                if attempts >= MAX_ATTEMPTS {
                    seen.insert(key.clone());
                    retry.remove(&key);
                } else {
                    let wait = retry_wait(attempts);
                    log(&format!(
                        "  photo of {key}: asked for again in {} s",
                        wait.as_secs()
                    ));
                    retry.insert(key, (std::time::Instant::now() + wait, attempts, services));
                }
            }
        }
    }
}

/// Hands a photo's thumbnail to the session (a picture that does not decode is skipped); an
/// error when the session is gone.
fn send_thumb(done: &Sender<Done>, uri: &str, bytes: &[u8]) -> Result<(), ()> {
    match thumbnail(bytes) {
        Some(rgba) => done
            .send(Done::Thumb(Thumb {
                uri: uri.to_string(),
                side: THUMB,
                rgba,
            }))
            .map_err(|_| ()),
        None => Ok(()),
    }
}

/// The server's photo policy allows photos (none known: allowed).
fn photos_allowed(services: Option<&ServerConfiguration>) -> bool {
    services.is_none_or(|c| c.photo_usage != Some(commscraft_provisioning::PhotoUsage::NoPhoto))
}

/// The UCWA session, opened if needed (or when its token is near its end); `None` after a failure
/// until [`UCWA_RETRY`] has passed.
#[allow(clippy::too_many_arguments)]
fn ucwa_session<'a>(
    slot: &'a mut Option<commscraft_client::ucwa::Session>,
    retry_at: &mut Option<std::time::Instant>,
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    ua: &str,
    ntlm: &mut commscraft_client::directory::HttpNtlm<'_>,
    grant: &mut commscraft_client::ucwa::PasswordGrant<'_>,
    log: &mut dyn FnMut(&str),
) -> Option<&'a mut commscraft_client::ucwa::Session> {
    if slot.as_ref().is_some_and(|s| s.expired()) {
        *slot = None;
    }
    if slot.is_none() {
        if retry_at.is_some_and(|t| std::time::Instant::now() < t) {
            return None;
        }
        match commscraft_client::ucwa::Session::open(tls, trust, aor, ua, ntlm, grant, log) {
            Ok(s) => {
                log(&format!("  UCWA: {s:?} opened"));
                *slot = Some(s);
                *retry_at = None;
            }
            Err(e) => {
                log(&format!("  UCWA not usable: {e}"));
                *retry_at = Some(std::time::Instant::now() + UCWA_RETRY);
                return None;
            }
        }
    }
    slot.as_mut()
}

/// After UCWA failed, it is tried again only after this time (design).
const UCWA_RETRY: std::time::Duration = std::time::Duration::from_secs(15 * 60);

/// The item ID of a history body job.
fn job_kind_body(job: &Job) -> Option<String> {
    match job {
        Job::HistoryBody(id) => Some(id.id.clone()),
        _ => None,
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Decodes a JPEG or PNG photo and makes a square thumbnail (centre crop, nearest pixel).
pub fn thumbnail(bytes: &[u8]) -> Option<Vec<u8>> {
    let (w, h, rgba) = decode(bytes)?;
    let side = w.min(h);
    let (x0, y0) = ((w - side) / 2, (h - side) / 2);
    let mut out = Vec::with_capacity(THUMB * THUMB * 4);
    for ty in 0..THUMB {
        for tx in 0..THUMB {
            let sx = x0 + tx * side / THUMB;
            let sy = y0 + ty * side / THUMB;
            let i = (sy * w + sx) * 4;
            out.extend_from_slice(rgba.get(i..i + 4)?);
        }
    }
    Some(out)
}

fn decode(bytes: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    match commscraft_client::photos::photo_kind(bytes)? {
        "jpg" => {
            use zune_jpeg::zune_core::{
                bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions,
            };
            let options = DecoderOptions::default()
                .jpeg_set_out_colorspace(ColorSpace::RGBA)
                .set_max_width(MAX_SIDE)
                .set_max_height(MAX_SIDE);
            let mut d = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(bytes), options);
            let px = d.decode().ok()?;
            let (w, h) = d.dimensions()?;
            (w > 0 && h > 0 && px.len() == w * h * 4).then_some((w, h, px))
        }
        _ => {
            let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
            dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
            let mut reader = dec.read_info().ok()?;
            let (w, h) = (reader.info().width as usize, reader.info().height as usize);
            if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE {
                return None;
            }
            let mut buf = vec![0; reader.output_buffer_size()?];
            let info = reader.next_frame(&mut buf).ok()?;
            let data = buf.get(..info.buffer_size())?;
            let rgba: Vec<u8> = match info.color_type {
                png::ColorType::Rgba => data.to_vec(),
                png::ColorType::Rgb => data
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .flat_map(|[r, g, b]| [*r, *g, *b, 255])
                    .collect(),
                png::ColorType::GrayscaleAlpha => data
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .flat_map(|[g, a]| [*g, *g, *g, *a])
                    .collect(),
                png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
                png::ColorType::Indexed => return None,
            };
            (rgba.len() == w * h * 4).then_some((w, h, rgba))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostile_or_odd_files_give_no_thumbnail() {
        assert!(thumbnail(b"").is_none());
        assert!(thumbnail(b"<svg/>").is_none());
        assert!(thumbnail(&[0xff, 0xd8, 0xff, 0xe0, 0, 0]).is_none());
        assert!(thumbnail(b"\x89PNG\r\n\x1a\n").is_none());
    }

    #[test]
    fn a_png_becomes_a_square_thumbnail() {
        let mut png_bytes = Vec::new();
        {
            let mut e = png::Encoder::new(&mut png_bytes, 4, 2);
            e.set_color(png::ColorType::Rgb);
            let mut w = e.write_header().unwrap();
            w.write_image_data(&[255u8; 4 * 2 * 3]).unwrap();
        }
        let t = thumbnail(&png_bytes).unwrap();
        assert_eq!(t.len(), THUMB * THUMB * 4);
        assert_eq!(&t[..4], &[255, 255, 255, 255]);
    }
}

#[cfg(test)]
mod kept_tests {
    use super::*;

    /// Photos shown are kept per account (names one-to-one), read back only as JPEG or PNG;
    /// failures are asked for again with a growing, bounded wait.
    #[test]
    fn kept_photos_and_retries() {
        let base = std::env::temp_dir().join(format!("commscraft-kept-{}", std::process::id()));
        let a = shown_file_in(&base, "sip:me@contoso.com", "sip:Anna@contoso.com").unwrap();
        let b = shown_file_in(&base, "sip:me@contoso.com", "sip:anna@contoso.com").unwrap();
        assert_eq!(a, b, "addresses compare without case");
        assert!(a.starts_with(base.join("me@contoso%2Ecom").join("shown")));
        assert_ne!(
            shown_file_in(&base, "sip:me@contoso.com", "sip:a.b@c").unwrap(),
            shown_file_in(&base, "sip:me@contoso.com", "sip:a_b@c").unwrap()
        );
        assert!(kept_at(&a).is_none());
        commscraft_client::photos::write_private(&a, b"not a picture").unwrap();
        assert!(kept_at(&a).is_none(), "only JPEG and PNG");
        let png = b"\x89PNG\r\n\x1a\nrest".to_vec();
        commscraft_client::photos::write_private(&a, &png).unwrap();
        assert_eq!(kept_at(&a), Some((png, true)));
        let _ = std::fs::remove_dir_all(&base);
        assert_eq!(retry_wait(1), RETRY_FIRST);
        assert_eq!(retry_wait(2), RETRY_FIRST * 2);
        assert_eq!(retry_wait(30), RETRY_MAX);
    }
}
