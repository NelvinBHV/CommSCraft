//! Function 44: the content of a meeting (whiteboard, slides, polls) through the web
//! conferencing MCU. A thread per meeting keeps the PSOM connection ([MS-PSOM] over TLS) and
//! reports what changes as lines for the terminal.

use commscraft_conf::DataConnection;
use commscraft_net::TlsClient;
use commscraft_psom::client::{Client, Event};
use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

pub enum ToContent {
    Vote { content: i64, choice: i64 },
    Stop,
}

/// Lines for the terminal, the events themselves (for a graphical interface), or the end of the
/// connection.
pub enum FromContent {
    Line(u32, String),
    Event(u32, Box<Event>),
    Ended(u32, String),
}

pub struct ContentHandle {
    pub tx: Sender<ToContent>,
}

pub fn spawn(
    conversation: u32,
    connection: DataConnection,
    tls: TlsClient,
    out: Sender<FromContent>,
) -> ContentHandle {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let reason = match run(conversation, &connection, &tls, &rx, &out) {
            Ok(()) => "closed".to_string(),
            Err(e) => e,
        };
        let _ = out.send(FromContent::Ended(conversation, reason));
    });
    ContentHandle { tx }
}

/// Bounds on what is kept of a meeting's content (issue #254): server events with ever new
/// IDs cannot grow the state without limit; past a bound, new entries are not kept (shown by
/// their number only). Design values: a meeting has at most 1000 users in its roster (the
/// conference roster's bound) and far fewer than 256 contents.
const MAX_USERS: usize = 1000;
const MAX_CONTENTS: usize = 256;
/// Longest name or title kept, in characters.
const MAX_TEXT: usize = 512;
/// A poll has seven places ([MS-PSOM] poll content).
const MAX_CHOICES: usize = 7;

/// Inserts into a bounded map: an existing key is updated, a new one only below `max`.
fn put<V>(map: &mut BTreeMap<i64, V>, key: i64, value: V, max: usize) {
    if map.len() < max || map.contains_key(&key) {
        map.insert(key, value);
    }
}

fn short(s: String) -> String {
    if s.chars().count() > MAX_TEXT {
        s.chars().take(MAX_TEXT).collect()
    } else {
        s
    }
}

/// Most events taken from one read of the network (16 KiB): room for these, and their lines,
/// is needed before reading (issue #253).
const MAX_EVENTS_PER_READ: usize = 1024;

/// Sends a content event or line counted in [`crate::backlog::CONTENT`]; the consumer gives
/// the place back with [`taken`].
fn send_counted(out: &Sender<FromContent>, item: FromContent) {
    // Room was checked before the read; a full count still sends (never drop content).
    let counted = crate::backlog::CONTENT.enter();
    if out.send(item).is_err() && counted {
        crate::backlog::CONTENT.leave();
    }
}

/// The consumer took a content event or line from the queue: gives its place back.
pub fn taken(item: &FromContent) {
    if matches!(item, FromContent::Event(..) | FromContent::Line(..)) {
        crate::backlog::CONTENT.leave();
    }
}

/// What the terminal shows of each content.
#[derive(Default)]
struct Shown {
    users: BTreeMap<i64, String>,
    kinds: BTreeMap<i64, String>,
    titles: BTreeMap<i64, String>,
    choices: BTreeMap<i64, Vec<String>>,
}

impl Shown {
    fn name(&self, content: i64) -> String {
        match (self.titles.get(&content), self.kinds.get(&content)) {
            (Some(t), _) => format!("{content} \"{t}\""),
            (None, Some(k)) => format!("{content} ({})", k.trim_start_matches("Content.")),
            _ => content.to_string(),
        }
    }

