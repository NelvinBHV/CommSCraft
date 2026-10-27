//! SIP messages: parsing, stream framing and serialization (RFC 3261 §7, §18.3, §20, §25).
//!
//! Every input is treated as hostile (security by design, see `docs/security.md`):
//! - all sizes are bounded by [`Limits`]; nothing grows without a check;
//! - parsing fails closed: a malformed or ambiguous message is an error, never a guess;
//! - a [`StreamDecoder`] that saw an error stays poisoned, so the connection has to be closed;
//! - identity headers (Call-ID, CSeq, From, To, Content-Length, Expires) must occur at most once;
//! - the serializer refuses CR, LF and other control characters in every field, so no header
//!   injection is possible through values that come from the network or the user.
//!
//! Header values are kept exactly as received (after unfolding of continuation lines), because the
//! MS-SIPAE signature buffer uses them "with the same syntax and case" (AE-29, AE-32).

/// Size limits. The defaults are a design choice (inference): they are far above the SIP messages
/// shown in the specifications and keep a hostile peer from making the client buffer without end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Start line, headers and body together.
    pub max_message_len: usize,
    /// Number of header fields (after unfolding).
    pub max_header_count: usize,
    /// One header field or the start line, after unfolding.
    pub max_line_len: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_message_len: 1024 * 1024,
            max_header_count: 256,
            max_line_len: 16 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The message is larger than [`Limits::max_message_len`].
    TooLarge,
    /// More header fields than [`Limits::max_header_count`].
    TooManyHeaders,
    /// A line longer than [`Limits::max_line_len`].
    LineTooLong,
    /// Not UTF-8, a bare CR or LF, NUL or another control character.
    BadCharacter,
    BadStartLine,
    BadVersion,
    BadStatusCode,
    /// A header line without a valid name and colon, or a continuation line without a header.
    BadHeader,
    /// Content-Length missing (mandatory on stream transports, RFC 3261 §18.3).
    MissingContentLength,
    /// Content-Length not a decimal number, or several Content-Length fields.
    BadContentLength,
    /// The input ends inside the message (only from [`parse`]).
    Truncated,
    /// Bytes after the body (only from [`parse`]).
    TrailingData,
    /// A header that must occur at most once occurs several times.
    DuplicateHeader(&'static str),
    BadCSeq,
    BadNameAddr,
    /// A value that cannot be serialized safely.
    InvalidValue,
    /// The decoder saw an error before; the connection must be closed.
    Poisoned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartLine {
    Request { method: String, uri: String },
    Response { code: u16, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub start: StartLine,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

/// CSeq value (RFC 3261 §20.16): the number text is kept as received for the signature buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CSeq<'a> {
    pub number_text: &'a str,
    pub number: u32,
    pub method: &'a str,
}

/// `name-addr` or `addr-spec` with header parameters (RFC 3261 §25.1: From, To, Contact,
/// P-Asserted-Identity, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameAddr {
    pub display: Option<String>,
    pub uri: String,
    pub params: Vec<(String, Option<String>)>,
}

impl NameAddr {
    /// Value of a header parameter; names compare case-insensitively (RFC 3261 §7.3.1).
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .and_then(|(_, v)| v.as_deref())
    }

    /// The URI scheme before the first colon, e.g. `sip` or `tel`.
    pub fn scheme(&self) -> Option<&str> {
        self.uri.split_once(':').map(|(s, _)| s)
    }
}

/// Compact header forms (RFC 3261 §7.3.3 and §20).
const COMPACT: &[(&str, &str)] = &[
    ("i", "Call-ID"),
    ("m", "Contact"),
    ("e", "Content-Encoding"),
    ("l", "Content-Length"),
    ("c", "Content-Type"),
    ("f", "From"),
    ("s", "Subject"),
    ("k", "Supported"),
    ("t", "To"),
    ("v", "Via"),
];

/// True if `name` (as received) denotes the header `canonical` (full name, any case, or its
/// compact form).
pub fn header_name_matches(name: &str, canonical: &str) -> bool {
    if name.eq_ignore_ascii_case(canonical) {
        return true;
    }
    COMPACT.iter().any(|(short, long)| {
        name.eq_ignore_ascii_case(short) && long.eq_ignore_ascii_case(canonical)
    })
}

