//! Formatted messages: the `text/html` body the Windows client sends once the session's
//! `a=accept-types` names `text/html` (interop, `tested-in-lab`: `<B><SPAN style='font-size:10pt;
//! font-family:Segoe UI; color:#000000;'>bold</SPAN></B><SPAN …> plain</SPAN>`; the first message
//! of a session also comes as `ms-im-format: text/html; …; ms-body=<base64>` in the INVITE).
//! [MS-CONFIM] 20250520 §2.2.5 (p. 14) only names the format; the body is not specified, so this
//! is an interop observation and the reading is deliberately narrow.
//!
//! Security by design: nothing is rendered as HTML. A small bounded scanner turns the body into
//! text runs with bold, italic, underline, strike-through and a colour; every other tag is
//! dropped (its text kept), `script` and `style` lose their content, entities are decoded, control
//! characters removed. Input, run count and nesting are bounded.

/// One run of text with its look.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub color: Option<[u8; 3]>,
}

/// The look of our own messages (the font settings of the options).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<[u8; 3]>,
}

impl Style {
    pub fn is_plain(&self) -> bool {
        *self == Style::default()
    }
}

/// Largest body read (design; the plain text limit of the session is 32 KiB).
pub const MAX_HTML: usize = 64 * 1024;
const MAX_SPANS: usize = 512;
const MAX_DEPTH: usize = 32;

#[derive(Clone, Copy, Default)]
struct Look {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    color: Option<[u8; 3]>,
}

/// The text runs of a `text/html` body; `None` when it is too large.
pub fn parse(html: &str) -> Option<Vec<Span>> {
    if html.len() > MAX_HTML {
        return None;
    }
    let mut out: Vec<Span> = Vec::new();
    // Open elements: tag name and the look before it.
    let mut stack: Vec<(String, Look)> = Vec::new();
    let mut look = Look::default();
    let mut skip_until: Option<&'static str> = None;
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after
                .find("-->")
                .and_then(|i| after.get(i + 3..))
                .unwrap_or("");
            continue;
        }
        if rest.starts_with('<') {
            let end = match rest.find('>') {
                Some(e) => e,
                None => break,
            };
            let tag = rest.get(1..end).unwrap_or("");
            rest = rest.get(end + 1..).unwrap_or("");
            let closing = tag.starts_with('/');
            let body = tag.trim_start_matches('/').trim_end_matches('/');
            let name: String = body
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            if let Some(until) = skip_until {
                if closing && name == until {
                    skip_until = None;
                }
                continue;
            }
            match (closing, name.as_str()) {
                (false, "script") => skip_until = Some("script"),
                (false, "style") => skip_until = Some("style"),
                (_, "br") => push(&mut out, "\n", look),
                (true, "p" | "div" | "li") => push(&mut out, "\n", look),
                (false, n) if is_styling(n) => {
                    if stack.len() >= MAX_DEPTH {
                        continue;
                    }
                    stack.push((name.clone(), look));
                    match n {
                        "b" | "strong" => look.bold = true,
                        "i" | "em" => look.italic = true,
                        "u" | "ins" => look.underline = true,
                        "s" | "strike" | "del" => look.strike = true,
                        _ => {}
                    }
                    if let Some(c) = color_of(body) {
                        look.color = Some(c);
                    }
                }
                (true, n) if is_styling(n) => {
                    // Close up to the matching element (unbalanced bodies are common).
                    if let Some(i) = stack.iter().rposition(|(t, _)| t == n) {
                        look = stack.get(i).map_or(look, |(_, l)| *l);
                        stack.truncate(i);
                    }
                }
                _ => {}
            }
            continue;
        }
        let end = rest.find('<').unwrap_or(rest.len());
        let (text, after) = rest.split_at(end);
        rest = after;
        if skip_until.is_none() {
            push(&mut out, &decode(text), look);
        }
        if out.len() > MAX_SPANS {
            return None;
        }
    }
    // A trailing line break of a closing block element is not part of the message.
    if let Some(last) = out.last_mut() {
        while last.text.ends_with('\n') {
            last.text.pop();
        }
    }
    out.retain(|s| !s.text.is_empty());
    Some(out)
}