    fn user(&self, id: i64) -> String {
        self.users
            .get(&id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    }

    fn line(&mut self, e: Event) -> Option<String> {
        Some(match e {
            Event::Connected => "connected to the meeting content".into(),
            Event::UsersAdded(users) => {
                for (id, uri, name) in users {
                    let shown = if name.is_empty() { uri } else { name };
                    put(&mut self.users, id, short(shown), MAX_USERS);
                }
                return None;
            }
            Event::UsersRemoved(ids) => {
                for id in ids {
                    self.users.remove(&id);
                }
                return None;
            }
            Event::ContentAdded { content, kind } => {
                let what = kind.trim_start_matches("Content.").to_string();
                put(&mut self.kinds, content, short(kind), MAX_CONTENTS);
                format!("content {content} added: {what}")
            }
            Event::ContentRemoved(content) => {
                let line = format!("content {} removed", self.name(content));
                self.kinds.remove(&content);
                self.titles.remove(&content);
                self.choices.remove(&content);
                line
            }
            Event::Title { content, title } => {
                put(
                    &mut self.titles,
                    content,
                    short(title.clone()),
                    MAX_CONTENTS,
                );
                format!("content {content} is \"{title}\"")
            }
            Event::ActiveContent(0) => "nothing is presented".into(),
            Event::ActiveContent(c) => format!("presented: {}", self.name(c)),
            Event::ActivePresenter(0) => return None,
            Event::ActivePresenter(u) => format!("presenter: {}", self.user(u)),
            Event::Owner { .. } | Event::Presented { .. } | Event::Ready | Event::UrlBase(_) => {
                return None
            }
            Event::FileInfo {
                content,
                name,
                size,
            } => format!("content {}: file {name} ({size} bytes)", self.name(content)),
            Event::PollQuestion {
                content,
                question,
                choices,
            } => {
                // The server fills all seven places; empty ones are no choice.
                let list: Vec<String> = choices
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| !c.is_empty())
                    .map(|(i, c)| format!("{i}) {c}"))
                    .collect();
                let kept: Vec<String> = choices.into_iter().take(MAX_CHOICES).map(short).collect();
                put(&mut self.choices, content, kept, MAX_CONTENTS);
                format!(
                    "poll {content}: {question} – {} – /vote {content} N",
                    list.join("  ")
                )
            }
            Event::PollResults { content, results } => {
                let names = self.choices.get(&content);
                let list: Vec<String> = results
                    .iter()
                    .enumerate()
                    .filter_map(|(i, n)| {
                        let label = names.and_then(|c| c.get(i)).cloned();
                        match label {
                            Some(l) if l.is_empty() => None,
                            Some(l) => Some(format!("{l}: {n}")),
                            None => Some(format!("{i}: {n}")),
                        }
                    })
                    .collect();
                format!("poll {content} results: {}", list.join(", "))
            }
            Event::PollOpen { content, open } => {
                format!("poll {content} is {}", if open { "open" } else { "closed" })
            }
            Event::PollResultsVisible { .. } => return None,
            Event::PollLocalVote { content, choice } if choice >= 0 => {
                format!("poll {content}: your vote is {choice}")
            }
            Event::PollLocalVote { .. } => return None,
            Event::Annotations { content, count } => {
                format!("{}: {count} annotations", self.name(content))
            }
            Event::Slides { content, count } => {
                format!("{}: {count} slides", self.name(content))
            }
            Event::SlideLocation { content, location } => {
                format!("{}: at {}", self.name(content), slide_of(&location))
            }
            Event::NotesTitles {
                content,
                notebook,
                section,
                page,
            } => format!(
                "{}: notes {notebook} / {section} / {page}",
                self.name(content)
            ),
            Event::Closed(reason) if reason.is_empty() => "the server closed the connection".into(),
            Event::Closed(reason) => format!("the server closed the connection ({reason})"),
        })
    }
}

/// The slide number of a PptContent location (`<PptLocation><SlideNumber>…`, namespace
/// `http://schemas.microsoft.com/2008/08/datamcu-content`, possibly with a prefix;
/// [MS-PSOM] §3.1.4.1.11, p. 41), or the text as it came, cut short.
fn slide_of(location: &str) -> String {
    let number = location
        .split_once("SlideNumber>")
        .and_then(|(_, rest)| rest.split_once('<'))
        .map(|(n, _)| n.trim())
        .filter(|n| n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty());
    match number {
        Some(n) => format!("slide {n}"),
        None => location.chars().take(60).collect(),
    }
}

