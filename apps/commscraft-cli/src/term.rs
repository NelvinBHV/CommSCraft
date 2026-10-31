//! Terminal output of untrusted text (issue #42): server and peer data (SIP, HTTP bodies, NTLM
//! names, instant messages) may contain control sequences that would change the terminal or forge
//! log lines. Every line the CLI prints goes through [`clean`] by the macros in `main.rs`.

/// Escapes C0 and C1 control characters (incl. ESC and DEL) and bidirectional formatting
/// characters (U+200E/U+200F, U+202A–U+202E, U+2066–U+2069, which reorder what the reader sees)
/// as `\u{…}`. Line feed and tab stay as they are.
pub fn clean(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let bidi = matches!(c, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
        if (c.is_control() && c != '\n' && c != '\t') || bidi {
            out.push_str(&format!("\\u{{{:x}}}", u32::from(c)));
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn control_sequences_are_escaped() {
        assert_eq!(clean("a\u{1b}[2Jb"), "a\\u{1b}[2Jb");
        assert_eq!(clean("x\ry\u{7f}\u{9b}"), "x\\u{d}y\\u{7f}\\u{9b}");
        assert_eq!(clean("left\u{202e}right"), "left\\u{202e}right");
        assert_eq!(clean("line 1\n\tline 2 äöü"), "line 1\n\tline 2 äöü");
    }
}
