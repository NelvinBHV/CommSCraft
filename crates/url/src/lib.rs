//! Checked host names and HTTPS URLs for values that arrive from the network (Autodiscover,
//! provisioning, DNS SRV).
//!
//! Security by design (`docs/security.md`, rules "Discovery" and "Provisioned values are input
//! too"): only `https://` with a DNS host name, no user info, no IP literal, a valid port, no
//! characters that could break an HTTP request line or a header; and a test whether a host lies in
//! the user's SIP domain, which decides whether credentials may go there without asking the user.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpsUrl {
    /// The URL as received.
    pub url: String,
    /// Lower-case DNS host name.
    pub host: String,
    pub port: u16,
    /// Path and query, starting with `/` (empty input path becomes `/`).
    pub path: String,
}

/// RFC 1035/1123 host name with at least two labels and a non-numeric last label (so no IPv4
/// literal).
pub fn dns_host(host: &str) -> Option<String> {
    let host = host.strip_suffix('.').unwrap_or(host).to_ascii_lowercase();
    let label_ok = |l: &str| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    };
    let numeric_tld = host
        .rsplit('.')
        .next()
        .is_some_and(|l| l.chars().all(|c| c.is_ascii_digit()));
    if host.len() > 253 || !host.contains('.') || numeric_tld || !host.split('.').all(label_ok) {
        return None;
    }
    Some(host)
}

/// True if `host` is `domain` or a subdomain of it (labels compared case-insensitively).
pub fn in_domain(host: &str, domain: &str) -> bool {
    let (h, d) = (host.to_ascii_lowercase(), domain.to_ascii_lowercase());
    let h = h.strip_suffix('.').unwrap_or(&h);
    let d = d.strip_suffix('.').unwrap_or(&d);
    !d.is_empty() && (h == d || h.ends_with(&format!(".{d}")))
}

pub fn parse_https(value: &str) -> Option<HttpsUrl> {
    if value.len() > 2048
        || value
            .chars()
            .any(|c| c.is_control() || " \"<>\\^`{|}".contains(c))
    {
        return None;
    }
    let scheme_end = value.find("://")?;
    if !value.get(..scheme_end)?.eq_ignore_ascii_case("https") {
        return None;
    }
    let rest = value.get(scheme_end + 3..)?;
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = rest.get(..authority_end)?;
    if authority.contains(['@', '[', ']']) {
        return None;
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => {
            if p.is_empty()
                || p.starts_with('0')
                || p.len() > 5
                || !p.bytes().all(|b| b.is_ascii_digit())
            {
                return None;
            }
            let n: u16 = p.parse().ok()?;
            (h, n)
        }
        None => (authority, 443),
    };
    if port == 0 {
        return None;
    }
    let mut path = rest.get(authority_end..)?.split('#').next()?.to_string();
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    Some(HttpsUrl {
        url: value.to_string(),
        host: dns_host(host)?,
        port,
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_https_urls() {
        let u = parse_https("https://Pool1.Contoso.com:4443/abs/handler?x=1#frag").unwrap();
        assert_eq!(
            (u.host.as_str(), u.port, u.path.as_str()),
            ("pool1.contoso.com", 4443, "/abs/handler?x=1")
        );
        let u = parse_https("https://lyncdiscover.contoso.com?sipuri=john@contoso.com").unwrap();
        assert_eq!(
            (u.port, u.path.as_str()),
            (443, "/?sipuri=john@contoso.com")
        );
    }

    #[test]
    fn refuses_everything_else() {
        for bad in [
            "http://contoso.com/",
            "https://user@contoso.com/",
            "https://[2001:db8::1]/",
            "https://192.168.1.1/",
            "https://contoso.com:0/",
            "https://contoso.com:080/",
            "https://contoso.com:70000/",
            "https://contoso.com:/",
            "https://localhost/",
            "https://-a.contoso.com/",
            "https://contoso.com/a b",
            "https://contoso.com/\r\nHost: evil",
            "javascript:alert(1)",
            "https:/contoso.com",
        ] {
            assert!(parse_https(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn domain_membership() {
        assert!(in_domain("contoso.com", "contoso.com"));
        assert!(in_domain("POOL1.contoso.com.", "Contoso.com"));
        assert!(!in_domain("evilcontoso.com", "contoso.com"));
        assert!(!in_domain("contoso.com.evil.example", "contoso.com"));
        assert!(!in_domain("contoso.com", ""));
    }
}
