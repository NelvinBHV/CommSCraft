//! A minimal HTTPS GET client for Lync Autodiscover (MS-OCDISCWS §2.1: HTTP over TLS).
//!
//! Security by design: only `https://` (checked by `commscraft-url`), certificate verification by
//! the shared [`TlsClient`](crate::TlsClient), `Connection: close`, a bounded response (size,
//! header count, line length), Content-Length and chunked encoding parsed strictly, both at once
//! refused (request smuggling), a response must end exactly where its framing says (RFC 9112 §6.3,
//! §7.1: last chunk, trailer section and final CRLF read completely; surplus octets refused, so a
//! kept-alive connection never starts the next response mid-stream), no cookies, no credentials. Redirects are not followed here; the
//! discovery state machine decides about them.

use std::io::{Read, Write};
use std::time::Duration;

use commscraft_url::HttpsUrl;

use crate::{NetError, TlsClient};

/// The authorization callback of [`post_with_auth`]: (challenge, server certificate DER) → the
/// next `Authorization` header value.
pub type Authorize<'a> = dyn FnMut(Option<&str>, Option<&[u8]>) -> Result<String, NetError> + 'a;

/// Upper bound of a whole response (head and body) unless the caller names another one.
pub const MAX_RESPONSE: usize = 256 * 1024;
/// Upper bound of the head of a response (status line and header fields).
const MAX_HEAD: usize = 256 * 1024;
const MAX_HEADERS: usize = 100;
const MAX_LINE: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    /// Header names in lower case, values trimmed.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// The value of a header that must occur at most once.
    pub fn header(&self, name: &str) -> Result<Option<&str>, NetError> {
        let mut it = self.headers.iter().filter(|(n, _)| n == name);
        let first = it.next().map(|(_, v)| v.as_str());
        if it.next().is_some() {
            return Err(bad("repeated header"));
        }
        Ok(first)
    }
}

fn bad(what: &str) -> NetError {
    NetError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("HTTP: {what}"),
    ))
}

/// Parses a complete response as read until the server closed the connection.
pub fn parse_response(data: &[u8]) -> Result<HttpResponse, NetError> {
    parse_response_within(data, MAX_RESPONSE)
}

/// [`parse_response`] for a response of up to `limit` octets.
fn parse_response_within(data: &[u8], limit: usize) -> Result<HttpResponse, NetError> {
    frame(data, true, limit)?.ok_or_else(|| bad("incomplete response"))
}

