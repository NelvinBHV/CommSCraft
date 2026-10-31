//! A small, bounded XML 1.0 reader for protocol documents (provisioning, presence, conference
//! information, address book search).
//!
//! Security by design (`docs/security.md`, rule "XML"):
//! - no DTD: `<!DOCTYPE` is refused, so there are no external entities, no entity expansion
//!   ("billion laughs") and no external fetches;
//! - only the five predefined entities and character references are resolved;
//! - limits on input size, depth, attributes, name length and node count; the tree is built with an
//!   explicit stack, never by recursion;
//! - fail closed: anything not well-formed (mismatched tags, duplicate attributes, unbound
//!   prefixes, text outside the root) is an error.
//!
//! Namespaces (Namespaces in XML 1.0) are resolved for elements; attribute prefixes are kept as
//! written (protocol documents use unprefixed attributes).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_input: usize,
    pub max_depth: usize,
    pub max_attributes: usize,
    pub max_name_len: usize,
    pub max_nodes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input: 4 * 1024 * 1024,
            max_depth: 64,
            max_attributes: 64,
            max_name_len: 256,
            max_nodes: 200_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    TooLarge,
    TooDeep,
    TooManyAttributes,
    TooManyNodes,
    NameTooLong,
    /// `<!DOCTYPE`, `<!ENTITY` or another markup declaration.
    DtdNotAllowed,
    UnknownEntity,
    BadCharReference,
    UnboundPrefix,
    DuplicateAttribute,
    MismatchedTag,
    /// Any other well-formedness error, with the byte offset.
    Syntax(usize),
    Unterminated,
    NoRoot,
    TrailingContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// Local name (without prefix).
    pub name: String,
    /// Resolved namespace URI, if any.
    pub namespace: Option<String>,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        })
    }

    pub fn child(&self, name: &str) -> Option<&Element> {
        self.elements().find(|e| e.name == name)
    }

    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.elements().filter(move |e| e.name == name)
    }

    /// The concatenated text content of the direct text children.
    pub fn text(&self) -> String {
        self.children
            .iter()
            .filter_map(|n| match n {
                Node::Text(t) => Some(t.as_str()),
                Node::Element(_) => None,
            })
            .collect()
    }
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == ':'
}

fn is_name_char(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit() || c == '-' || c == '.' || c.is_numeric()
}

/// XML 1.0 `Char` production.
fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && c != '\u{fffe}' && c != '\u{ffff}')
}

struct Open {
    element: Element,
    raw_name: String,
    /// Namespace bindings declared on this element (prefix "" = default).
    bindings: Vec<(String, String)>,
}

struct Parser<'a> {
    s: &'a str,
    pos: usize,
    limits: Limits,
    nodes: usize,
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        self.s.get(self.pos..).unwrap_or("")
    }

    fn starts(&self, p: &str) -> bool {
        self.rest().starts_with(p)
    }

    fn err(&self) -> Error {
        Error::Syntax(self.pos)
    }

    fn skip_ws(&mut self) {
        let n = self.rest().len()
            - self
                .rest()
                .trim_start_matches([' ', '\t', '\r', '\n'])
                .len();
        self.pos += n;
    }

    fn expect(&mut self, p: &str) -> Result<(), Error> {
        if self.starts(p) {
            self.pos += p.len();
            Ok(())
        } else {
            Err(self.err())
        }
    }

    /// Skips to just after `end`.
    fn skip_past(&mut self, end: &str) -> Result<&'a str, Error> {
        let rest = self.rest();
        let i = rest.find(end).ok_or(Error::Unterminated)?;
        self.pos += i + end.len();
        rest.get(..i).ok_or(Error::Unterminated)
    }

    fn name(&mut self) -> Result<&'a str, Error> {
        let rest = self.rest();
        let mut chars = rest.char_indices();
        match chars.next() {
            Some((_, c)) if is_name_start(c) => {}
            _ => return Err(self.err()),
        }
        let end = chars
            .find(|(_, c)| !is_name_char(*c))
            .map_or(rest.len(), |(i, _)| i);
        if end > self.limits.max_name_len {
            return Err(Error::NameTooLong);
        }
        self.pos += end;
        rest.get(..end).ok_or(Error::Syntax(self.pos))
    }

    fn count_node(&mut self) -> Result<(), Error> {
        self.nodes += 1;
        if self.nodes > self.limits.max_nodes {
            return Err(Error::TooManyNodes);
        }
        Ok(())
    }
}

