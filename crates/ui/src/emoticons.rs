//! Emoticons in conversations, as the Windows client offers them: a picker in the composer that
//! writes the text code (the plain text every client understands; the Windows client shows its
//! own picture for it), and received or sent codes shown as the matching emoji when a system
//! font has it – otherwise the code stays as written.
//!
//! The codes are the common text emoticons; the pictures come from the system's emoji font
//! (no images are shipped).

/// Code, emoji, German and English name.
pub const EMOTICONS: &[(&str, &str, &str, &str)] = &[
    (":)", "🙂", "Lächeln", "Smile"),
    (":D", "😀", "Lachen", "Laugh"),
    (";)", "😉", "Zwinkern", "Wink"),
    (":(", "🙁", "Traurig", "Sad"),
    (":'(", "😢", "Weinen", "Crying"),
    (":P", "😛", "Zunge", "Tongue out"),
    (":O", "😮", "Überrascht", "Surprised"),
    (":|", "😐", "Neutral", "Neutral"),
    (":@", "😠", "Wütend", "Angry"),
    ("8)", "😎", "Cool", "Cool"),
    ("(y)", "👍", "Daumen hoch", "Thumbs up"),
    ("(n)", "👎", "Daumen runter", "Thumbs down"),
    ("<3", "❤", "Herz", "Heart"),
    ("(coffee)", "☕", "Kaffee", "Coffee"),
    ("(flower)", "🌸", "Blume", "Flower"),
    ("(cake)", "🎂", "Kuchen", "Cake"),
];

/// The text with each emoticon code that stands alone (between spaces or at the ends) replaced
/// by its emoji; `has` tells whether the font has an emoji.
pub fn show(text: &str, has: &mut dyn FnMut(&str) -> bool) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, word) in text.split(' ').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        // A code may end a line or a sentence: only the word itself is compared.
        let (core, rest) = split_line_end(word);
        match EMOTICONS
            .iter()
            .find(|(code, ..)| code.eq_ignore_ascii_case(core))
        {
            Some((_, emoji, ..)) if has(emoji) => {
                out.push_str(emoji);
                out.push_str(rest);
            }
            _ => out.push_str(word),
        }
    }
    out
}

/// A word and the line break that may follow it.
fn split_line_end(word: &str) -> (&str, &str) {
    match word.find('\n') {
        Some(i) => word.split_at(i),
        None => (word, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_that_stand_alone_become_emoji() {
        let mut yes = |_: &str| true;
        assert_eq!(show("Gut :) danke (y)", &mut yes), "Gut 🙂 danke 👍");
        assert_eq!(show(":d", &mut yes), "😀");
        assert_eq!(show("ok :)\nbis dann", &mut yes), "ok 🙂\nbis dann");
        // Inside a word or a link nothing changes.
        assert_eq!(show("http://x:)", &mut yes), "http://x:)");
        assert_eq!(show("f(n)", &mut yes), "f(n)");
        // Without the font the code stays.
        assert_eq!(show("Gut :)", &mut |_| false), "Gut :)");
    }

    #[test]
    fn every_code_is_unique_and_short() {
        for (i, (code, ..)) in EMOTICONS.iter().enumerate() {
            assert!(code.len() <= 8 && !code.contains(' '));
            assert!(EMOTICONS
                .iter()
                .skip(i + 1)
                .all(|(c, ..)| !c.eq_ignore_ascii_case(code)));
        }
    }
}
