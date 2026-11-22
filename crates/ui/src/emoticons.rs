//! Emoticons in conversations, as the Windows client offers them: a picker in the composer that
//! writes the text code (the plain text every client understands; the Windows client shows its
//! own picture for it), and received or sent codes shown as the matching emoji when a system
//! font has it – otherwise the code stays as written.
//!
//! The codes are those the Windows client sends; the desktop app shows coloured pictures
//! (Noto Color Emoji) and the emoji character where it has none.

/// Code, emoji, German and English name: the emoticons of the Windows client's picker in its
/// order, with the text codes it sends (lab capture 2026-11-22); the pictures are our own
/// choice (Noto Color Emoji).
pub const EMOTICONS: &[(&str, &str, &str, &str)] = &[
    (":)", "🙂", "Lächeln", "Smile"),
    (":(", "🙁", "Traurig", "Sad"),
    (":D", "😀", "Lachen", "Big smile"),
    ("B-)", "😎", "Cool", "Cool"),
    (":O", "😮", "Überrascht", "Surprised"),
    (";)", "😉", "Zwinkern", "Wink"),
    (":'(", "😢", "Weinen", "Crying"),
    ("(:|", "😓", "Schwitzen", "Sweating"),
    (":|", "😐", "Sprachlos", "Speechless"),
    ("(kiss)", "😘", "Kuss", "Kiss"),
    (":P", "😛", "Zunge", "Tongue out"),
    (":$", "😳", "Verlegen", "Blush"),
    (":^)", "🤔", "Nachdenklich", "Wondering"),
    ("|-)", "😪", "Schläfrig", "Sleepy"),
    ("|-(", "😑", "Gelangweilt", "Dull"),
    (":]", "😊", "Verliebt", "In love"),
    ("(yawn)", "🥱", "Gähnen", "Yawn"),
    ("(doh)", "😖", "Mist", "Doh"),
    ("8-)", "🙄", "Augenrollen", "Eye roll"),
    (":S", "😕", "Besorgt", "Worried"),
    ("8-|", "🤓", "Streber", "Nerd"),
    ("(grin)", "😁", "Grinsen", "Grin"),
    (":&&", "🤐", "Verschwiegen", "Lips sealed"),
    (":@", "😠", "Wütend", "Angry"),
    ("(devil)", "😈", "Teufel", "Devil"),
    ("(envy)", "🤢", "Neidisch", "Envy"),
    (":-?", "🤨", "Grübeln", "Thinking"),
    ("(happy)", "😄", "Fröhlich", "Happy"),
    ("<O)", "🥳", "Party", "Party"),
    ("(mm)", "🧐", "Hmm", "Mmm"),
    (":x", "🙊", "Kein Kommentar", "Not saying"),
    ("(angel)", "😇", "Engel", "Angel"),
    ("(nod)", "🙂‍↕️", "Nicken", "Nod"),
    ("(waiting)", "⏳", "Warten", "Waiting"),
    ("(makeup)", "💄", "Schminken", "Make-up"),
    ("(rofl)", "🤣", "Vor Lachen", "Rolling on the floor"),
    ("^o)", "😏", "Skeptisch", "Sarcastic"),
    ("(shake)", "🙂‍↔️", "Kopfschütteln", "Shake"),
    ("(emo)", "🖤", "Emo", "Emo"),
    ("(wave)", "👋", "Winken", "Wave"),
    ("(yn)", "🤞", "Daumen drücken", "Fingers crossed"),
    ("(facepalm)", "🤦", "Facepalm", "Facepalm"),
    ("(chuckle)", "😆", "Kichern", "Chuckle"),
    ("(whew)", "😅", "Puh", "Relieved"),
    ("(wait)", "✋", "Moment", "Wait"),
    ("(clap)", "👏", "Applaus", "Clapping"),
    ("(highfive)", "🙌", "High five", "High five"),
    ("(call)", "🤙", "Ruf an", "Call me"),
    ("(tmi)", "🙉", "Zu viel", "Too much information"),
    (":i", "💡", "Idee", "Idea"),
    ("(headbang)", "🤘", "Headbang", "Headbang"),
    ("(lalala)", "🙉", "Lalala", "Not listening"),
    ("*|", "😱", "Erschrocken", "Scared"),
    ("(rock)", "🎸", "Rock", "Rock"),
    ("(swear)", "🤬", "Fluchen", "Swearing"),
    ("(talk)", "🗣", "Reden", "Talking"),
    (
        "(talktothehand)",
        "🤚",
        "Sprich zur Hand",
        "Talk to the hand",
    ),
    ("(bandit)", "🦹", "Bandit", "Bandit"),
    ("<3", "❤", "Herz", "Heart"),
    ("(u)", "💔", "Gebrochenes Herz", "Broken heart"),
    ("(y)", "👍", "Daumen hoch", "Yes"),
    ("(n)", "👎", "Daumen runter", "No"),
    ("(handshake)", "🤝", "Handschlag", "Handshake"),
    ("(flex)", "💪", "Stark", "Muscle"),
    ("(man)", "👨", "Mann", "Man"),
    ("(woman)", "👩", "Frau", "Woman"),
    ("(bow)", "🙇", "Verbeugen", "Bow"),
    ("(gottarun)", "🏃", "Muss los", "Gotta run"),
    ("(stop)", "🛑", "Stopp", "Stop"),
    ("(dance)", "💃", "Tanzen", "Dance"),
    ("(bartlett)", "🧔", "Bartlett", "Bartlett"),
    ("(bike)", "🚴", "Fahrrad", "Bike"),
    ("(ninja)", "🥷", "Ninja", "Ninja"),
    ("(cat)", "🐱", "Katze", "Cat"),
    (":o3", "🐶", "Hund", "Dog"),
    ("(hug)", "🧸", "Umarmung", "Hug"),
    ("(heidy)", "🐿", "Eichhörnchen", "Squirrel"),
    ("(snail)", "🐌", "Schnecke", "Snail"),
    ("(F)", "🌸", "Blume", "Flower"),
    ("(goodluck)", "🍀", "Viel Glück", "Good luck"),
    ("(sun)", "☀", "Sonne", "Sun"),
    ("(island)", "🏝", "Insel", "Island"),
    ("(rain)", "🌧", "Regen", "Rain"),
    ("(umbrella)", "☂", "Schirm", "Umbrella"),
    ("(rainbow)", "🌈", "Regenbogen", "Rainbow"),
    ("(*)", "⭐", "Stern", "Star"),
    ("(tumbleweed)", "🍂", "Steppenläufer", "Tumbleweed"),
    ("(pi)", "🍕", "Pizza", "Pizza"),
    ("(^)", "🎂", "Kuchen", "Cake"),
    ("(coffee)", "☕", "Kaffee", "Coffee"),
    ("(beer)", "🍺", "Bier", "Beer"),
    ("(d)", "🍸", "Drink", "Drink"),
    ("(canyoutalk)", "📞", "Kannst du reden?", "Can you talk?"),
    ("(camera)", "📷", "Kamera", "Camera"),
    ("(plane)", "✈", "Flugzeug", "Plane"),
    ("(car)", "🚗", "Auto", "Car"),
    ("(computer)", "💻", "Computer", "Computer"),
    ("(wfh)", "🏠", "Homeoffice", "Working from home"),
    ("(games)", "🎮", "Spiele", "Games"),
    ("(mp)", "📱", "Handy", "Phone"),
    ("(gift)", "🎁", "Geschenk", "Gift"),
    ("(holdon)", "⌛", "Moment bitte", "Hold on"),
    ("(e)", "✉", "E-Mail", "Mail"),
    ("(cash)", "💰", "Geld", "Cash"),
    ("(music)", "🎵", "Musik", "Music"),
    ("(whatsgoingon)", "❓", "Was ist los?", "What's going on"),
];

