//! Spell checking in the composer (the Windows client's "check spelling as I type"): the
//! interface asks a [`Speller`] the application provides (Hunspell dictionaries through
//! `zspell`) and underlines the words it does not know; a right click offers suggestions.

use std::ops::Range;

/// A dictionary for one language.
pub trait Speller: Send + Sync {
    /// The word is spelled right (or not a word to check).
    fn check(&self, word: &str) -> bool;
    /// Up to a few spellings for a word it does not know.
    fn suggest(&self, word: &str) -> Vec<String>;
}

/// The words of a text as byte ranges: letters, with an apostrophe or hyphen inside a word.
/// Words with digits, addresses (`@`, `/`, `.` inside) and single letters are left out.
pub fn words(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let joiner = |c: char| c == '\'' || c == '-' || c == '’';
    for (k, &(i, c)) in chars.iter().enumerate() {
        let next_letter = chars.get(k + 1).is_some_and(|(_, n)| n.is_alphabetic());
        let in_word = c.is_alphabetic() || (start.is_some() && joiner(c) && next_letter);
        match (in_word, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push(s..i);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(s..text.len());
    }
    // A word glued to digits or address characters is no word to check.
    let glued = |r: &Range<usize>| {
        let before = text.get(..r.start).and_then(|t| t.chars().last());
        let after = text.get(r.end..).and_then(|t| t.chars().next());
        [before, after]
            .into_iter()
            .flatten()
            .any(|c| c.is_ascii_digit() || matches!(c, '@' | '/' | '_' | '\\' | ':'))
            || (after == Some('.')
                && text
                    .get(r.end + 1..)
                    .and_then(|t| t.chars().next())
                    .is_some_and(char::is_alphanumeric))
    };
    out.retain(|r| r.len() > 1 && !glued(r));
    out
}

/// The misspelled words of a text; the last word is left out while it is still being typed
/// (the text ends inside it).
pub fn misspelled(speller: &dyn Speller, text: &str) -> Vec<Range<usize>> {
    words(text)
        .into_iter()
        .filter(|r| r.end != text.len())
        .filter(|r| text.get(r.clone()).is_some_and(|w| !speller.check(w)))
        .collect()
}

/// The word around a byte position.
pub fn word_at(text: &str, at: usize) -> Option<Range<usize>> {
    words(text)
        .into_iter()
        .find(|r| r.start <= at && at <= r.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tiny;
    impl Speller for Tiny {
        fn check(&self, w: &str) -> bool {
            ["Hallo", "Welt", "heute"].contains(&w)
        }
        fn suggest(&self, _: &str) -> Vec<String> {
            vec!["Welt".into()]
        }
    }

    #[test]
    fn words_and_misspelled_ones() {
        let t = "Hallo Wlet, heute um 10 Uhr: alice@contoso.com www.x.de don't B2B Wlet";
        let w: Vec<&str> = words(t).into_iter().filter_map(|r| t.get(r)).collect();
        assert!(w.contains(&"Hallo") && w.contains(&"Wlet") && w.contains(&"don't"));
        assert!(!w
            .iter()
            .any(|x| x.contains("alice") || *x == "www" || *x == "B"));
        let bad: Vec<&str> = misspelled(&Tiny, t)
            .into_iter()
            .filter_map(|r| t.get(r))
            .collect();
        // The last "Wlet" is still being typed; "Uhr", "um" and "don't" are unknown to Tiny.
        assert_eq!(bad.first(), Some(&"Wlet"));
        assert_eq!(bad.iter().filter(|w| **w == "Wlet").count(), 1);
        assert_eq!(word_at(t, 7).and_then(|r| t.get(r)), Some("Wlet"));
    }
}