/// One response from `data`: `Ok(None)` while more octets are needed, an error for anything
/// malformed or for octets after the end of the response. `closed`: the server closed the
/// connection, so nothing more will come (a body without framing ends here). `limit`: the
/// largest whole response the caller accepts (issue #130: the same bound on every layer).
fn frame(data: &[u8], closed: bool, limit: usize) -> Result<Option<HttpResponse>, NetError> {
    if data.len() > limit {
        return Err(bad("response too large"));
    }
    let Some(head_end) = data.windows(4).position(|w| w == b"\r\n\r\n") else {
        return if closed {
            Err(bad("incomplete head"))
        } else {
            Ok(None)
        };
    };
    let head = std::str::from_utf8(data.get(..head_end).unwrap_or_default())
        .map_err(|_| bad("head not UTF-8"))?;
    let mut lines = head.split("\r\n");
    let status_line = lines.next().ok_or_else(|| bad("no status line"))?;
    let mut parts = status_line.splitn(3, ' ');
    let version = parts.next().unwrap_or("");
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(bad("version"));
    }
    let code = parts.next().unwrap_or("");
    if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("status code"));
    }
    let status: u16 = code.parse().map_err(|_| bad("status code"))?;
    let mut headers = Vec::new();
    for line in lines {
        if line.len() > MAX_LINE || line.chars().any(|c| c.is_control() && c != '\t') {
            return Err(bad("header line"));
        }
        if line.starts_with([' ', '\t']) {
            return Err(bad("obsolete line folding"));
        }
        let (n, v) = line
            .split_once(':')
            .ok_or_else(|| bad("header without colon"))?;
        if n.is_empty()
            || !n
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return Err(bad("header name"));
        }
        if headers.len() >= MAX_HEADERS {
            return Err(bad("too many headers"));
        }
        headers.push((n.to_ascii_lowercase(), v.trim().to_string()));
    }
    let resp = HttpResponse {
        status,
        headers,
        body: Vec::new(),
    };
    let raw_body = data.get(head_end + 4..).unwrap_or_default();
    let te = resp.header("transfer-encoding")?;
    let cl = resp.header("content-length")?;
    let incomplete = |what| if closed { Err(bad(what)) } else { Ok(None) };
    let body = match (te, cl) {
        (Some(_), Some(_)) => return Err(bad("both Transfer-Encoding and Content-Length")),
        (Some(t), None) if t.eq_ignore_ascii_case("chunked") => match dechunk(raw_body, limit)? {
            Some((body, used)) if used == raw_body.len() => body,
            Some(_) => return Err(bad("data after the last chunk")),
            None => return incomplete("chunked body incomplete"),
        },
        (Some(_), None) => return Err(bad("unsupported transfer coding")),
        (None, Some(l)) => {
            let n = content_length(l, limit)?;
            match raw_body.len().cmp(&n) {
                std::cmp::Ordering::Less => return incomplete("body shorter than Content-Length"),
                std::cmp::Ordering::Greater => return Err(bad("data after Content-Length")),
                std::cmp::Ordering::Equal => raw_body.to_vec(),
            }
        }
        // RFC 9112 §6.3: 1xx, 204 and 304 never have a body.
        (None, None) if matches!(status, 100..=199 | 204 | 304) => {
            if !raw_body.is_empty() {
                return Err(bad("body in a response without one"));
            }
            Vec::new()
        }
        // Without framing the body ends when the server closes the connection.
        (None, None) if closed => raw_body.to_vec(),
        (None, None) => return Ok(None),
    };
    Ok(Some(HttpResponse { body, ..resp }))
}

/// A Content-Length value: decimal digits only, at most `limit` (the whole response cannot be
/// shorter than its body).
fn content_length(l: &str, limit: usize) -> Result<usize, NetError> {
    if l.is_empty() || l.len() > 19 || !l.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("Content-Length"));
    }
    let n: u64 = l.parse().map_err(|_| bad("Content-Length"))?;
    usize::try_from(n)
        .ok()
        .filter(|n| *n <= limit)
        .ok_or_else(|| bad("response too large"))
}

/// Chunked body (RFC 9112 §7.1): `Ok(None)` while incomplete, else the body and the octets used
/// up to and including the CRLF after the trailer section (trailer fields are read and dropped).
fn dechunk(all: &[u8], limit: usize) -> Result<Option<(Vec<u8>, usize)>, NetError> {
    let mut data = all;
    let mut out = Vec::new();
    let next_line = |d: &[u8]| -> Result<Option<usize>, NetError> {
        match d.windows(2).position(|w| w == b"\r\n") {
            Some(n) if n > MAX_LINE => Err(bad("line too long")),
            Some(n) => Ok(Some(n)),
            None if d.len() > MAX_LINE => Err(bad("line too long")),
            None => Ok(None),
        }
    };
    loop {
        let Some(line_end) = next_line(data)? else {
            return Ok(None);
        };
        let line = std::str::from_utf8(data.get(..line_end).unwrap_or_default())
            .map_err(|_| bad("chunk size"))?;
        let size_text = line.split(';').next().unwrap_or("").trim();
        if size_text.is_empty() || size_text.len() > 8 {
            return Err(bad("chunk size"));
        }
        if !size_text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(bad("chunk size"));
        }
        let size = usize::from_str_radix(size_text, 16).map_err(|_| bad("chunk size"))?;
        data = data.get(line_end + 2..).ok_or_else(|| bad("chunk"))?;
        if size == 0 {
            // Trailer section: field lines up to an empty line.
            for _ in 0..=MAX_HEADERS {
                let Some(end) = next_line(data)? else {
                    return Ok(None);
                };
                let field = data.get(..end).unwrap_or_default();
                data = data.get(end + 2..).ok_or_else(|| bad("trailer"))?;
                if field.is_empty() {
                    return Ok(Some((out, all.len() - data.len())));
                }
                if !field.contains(&b':') || field.iter().any(|&b| b < 0x20 && b != b'\t') {
                    return Err(bad("trailer field"));
                }
            }
            return Err(bad("too many trailer fields"));
        }
        if out.len().saturating_add(size) > limit {
            return Err(bad("body too large"));
        }
        let Some(chunk) = data.get(..size) else {
            return Ok(None);
        };
        out.extend_from_slice(chunk);
        match data.get(size..size + 2) {
            None if data.len() < size + 2 => return Ok(None),
            Some(b"\r\n") => {}
            _ => return Err(bad("chunk terminator")),
        }
        data = data.get(size + 2..).ok_or_else(|| bad("chunk"))?;
    }
}

