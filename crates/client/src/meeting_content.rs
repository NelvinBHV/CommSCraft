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

/// Lines for the terminal, or the end of the connection.
pub enum FromContent {
    Line(u32, String),
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
                    self.users.insert(id, shown);
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
                self.kinds.insert(content, kind);
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
                self.titles.insert(content, title.clone());
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
                self.choices.insert(content, choices);
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
                if let Some(line) = shown.line(e) {
                    let _ = out.send(FromContent::Line(conversation, line));
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