/// Resolves `&…;` references in text or attribute values.
fn unescape(raw: &str, at: usize) -> Result<String, Error> {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(i) = rest.find(['&', '<']) {
        out.push_str(rest.get(..i).ok_or(Error::Syntax(at))?);
        let tail = rest.get(i..).ok_or(Error::Syntax(at))?;
        if tail.starts_with('<') {
            return Err(Error::Syntax(at));
        }
        let semi = tail.find(';').ok_or(Error::UnknownEntity)?;
        let name = tail.get(1..semi).ok_or(Error::UnknownEntity)?;
        let c = match name {
            "lt" => '<',
            "gt" => '>',
            "amp" => '&',
            "apos" => '\'',
            "quot" => '"',
            _ => {
                let code = if let Some(h) = name.strip_prefix("#x") {
                    u32::from_str_radix(h, 16).map_err(|_| Error::BadCharReference)?
                } else if let Some(d) = name.strip_prefix('#') {
                    d.parse::<u32>().map_err(|_| Error::BadCharReference)?
                } else {
                    return Err(Error::UnknownEntity);
                };
                char::from_u32(code)
                    .filter(|c| is_xml_char(*c))
                    .ok_or(Error::BadCharReference)?
            }
        };
        out.push(c);
        rest = tail.get(semi + 1..).ok_or(Error::Syntax(at))?;
    }
    out.push_str(rest);
    Ok(out)
}

fn resolve<'b>(stack: &'b [Open], own: &'b [(String, String)], prefix: &str) -> Option<&'b str> {
    own.iter()
        .chain(stack.iter().rev().flat_map(|o| o.bindings.iter()))
        .find(|(p, _)| p == prefix)
        .map(|(_, uri)| uri.as_str())
        .filter(|uri| !uri.is_empty())
}