/// GET `url` with the given `Accept` header over verified TLS.
pub fn get(
    tls: &TlsClient,
    url: &HttpsUrl,
    accept: &str,
    user_agent: &str,
    timeout: Duration,
) -> Result<HttpResponse, NetError> {
    let conn = tls.connect(&url.host, url.port, timeout)?;
    request(conn, url, accept, user_agent)
}

/// Like [`get`], but connects to `addr` (the certificate is still checked against `url.host`).
pub fn get_at(
    tls: &TlsClient,
    addr: std::net::SocketAddr,
    url: &HttpsUrl,
    accept: &str,
    user_agent: &str,
    timeout: Duration,
) -> Result<HttpResponse, NetError> {
    let conn = tls.connect_addr(addr, &url.host, timeout)?;
    request(conn, url, accept, user_agent)
}

fn request(
    mut conn: crate::SipConnection,
    url: &HttpsUrl,
    accept: &str,
    user_agent: &str,
) -> Result<HttpResponse, NetError> {
    let host = if url.port == 443 {
        url.host.clone()
    } else {
        format!("{}:{}", url.host, url.port)
    };
    let safe = |s: &str| !s.chars().any(|c| c.is_control());
    if !safe(accept) || !safe(user_agent) || !safe(&url.path) {
        return Err(bad("unsafe request value"));
    }
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {host}\r\nAccept: {accept}\r\nUser-Agent: {user_agent}\r\nConnection: close\r\n\r\n",
        url.path
    );
    conn.tls
        .write_all(request.as_bytes())
        .map_err(NetError::Io)?;
    conn.tls.flush().map_err(NetError::Io)?;
    let mut data = Vec::new();
    let mut buf = [0u8; 16 * 1024];
    loop {
        match conn.tls.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                data.extend_from_slice(buf.get(..n).unwrap_or_default());
                if data.len() > MAX_RESPONSE {
                    return Err(bad("response too large"));
                }
            }
            // Servers that close without close_notify: rustls reports UnexpectedEof.
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof && !data.is_empty() => break,
            Err(e) => return Err(NetError::Io(e)),
        }
    }
    parse_response(&data)
}

