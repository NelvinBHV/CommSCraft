//! A minimal HTTPS GET client for Lync Autodiscover (MS-OCDISCWS §2.1: HTTP over TLS).
//!
//! Security by design: only `https://` (checked by `commscraft-url`), certificate verification by
//! the shared [`TlsClient`](crate::TlsClient), `Connection: close`, a bounded response (size,
//! header count, line length), Content-Length and chunked encoding parsed strictly, both at once
//! refused (request smuggling), no cookies, no credentials. Redirects are not followed here; the
//! discovery state machine decides about them.

use std::io::{Read, Write};
use std::time::Duration;

use commscraft_url::HttpsUrl;

use crate::{NetError, TlsClient};

/// Upper bound of a whole response (head and body).
pub const MAX_RESPONSE: usize = 256 * 1024;
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
    if data.len() > MAX_RESPONSE {
        return Err(bad("response too large"));
    }
    let head_end = data
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| bad("incomplete head"))?;
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
    let body = match (te, cl) {
        (Some(_), Some(_)) => return Err(bad("both Transfer-Encoding and Content-Length")),
        (Some(t), None) if t.eq_ignore_ascii_case("chunked") => dechunk(raw_body)?,
        (Some(_), None) => return Err(bad("unsupported transfer coding")),
        (None, Some(l)) => {
            if l.is_empty() || l.len() > 7 || !l.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad("Content-Length"));
            }
            let n: usize = l.parse().map_err(|_| bad("Content-Length"))?;
            raw_body
                .get(..n)
                .ok_or_else(|| bad("body shorter than Content-Length"))?
                .to_vec()
        }
        (None, None) => raw_body.to_vec(),
    };
    Ok(HttpResponse { body, ..resp })
}

fn dechunk(mut data: &[u8]) -> Result<Vec<u8>, NetError> {
    let mut out = Vec::new();
    loop {
        let line_end = data
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or_else(|| bad("chunk size line"))?;
        let line = std::str::from_utf8(data.get(..line_end).unwrap_or_default())
            .map_err(|_| bad("chunk size"))?;
        let size_text = line.split(';').next().unwrap_or("").trim();
        if size_text.is_empty() || size_text.len() > 6 {
            return Err(bad("chunk size"));
        }
        let size = usize::from_str_radix(size_text, 16).map_err(|_| bad("chunk size"))?;
        data = data.get(line_end + 2..).ok_or_else(|| bad("chunk"))?;
        if size == 0 {
            return Ok(out);
        }
        out.extend_from_slice(
            data.get(..size)
                .ok_or_else(|| bad("chunk shorter than announced"))?,
        );
        if out.len() > MAX_RESPONSE {
            return Err(bad("body too large"));
        }
        if data.get(size..size + 2) != Some(b"\r\n".as_slice()) {
            return Err(bad("chunk terminator"));
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
/// without either, until the server closes the connection.
fn read_one(conn: &mut crate::SipConnection) -> Result<HttpResponse, NetError> {
    let mut data = Vec::new();
    let mut buf = [0u8; 16 * 1024];
    loop {
        if let Some(head_end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(data.get(..head_end).unwrap_or_default())
                .to_ascii_lowercase();
            let framed =
                head.contains("\r\ncontent-length:") || head.contains("\r\ntransfer-encoding:");
            if framed {
                if let Ok(r) = parse_response(&data) {
                    let complete = match r.header("content-length")? {
                        Some(l) => l
                            .parse::<usize>()
                            .is_ok_and(|n| data.len() >= head_end + 4 + n),
                        None => true, // chunked: parse_response only succeeds when complete
                    };
                    if complete {
                        return Ok(r);
                    }
                }
            }
        }
        match conn.tls.read(&mut buf) {
            Ok(0) => return parse_response(&data),
            Ok(n) => {
                data.extend_from_slice(buf.get(..n).unwrap_or_default());
                if data.len() > MAX_RESPONSE {
                    return Err(bad("response too large"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof && !data.is_empty() => {
                return parse_response(&data)
            }
            Err(e) => return Err(NetError::Io(e)),
        }
    }
}

/// POST with HTTP authentication on one kept-alive connection (RFC 4559 / MS-NTHT: NTLM is bound
/// to the connection). `authorize` gets the server's challenge header value (`None` before the
/// first request) and returns the next `Authorization` value. At most three rounds.
#[allow(clippy::too_many_arguments)]
pub fn post_with_auth(
    tls: &TlsClient,
    url: &HttpsUrl,
    extra_headers: &[(&str, &str)],
    body: &[u8],
    user_agent: &str,
    timeout: Duration,
    authorize: &mut dyn FnMut(Option<&str>) -> Result<String, NetError>,
) -> Result<HttpResponse, NetError> {
    let conn = tls.connect(&url.host, url.port, timeout)?;
    post_with_auth_on(conn, url, extra_headers, body, user_agent, authorize)
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
    authorize: &mut dyn FnMut(Option<&str>) -> Result<String, NetError>,
) -> Result<HttpResponse, NetError> {
    let conn = tls.connect_addr(addr, &url.host, timeout)?;
    post_with_auth_on(conn, url, extra_headers, body, user_agent, authorize)
}

fn post_with_auth_on(
    mut conn: crate::SipConnection,
    url: &HttpsUrl,
    extra_headers: &[(&str, &str)],
    body: &[u8],
    user_agent: &str,
    authorize: &mut dyn FnMut(Option<&str>) -> Result<String, NetError>,
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
        let auth = authorize(challenge.as_deref())?;
        if !safe(&auth) {
            return Err(bad("unsafe authorization value"));
        }
        let mut req = format!("POST {} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: {user_agent}\r\nConnection: keep-alive\r\nAuthorization: {auth}\r\n", url.path);
        for (k, v) in extra_headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
        conn.tls.write_all(req.as_bytes()).map_err(NetError::Io)?;
        conn.tls.write_all(body).map_err(NetError::Io)?;
        conn.tls.flush().map_err(NetError::Io)?;
        let r = read_one(&mut conn)?;
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
        let r = parse_response(b"HTTP/1.1 200 OK\r\nContent-Type: application/x\r\nContent-Length: 5\r\n\r\nhelloEXTRA").unwrap();
        assert_eq!((r.status, r.body.as_slice()), (200, b"hello".as_slice()));
        assert_eq!(r.header("content-type").unwrap(), Some("application/x"));
        let r = parse_response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6;ext\r\n world\r\n0\r\n\r\n").unwrap();
        assert_eq!(r.body, b"hello world");
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
        ] {
            assert!(
                parse_response(bad).is_err(),
                "{:?}",
                String::from_utf8_lossy(bad)
            );
        }
        assert!(parse_response(&vec![b'a'; MAX_RESPONSE + 1]).is_err());
    }
}