fn is_styling(n: &str) -> bool {
    matches!(
        n,
        "b" | "strong"
            | "i"
            | "em"
            | "u"
            | "ins"
            | "s"
            | "strike"
            | "del"
            | "span"
            | "font"
            | "a"
            | "p"
            | "div"
            | "li"
            | "ul"
            | "ol"
    )
}

/// Appends text with a look, merging with the previous run of the same look.
fn push(out: &mut Vec<Span>, text: &str, l: Look) {
    let text: String = text
        .chars()
        .filter(|c| *c == '\n' || !c.is_control())
        .collect();
    if text.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.bold == l.bold
            && last.italic == l.italic
            && last.underline == l.underline
            && last.strike == l.strike
            && last.color == l.color
        {
            last.text.push_str(&text);
            return;
        }
    }
    out.push(Span {
        text,
        bold: l.bold,
        italic: l.italic,
        underline: l.underline,
        strike: l.strike,
        color: l.color,
    });
}

/// `color:#RRGGBB` in a `style` attribute, or `color="#RRGGBB"` (FONT).
fn color_of(tag: &str) -> Option<[u8; 3]> {
    let lower = tag.to_ascii_lowercase();
    let i = lower.find("color")?;
    let after = lower.get(i + 5..)?;
    let hash = after.find('#')?;
    // Only directly after `:` or `=` (and quotes or spaces).
    if !after
        .get(..hash)?
        .chars()
        .all(|c| matches!(c, ':' | '=' | ' ' | '"' | '\''))
    {
        return None;
    }
    let hex = after.get(hash + 1..hash + 7)?;
    let v = u32::from_str_radix(hex, 16).ok()?;
    let [_, r, g, b] = v.to_be_bytes();
    // Black is the default text colour: not a colour of its own.
    (v != 0).then_some([r, g, b])
}

fn decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(rest.get(..i).unwrap_or(""));
        rest = rest.get(i..).unwrap_or("");
        let Some(end) = rest.find(';').filter(|&e| e <= 10) else {
            out.push('&');
            rest = rest.get(1..).unwrap_or("");
            continue;
        };
        let entity = rest.get(1..end).unwrap_or("");
        let ch = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            e if e.starts_with("#x") || e.starts_with("#X") => e
                .get(2..)
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .and_then(char::from_u32),
            e if e.starts_with('#') => e
                .get(1..)
                .and_then(|d| d.parse::<u32>().ok())
                .and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = rest.get(end + 1..).unwrap_or("");
            }
            None => {
                out.push('&');
                rest = rest.get(1..).unwrap_or("");
            }
        }
    }
    out.push_str(rest);
    out
}

/// The text of the runs (for plain displays, history and notifications).
pub fn plain(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// Our message as `text/html` in the form the Windows client sends.
pub fn to_html(text: &str, style: &Style) -> String {
    let mut esc = String::with_capacity(text.len() + 32);
    for c in text.chars() {
        match c {
            '&' => esc.push_str("&amp;"),
            '<' => esc.push_str("&lt;"),
            '>' => esc.push_str("&gt;"),
            '"' => esc.push_str("&quot;"),
            '\'' => esc.push_str("&#39;"),
            '\n' => esc.push_str("<BR>"),
            c if c.is_control() => {}
            c => esc.push(c),
        }
    }
    let color = style.color.unwrap_or([0, 0, 0]);
    let mut s = format!(
        "<SPAN style='color:#{:02X}{:02X}{:02X};'>{esc}</SPAN>",
        color[0], color[1], color[2]
    );
    for (on, tag) in [
        (style.underline, "U"),
        (style.italic, "I"),
        (style.bold, "B"),
    ] {
        if on {
            s = format!("<{tag}>{s}</{tag}>");
        }
    }
    s
}
