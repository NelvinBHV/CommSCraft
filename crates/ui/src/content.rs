//! The content of a meeting as the web conferencing server reports it (function 44): whiteboards,
//! slides, polls, notes and files, with what is presented and by whom. The adapter turns the
//! PSOM client's events into [`ContentChange`]s; this module keeps the state, cleaned and bounded.

use crate::model::{clean, label, MAX_TEXT};

/// Content items kept per meeting.
pub const MAX_CONTENT_ITEMS: usize = 200;
/// Choices of a poll kept (the server fills seven places).
pub const MAX_CHOICES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MeetingContent {
    pub state: ContentState,
    pub items: Vec<ContentItem>,
    /// The content shown to everybody (0: none).
    pub presented: i64,
    pub presenter: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ContentState {
    #[default]
    Connecting,
    Connected,
    Ended(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    Whiteboard,
    Slides,
    Poll,
    Notes,
    File,
    Other,
}

impl ContentKind {
    /// From the server's content type (`Content.Whiteboard`, `Content.Poll`, …).
    pub fn of(kind: &str) -> Self {
        let k = kind.trim_start_matches("Content.").to_ascii_lowercase();
        if k.contains("whiteboard") {
            Self::Whiteboard
        } else if k.contains("ppt") || k.contains("powerpoint") {
            Self::Slides
        } else if k.contains("poll") {
            Self::Poll
        } else if k.contains("onenote") || k.contains("notes") {
            Self::Notes
        } else if k.contains("file") {
            Self::File
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentItem {
    pub id: i64,
    pub kind: ContentKind,
    pub title: String,
    pub poll: Option<Poll>,
    /// Number of slides.
    pub slides: Option<usize>,
    /// Where the presentation is ("slide 3" or the server's words, cut short).
    pub slide_at: String,
    pub annotations: usize,
    /// File name and size in bytes.
    pub file: Option<(String, i64)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Poll {
    pub question: String,
    /// The choices by their index; empty ones are no choice.
    pub choices: Vec<String>,
    pub results: Vec<i64>,
    pub open: bool,
    pub my_vote: Option<i64>,
}

/// What changed in a meeting's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentChange {
    Connected,
    Added {
        id: i64,
        kind: ContentKind,
    },
    Removed(i64),
    Title {
        id: i64,
        title: String,
    },
    Presented(i64),
    Presenter(String),
    File {
        id: i64,
        name: String,
        size: i64,
    },
    PollQuestion {
        id: i64,
        question: String,
        choices: Vec<String>,
    },
    PollResults {
        id: i64,
        results: Vec<i64>,
    },
    PollOpen {
        id: i64,
        open: bool,
    },
    Voted {
        id: i64,
        choice: i64,
    },
    Annotations {
        id: i64,
        count: usize,
    },
    Slides {
        id: i64,
        count: usize,
    },
    SlideAt {
        id: i64,
        at: String,
    },
}

impl MeetingContent {
    fn item(&mut self, id: i64) -> Option<&mut ContentItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    fn poll(&mut self, id: i64) -> Option<&mut Poll> {
        self.item(id)
            .map(|i| i.poll.get_or_insert_with(Poll::default))
    }

    /// Applies one change; text is cleaned and bounded, changes of unknown contents are ignored.
    pub fn apply(&mut self, c: ContentChange) {
        match c {
            ContentChange::Connected => self.state = ContentState::Connected,
            ContentChange::Added { id, kind } => {
                if self.item(id).is_none() && self.items.len() < MAX_CONTENT_ITEMS {
                    self.items.push(ContentItem {
                        id,
                        kind,
                        title: String::new(),
                        poll: None,
                        slides: None,
                        slide_at: String::new(),
                        annotations: 0,
                        file: None,
                    });
                }
            }
            ContentChange::Removed(id) => {
                self.items.retain(|i| i.id != id);
                if self.presented == id {
                    self.presented = 0;
                }
            }
            ContentChange::Title { id, title } => {
                if let Some(i) = self.item(id) {
                    i.title = label(&title);
                }
            }
            ContentChange::Presented(id) => self.presented = id,
            ContentChange::Presenter(name) => self.presenter = label(&name),
            ContentChange::File { id, name, size } => {
                if let Some(i) = self.item(id) {
                    i.file = Some((label(&name), size.max(0)));
                }
            }
            ContentChange::PollQuestion {
                id,
                question,
                choices,
            } => {
                if let Some(p) = self.poll(id) {
                    p.question = clean(&question, MAX_TEXT, false);
                    p.choices = choices.iter().take(MAX_CHOICES).map(|c| label(c)).collect();
                }
            }
            ContentChange::PollResults { id, results } => {
                if let Some(p) = self.poll(id) {
                    p.results = results.into_iter().take(MAX_CHOICES).collect();
                }
            }
            ContentChange::PollOpen { id, open } => {
                if let Some(p) = self.poll(id) {
                    p.open = open;
                }
            }
            ContentChange::Voted { id, choice } => {
                if let Some(p) = self.poll(id) {
                    p.my_vote = (choice >= 0).then_some(choice);
                }
            }
            ContentChange::Annotations { id, count } => {
                if let Some(i) = self.item(id) {
                    i.annotations = count;
                }
            }
            ContentChange::Slides { id, count } => {
                if let Some(i) = self.item(id) {
                    i.slides = Some(count);
                }
            }
            ContentChange::SlideAt { id, at } => {
                if let Some(i) = self.item(id) {
                    i.slide_at = label(&at);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poll_is_built_from_its_changes_and_bounded() {
        let mut m = MeetingContent::default();
        m.apply(ContentChange::Connected);
        m.apply(ContentChange::Added {
            id: 7,
            kind: ContentKind::of("Content.Poll"),
        });
        // Changes of contents that were never added are ignored.
        m.apply(ContentChange::Title {
            id: 8,
            title: "x".into(),
        });
        m.apply(ContentChange::PollQuestion {
            id: 7,
            question: "Mittag?\u{202E}".into(),
            choices: (0..30).map(|i| format!("c{i}")).collect(),
        });
        m.apply(ContentChange::PollOpen { id: 7, open: true });
        m.apply(ContentChange::Voted { id: 7, choice: 1 });
        m.apply(ContentChange::PollResults {
            id: 7,
            results: vec![0, 1],
        });
        m.apply(ContentChange::Presented(7));
        assert_eq!(m.state, ContentState::Connected);
        assert_eq!(m.items.len(), 1);
        let p = m.items[0].poll.clone().unwrap();
        assert_eq!(p.question, "Mittag?");
        assert_eq!(p.choices.len(), MAX_CHOICES);
        assert!(p.open);
        assert_eq!(p.my_vote, Some(1));
        assert_eq!(p.results, vec![0, 1]);
        m.apply(ContentChange::Removed(7));
        assert!(m.items.is_empty());
        assert_eq!(m.presented, 0);
    }

    #[test]
    fn content_kinds() {
        assert_eq!(
            ContentKind::of("Content.Whiteboard"),
            ContentKind::Whiteboard
        );
        assert_eq!(ContentKind::of("Content.Ppt"), ContentKind::Slides);
        assert_eq!(ContentKind::of("Content.Poll"), ContentKind::Poll);
        assert_eq!(ContentKind::of("Content.NativeFileOnly"), ContentKind::File);
        assert_eq!(ContentKind::of("Content.Something"), ContentKind::Other);
    }
}
