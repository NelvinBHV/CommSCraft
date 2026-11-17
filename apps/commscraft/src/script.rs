//! Test commands (`--commands FILE`): one command per line, appended to a file that the client
//! follows. They trigger exactly the actions a user can take in the interface, nothing more;
//! they exist for scripted tests on the owner's lab and for screenshots.
//!
//! `view chats|contacts|calls|meetings|settings` · `chat sip:ADDRESS` · `select TEXT|group` ·
//! `search TEXT` ·
//! `send TEXT` (to the selected conversation) · `group sip:ADDRESS` · `accept [N]` (the first invitation, or invitation N from 0) ·
//! `call sip:ADDRESS` · `videocall sip:ADDRESS` · `transfer sip:ADDRESS` · `answer` · `answervideo` ·
//! `viewscreen` · `hangup` · `hold` · `resume` · `mute` · `unmute` ·
//! `content` · `vote CONTENT CHOICE` ·
//! `status available|busy|dnd|away` · `shot FILE.png [conv|call|options]` · `quit` (signs out and closes).

use crate::{now, DesktopApp};
use commscraft_ui::app::UiIntent;
use commscraft_ui::app::Win;
use commscraft_ui::model::{AppEvent, CallState, Presence, View};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

/// Longest command line taken.
const MAX_LINE: usize = 4096;

