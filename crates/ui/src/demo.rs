//! Demo fixtures and the demo adapter: made-up people at `beispiel.invalid` (a reserved name that
//! never resolves), no network, no files. Every demo conversation, call and meeting is marked.

use crate::app::UiIntent;
use crate::model::{
    AppEvent, AppState, Call, CallDirection, CallRecord, CallState, Capabilities, Connection,
    Contact, ConvKey, ConvState, Delivery, Meeting, Message, NoticeKind, Participant, Presence,
    Profile, Source,
};
use crate::text::Lang;

const DOMAIN: &str = "beispiel.invalid";

fn uri(local: &str) -> String {
    format!("sip:{local}@{DOMAIN}")
}

fn person(local: &str, name: &str, title: &str, dept: &str, p: Presence, group: &str) -> Contact {
    let mut c = Contact::new(&uri(local), name, Source::Demo);
    c.title = title.into();
    c.department = dept.into();
    c.company = "Organisation Nord".into();
    c.presence = p;
    c.group = group.into();
    c
}

/// Fills a fresh state with the demo data, relative to `now` (Unix seconds).
pub fn populate(state: &mut AppState, now: u64, lang: Lang) {
    let de = lang == Lang::De;
    let team = if de { "Teamkontakte" } else { "Team contacts" };
    let service = "Service";
    state.apply(AppEvent::Connection(Connection::Demo));
    state.apply(AppEvent::Capabilities(Capabilities::demo()));
    state.apply(AppEvent::SignedIn(Profile {
        uri: uri("mara.klein"),
        name: "Mara Klein".into(),
        presence: Presence::Available,
        organisation: "Organisation Nord".into(),
    }));
    let people = vec![
        person(
            "sarah.weber",
            "Sarah Weber",
            if de {
                "Projektkoordination"
            } else {
                "Project coordination"
            },
            if de { "Projektteam" } else { "Project team" },
            Presence::Available,
            team,
        ),
        person(
            "tim.lange",
            "Tim Lange",
            if de { "Entwickler" } else { "Developer" },
            if de { "Entwicklung" } else { "Development" },
            Presence::InACall,
            team,
        ),
        person(
            "anna.weber",
            "Anna Weber",
            if de { "Projektleitung" } else { "Project lead" },
            if de { "Projektteam" } else { "Project team" },
            Presence::Busy,
            team,
        ),
        person(
            "max.richter",
            "Max Richter",
            "Facility",
            "Facility",
            Presence::Away,
            team,
        ),
        person(
            "it-support",
            "IT-Support",
            service,
            service,
            Presence::Available,
            service,
        ),
        person(
            "lena.vogt",
            "Lena Vogt",
            if de {
                "Personalreferentin"
            } else {
                "HR officer"
            },
            if de { "Personal" } else { "HR" },
            Presence::Offline,
            team,
        ),
    ];
    state.apply(AppEvent::Contacts(people));
    let at = |mins_ago: u64| now.saturating_sub(mins_ago * 60);
    let msg = |sender: Option<&str>, text: &str, at: u64, d: Delivery| Message {
        sender: sender.map(uri),
        text: text.into(),
        at,
        delivery: d,
    };
    let add = |state: &mut AppState,
               title: &str,
               peer: Option<&str>,
               unread: usize,
               msgs: Vec<Message>,
               parts: &[&str]| {
        let key = state.new_key();
        let last = msgs.iter().map(|m| m.at).max().unwrap_or(0);
        state.conversations.push(crate::model::Conversation {
            key,
            session: None,
            title: title.into(),
            peer: peer.map(uri),
            participants: parts
                .iter()
                .map(|p| Participant {
                    uri: uri(p),
                    name: String::new(),
                    presenter: *p == "anna.weber",
                })
                .collect(),
            messages: msgs.into(),
            unread,
            typing: false,
            state: ConvState::Open,
            last_activity: last,
            demo: true,
        });
        key
    };
    let group = add(
        state,
        "Projekt Nord",
        None,
        2,
        vec![
            msg(
                Some("anna.weber"),
                if de {
                    "Guten Morgen! Der überarbeitete Entwurf ist fertig. Könnt ihr bitte die Änderungen im Abschnitt 3 prüfen?"
                } else {
                    "Good morning! The revised draft is ready. Could you check the changes in section 3?"
                },
                at(14),
                Delivery::Unknown,
            ),
            msg(
                Some("tim.lange"),
                if de {
                    "Danke! Ich prüfe den Entwurf bis 14 Uhr und gebe euch Rückmeldung."
                } else {
                    "Thanks! I'll review it by 2 pm and get back to you."
                },
                at(13),
                Delivery::Unknown,
            ),
            msg(
                None,
                if de {
                    "Perfekt. Bitte markiert offene Punkte im Chat, damit nichts verloren geht."
                } else {
                    "Perfect. Please mark open points in the chat so nothing gets lost."
                },
                at(12),
                Delivery::Delivered,
            ),
            msg(
                Some("anna.weber"),
                if de {
                    "Bitte um Feedback bis morgen."
                } else {
                    "Feedback by tomorrow, please."
                },
                at(4),
                Delivery::Unknown,
            ),
        ],
        &[
            "anna.weber",
            "tim.lange",
            "sarah.weber",
            "max.richter",
            "mara.klein",
        ],
    );
    add(
        state,
        "IT-Support",
        Some("it-support"),
        0,
        vec![msg(
            Some("it-support"),
            if de {
                "Update zum Wartungsfenster: Samstag 8–10 Uhr."
            } else {
                "Maintenance window update: Saturday 8–10 am."
            },
            at(60),
            Delivery::Unknown,
        )],
        &[],
    );
    add(
        state,
        "Sarah Weber",
        Some("sarah.weber"),
        0,
        vec![
            msg(
                None,
                if de {
                    "Sehen wir uns gleich im Meeting?"
                } else {
                    "See you in the meeting?"
                },
                at(75),
                Delivery::Delivered,
            ),
            msg(
                Some("sarah.weber"),
                if de {
                    "Bis gleich im Meeting"
                } else {
                    "See you there"
                },
                at(74),
                Delivery::Unknown,
            ),
        ],
        &[],
    );
    add(
        state,
        "Max Richter",
        Some("max.richter"),
        1,
        vec![
            msg(
                None,
                if de {
                    "Kannst du den Raumplan übernehmen?"
                } else {
                    "Could you take over the room plan?"
                },
                at(26 * 60),
                Delivery::Failed(480),
            ),
            msg(
                Some("max.richter"),
                if de {
                    "Ich übernehme das."
                } else {
                    "I'll take care of it."
                },
                at(25 * 60),
                Delivery::Unknown,
            ),
        ],
        &[],
    );
    state.calls.push(Call {
        id: 1,
        peer: uri("tim.lange"),
        incoming: false,
        state: CallState::Connected {
            since: now.saturating_sub(228),
        },
        muted: false,
        held: false,
        started: now.saturating_sub(240),
        conference: None,
        demo: true,
    });
    for (peer, direction, mins, secs, status) in [
        ("anna.weber", CallDirection::Incoming, 140, 312, 0),
        ("it-support", CallDirection::Outgoing, 230, 95, 0),
        ("sarah.weber", CallDirection::Missed, 26 * 60, 0, 487),
    ] {
        state.call_history.push_back(CallRecord {
            peer: uri(peer),
            direction,
            at: at(mins),
            seconds: secs,
            status,
        });
    }
    state.meetings = vec![
        Meeting {
            subject: if de {
                "Wochenabstimmung Projekt Nord".into()
            } else {
                "Weekly sync project north".into()
            },
            focus: format!("{};gruu;opaque=app:conf:focus:id:DEMO1", uri("mara.klein")),
            conference_id: "DEMO1".into(),
            expires: Some(now + 14 * 86_400),
            demo: true,
        },
        Meeting {
            subject: if de {
                "Einarbeitung neue Kolleginnen".into()
            } else {
                "Onboarding new colleagues".into()
            },
            focus: format!("{};gruu;opaque=app:conf:focus:id:DEMO2", uri("mara.klein")),
            conference_id: "DEMO2".into(),
            expires: Some(now + 30 * 86_400),
            demo: true,
        },
    ];
    let _ = group;
}