/// Further codes understood when received (common spellings and earlier CommSCraft codes),
/// each mapped to a code of [`EMOTICONS`].
pub const ALIASES: &[(&str, &str)] = &[
    (":-)", ":)"),
    (":-(", ":("),
    (":-D", ":D"),
    (";-)", ";)"),
    (":-P", ":P"),
    (":-O", ":O"),
    ("8)", "B-)"),
    ("(flower)", "(F)"),
    ("(cake)", "(^)"),
    ("(c)", "(coffee)"),
    ("(b)", "(beer)"),
];

/// The emoji of a code or alias (case does not matter).
pub fn emoji_of(code: &str) -> Option<&'static str> {
    let code = ALIASES
        .iter()
        .find(|(a, _)| a.eq_ignore_ascii_case(code))
        .map_or(code, |(_, c)| *c);
    EMOTICONS
        .iter()
        .find(|(c, ..)| c.eq_ignore_ascii_case(code))
        .map(|(_, e, ..)| *e)
}

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
        match emoji_of(core) {
            Some(emoji) if has(emoji) => {
                out.push_str(emoji);
                out.push_str(rest);
            }
            _ => out.push_str(word),
        }
    }
    out
}

/// A piece of a message: text, or an emoticon (its emoji) where a code stands alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece<'a> {
    Text(String),
    Emoticon(&'a str),
}

/// The text split into text and emoticons (codes standing alone, as in [`show`]); neighbouring
/// text is joined.
pub fn pieces(text: &str) -> Vec<Piece<'static>> {
    let mut out: Vec<Piece<'static>> = Vec::new();
    let push_text = |out: &mut Vec<Piece<'static>>, t: &str| match out.last_mut() {
        Some(Piece::Text(s)) => s.push_str(t),
        _ => out.push(Piece::Text(t.to_string())),
    };
    for (i, word) in text.split(' ').enumerate() {
        if i > 0 {
            push_text(&mut out, " ");
        }
        let (core, rest) = split_line_end(word);
        match emoji_of(core) {
            Some(emoji) => {
                out.push(Piece::Emoticon(emoji));
                if !rest.is_empty() {
                    push_text(&mut out, rest);
                }
            }
            None => push_text(&mut out, word),
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
    fn a_message_splits_into_text_and_emoticons() {
        assert_eq!(
            pieces("Gut :) danke (y)"),
            vec![
                Piece::Text("Gut ".into()),
                Piece::Emoticon("🙂"),
                Piece::Text(" danke ".into()),
                Piece::Emoticon("👍"),
            ]
        );
        assert_eq!(pieces("f(n) ok"), vec![Piece::Text("f(n) ok".into())]);
    }

    #[test]
    fn every_code_is_unique_and_short() {
        for (i, (code, ..)) in EMOTICONS.iter().enumerate() {
            assert!(code.len() <= 16 && !code.contains(' '));
            assert!(EMOTICONS
                .iter()
                .skip(i + 1)
                .all(|(c, ..)| !c.eq_ignore_ascii_case(code)));
        }
    }
}