/// Reads one complete response from a kept-alive connection: Content-Length or chunked framing;
/// without either, until the server closes the connection. Octets beyond the response are an
/// error (we never pipeline, so the server has no reason to send any). Issue #130: the caller's
/// `limit` holds on every layer, and the response is parsed only once its framing says it is
/// complete ([`Progress`]), so the work grows linearly with its size.
fn read_one(conn: &mut crate::SipConnection, limit: usize) -> Result<HttpResponse, NetError> {
    let mut data = Vec::new();
    let mut buf = [0u8; 16 * 1024];
    let mut progress = Progress::default();
    loop {
        if progress.ready(&data, limit)? {
            if let Some(r) = frame(&data, false, limit)? {
                return Ok(r);
            }
        }
        match conn.tls.read(&mut buf) {
            Ok(0) => return parse_response_within(&data, limit),
            Ok(n) => {
                data.extend_from_slice(buf.get(..n).unwrap_or_default());
                if data.len() > limit {
                    return Err(bad("response too large"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof && !data.is_empty() => {
                return parse_response_within(&data, limit)
            }
            Err(e) => return Err(NetError::Io(e)),
        }
    }
}

/// How far a response being read is known to be: where its head ends and how its body is
/// framed. Each octet is looked at a bounded number of times.
#[derive(Debug, Default)]
struct Progress {
    /// Octets already searched for the end of the head.
    searched: usize,
    head_end: Option<usize>,
    framing: Framing,
    /// Chunked: the body offset of the first chunk not yet complete.
    chunk_pos: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Framing {
    #[default]
    Unknown,
    /// The whole response has this many octets.
    Length(usize),
    Chunked,
    /// No framing: the body ends when the server closes the connection.
    UntilClose,
    /// Let [`frame`] decide (it has the complete view, including errors).
    Check,
}

impl Progress {
    /// Whether [`frame`] should look at `data` now: the response is complete by its framing, or
    /// [`frame`] has to report an error.
    fn ready(&mut self, data: &[u8], limit: usize) -> Result<bool, NetError> {
        if data.is_empty() {
            return Ok(false);
        }
        let head_end = match self.head_end {
            Some(h) => h,
            None => {
                let from = self.searched.saturating_sub(3);
                let found = data
                    .get(from..)
                    .and_then(|d| d.windows(4).position(|w| w == b"\r\n\r\n"))
                    .map(|p| from + p);
                self.searched = data.len();
                let Some(h) = found else {
                    if data.len() > MAX_HEAD {
                        return Err(bad("head too large"));
                    }
                    return Ok(false);
                };
                self.head_end = Some(h);
                self.framing = framing_of(data.get(..h).unwrap_or_default(), h, limit);
                h
            }
        };
        let body = data.get(head_end + 4..).unwrap_or_default();
        Ok(match self.framing {
            Framing::Length(total) => data.len() >= total,
            Framing::Chunked => chunks_complete(body, &mut self.chunk_pos),
            Framing::UntilClose => false,
            Framing::Unknown | Framing::Check => true,
        })
    }
}

/// The framing a head announces; anything unusual is left to [`frame`].
fn framing_of(head: &[u8], head_end: usize, limit: usize) -> Framing {
    let Ok(head) = std::str::from_utf8(head) else {
        return Framing::Check;
    };
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|l| l.split(' ').nth(1))
        .and_then(|c| c.parse::<u16>().ok());
    let mut te = Vec::new();
    let mut cl = Vec::new();
    for line in lines {
        let Some((n, v)) = line.split_once(':') else {
            return Framing::Check;
        };
        if n.eq_ignore_ascii_case("transfer-encoding") {
            te.push(v.trim());
        } else if n.eq_ignore_ascii_case("content-length") {
            cl.push(v.trim());
        }
    }
    match (te.as_slice(), cl.as_slice()) {
        ([t], []) if t.eq_ignore_ascii_case("chunked") => Framing::Chunked,
        ([], [l]) => match content_length(l, limit) {
            Ok(n) => Framing::Length(head_end + 4 + n),
            Err(_) => Framing::Check,
        },
        ([], []) if matches!(status, Some(100..=199 | 204 | 304)) => Framing::Check,
        ([], []) => Framing::UntilClose,
        _ => Framing::Check,
    }
}

/// Walks the chunk size lines of a chunked body from `*pos` without copying: whether the last
/// chunk and the trailer section are complete. `*pos` moves past every complete chunk, so each
/// octet is walked once; a malformed size line returns `true` so [`frame`] reports it.
fn chunks_complete(body: &[u8], pos: &mut usize) -> bool {
    loop {
        let rest = body.get(*pos..).unwrap_or_default();
        let Some(line_end) = rest.windows(2).position(|w| w == b"\r\n") else {
            return rest.len() > MAX_LINE;
        };
        let size = std::str::from_utf8(rest.get(..line_end).unwrap_or_default())
            .ok()
            .map(|l| l.split(';').next().unwrap_or("").trim())
            .filter(|t| !t.is_empty() && t.len() <= 8)
            .and_then(|t| usize::from_str_radix(t, 16).ok());
        let Some(size) = size else {
            return true;
        };
        let after_line = line_end + 2;
        if size == 0 {
            // The trailer section ends with an empty line.
            let trailer = rest.get(after_line..).unwrap_or_default();
            return trailer.starts_with(b"\r\n")
                || trailer.windows(4).any(|w| w == b"\r\n\r\n")
                || trailer.len() > MAX_HEAD;
        }
        let next = after_line.saturating_add(size).saturating_add(2);
        if rest.len() < next {
            return false;
        }
        *pos += next;
    }
}

/// POST with HTTP authentication on one kept-alive connection (RFC 4559 / MS-NTHT: NTLM is bound
/// to the connection). `authorize` gets the server's challenge header value (`None` before the
/// first request) and the server certificate of this connection (DER, for NTLM channel bindings,
/// issue #42) and returns the next `Authorization` value. At most three rounds.
#[allow(clippy::too_many_arguments)]
pub fn post_with_auth(
    tls: &TlsClient,
    url: &HttpsUrl,
    extra_headers: &[(&str, &str)],
    body: &[u8],
    user_agent: &str,
    timeout: Duration,
    authorize: &mut Authorize<'_>,
) -> Result<HttpResponse, NetError> {
    let conn = tls.connect(&url.host, url.port, timeout)?;
    with_auth_on(
        conn,
        "POST",
        url,
        extra_headers,
        Some(body),
        user_agent,
        MAX_RESPONSE,
        authorize,
    )
}

/// POST without HTTP authentication (the request authenticates itself, e.g. with a web ticket in
/// its SOAP header, [MS-OCAUTHWS] §3.2); a 401 is returned like any answer.
pub fn post(
    tls: &TlsClient,
    url: &HttpsUrl,
    extra_headers: &[(&str, &str)],
    body: &[u8],
    user_agent: &str,
    timeout: Duration,
) -> Result<HttpResponse, NetError> {
    let mut conn = tls.connect(&url.host, url.port, timeout)?;
    let host = if url.port == 443 {
        url.host.clone()
    } else {
        format!("{}:{}", url.host, url.port)
    };
    let safe = |s: &str| !s.chars().any(|c| c.is_control());
    if !safe(&url.path)
        || !safe(user_agent)
        || extra_headers.iter().any(|(k, v)| !safe(k) || !safe(v))
    {
        return Err(bad("unsafe request value"));
    }
    let mut req = format!(
        "POST {} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: {user_agent}\r\nConnection: close\r\n",
        url.path
    );
    for (k, v) in extra_headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    conn.tls.write_all(req.as_bytes()).map_err(NetError::Io)?;
    conn.tls.write_all(body).map_err(NetError::Io)?;
    conn.tls.flush().map_err(NetError::Io)?;
    read_one(&mut conn, MAX_RESPONSE)
}

/// GET with HTTP authentication on one kept-alive connection, like [`post_with_auth`], for a
/// response of up to `limit` bytes (e.g. an address book file).
#[allow(clippy::too_many_arguments)]
pub fn get_with_auth(
    tls: &TlsClient,
    url: &HttpsUrl,
    extra_headers: &[(&str, &str)],
    user_agent: &str,
    timeout: Duration,
    limit: usize,
    authorize: &mut Authorize<'_>,
) -> Result<HttpResponse, NetError> {
    let conn = tls.connect(&url.host, url.port, timeout)?;
    with_auth_on(
        conn,
        "GET",
        url,
        extra_headers,
        None,
        user_agent,
        limit,
        authorize,
    )
}

/// Like [`post_with_auth`], connecting to `addr` (the certificate is checked against `url.host`).
#[allow(clippy::too_many_arguments)]
pub fn post_with_auth_at(
    tls: &TlsClient,
    addr: std::net::SocketAddr,
    url: &HttpsUrl,
    extra_headers: &[(&str, &str)],
    body: &[u8],
    user_agent: &str,
    timeout: Duration,
    authorize: &mut Authorize<'_>,
) -> Result<HttpResponse, NetError> {
    let conn = tls.connect_addr(addr, &url.host, timeout)?;
    with_auth_on(
        conn,
        "POST",
        url,
        extra_headers,
        Some(body),
        user_agent,
        MAX_RESPONSE,
        authorize,
    )
}

#[allow(clippy::too_many_arguments)]
fn with_auth_on(
    mut conn: crate::SipConnection,
    method: &str,
    url: &HttpsUrl,
    extra_headers: &[(&str, &str)],
    body: Option<&[u8]>,
    user_agent: &str,
    limit: usize,
    authorize: &mut Authorize<'_>,
) -> Result<HttpResponse, NetError> {
    let host = if url.port == 443 {
        url.host.clone()
    } else {
        format!("{}:{}", url.host, url.port)
    };
    let safe = |s: &str| !s.chars().any(|c| c.is_control());
    if !safe(&url.path)
        || !safe(user_agent)
        || extra_headers.iter().any(|(k, v)| !safe(k) || !safe(v))
    {
        return Err(bad("unsafe request value"));
    }
    let mut challenge: Option<String> = None;
    for _ in 0..3 {
        let cert = conn.server_certificate();
        let auth = authorize(challenge.as_deref(), cert.as_deref())?;
        if !safe(&auth) {
            return Err(bad("unsafe authorization value"));
        }
        let mut req = format!("{method} {} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: {user_agent}\r\nConnection: keep-alive\r\nAuthorization: {auth}\r\n", url.path);
        for (k, v) in extra_headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        match body {
            Some(b) => req.push_str(&format!("Content-Length: {}\r\n\r\n", b.len())),
            None => req.push_str("\r\n"),
        }
        conn.tls.write_all(req.as_bytes()).map_err(NetError::Io)?;
        if let Some(b) = body {
            conn.tls.write_all(b).map_err(NetError::Io)?;
        }
        conn.tls.flush().map_err(NetError::Io)?;
        let r = read_one(&mut conn, limit)?;
        if r.status != 401 {
            return Ok(r);
        }
        // The challenge: a WWW-Authenticate value with a token (NTLM or Negotiate).
        challenge = r
            .headers
            .iter()
            .filter(|(n, _)| n == "www-authenticate")
            .map(|(_, v)| v.clone())
            .find(|v| v.split_whitespace().nth(1).is_some());
        if challenge.is_none() {
            return Ok(r);
        }
    }
    Err(bad("authentication did not complete"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lengths_and_chunks() {
        let r = parse_response(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/x\r\nContent-Length: 5\r\n\r\nhello",
        )
        .unwrap();
        assert_eq!((r.status, r.body.as_slice()), (200, b"hello".as_slice()));
        assert_eq!(r.header("content-type").unwrap(), Some("application/x"));
        let r = parse_response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6;ext\r\n world\r\n0\r\n\r\n").unwrap();
        assert_eq!(r.body, b"hello world");
        let r = parse_response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nhi\r\n0\r\nX-Trailer: 1\r\n\r\n").unwrap();
        assert_eq!(r.body, b"hi", "trailer section read and dropped");
        let r = parse_response(b"HTTP/1.1 204 No Content\r\n\r\n").unwrap();
        assert!(r.body.is_empty());
        let r = parse_response(b"HTTP/1.1 302 Found\r\nLocation: https://a.contoso.com/x\r\n\r\n")
            .unwrap();
        assert_eq!(
            r.header("location").unwrap(),
            Some("https://a.contoso.com/x")
        );
    }

    #[test]
    fn refuses_ambiguous_or_broken_responses() {
        for bad in [
            &b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n"
                [..],
            b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\nab",
            b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nshort",
            b"HTTP/1.1 200 OK\r\nContent-Length: -1\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhel",
            b"HTTP/2 200 OK\r\n\r\n",
            b"HTTP/1.1 2000 OK\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nX: a\r\n b\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nBad Name: x\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nX: \x01\r\n\r\n",
            b"HTTP/1.1 200 OK\r\n",
            // Issue #42: surplus octets and an unterminated last chunk.
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhelloEXTRA",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nHTTP/1.1 200 OK\r\n",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nno colon\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n+5\r\nhello\r\n0\r\n\r\n",
            b"HTTP/1.1 204 No Content\r\n\r\nbody",
        ] {
            assert!(
                parse_response(bad).is_err(),
                "{:?}",
                String::from_utf8_lossy(bad)
            );
        }
        assert!(parse_response(&vec![b'a'; MAX_RESPONSE + 1]).is_err());
    }

    /// Issue #42: on a kept-alive connection a response is complete only with its last octet;
    /// a chunked response is not complete after `0\r\n` (the final CRLF used to stay behind).
    #[test]
    fn keep_alive_framing() {
        let full: &[u8] =
            b"HTTP/1.1 401 Unauthorized\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n0\r\n\r\n";
        for cut in 0..full.len() {
            assert!(
                frame(&full[..cut], false, MAX_RESPONSE).unwrap().is_none(),
                "complete too early at {cut}"
            );
        }
        assert_eq!(
            frame(full, false, MAX_RESPONSE).unwrap().unwrap().body,
            b"abc"
        );
        let cl: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc";
        assert!(frame(&cl[..cl.len() - 1], false, MAX_RESPONSE)
            .unwrap()
            .is_none());
        assert!(frame(cl, false, MAX_RESPONSE).unwrap().is_some());
        // Without framing on a kept-alive connection: wait for the close.
        assert!(frame(b"HTTP/1.1 200 OK\r\n\r\nabc", false, MAX_RESPONSE)
            .unwrap()
            .is_none());
    }

    /// Issue #130: the caller's limit holds on every layer – a larger limit admits a response
    /// over 256 KiB (Content-Length of more than seven digits, chunks), a body over the limit
    /// is refused.
    #[test]
    fn caller_limit_on_every_layer() {
        let body = vec![b'x'; 10 * 1024 * 1024 + 1];
        let mut cl =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes();
        cl.extend_from_slice(&body);
        let big = 64 * 1024 * 1024;
        assert_eq!(
            frame(&cl, false, big).unwrap().unwrap().body.len(),
            body.len()
        );
        assert!(frame(&cl, false, MAX_RESPONSE).is_err());
        assert!(frame(
            b"HTTP/1.1 200 OK\r\nContent-Length: 99999999999\r\n\r\n",
            false,
            big
        )
        .is_err());
        assert!(frame(
            b"HTTP/1.1 200 OK\r\nContent-Length: 99999999999999999999\r\n\r\n",
            false,
            big
        )
        .is_err());
        let mut chunked = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
        for _ in 0..80 {
            chunked.extend_from_slice(b"1000\r\n");
            chunked.extend_from_slice(&[b'y'; 4096]);
            chunked.extend_from_slice(b"\r\n");
        }
        chunked.extend_from_slice(b"0\r\n\r\n");
        assert_eq!(
            frame(&chunked, false, big).unwrap().unwrap().body.len(),
            80 * 4096
        );
        assert!(frame(&chunked, false, MAX_RESPONSE).is_err());
    }

    /// Issue #130: the response is parsed only when its framing says it is complete, so reading
    /// a large response in 16 KiB pieces does linear work.
    #[test]
    fn progress_waits_for_the_framing() {
        let big = 64 * 1024 * 1024;
        let mut chunked = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
        for _ in 0..2048 {
            chunked.extend_from_slice(b"1000\r\n");
            chunked.extend_from_slice(&[b'z'; 4096]);
            chunked.extend_from_slice(b"\r\n");
        }
        chunked.extend_from_slice(b"0\r\nX-T: 1\r\n\r\n");
        let body = vec![b'w'; 8 * 1024 * 1024];
        let mut cl =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes();
        cl.extend_from_slice(&body);
        let start = std::time::Instant::now();
        for full in [&chunked, &cl] {
            let mut p = Progress::default();
            let mut ready_at = None;
            for end in (16 * 1024..full.len() + 16 * 1024).step_by(16 * 1024) {
                let end = end.min(full.len());
                if p.ready(&full[..end], big).unwrap() {
                    ready_at = Some(end);
                    break;
                }
            }
            assert_eq!(ready_at, Some(full.len()));
            assert!(frame(full, false, big).unwrap().is_some());
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(20));
        // Without framing: only the end of the connection completes the body.
        let mut p = Progress::default();
        assert!(!p.ready(b"HTTP/1.1 200 OK\r\n\r\nabc", big).unwrap());
        // A malformed chunk size is handed to frame, which refuses it.
        let mut p = Progress::default();
        let bad_chunk = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n";
        assert!(p.ready(bad_chunk, big).unwrap());
        assert!(frame(bad_chunk, false, big).is_err());
        // A head that never ends is refused.
        let mut p = Progress::default();
        assert!(p.ready(&vec![b'a'; MAX_HEAD + 1], big).is_err());
    }
}
