//! Phone number normalization with the rules of an address book file (AB-11; [MS-ABS] 20250520
//! §2.2.5, p. 20): the rules are tried in order and the first matching one normalizes the number;
//! the built-in rule `E164` matches up to 15 digits (spaces, periods, hyphens and parentheses
//! ignored) and gives `tel:+` and the digits. Without a matching rule the number stays as it is.
//!
//! The patterns use [MC-RegEx] (.NET) syntax. They are compiled with the `regex` crate, which
//! covers the syntax such rules use (classes, quantifiers, groups, anchors) and matches in linear
//! time; a rule with a construct it does not have (look-around, back-references) is left out
//! (design: fail closed – the number is then not normalized by that rule). The replacement is
//! translated from .NET syntax: `$n` and `${n}` (the longest digit run, inference from the .NET
//! substitution rules), `${name}`, `$&` (whole match), `$$`; any other `$` is literal.
//!
//! Matching (inference, recomputed against the example §3.1, where each phone value is stored
//! next to its normalized form): a rule matches only the whole number – with the example's rules
//! `555-533-4312` gives `tel:+5555334312` only so, as an unanchored rule 10 would take
//! `533-4312` first – and a result without `tel:` gets it in front (RFC 3966). A rule whose
//! replacement is `null` (second rule of the example) leaves the number as it is.
//!
//! Security by design: patterns and numbers are bounded in length, the compiled program in size.

use crate::book::Rule;
use regex::{Regex, RegexBuilder};

/// Longest pattern and replacement compiled, longest number normalized (design).
pub const MAX_PATTERN: usize = 1024;
pub const MAX_NUMBER: usize = 64;
/// Most rules compiled (design).
pub const MAX_RULES: usize = 256;
const MAX_PROGRAM: usize = 256 * 1024;

#[derive(Debug, Clone)]
enum Compiled {
    E164,
    Rule(Regex, String),
}

/// The compiled rules of an address book file.
#[derive(Debug, Clone, Default)]
pub struct Normalizer {
    rules: Vec<Compiled>,
    /// How many rules could not be compiled.
    pub skipped: usize,
}

/// A .NET replacement string in the syntax of the `regex` crate.
fn replacement(dotnet: &str) -> String {
    let mut out = String::new();
    let mut chars = dotnet.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('$') => {
                chars.next();
                out.push_str("$$");
            }
            Some('&') => {
                chars.next();
                out.push_str("${0}");
            }
            Some(d) if d.is_ascii_digit() => {
                let mut n = String::new();
                while let Some(d) = chars.peek().copied().filter(char::is_ascii_digit) {
                    n.push(d);
                    chars.next();
                }
                out.push_str(&format!("${{{n}}}"));
            }
            Some('{') => {
                let rest: String = chars.clone().collect();
                match rest.find('}') {
                    Some(end) if end > 1 => {
                        let name = rest.get(1..end).unwrap_or_default();
                        if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                            out.push_str(&format!("${{{name}}}"));
                            for _ in 0..=end {
                                chars.next();
                            }
                        } else {
                            out.push_str("$$");
                        }
                    }
                    _ => out.push_str("$$"),
                }
            }
            _ => out.push_str("$$"),
        }
    }
    out
}

/// The built-in rule: at most 15 digits once spaces, periods, hyphens and parentheses are
/// removed.
fn e164(number: &str) -> Option<String> {
    let digits: String = number
        .chars()
        .filter(|c| !matches!(c, ' ' | '.' | '-' | '(' | ')'))
        .collect();
    let ok = !digits.is_empty() && digits.len() <= 15 && digits.chars().all(|c| c.is_ascii_digit());
    ok.then(|| format!("tel:+{digits}"))
}

impl Normalizer {
    pub fn new(rules: &[Rule]) -> Normalizer {
        let mut n = Normalizer::default();
        for r in rules.iter().take(MAX_RULES) {
            if r.pattern == "E164" {
                n.rules.push(Compiled::E164);
                continue;
            }
            if r.pattern.len() > MAX_PATTERN || r.replacement.len() > MAX_PATTERN {
                n.skipped += 1;
                continue;
            }
            match RegexBuilder::new(&format!("^(?:{})$", r.pattern))
                .size_limit(MAX_PROGRAM)
                .dfa_size_limit(MAX_PROGRAM)
                .build()
            {
                Ok(re) => n
                    .rules
                    .push(Compiled::Rule(re, replacement(&r.replacement))),
                Err(_) => n.skipped += 1,
            }
        }
        n.skipped += rules.len().saturating_sub(MAX_RULES);
        n
    }

    /// The number normalized by the first matching rule, `None` when no rule matches.
    pub fn normalize(&self, number: &str) -> Option<String> {
        let number = number.trim();
        if number.is_empty() || number.len() > MAX_NUMBER {
            return None;
        }
        let (re, with) = match self.rules.iter().find(|r| match r {
            Compiled::E164 => e164(number).is_some(),
            Compiled::Rule(re, _) => re.is_match(number),
        })? {
            Compiled::E164 => return e164(number),
            Compiled::Rule(re, with) => (re, with),
        };
        if with == "null" {
            return None;
        }
        let out = re.replace(number, with.as_str());
        let out = out.trim();
        if out.is_empty() {
            return None;
        }
        Some(if out.starts_with("tel:") {
            out.to_string()
        } else {
            format!("tel:{out}")
        })
    }
}