/// Follows the file from its current end (like `tail -f`) in a thread.
pub fn follow(path: PathBuf) -> Receiver<String> {
    let (tx, rx) = mpsc::sync_channel(64);
    std::thread::spawn(move || {
        let Ok(file) = std::fs::File::open(&path) else {
            eprintln!("commands: cannot open {}", path.display());
            return;
        };
        let mut r = BufReader::new(file);
        let _ = r.seek(SeekFrom::End(0));
        let mut line = String::new();
        loop {
            line.clear();
            match r.read_line(&mut line) {
                Ok(0) => std::thread::sleep(Duration::from_millis(200)),
                Ok(_) => {
                    let l: String = line.trim().chars().take(MAX_LINE).collect();
                    if !l.is_empty() && tx.send(l).is_err() {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    });
    rx
}

pub fn run(app: &mut DesktopApp, line: &str) {
    let (cmd, arg) = line.split_once(' ').unwrap_or((line, ""));
    let arg = arg.trim();
    match cmd {
        "view" => {
            let v = match arg {
                "contacts" => View::Contacts,
                "calls" => View::Calls,
                "meetings" => View::Meetings,
                "settings" => View::Settings,
                _ => View::Chats,
            };
            app.ui.set_view(v);
        }
        "chat" => {
            app.ui.chat_with(arg);
            let intents = app.ui.take_intents();
            for i in intents {
                app.handle(i);
            }
        }
        "select" => {
            // The newest conversation whose title contains the text ("group" picks groups).
            let mut list: Vec<_> = app.ui.state.conversations.iter().collect();
            list.sort_by_key(|c| std::cmp::Reverse(c.last_activity));
            let hit = list
                .iter()
                .find(|c| {
                    if arg == "group" {
                        c.is_group()
                    } else {
                        c.title.contains(arg)
                    }
                })
                .map(|c| c.key);
            if let Some(k) = hit {
                app.ui.select_conversation(k);
            }
        }
        "send" => {
            if let Some(key) = app.ui.selected_conversation() {
                app.ui.state.apply(AppEvent::OwnText {
                    key,
                    text: arg.to_string(),
                    at: now(),
                });
                app.handle(UiIntent::SendMessage {
                    key,
                    text: arg.to_string(),
                });
            }
        }
        "search" => app.ui.set_search(arg),
        "group" => app.handle(UiIntent::StartGroup {
            invite: vec![arg.to_string()],
        }),
        // `addcontact sip:ADDRESS [GROUP]` · `removecontact sip:ADDRESS` ·
        // `movecontact sip:ADDRESS GROUP` · `addgroup NAME` · `delgroup NAME`
        "addcontact" | "movecontact" => {
            let (uri, group) = match arg.split_once(' ') {
                Some((u, g)) => (u.to_string(), Some(g.trim().to_string())),
                None => (arg.to_string(), None),
            };
            if cmd == "addcontact" {
                let name = app.ui.state.known_name(&uri);
                app.handle(UiIntent::AddContact { uri, name, group })
            } else if let Some(group) = group {
                app.handle(UiIntent::MoveContact { uri, group })
            }
        }
        // `relation sip:ADDRESS blocked|colleagues|workgroup|family|external|auto`
        "relation" => {
            use commscraft_ui::model::Relation as R;
            let (uri, r) = arg.split_once(' ').unwrap_or((arg, ""));
            let relation = match r.trim() {
                "blocked" => Some(R::Blocked),
                "colleagues" => Some(R::Colleagues),
                "workgroup" => Some(R::Workgroup),
                "family" => Some(R::Family),
                "external" => Some(R::External),
                _ => None,
            };
            app.handle(UiIntent::SetRelation {
                uri: uri.to_string(),
                relation,
            })
        }
        "removecontact" => app.handle(UiIntent::RemoveContact {
            uri: arg.to_string(),
        }),
        "addgroup" => app.handle(UiIntent::AddGroup {
            name: arg.to_string(),
        }),
        "delgroup" => app.handle(UiIntent::DeleteGroup {
            name: arg.to_string(),
        }),
        // `conf promote|demote|remove|mute|unmute sip:ADDRESS`, `conf admit [sip:ADDRESS]`,
        // `conf deny sip:ADDRESS`, `conf lock|unlock|muteall|unmuteall` – in the selected meeting
        "conf" => {
            use commscraft_ui::model::ConfCommand as K;
            let (what, user) = arg.split_once(' ').unwrap_or((arg, ""));
            let user = user.trim().to_string();
            let command = match what {
                "promote" | "demote" => Some(K::Role {
                    user,
                    presenter: what == "promote",
                }),
                "remove" => Some(K::Remove(user)),
                "mute" | "unmute" => Some(K::MuteUser {
                    user,
                    mute: what == "mute",
                }),
                "admit" => Some(K::Admit(if user.is_empty() {
                    Vec::new()
                } else {
                    vec![user]
                })),
                "deny" => Some(K::Deny(user)),
                "lock" | "unlock" => Some(K::Lock(what == "lock")),
                "muteall" | "unmuteall" => Some(K::MuteAudience(what == "muteall")),
                _ => None,
            };
            if let (Some(key), Some(command)) = (app.ui.selected_conversation(), command) {
                app.handle(UiIntent::ConferenceCommand { key, command })
            }
        }
        // `schedule MINUTES SUBJECT` · `cancelmeeting CONFERENCE-ID` · `meetings`
        "schedule" => {
            let (m, subject) = arg.split_once(' ').unwrap_or((arg, "Test"));
            app.handle(UiIntent::ScheduleMeeting {
                subject: subject.to_string(),
                minutes: m.parse().unwrap_or(60),
                access: commscraft_ui::model::MeetingAccess::Organisation,
                join: false,
            })
        }
        "cancelmeeting" => app.handle(UiIntent::CancelMeeting {
            conference_id: arg.trim().to_string(),
        }),
        "meetings" => app.handle(UiIntent::RefreshMeetings),
        // `meetingaudio`: join the audio of the selected meeting
        "meetingaudio" => {
            if let Some(key) = app.ui.selected_conversation() {
                app.handle(UiIntent::JoinMeetingAudio { key })
            }
        }
        // `joinlink https://…`
        "joinlink" => app.handle(UiIntent::JoinMeetingLink {
            link: arg.trim().to_string(),
        }),
        "accept" => app.handle(UiIntent::AcceptInvitation {
            index: arg.parse().unwrap_or(0),
        }),
        // The content of the selected meeting conversation; `vote CONTENT CHOICE` in its polls.
        "content" => {
            if let Some(key) = app.ui.selected_conversation() {
                app.handle(UiIntent::JoinContent { key })
            }
        }
        "vote" => {
            let mut w = arg.split_whitespace().map(|x| x.parse::<i64>().ok());
            if let (Some(key), Some(Some(content)), Some(Some(choice))) =
                (app.ui.selected_conversation(), w.next(), w.next())
            {
                app.handle(UiIntent::Vote {
                    key,
                    content,
                    choice,
                })
            }
        }
        "call" => {
            app.ui.set_view(View::Calls);
            app.handle(UiIntent::CallUser {
                peer: arg.to_string(),
            })
        }
        "videocall" => {
            app.ui.set_view(View::Calls);
            app.handle(UiIntent::VideoCall {
                peer: arg.to_string(),
            })
        }
        "transfer" => {
            if let Some(c) = app.ui.state.active_call().cloned() {
                app.handle(UiIntent::Transfer {
                    id: c.id,
                    peer: arg.to_string(),
                });
            }
        }
        "answer" | "answervideo" | "viewscreen" | "hangup" | "hold" | "resume" | "mute"
        | "unmute" => {
            let Some(c) = app.ui.state.active_call().cloned() else {
                eprintln!("commands: no call");
                return;
            };
            let i = match cmd {
                "answer" if c.state == CallState::Incoming => UiIntent::AnswerCall { id: c.id },
                "answer" => return,
                "answervideo" if c.state == CallState::Incoming => {
                    UiIntent::AnswerVideo { id: c.id }
                }
                "viewscreen" if c.state == CallState::Incoming => UiIntent::ViewScreen { id: c.id },
                "answervideo" | "viewscreen" => return,
                "hangup" => UiIntent::HangUp { id: c.id },
                "hold" => UiIntent::Hold {
                    id: c.id,
                    hold: true,
                },
                "resume" => UiIntent::Hold {
                    id: c.id,
                    hold: false,
                },
                "mute" => UiIntent::Mute {
                    id: c.id,
                    muted: true,
                },
                _ => UiIntent::Mute {
                    id: c.id,
                    muted: false,
                },
            };
            app.handle(i);
        }
        "status" => {
            let p = match arg {
                "busy" => Presence::Busy,
                "dnd" => Presence::DoNotDisturb,
                "away" => Presence::Away,
                "brb" => Presence::BeRightBack,
                "offwork" => Presence::OffWork,
                "reset" => return app.handle(UiIntent::ResetStatus),
                _ => Presence::Available,
            };
            app.handle(UiIntent::SetPresence(p));
        }
        // `idle SECONDS` (a test's idle time instead of the desktop's; `idle` alone: the desktop's)
        "idle" => app.idle_override = arg.trim().parse().ok(),
        // `note TEXT` (empty: remove the note)
        "note" => app.handle(UiIntent::SetNote(arg.trim().to_string())),
        // `shot FILE [conv|call|options]`: the main window, or the first window of that kind.
        "shot" => {
            let mut w = arg.split_whitespace();
            let file = w.next().unwrap_or("shot.png");
            let target = w.next().and_then(|kind| {
                app.ui.windows().iter().copied().find(|x| {
                    matches!(
                        (kind, x),
                        ("conv", Win::Conv(_)) | ("call", Win::Call(_)) | ("options", Win::Options)
                    )
                })
            });
            app.shots.push_back((PathBuf::from(file), false, target))
        }
        "quit" => {
            app.handle(UiIntent::SignOut);
            app.close_at = Some(std::time::Instant::now());
        }
        other => eprintln!("commands: unknown command {other}"),
    }
}