/// Starts the demo in an interface: data, the group conversation and one contact selected.
pub fn start(ui: &mut crate::app::CommsUi, now: u64) {
    let lang = ui.t.lang();
    let view = ui.view();
    ui.demo = true;
    populate(&mut ui.state, now, lang);
    if let Some(uri) = ui
        .state
        .contacts
        .keys()
        .find(|u| u.contains("sarah"))
        .cloned()
    {
        ui.select_contact(&uri);
    }
    if let Some(k) = first_conversation(&ui.state) {
        ui.select_conversation(k);
    }
    ui.set_view(view);
}

/// The key of the demo group conversation (the first one).
pub fn first_conversation(state: &AppState) -> Option<ConvKey> {
    state.conversations.first().map(|c| c.key)
}

/// The demo's answer to an intent: changes only local state, never claims a real effect.
pub fn handle(state: &mut AppState, intent: &UiIntent, now: u64, lang: Lang) {
    let de = lang == Lang::De;
    let notice = |state: &mut AppState, de_text: &str, en_text: &str| {
        state.apply(AppEvent::Notice(
            NoticeKind::Info,
            if de { de_text } else { en_text }.into(),
            now,
        ));
    };
    match intent {
        UiIntent::SendMessage { key, .. } => {
            // Shown, but not sent: the delivery stays unknown.
            if let Some(c) = state.conversation_mut(*key) {
                if let Some(m) = c.messages.iter_mut().rev().find(|m| m.sender.is_none()) {
                    m.delivery = Delivery::Unknown;
                }
            }
            notice(
                state,
                "Demo: Nachricht nur angezeigt, nicht gesendet",
                "Demo: message shown, not sent",
            );
        }
        UiIntent::StartConversation { key, .. } => {
            if let Some(c) = state.conversation_mut(*key) {
                c.state = ConvState::Open;
                c.demo = true;
            }
        }
        UiIntent::SetPresence(p) => state.apply(AppEvent::OwnPresence(*p)),
        UiIntent::Hold { id, hold } => state.apply(AppEvent::CallHeld {
            id: *id,
            held: *hold,
        }),
        UiIntent::Mute { id, muted } => state.apply(AppEvent::CallMuted {
            id: *id,
            muted: *muted,
        }),
        UiIntent::HangUp { id } => state.apply(AppEvent::CallEnded {
            id: *id,
            status: 0,
            at: now,
        }),
        UiIntent::CallUser { .. } => notice(
            state,
            "Demo: Es wird kein Anruf aufgebaut",
            "Demo: no call is placed",
        ),
        UiIntent::SearchDirectory { .. } => {
            state.apply(AppEvent::SearchResults(Vec::new()));
        }
        UiIntent::JoinMeeting { .. }
        | UiIntent::StartGroup { .. }
        | UiIntent::InviteToGroup { .. } => notice(
            state,
            "Demo: Keine Konferenz – nur Vorschau",
            "Demo: no conference – preview only",
        ),
        UiIntent::EndConversation { key } => {
            if let Some(c) = state.conversation_mut(*key) {
                c.state = ConvState::Ended;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_data_is_synthetic_and_marked() {
        let mut s = AppState::default();
        populate(&mut s, 1_779_000_000, Lang::De);
        assert_eq!(s.connection, Connection::Demo);
        assert!(s.contacts.keys().all(|u| u.ends_with("@beispiel.invalid")));
        assert!(s.conversations.iter().all(|c| c.demo));
        assert!(s.calls.iter().all(|c| c.demo));
        assert!(s.meetings.iter().all(|m| m.demo));
        // Functions the project lacks stay unavailable, also in the demo.
        assert!(!s.capabilities.file_transfer.usable());
        assert!(!s.capabilities.read_receipts.usable());
        assert!(!s.capabilities.history_search.usable());
    }

    #[test]
    fn a_demo_message_is_not_shown_as_delivered() {
        let mut s = AppState::default();
        populate(&mut s, 1_779_000_000, Lang::En);
        let key = first_conversation(&s).unwrap();
        s.apply(AppEvent::OwnText {
            key,
            text: "hi".into(),
            at: 1_779_000_000,
        });
        handle(
            &mut s,
            &UiIntent::SendMessage {
                key,
                text: "hi".into(),
            },
            1_779_000_000,
            Lang::En,
        );
        let last = s
            .conversation(key)
            .unwrap()
            .messages
            .back()
            .unwrap()
            .clone();
        assert_eq!(last.delivery, Delivery::Unknown);
    }
}
