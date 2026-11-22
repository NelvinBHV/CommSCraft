//! The offline address book (function 13; [MS-ABS] AB-01, AB-10), shared by the command-line
//! and the desktop client: the newest full file or a delta from the provisioned address book
//! service, kept in the user's cache directory. The caller does the authenticated GET.
//!
//! Security by design: only hosts in the user's SIP domain or a trusted domain are asked (they
//! get credentials); downloads are bounded by [`ADDRESS_BOOK_MAX`]; cache directories are 0700,
//! files 0600; names are encoded one-to-one.

use crate::photos::{cache_name, legacy_cache_name};
use crate::setup::Trust;
use std::io::Write;
use std::path::PathBuf;

/// How many days back the newest full address book file is looked for (design: the server
/// keeps the most recent N > 0 days, [MS-ABS] §1.3, p. 9; it writes the day's file at night).
pub const ADDRESS_BOOK_DAYS: u16 = 7;
/// Largest compressed address book file downloaded (design).
pub const ADDRESS_BOOK_MAX: usize = 64 * 1024 * 1024;

/// A GET of one URL (the caller authenticates it, NTLM per MS-NTHT).
pub type Get<'a> =
    dyn FnMut(&commscraft_url::HttpsUrl) -> Result<commscraft_net::http::HttpResponse, String> + 'a;

/// One address book file under a base URL: `Ok(None)` when the server does not have it (404).
fn abs_file(
    get: &mut Get<'_>,
    base: &commscraft_provisioning::ServiceUrl,
    name: &str,
    log: &mut dyn FnMut(&str),
) -> Result<Option<Vec<u8>>, String> {
    let full = format!("{}/{name}", base.url.trim_end_matches('/'));
    let url = commscraft_url::parse_https(&full).ok_or("unusable address book URL")?;
    log(&format!("  GET {full}"));
    let r = get(&url)?;
    match r.status {
        200 => Ok(Some(r.body)),
        404 => Ok(None),
        401 => Err("not authorized (401)".into()),
        s => Err(format!("HTTP {s}")),
    }
}

/// Where the newest full address book file of a service is kept: the user's cache directory,
/// one directory per signed-in account and one file per service URL
/// (`$XDG_CACHE_HOME/CommSCraft/abs/ACCOUNT/HOST_PATH.lsabs`, directories 0700, names encoded
/// one-to-one by [`cache_name`]). Issues #124, #129: two accounts of the same Unix user never
/// share a cached directory. Files of the earlier layouts (one per host for every account; a
/// lossy account name) are removed.
pub fn abs_cache_path(
    account: &str,
    base: &commscraft_provisioning::ServiceUrl,
) -> Option<PathBuf> {
    use std::os::unix::fs::DirBuilderExt;
    let (dir, file) = abs_cache_names(account, base)?;
    let root = crate::xdg::dir(crate::xdg::Kind::Cache)?.join("abs");
    let _ = std::fs::remove_file(root.join(format!("{}.lsabs", base.host)));
    let legacy = legacy_cache_name(account.trim_start_matches("sip:"));
    if legacy != dir && legacy.contains('@') && !legacy.starts_with('.') {
        let _ = std::fs::remove_dir_all(root.join(legacy));
    }
    let dir = root.join(dir);
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)
        .ok()?;
    Some(dir.join(file))
}

/// The account directory and the file name of [`abs_cache_path`].
pub fn abs_cache_names(
    account: &str,
    base: &commscraft_provisioning::ServiceUrl,
) -> Option<(String, String)> {
    let host = &base.host;
    if host.is_empty()
        || !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-".contains(c))
    {
        return None;
    }
    let account = account.trim_start_matches("sip:");
    if !account.contains('@') {
        return None;
    }
    let path = commscraft_url::parse_https(&base.url)
        .map(|u| u.path)
        .unwrap_or_default();
    Some((
        cache_name(account)?,
        format!("{}.lsabs", cache_name(&format!("{host}{path}"))?),
    ))
}

/// Keeps a downloaded full file (compressed, as the server sent it; mode 0600).
fn abs_cache_store(path: &std::path::Path, file: &[u8], log: &mut dyn FnMut(&str)) {
    use std::os::unix::fs::OpenOptionsExt;
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .and_then(|mut f| f.write_all(file));
    if let Err(e) = written {
        log(&format!("address book not cached: {e}"));
    }
}