fn now_ms(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn run(
    conversation: u32,
    connection: &DataConnection,
    tls: &TlsClient,
    rx: &Receiver<ToContent>,
    out: &Sender<FromContent>,
) -> Result<(), String> {
    let start = Instant::now();
    let mut stream = tls
        .connect_stream(
            &connection.host,
            connection.port,
            &connection.certificate_name,
            Duration::from_secs(10),
        )
        .map_err(|e| format!("connection failed: {e:?}"))?;
    stream
        .set_timeout(Duration::from_millis(200))
        .map_err(|e| format!("{e:?}"))?;
    let (mut client, first) =
        Client::new(&connection.auth_id, now_ms(start)).map_err(|e| format!("{e:?}"))?;
    stream.send(&first).map_err(|e| format!("{e:?}"))?;
    let mut shown = Shown::default();
    let mut buf = vec![0u8; 16 * 1024];
    let mut waited_ms = 0u64;
    loop {
        match rx.try_recv() {
            Ok(ToContent::Vote { content, choice }) => {
                if let Err(e) = client.vote(content, choice) {
                    let _ = out.send(FromContent::Line(
                        conversation,
                        format!("no vote possible ({e:?})"),
                    ));
                }
            }
            Ok(ToContent::Stop) | Err(TryRecvError::Disconnected) => {
                let _ = client.close();
                let _ = stream.send(&client.take_output());
                return Ok(());
            }
            Err(TryRecvError::Empty) => {}
        }
        // Backpressure (issue #253): no more reading while the session loop is behind.
        if !crate::backlog::CONTENT.room_for(MAX_EVENTS_PER_READ) {
            if waited_ms >= crate::backlog::CONTENT_WAIT_MS {
                return Err("meeting content events were not taken".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
            waited_ms += 20;
            continue;
        }
        waited_ms = 0;
        let n = stream.recv(&mut buf).map_err(|e| format!("{e:?}"))?;
        let now = now_ms(start);
        if n == Some(0) {
            return Err("the server ended the connection".into());
        }
        if let Some(n) = n {
            let data = buf.get(..n).unwrap_or(&[]);
            let events = client
                .on_bytes(data, now)
                .map_err(|e| format!("protocol error ({e:?})"))?;
            let closed = events.iter().any(|e| matches!(e, Event::Closed(_)));
            for e in events {
                // Counted (issue #253); places are checked before reading, so this fits.
                send_counted(out, FromContent::Event(conversation, Box::new(e.clone())));
                if let Some(line) = shown.line(e) {
                    send_counted(out, FromContent::Line(conversation, line));
                }
            }
            if closed {
                return Ok(());
            }
        }
        client.tick(now).map_err(|e| format!("{e:?}"))?;
        let pending = client.take_output();
        if !pending.is_empty() {
            stream.send(&pending).map_err(|e| format!("{e:?}"))?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_number_from_the_location() {
        assert_eq!(
            slide_of("<PptLocation xmlns=\"http://schemas.microsoft.com/2008/08/datamcu-content\"><SlideNumber>3</SlideNumber><Click>0</Click></PptLocation>"),
            "slide 3"
        );
        assert_eq!(
            slide_of("<t:PptLocation><t:SlideNumber>12</t:SlideNumber></t:PptLocation>"),
            "slide 12"
        );
        assert_eq!(slide_of("odd"), "odd");
    }

    #[test]
    fn poll_lines_name_the_choices() {
        let mut s = Shown::default();
        s.line(Event::Title {
            content: 7,
            title: "Lunch".into(),
        });
        let q = s
            .line(Event::PollQuestion {
                content: 7,
                question: "Pizza or pasta?".into(),
                choices: vec!["Pizza".into(), "Pasta".into()],
            })
            .unwrap();
        assert_eq!(
            q,
            "poll 7: Pizza or pasta? – 0) Pizza  1) Pasta – /vote 7 N"
        );
        let r = s
            .line(Event::PollResults {
                content: 7,
                results: vec![2, 1],
            })
            .unwrap();
        assert_eq!(r, "poll 7 results: Pizza: 2, Pasta: 1");
        assert_eq!(
            s.line(Event::Annotations {
                content: 7,
                count: 4
            })
            .unwrap(),
            "7 \"Lunch\": 4 annotations"
        );
    }
}

#[cfg(test)]
mod bound_tests {
    use super::*;

    /// Issue #254: ever new IDs do not grow the state past its bounds; known IDs still update.
    #[test]
    fn content_state_is_bounded() {
        let mut m = BTreeMap::new();
        for i in 0..2000 {
            put(&mut m, i, i, MAX_USERS);
        }
        assert_eq!(m.len(), MAX_USERS);
        put(&mut m, 5, -5, MAX_USERS);
        assert_eq!(m.get(&5), Some(&-5));
        assert_eq!(short("x".repeat(5000)).chars().count(), MAX_TEXT);
    }
}