/// RFC 3261 §25.1 `token` characters.
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-.!%*_+`'~".contains(c)
}

fn is_token(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_token_char)
}

/// Control characters other than HTAB are never allowed in a header or start line.
fn has_forbidden_control(s: &str) -> bool {
    s.chars()
        .any(|c| (c.is_control() && c != '\t') || c == '\u{7f}')
}

fn find_head_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_start_line(line: &str) -> Result<StartLine, Error> {
    // RFC 3261 §7.1: the version string is case-insensitive.
    if line
        .get(..8)
        .is_some_and(|v| v.eq_ignore_ascii_case("SIP/2.0 "))
    {
        let after = line.get(8..).ok_or(Error::BadStartLine)?;
        let (code, reason) = match after.split_once(' ') {
            Some((c, r)) => (c, r),
            None => (after, ""),
        };
        if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Error::BadStatusCode);
        }
        let code: u16 = code.parse().map_err(|_| Error::BadStatusCode)?;
        if !(100..=699).contains(&code) {
            return Err(Error::BadStatusCode);
        }
        return Ok(StartLine::Response {
            code,
            reason: reason.to_string(),
        });
    }
    let mut parts = line.split(' ');
    let (Some(method), Some(uri), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Error::BadStartLine);
    };
    if !is_token(method) || uri.is_empty() || !uri.contains(':') {
        return Err(Error::BadStartLine);
    }
    if !version.eq_ignore_ascii_case("SIP/2.0") {
        return Err(Error::BadVersion);
    }
    Ok(StartLine::Request {
        method: method.to_string(),
        uri: uri.to_string(),
    })
}

/// Parses the head (start line and headers, without the final empty line).
fn parse_head(head: &[u8], limits: &Limits) -> Result<(StartLine, Vec<Header>), Error> {
    let text = std::str::from_utf8(head).map_err(|_| Error::BadCharacter)?;
    let mut lines = text.split("\r\n");
    let first = lines.next().ok_or(Error::BadStartLine)?;
    if first.len() > limits.max_line_len {
        return Err(Error::LineTooLong);
    }
    if first.contains(['\r', '\n']) || has_forbidden_control(first) {
        return Err(Error::BadCharacter);
    }
    let start = parse_start_line(first)?;
    let mut headers: Vec<Header> = Vec::new();
    for line in lines {
        if line.contains(['\r', '\n']) || has_forbidden_control(line) {
            return Err(Error::BadCharacter);
        }
        if line.starts_with([' ', '\t']) {
            // Continuation line (RFC 3261 §7.3.1): folded into the previous value with one SP.
            let last = headers.last_mut().ok_or(Error::BadHeader)?;
            let more = line.trim_matches([' ', '\t']);
            if last.value.len() + 1 + more.len() > limits.max_line_len {
                return Err(Error::LineTooLong);
            }
            if !more.is_empty() {
                if !last.value.is_empty() {
                    last.value.push(' ');
                }
                last.value.push_str(more);
            }
            continue;
        }
        if line.len() > limits.max_line_len {
            return Err(Error::LineTooLong);
        }
        let (name, value) = line.split_once(':').ok_or(Error::BadHeader)?;
        let name = name.trim_end_matches([' ', '\t']);
        if !is_token(name) {
            return Err(Error::BadHeader);
        }
        if headers.len() >= limits.max_header_count {
            return Err(Error::TooManyHeaders);
        }
        headers.push(Header {
            name: name.to_string(),
            value: value.trim_matches([' ', '\t']).to_string(),
        });
    }
    Ok((start, headers))
}

fn content_length(headers: &[Header]) -> Result<Option<usize>, Error> {
    let mut found: Option<usize> = None;
    for h in headers
        .iter()
        .filter(|h| header_name_matches(&h.name, "Content-Length"))
    {
        if found.is_some() {
            // Several Content-Length fields are ambiguous (message smuggling): fail closed.
            return Err(Error::BadContentLength);
        }
        let v = h.value.as_str();
        if v.is_empty() || v.len() > 10 || !v.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Error::BadContentLength);
        }
        found = Some(v.parse().map_err(|_| Error::BadContentLength)?);
    }
    Ok(found)
}

/// Parses exactly one complete message as received on a stream transport (Content-Length
/// required, no trailing bytes).
pub fn parse(data: &[u8], limits: &Limits) -> Result<Message, Error> {
    if data.len() > limits.max_message_len {
        return Err(Error::TooLarge);
    }
    let head_end = find_head_end(data).ok_or(Error::Truncated)?;
    let head = data.get(..head_end).ok_or(Error::Truncated)?;
    let (start, headers) = parse_head(head, limits)?;
    let len = content_length(&headers)?.ok_or(Error::MissingContentLength)?;
    let body_start = head_end + 4;
    let body_end = body_start.checked_add(len).ok_or(Error::TooLarge)?;
    if body_end > limits.max_message_len {
        return Err(Error::TooLarge);
    }
    let body = data.get(body_start..body_end).ok_or(Error::Truncated)?;
    if data.len() > body_end {
        return Err(Error::TrailingData);
    }
    Ok(Message {
        start,
        headers,
        body: body.to_vec(),
    })
}

/// Sans-I/O framing of a SIP byte stream (TLS/TCP, RFC 3261 §18.3). CRLFs before a start line are
/// ignored (RFC 3261 §7.5; they also carry the RFC 5626 keep-alive). After any error the decoder is
/// poisoned and the caller must close the connection.
#[derive(Debug, Default)]
pub struct StreamDecoder {
    buf: Vec<u8>,
    limits: Limits,
    poisoned: bool,
}

impl StreamDecoder {
    pub fn new(limits: Limits) -> Self {
        Self {
            buf: Vec::new(),
            limits,
            poisoned: false,
        }
    }

    /// Appends received bytes. Fails if more than one maximal message would be buffered.
    pub fn push(&mut self, data: &[u8]) -> Result<(), Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        // Leading CRLFs are dropped in `next`, so allow their length on top of one message.
        if self.buf.len().saturating_add(data.len()) > self.limits.max_message_len.saturating_mul(2)
        {
            return self.fail(Error::TooLarge);
        }
        self.buf.extend_from_slice(data);
        Ok(())
    }

    /// Returns the next complete message, `None` if more bytes are needed.
    pub fn next_message(&mut self) -> Result<Option<Message>, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        let skip = self.buf.chunks(2).take_while(|c| *c == b"\r\n").count() * 2;
        self.buf.drain(..skip);
        let Some(head_end) = find_head_end(&self.buf) else {
            if self.buf.len() > self.limits.max_message_len {
                return self.fail(Error::TooLarge);
            }
            return Ok(None);
        };
        if head_end + 4 > self.limits.max_message_len {
            return self.fail(Error::TooLarge);
        }
        let parsed = self
            .buf
            .get(..head_end)
            .ok_or(Error::Truncated)
            .and_then(|head| parse_head(head, &self.limits))
            .and_then(|(start, headers)| {
                let len = content_length(&headers)?.ok_or(Error::MissingContentLength)?;
                Ok((start, headers, len))
            });
        let (start, headers, len) = match parsed {
            Ok(v) => v,
            Err(e) => return self.fail(e),
        };
        let total = match (head_end + 4).checked_add(len) {
            Some(t) if t <= self.limits.max_message_len => t,
            _ => return self.fail(Error::TooLarge),
        };
        if self.buf.len() < total {
            return Ok(None);
        }
        let body = self
            .buf
            .get(head_end + 4..total)
            .unwrap_or_default()
            .to_vec();
        self.buf.drain(..total);
        Ok(Some(Message {
            start,
            headers,
            body,
        }))
    }

    fn fail<T>(&mut self, e: Error) -> Result<T, Error> {
        self.poisoned = true;
        self.buf.clear();
        Err(e)
    }
}

/// Splits a header value at commas that are outside quoted strings and angle brackets
/// (RFC 3261 §7.3.1).
pub fn split_list(value: &str) -> Result<Vec<&str>, Error> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut in_quote = false;
    let mut escaped = false;
    let mut in_angle = false;
    for (i, c) in value.char_indices() {
        if in_quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_quote = false;
            }
            continue;
        }
        match c {
            '"' => in_quote = true,
            '<' if !in_angle => in_angle = true,
            '>' if in_angle => in_angle = false,
            ',' if !in_angle => {
                out.push(value.get(start..i).ok_or(Error::BadNameAddr)?.trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if in_quote || in_angle {
        return Err(Error::BadNameAddr);
    }
    out.push(value.get(start..).ok_or(Error::BadNameAddr)?.trim());
    Ok(out)
}

/// Reads a quoted string starting at the opening quote; returns the unescaped content and the
/// rest after the closing quote.
pub fn read_quoted(s: &str) -> Option<(String, &str)> {
    let mut chars = s.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for (i, c) in chars {
        if escaped {
            out.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some((out, s.get(i + 1..)?));
        } else {
            out.push(c);
        }
    }
    None
}

fn parse_params(mut rest: &str) -> Result<Vec<(String, Option<String>)>, Error> {
    let mut params = Vec::new();
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            return Ok(params);
        }
        rest = rest
            .strip_prefix(';')
            .ok_or(Error::BadNameAddr)?
            .trim_start();
        let end = rest.find(|c: char| !is_token_char(c)).unwrap_or(rest.len());
        let name = rest.get(..end).ok_or(Error::BadNameAddr)?;
        if name.is_empty() {
            return Err(Error::BadNameAddr);
        }
        rest = rest.get(end..).ok_or(Error::BadNameAddr)?.trim_start();
        let value = if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            if after.starts_with('"') {
                let (v, r) = read_quoted(after).ok_or(Error::BadNameAddr)?;
                rest = r;
                // Keep the quotes: the value is used "with the same syntax" where it matters.
                Some(format!(
                    "\"{}\"",
                    v.replace('\\', "\\\\").replace('"', "\\\"")
                ))
            } else {
                let end = after
                    .find(|c: char| !(is_token_char(c) || "[]:".contains(c)))
                    .unwrap_or(after.len());
                let v = after.get(..end).ok_or(Error::BadNameAddr)?;
                if v.is_empty() {
                    return Err(Error::BadNameAddr);
                }
                rest = after.get(end..).ok_or(Error::BadNameAddr)?;
                Some(v.to_string())
            }
        } else {
            None
        };
        params.push((name.to_string(), value));
    }
}

/// Parses one `name-addr` or `addr-spec` with parameters (RFC 3261 §25.1).
pub fn parse_name_addr(value: &str) -> Result<NameAddr, Error> {
    let v = value.trim();
    let (display, after_display) = if v.starts_with('"') {
        let (d, rest) = read_quoted(v).ok_or(Error::BadNameAddr)?;
        let rest = rest.trim_start();
        if !rest.starts_with('<') {
            return Err(Error::BadNameAddr);
        }
        (Some(d), rest)
    } else if let Some(lt) = v
        .find('<')
        .filter(|lt| !v.get(..*lt).unwrap_or("").contains([';', ',']))
    {
        let d = v.get(..lt).ok_or(Error::BadNameAddr)?.trim();
        if !d.split_whitespace().all(is_token) {
            return Err(Error::BadNameAddr);
        }
        let display = (!d.is_empty()).then(|| d.to_string());
        (display, v.get(lt..).ok_or(Error::BadNameAddr)?)
    } else {
        (None, v)
    };
    let (uri, rest) = if let Some(inner) = after_display.strip_prefix('<') {
        let gt = inner.find('>').ok_or(Error::BadNameAddr)?;
        (
            inner.get(..gt).ok_or(Error::BadNameAddr)?,
            inner.get(gt + 1..).ok_or(Error::BadNameAddr)?,
        )
    } else {
        // addr-spec: the first ';' starts the header parameters.
        let end = after_display.find(';').unwrap_or(after_display.len());
        (
            after_display.get(..end).ok_or(Error::BadNameAddr)?,
            after_display.get(end..).ok_or(Error::BadNameAddr)?,
        )
    };
    let uri = uri.trim();
    if uri.is_empty() || !uri.contains(':') || uri.contains(['<', '>', '"', ' ', ',']) {
        return Err(Error::BadNameAddr);
    }
    Ok(NameAddr {
        display,
        uri: uri.to_string(),
        params: parse_params(rest)?,
    })
}

impl Message {
    pub fn request(method: &str, uri: &str) -> Self {
        Self {
            start: StartLine::Request {
                method: method.to_string(),
                uri: uri.to_string(),
            },
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn response(code: u16, reason: &str) -> Self {
        Self {
            start: StartLine::Response {
                code,
                reason: reason.to_string(),
            },
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn push_header(&mut self, name: &str, value: &str) {
        self.headers.push(Header {
            name: name.to_string(),
            value: value.to_string(),
        });
    }

    pub fn is_response(&self) -> bool {
        matches!(self.start, StartLine::Response { .. })
    }

    pub fn status_code(&self) -> Option<u16> {
        match self.start {
            StartLine::Response { code, .. } => Some(code),
            StartLine::Request { .. } => None,
        }
    }

    pub fn method(&self) -> Option<&str> {
        match &self.start {
            StartLine::Request { method, .. } => Some(method),
            StartLine::Response { .. } => None,
        }
    }

    /// All values of a header in order (full or compact name, any case).
    pub fn header_values<'a>(&'a self, canonical: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.headers
            .iter()
            .filter(move |h| header_name_matches(&h.name, canonical))
            .map(|h| h.value.as_str())
    }

    /// The value of a header that may occur at most once.
    pub fn single(&self, canonical: &'static str) -> Result<Option<&str>, Error> {
        let mut values = self.header_values(canonical);
        let first = values.next();
        if values.next().is_some() {
            return Err(Error::DuplicateHeader(canonical));
        }
        Ok(first)
    }

    pub fn call_id(&self) -> Result<Option<&str>, Error> {
        self.single("Call-ID")
    }

    pub fn expires(&self) -> Result<Option<&str>, Error> {
        self.single("Expires")
    }

    /// CSeq (RFC 3261 §8.1.1.5: the number is below 2^31).
    pub fn cseq(&self) -> Result<Option<CSeq<'_>>, Error> {
        let Some(v) = self.single("CSeq")? else {
            return Ok(None);
        };
        let mut parts = v.split_whitespace();
        let (Some(n), Some(m), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(Error::BadCSeq);
        };
        if n.is_empty() || n.len() > 10 || !n.bytes().all(|b| b.is_ascii_digit()) || !is_token(m) {
            return Err(Error::BadCSeq);
        }
        let number: u32 = n.parse().map_err(|_| Error::BadCSeq)?;
        if number >= 1 << 31 {
            return Err(Error::BadCSeq);
        }
        Ok(Some(CSeq {
            number_text: n,
            number,
            method: m,
        }))
    }

    /// From or To: exactly one name-addr.
    pub fn single_name_addr(&self, canonical: &'static str) -> Result<Option<NameAddr>, Error> {
        let Some(v) = self.single(canonical)? else {
            return Ok(None);
        };
        let list = split_list(v)?;
        if list.len() != 1 {
            return Err(Error::BadNameAddr);
        }
        parse_name_addr(v).map(Some)
    }

    pub fn from(&self) -> Result<Option<NameAddr>, Error> {
        self.single_name_addr("From")
    }

    pub fn to(&self) -> Result<Option<NameAddr>, Error> {
        self.single_name_addr("To")
    }

    /// All name-addrs of a list header over all its occurrences (e.g. P-Asserted-Identity).
    pub fn name_addrs(&self, canonical: &str) -> Result<Vec<NameAddr>, Error> {
        let mut out = Vec::new();
        for v in self.header_values(canonical) {
            for item in split_list(v)? {
                out.push(parse_name_addr(item)?);
            }
        }
        Ok(out)
    }

    /// Serializes the message for a stream transport. Any existing Content-Length is replaced by
    /// the real body length. Fails closed on values that would break the framing.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let safe = |s: &str| !s.contains(['\r', '\n']) && !has_forbidden_control(s);
        let mut out = String::new();
        match &self.start {
            StartLine::Request { method, uri } => {
                if !is_token(method) || uri.is_empty() || uri.contains([' ', '\t']) || !safe(uri) {
                    return Err(Error::InvalidValue);
                }
                out.push_str(&format!("{method} {uri} SIP/2.0\r\n"));
            }
            StartLine::Response { code, reason } => {
                if !(100..=699).contains(code) || !safe(reason) {
                    return Err(Error::InvalidValue);
                }
                out.push_str(&format!("SIP/2.0 {code} {reason}\r\n"));
            }
        }
        for h in self
            .headers
            .iter()
            .filter(|h| !header_name_matches(&h.name, "Content-Length"))
        {
            if !is_token(&h.name) || !safe(&h.value) {
                return Err(Error::InvalidValue);
            }
            out.push_str(&format!("{}: {}\r\n", h.name, h.value));
        }
        out.push_str(&format!("Content-Length: {}\r\n\r\n", self.body.len()));
        let mut bytes = out.into_bytes();
        bytes.extend_from_slice(&self.body);
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests;