/// The offline address book (function 13; [MS-ABS] AB-01, AB-10) from the provisioned address
/// book service (internal first, then external); only hosts in the user's SIP domain or a
/// trusted domain get credentials; NTLM over HTTP as for the search.
///
/// With the disk cache (default; `--no-address-book-cache` turns it off) and a cached full file
/// F-XXXX the newest delta D-XXXX-YYYY of the last [`ADDRESS_BOOK_DAYS`] days is fetched and applied (the server makes deltas from each full
/// file it keeps to the newest one, §1.3, p. 10). Without a delta a newer full file is fetched;
/// if there is none, the cache is current. Without a cache, or when it is unusable, the newest
/// full file is fetched and cached. Returns the book and how it was obtained.
#[allow(clippy::too_many_arguments)]
pub fn fetch(
    get: &mut Get<'_>,
    account: &str,
    trust: &Trust,
    services: Option<&commscraft_provisioning::ServerConfiguration>,
    use_cache: bool,
    today: u16,
    log: &mut dyn FnMut(&str),
) -> Result<(commscraft_abs::book::AddressBook, String), String> {
    use commscraft_abs::book::{decompress, AddressBook, Delta};
    let c = services.ok_or("no provisioning yet (address book service unknown)")?;
    if !address_book_file_allowed(c) {
        return Err("the server's policy allows the web search only (AbsUsage)".into());
    }
    let bases = address_book_bases(c);
    if bases.is_empty() {
        return Err("the server announced no address book service".into());
    }
    let mut last = String::from("no usable service");
    'bases: for base in bases {
        if !base.in_user_domain && !trust.trusts(&base.host) {
            last = format!(
                "{} is outside your SIP domain; add --trust-domain to use it",
                base.host
            );
            continue;
        }
        // The disk cache unless the user turned it off (`--no-address-book-cache`).
        let cache = use_cache.then(|| abs_cache_path(account, base)).flatten();
        let cached = cache
            .as_deref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|f| decompress(&f).ok())
            .and_then(|raw| AddressBook::parse(&raw).ok())
            .filter(|b| b.created <= today);
        let oldest = today.saturating_sub(ADDRESS_BOOK_DAYS - 1);
        if let Some(mut book) = cached {
            let from = book.created;
            // Issue #126: a refused delta leaves the book as it was (`AddressBook::apply`), but
            // the book is then not reported as current.
            let mut unusable: Option<String> = None;
            for day in (oldest.max(from + 1)..=today).rev() {
                let name = format!("D-{from:04X}-{day:04X}.lsabs");
                match abs_file(get, base, &name, log) {
                    Ok(Some(file)) => {
                        let applied = decompress(&file)
                            .map_err(|e| format!("{e:?}"))
                            .and_then(|raw| Delta::parse(&raw).map_err(|e| format!("{e:?}")))
                            .and_then(|d| book.apply(&d).map_err(|e| format!("{e:?}")));
                        match applied {
                            Ok(()) => return Ok((book, format!("cache and {name}"))),
                            Err(e) => {
                                // A bad delta: fall back to the full file.
                                log(&format!("{name} not used: {e}"));
                                unusable = Some(name);
                                break;
                            }
                        }
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        last = e;
                        continue 'bases;
                    }
                }
            }
            // No usable delta: a newer full file, else the cache is current.
            for day in (oldest.max(from + 1)..=today).rev() {
                let name = format!("F-{day:04X}.lsabs");
                match abs_file(get, base, &name, log) {
                    Ok(Some(file)) => {
                        let raw = decompress(&file).map_err(|e| format!("{name}: {e:?}"))?;
                        let b = AddressBook::parse(&raw).map_err(|e| format!("{name}: {e:?}"))?;
                        if let Some(p) = cache.as_deref() {
                            abs_cache_store(p, &file, log);
                        }
                        return Ok((b, name));
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        last = e;
                        continue 'bases;
                    }
                }
            }
            if book.created == from && from >= oldest {
                let source = match unusable {
                    Some(name) => format!("cache (not updated: {name} not usable)"),
                    None => "cache (current)".into(),
                };
                return Ok((book, source));
            }
        }
        for day in (oldest..=today).rev() {
            let name = format!("F-{day:04X}.lsabs");
            match abs_file(get, base, &name, log) {
                Ok(Some(file)) => {
                    let raw = decompress(&file).map_err(|e| format!("{name}: {e:?}"))?;
                    let b = AddressBook::parse(&raw).map_err(|e| format!("{name}: {e:?}"))?;
                    if let Some(p) = cache.as_deref() {
                        abs_cache_store(p, &file, log);
                    }
                    return Ok((b, name));
                }
                // Not (yet) written for this day: the day before.
                Ok(None) => continue,
                Err(e) => {
                    last = e;
                    break;
                }
            }
        }
    }
    Err(last)
}

/// Today as an address book day number: days since 2001-01-01 (day 11 323 since 1970-01-01).
pub fn abs_today(now: u64) -> Option<u16> {
    u16::try_from((now / 86_400).checked_sub(11_323)?).ok()
}

/// The provisioned address book services, internal first.
pub fn address_book_bases(
    c: &commscraft_provisioning::ServerConfiguration,
) -> Vec<&commscraft_provisioning::ServiceUrl> {
    [&c.abs_internal, &c.abs_external]
        .into_iter()
        .flatten()
        .collect()
}

/// RG-53: the address book file may be used unless the policy allows the web search only (no
/// `AbsUsage` counts as the server example's `WebSearchAndFileDownload`).
pub fn address_book_file_allowed(c: &commscraft_provisioning::ServerConfiguration) -> bool {
    c.abs_usage.is_none_or(|u| u.file_allowed())
}

/// The address book kept from an earlier session, without network access (the Windows client
/// uses its local copy at once, too): the cached full file of the first usable service. Its
/// changes since then come with the next fetch.
pub fn cached(
    account: &str,
    trust: &Trust,
    c: &commscraft_provisioning::ServerConfiguration,
    today: u16,
) -> Option<commscraft_abs::book::AddressBook> {
    use commscraft_abs::book::{decompress, AddressBook};
    if !address_book_file_allowed(c) {
        return None;
    }
    address_book_bases(c)
        .into_iter()
        .filter(|b| b.in_user_domain || trust.trusts(&b.host))
        .find_map(|b| {
            let file = std::fs::read(abs_cache_path(account, b)?).ok()?;
            let raw = decompress(&file).ok()?;
            AddressBook::parse(&raw).ok().filter(|b| b.created <= today)
        })
}