/// Parses a complete document and returns its root element.
pub fn parse(input: &str, limits: &Limits) -> Result<Element, Error> {
    if input.len() > limits.max_input {
        return Err(Error::TooLarge);
    }
    if let Some(i) = input.find(|c: char| !is_xml_char(c)) {
        return Err(Error::Syntax(i));
    }
    let mut p = Parser {
        s: input.strip_prefix('\u{feff}').unwrap_or(input),
        pos: 0,
        limits: *limits,
        nodes: 0,
    };
    let mut stack: Vec<Open> = Vec::new();
    let mut root: Option<Element> = None;
    loop {
        if p.pos >= p.s.len() {
            break;
        }
        if p.starts("<?") {
            p.skip_past("?>")?;
        } else if p.starts("<!--") {
            p.pos += 4;
            let body = p.skip_past("-->")?;
            if body.contains("--") || body.ends_with('-') {
                return Err(p.err());
            }
        } else if p.starts("<![CDATA[") {
            let at = p.pos;
            p.pos += 9;
            let text = p.skip_past("]]>")?;
            let parent = stack.last_mut().ok_or(Error::Syntax(at))?;
            p.nodes += 1;
            if p.nodes > limits.max_nodes {
                return Err(Error::TooManyNodes);
            }
            parent.element.children.push(Node::Text(text.to_string()));
        } else if p.starts("<!") {
            return Err(Error::DtdNotAllowed);
        } else if p.starts("</") {
            p.pos += 2;
            let name = p.name()?;
            p.skip_ws();
            p.expect(">")?;
            let open = stack.pop().ok_or(Error::MismatchedTag)?;
            if open.raw_name != name {
                return Err(Error::MismatchedTag);
            }
            match stack.last_mut() {
                Some(parent) => parent.element.children.push(Node::Element(open.element)),
                None => root = Some(open.element),
            }
        } else if p.starts("<") {
            if root.is_some() {
                return Err(Error::TrailingContent);
            }
            p.pos += 1;
            p.count_node()?;
            let raw_name = p.name()?.to_string();
            let mut raw_attrs: Vec<(String, String)> = Vec::new();
            loop {
                let before = p.pos;
                p.skip_ws();
                if p.starts("/>") || p.starts(">") {
                    break;
                }
                if p.pos == before {
                    return Err(p.err());
                }
                let an = p.name()?.to_string();
                p.skip_ws();
                p.expect("=")?;
                p.skip_ws();
                let quote = p.rest().chars().next().ok_or(Error::Unterminated)?;
                if quote != '"' && quote != '\'' {
                    return Err(p.err());
                }
                p.pos += 1;
                let at = p.pos;
                let raw = p.skip_past(&quote.to_string())?;
                let value = unescape(raw, at)?;
                if raw_attrs.iter().any(|(n, _)| *n == an) {
                    return Err(Error::DuplicateAttribute);
                }
                if raw_attrs.len() >= limits.max_attributes {
                    return Err(Error::TooManyAttributes);
                }
                raw_attrs.push((an, value));
            }
            let empty = p.starts("/>");
            p.pos += if empty { 2 } else { 1 };
            let mut bindings = Vec::new();
            for (n, v) in &raw_attrs {
                if n == "xmlns" {
                    bindings.push((String::new(), v.clone()));
                } else if let Some(prefix) = n.strip_prefix("xmlns:") {
                    if v.is_empty() {
                        return Err(Error::UnboundPrefix);
                    }
                    bindings.push((prefix.to_string(), v.clone()));
                }
            }
            let mut attributes = Vec::new();
            for (n, v) in raw_attrs {
                if n == "xmlns" || n.starts_with("xmlns:") {
                    continue;
                }
                if let Some((prefix, _)) = n.split_once(':') {
                    if prefix != "xml" && resolve(&stack, &bindings, prefix).is_none() {
                        return Err(Error::UnboundPrefix);
                    }
                }
                attributes.push(Attribute { name: n, value: v });
            }
            let (prefix, local) = match raw_name.split_once(':') {
                Some((pfx, l)) => (pfx, l),
                None => ("", raw_name.as_str()),
            };
            let namespace = resolve(&stack, &bindings, prefix).map(str::to_string);
            if !prefix.is_empty() && namespace.is_none() {
                return Err(Error::UnboundPrefix);
            }
            let element = Element {
                name: local.to_string(),
                namespace,
                attributes,
                children: Vec::new(),
            };
            if empty {
                match stack.last_mut() {
                    Some(parent) => parent.element.children.push(Node::Element(element)),
                    None => root = Some(element),
                }
            } else {
                if stack.len() >= limits.max_depth {
                    return Err(Error::TooDeep);
                }
                stack.push(Open {
                    element,
                    raw_name,
                    bindings,
                });
            }
        } else {
            let at = p.pos;
            let rest = p.rest();
            let end = rest.find('<').unwrap_or(rest.len());
            let raw = rest.get(..end).ok_or(Error::Syntax(at))?;
            p.pos += end;
            match stack.last_mut() {
                Some(parent) => {
                    if raw.contains("]]>") {
                        return Err(Error::Syntax(at));
                    }
                    let text = unescape(raw, at)?;
                    p.nodes += 1;
                    if p.nodes > limits.max_nodes {
                        return Err(Error::TooManyNodes);
                    }
                    parent.element.children.push(Node::Text(text));
                }
                None if raw.trim_matches([' ', '\t', '\r', '\n']).is_empty() => {}
                None if root.is_some() => return Err(Error::TrailingContent),
                None => return Err(Error::Syntax(at)),
            }
        }
    }
    if !stack.is_empty() {
        return Err(Error::Unterminated);
    }
    root.ok_or(Error::NoRoot)
}

/// Escapes text for element content or attribute values when writing XML.
/// XML Schema dateTime in UTC (`YYYY-MM-DDTHH:MM:SSZ`) of a Unix time; days to civil date after
/// H. Hinnant's algorithm (proleptic Gregorian calendar). Years after 9999 are refused.
pub fn xs_datetime(unix: u64) -> Option<String> {
    let days = i64::try_from(unix / 86_400).ok()?;
    let secs = unix % 86_400;
    let z = days.checked_add(719_468)?;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    if !(1970..=9999).contains(&year) {
        return None;
    }
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    ))
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if is_xml_char(c) => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests;
