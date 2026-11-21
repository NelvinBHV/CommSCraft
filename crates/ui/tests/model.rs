//! The state reducer: event replay, delivery semantics, bounds and clean-up.

use commscraft_ui::model::*;

fn online() -> AppState {
    let mut s = AppState::default();
    s.apply(AppEvent::Connection(Connection::Connected));
    s
}

#[test]
fn invisible_and_control_characters_are_removed() {
    assert_eq!(clean("a\u{202E}b\u{200B}c\u{0007}", 100, false), "abc");
    assert_eq!(clean("x\ny", 100, true), "x\ny");
    assert_eq!(clean("x\ny", 100, false), "xy");
    assert_eq!(clean("abcdef", 3, false), "abc…");
}

#[test]
fn an_opening_conversation_gets_its_session_id() {
    let mut s = online();
    let key = s.new_key();
    s.apply(AppEvent::ConversationOpening {
        key,
        peer: "sip:tim@beispiel.invalid".into(),
    });
    s.apply(AppEvent::OwnText {
        key,
        text: "hallo".into(),
        at: 10,
    });
    s.apply(AppEvent::ConversationStarted {
        session: 4,
        peer: "sip:tim@beispiel.invalid".into(),
        incoming: false,
    });
    assert_eq!(s.conversations.len(), 1);
    assert_eq!(s.key_of_session(4), Some(key));
    assert_eq!(s.conversation(key).unwrap().state, ConvState::Open);
    // Replayed: no second conversation.
    s.apply(AppEvent::ConversationStarted {
        session: 4,
        peer: "sip:tim@beispiel.invalid".into(),
        incoming: false,
    });
    assert_eq!(s.conversations.len(), 1);
}

#[test]
fn delivered_marks_the_oldest_sending_message_and_never_means_read() {
    let mut s = online();
    s.apply(AppEvent::ConversationStarted {
        session: 1,
        peer: "sip:a@beispiel.invalid".into(),
        incoming: true,
    });
    let key = s.key_of_session(1).unwrap();
    for t in ["eins", "zwei"] {
        s.apply(AppEvent::OwnText {
            key,
            text: t.into(),
            at: 1,
        });
    }
    s.apply(AppEvent::Delivered { session: 1 });
    let m: Vec<Delivery> = s
        .conversation(key)
        .unwrap()
        .messages
        .iter()
        .map(|m| m.delivery.clone())
        .collect();
    assert_eq!(m, vec![Delivery::Delivered, Delivery::Sending]);
    s.apply(AppEvent::NotDelivered {
        session: 1,
        status: 480,
    });
    assert_eq!(
        s.conversation(key)
            .unwrap()
            .messages
            .back()
            .unwrap()
            .delivery,
        Delivery::Failed(480)
    );
}

#[test]
fn imdn_failures_name_the_recipients() {
    let mut s = online();
    s.apply(AppEvent::GroupStarted {
        session: 2,
        title: "Team".into(),
    });
    let key = s.key_of_session(2).unwrap();
    s.apply(AppEvent::OwnText {
        key,
        text: "hi".into(),
        at: 1,
    });
    s.apply(AppEvent::Delivered { session: 2 });
    s.apply(AppEvent::NotDeliveredTo {
        session: 2,
        recipient: "sip:b@beispiel.invalid".into(),
        status: 404,
    });
    assert_eq!(
        s.conversation(key)
            .unwrap()
            .messages
            .back()
            .unwrap()
            .delivery,
        Delivery::PartlyFailed(vec![("sip:b@beispiel.invalid".into(), 404)])
    );
}

#[test]
fn messages_are_bounded() {
    let mut s = online();
    s.apply(AppEvent::ConversationStarted {
        session: 3,
        peer: "sip:c@beispiel.invalid".into(),
        incoming: true,
    });
    for i in 0..(MAX_MESSAGES + 20) {
        s.apply(AppEvent::Text {
            session: 3,
            sender: None,
            text: "x".repeat(MAX_TEXT + 5),
            runs: Vec::new(),
            at: i as u64,
        });
    }
    let c = s.conversation(s.key_of_session(3).unwrap()).unwrap();
    assert_eq!(c.messages.len(), MAX_MESSAGES);
    assert_eq!(c.messages.front().unwrap().at, 20);
    assert!(c.messages.back().unwrap().text.chars().count() <= MAX_TEXT + 1);
    assert_eq!(c.unread, MAX_MESSAGES + 20);
}

#[test]
fn losing_the_connection_ends_calls_and_conversations() {
    let mut s = online();
    s.apply(AppEvent::Contacts(vec![{
        let mut c = Contact::new("sip:d@beispiel.invalid", "D", Source::ContactList);
        c.presence = Presence::Available;
        c
    }]));
    s.apply(AppEvent::ConversationStarted {
        session: 5,
        peer: "sip:d@beispiel.invalid".into(),
        incoming: true,
    });
    s.apply(AppEvent::CallStarted {
        id: 9,
        peer: "sip:d@beispiel.invalid".into(),
        incoming: true,
        at: 100,
    });
    s.apply(AppEvent::Connection(Connection::Disconnected("TLS".into())));
    assert!(s.active_call().is_none());
    assert_eq!(
        s.call_history.front().unwrap().direction,
        CallDirection::Missed
    );
    assert_eq!(s.conversations[0].state, ConvState::Ended);
    assert_eq!(
        s.contacts["sip:d@beispiel.invalid"].presence,
        Presence::Unknown
    );
}

#[test]
fn call_history_records_the_duration() {
    let mut s = online();
    s.apply(AppEvent::CallStarted {
        id: 1,
        peer: "sip:e@beispiel.invalid".into(),
        incoming: false,
        at: 100,
    });
    s.apply(AppEvent::CallRinging { id: 1 });
    assert_eq!(s.active_call().unwrap().state, CallState::Ringing);
    s.apply(AppEvent::CallConnected { id: 1, at: 110 });
    s.apply(AppEvent::CallEnded {
        id: 1,
        status: 0,
        at: 170,
    });
    let r = s.call_history.front().unwrap();
    assert_eq!((r.direction, r.seconds), (CallDirection::Outgoing, 60));
    // Late events for the ended call change nothing.
    s.apply(AppEvent::CallConnected { id: 1, at: 200 });
    assert!(s.active_call().is_none());
}

#[test]
fn contact_list_updates_keep_known_presence() {
    let mut s = online();
    let c = Contact::new("sip:f@beispiel.invalid", "F", Source::ContactList);
    s.apply(AppEvent::Contacts(vec![c.clone()]));
    s.apply(AppEvent::Presence {
        uri: "sip:f@beispiel.invalid".into(),
        presence: Some(Presence::Busy),
        note: "Termin".into(),
        name: Some("Frieda".into()),
    });
    s.apply(AppEvent::Contacts(vec![c]));
    assert_eq!(
        s.contacts["sip:f@beispiel.invalid"].presence,
        Presence::Busy
    );
}

#[test]
fn room_messages_are_counted_once_and_our_echo_confirms_delivery() {
    let mut s = online();
    s.apply(AppEvent::SignedIn(Profile {
        uri: "sip:mara@beispiel.invalid".into(),
        ..Profile::default()
    }));
    let uri = "ma-chan://beispiel.invalid/0d3f-1";
    s.apply(AppEvent::Room {
        uri: uri.into(),
        name: "Raum".into(),
        topic: String::new(),
        participants: Vec::new(),
        joined: false,
    });
    let key = s.room_key(uri).unwrap();
    assert_eq!(s.conversation(key).unwrap().state, ConvState::Opening);
    assert!(s.conversation(key).unwrap().is_room());
    assert!(!s.conversation(key).unwrap().is_group());
    s.apply(AppEvent::Room {
        uri: uri.into(),
        name: "Raum".into(),
        topic: "Thema".into(),
        participants: vec!["sip:tim@beispiel.invalid".into()],
        joined: true,
    });
    assert_eq!(s.conversations.len(), 1);
    let c = s.conversation(key).unwrap();
    assert_eq!(c.state, ConvState::Open);
    assert_eq!(c.participants.len(), 1);
    assert_eq!(c.room.as_ref().unwrap().topic, "Thema");
    let post = |id, text: &str, author: &str, ours| RoomPost {
        author: author.into(),
        name: "Tim".into(),
        text: text.into(),
        at: 100,
        chat_id: id,
        ours,
    };
    s.apply(AppEvent::RoomPosts {
        room: uri.into(),
        posts: vec![
            post(1, "alt", "sip:tim@beispiel.invalid", false),
            post(2, "eigen", "sip:mara@beispiel.invalid", false),
        ],
        history: true,
    });
    // The history again (after a new join): nothing twice; history is not unread.
    s.apply(AppEvent::RoomPosts {
        room: uri.into(),
        posts: vec![post(2, "eigen", "sip:mara@beispiel.invalid", false)],
        history: true,
    });
    let c = s.conversation(key).unwrap();
    assert_eq!(c.messages.len(), 2);
    assert_eq!(c.unread, 0);
    assert_eq!(c.messages[1].sender, None);
    s.apply(AppEvent::OwnText {
        key,
        text: "neu".into(),
        at: 200,
    });
    s.apply(AppEvent::RoomPosts {
        room: uri.into(),
        posts: vec![post(3, "neu", "sip:mara@beispiel.invalid", true)],
        history: false,
    });
    let c = s.conversation(key).unwrap();
    assert_eq!(c.messages.len(), 3);
    assert_eq!(c.messages[2].delivery, Delivery::Delivered);
    s.apply(AppEvent::RoomPosts {
        room: uri.into(),
        posts: vec![post(4, "hi", "sip:tim@beispiel.invalid", false)],
        history: false,
    });
    assert_eq!(s.conversation(key).unwrap().unread, 1);
    // A message not taken stays visible as not delivered.
    s.apply(AppEvent::OwnText {
        key,
        text: "weg".into(),
        at: 300,
    });
    s.apply(AppEvent::RoomNotSent {
        room: uri.into(),
        status: 408,
    });
    assert_eq!(
        s.conversation(key)
            .unwrap()
            .messages
            .back()
            .unwrap()
            .delivery,
        Delivery::Failed(408)
    );
}

#[test]
fn a_room_needs_a_room_uri() {
    let mut s = online();
    for bad in ["sip:x@y", "ma-chan://", "ma-chan://d/", "ma-chan://d/a b"] {
        s.apply(AppEvent::Room {
            uri: bad.into(),
            name: "x".into(),
            topic: String::new(),
            participants: Vec::new(),
            joined: true,
        });
    }
    assert!(s.conversations.is_empty());
}

#[test]
fn only_an_incoming_call_offers_video_and_a_video_failure_is_cleaned() {
    let mut s = online();
    for (id, incoming) in [(1, true), (2, false)] {
        s.apply(AppEvent::CallStarted {
            id,
            peer: "sip:tim@beispiel.invalid".into(),
            incoming,
            at: 1,
        });
        s.apply(AppEvent::CallOffersVideo { id });
    }
    assert!(s.calls[0].offers_video);
    assert!(!s.calls[1].offers_video);
    s.apply(AppEvent::CallKind {
        id: 2,
        kind: CallKind::Video,
    });
    s.apply(AppEvent::CallVideo {
        id: 2,
        state: VideoState::Failed("no camera\u{202E}x".into()),
    });
    assert_eq!(s.calls[1].kind, CallKind::Video);
    assert_eq!(s.calls[1].video, VideoState::Failed("no camerax".into()));
}

#[test]
fn the_same_invitation_is_listed_once() {
    let mut s = online();
    for _ in 0..2 {
        s.apply(AppEvent::Invitation(Invitation {
            from: "sip:erin@beispiel.invalid".into(),
            focus: "sip:erin@beispiel.invalid;gruu;opaque=app:conf:focus:id:X".into(),
            subject: String::new(),
        }));
    }
    assert_eq!(s.invitations.len(), 1);
}

#[test]
fn photos_are_taken_only_in_their_bounds() {
    let mut s = online();
    let uri = "sip:carol@beispiel.invalid";
    s.apply(AppEvent::ContactPhoto {
        uri: uri.into(),
        side: 96,
        rgba: vec![0; 96 * 96 * 4],
    });
    assert_eq!(s.photos.get(uri).map(|p| p.side), Some(96));
    // Wrong length, too small, too large: ignored.
    for (side, len) in [(96, 10), (8, 8 * 8 * 4), (512, 4)] {
        s.apply(AppEvent::ContactPhoto {
            uri: "sip:x@beispiel.invalid".into(),
            side,
            rgba: vec![0; len],
        });
    }
    assert_eq!(s.photos.len(), 1);
}

#[test]
fn distribution_group_members_keep_their_presence() {
    let mut s = online();
    s.apply(AppEvent::DistributionGroups(vec![(
        "Team".into(),
        "team@beispiel.invalid".into(),
    )]));
    let bob = Contact::new("sip:bob@beispiel.invalid", "Bob", Source::Directory);
    s.apply(AppEvent::DistributionMembers {
        email: "team@beispiel.invalid".into(),
        members: Ok(vec![bob]),
        partial: true,
    });
    s.apply(AppEvent::Presence {
        uri: "sip:bob@beispiel.invalid".into(),
        presence: Some(Presence::Busy),
        note: String::new(),
        name: None,
    });
    let g = &s.distribution[0];
    assert!(g.partial);
    assert_eq!(g.members.as_ref().unwrap()[0].presence, Presence::Busy);
    // The list again (a new contact list): the members stay until asked again.
    s.apply(AppEvent::DistributionGroups(vec![(
        "Team".into(),
        "team@beispiel.invalid".into(),
    )]));
    assert!(s.distribution[0].members.is_some());
}

#[test]
fn groups_subscribers_and_known_names() {
    let mut s = online();
    s.apply(AppEvent::Groups(vec!["Team".into(), "Freunde\u{7}".into()]));
    assert_eq!(s.groups[0], "Team");
    assert!(!s.groups[1].contains('\u{7}'));
    let sub = commscraft_ui::model::Subscriber {
        uri: "sip:jana@beispiel.invalid".into(),
        name: "Jana".into(),
    };
    s.apply(AppEvent::NewSubscriber(sub.clone()));
    s.apply(AppEvent::NewSubscriber(sub));
    assert_eq!(s.subscribers.len(), 1);
    // Only a real name counts, never the bare address.
    assert_eq!(s.known_name("sip:jana@beispiel.invalid"), "");
    s.apply(AppEvent::Contacts(vec![Contact::new(
        "sip:bob@beispiel.invalid",
        "Bob",
        Source::ContactList,
    )]));
    assert_eq!(s.known_name("sip:bob@beispiel.invalid"), "Bob");
}

#[test]
fn meeting_details_fill_the_entry_and_a_cancelled_meeting_goes() {
    use commscraft_ui::model::Meeting;
    let mut s = online();
    let m = |id: &str, subject: &str| Meeting {
        subject: subject.into(),
        focus: format!("sip:a@b.c;gruu;opaque=app:conf:focus:id:{id}"),
        conference_id: id.into(),
        expires: None,
        demo: false,
        join_url: None,
    };
    s.apply(AppEvent::Meetings(vec![m("A1", ""), m("B2", "")]));
    s.apply(AppEvent::MeetingDetails(Meeting {
        expires: Some(99),
        join_url: Some("javascript:alert(1)".into()),
        ..m("A1", "Planung")
    }));
    assert_eq!(s.meetings[0].subject, "Planung");
    assert_eq!(s.meetings[0].expires, Some(99));
    // Only an https link is kept.
    assert_eq!(s.meetings[0].join_url, None);
    s.apply(AppEvent::MeetingCancelled("B2".into()));
    assert_eq!(s.meetings.len(), 1);
}

#[test]
fn video_frames_are_checked_and_go_with_their_call() {
    let mut s = online();
    s.apply(AppEvent::CallStarted {
        id: 4,
        peer: "sip:bob@beispiel.invalid".into(),
        incoming: false,
        at: 1,
    });
    // A picture whose data does not fit its size is dropped.
    s.apply(AppEvent::VideoFrame {
        call: 4,
        width: 2,
        height: 2,
        rgba: vec![0; 3],
    });
    assert!(s.video.is_empty());
    s.apply(AppEvent::VideoFrame {
        call: 4,
        width: 2,
        height: 2,
        rgba: vec![0; 16],
    });
    assert_eq!(s.video[&4].seq, 1);
    // No call, no picture; the end of the call drops it.
    s.apply(AppEvent::VideoFrame {
        call: 9,
        width: 2,
        height: 2,
        rgba: vec![0; 16],
    });
    assert!(!s.video.contains_key(&9));
    s.apply(AppEvent::CallEnded {
        id: 4,
        status: 0,
        at: 2,
    });
    assert!(s.video.is_empty());
}

#[test]
fn a_call_writes_its_connected_and_ended_lines_into_the_conversation() {
    use commscraft_ui::model::SYSTEM_SENDER;
    let mut s = online();
    let key = s.new_key();
    s.apply(AppEvent::ConversationOpening {
        key,
        peer: "sip:bob@beispiel.invalid".into(),
    });
    s.apply(AppEvent::CallStarted {
        id: 3,
        peer: "sip:bob@beispiel.invalid".into(),
        incoming: false,
        at: 100,
    });
    s.apply(AppEvent::CallConnected { id: 3, at: 110 });
    s.apply(AppEvent::CallEnded {
        id: 3,
        status: 0,
        at: 230,
    });
    let lines: Vec<String> = s
        .conversation(key)
        .unwrap()
        .messages
        .iter()
        .filter(|m| m.sender.as_deref() == Some(SYSTEM_SENDER))
        .map(|m| m.text.clone())
        .collect();
    assert_eq!(
        lines,
        vec!["connected".to_string(), "ended 120".to_string()]
    );
}

#[test]
fn the_self_view_belongs_to_its_call_and_goes_with_it() {
    use commscraft_ui::model::SELF_VIEW;
    let mut s = online();
    s.apply(AppEvent::CallStarted {
        id: 4,
        peer: "sip:bob@beispiel.invalid".into(),
        incoming: false,
        at: 100,
    });
    let frame = |call| AppEvent::VideoFrame {
        call,
        width: 2,
        height: 1,
        rgba: vec![0; 8],
    };
    s.apply(frame(4 | SELF_VIEW));
    assert!(s.video.contains_key(&(4 | SELF_VIEW)));
    // Not for a call that does not exist.
    s.apply(frame(9 | SELF_VIEW));
    assert!(!s.video.contains_key(&(9 | SELF_VIEW)));
    s.apply(AppEvent::CallEnded {
        id: 4,
        status: 0,
        at: 120,
    });
    assert!(!s.video.contains_key(&(4 | SELF_VIEW)));
}

/// Issue #223: a distribution group member's presence arrives even when the directory spells
/// the SIP address with other letter case than the presence document.
#[test]
fn distribution_members_get_presence_regardless_of_letter_case() {
    let mut s = online();
    s.apply(AppEvent::DistributionGroups(vec![(
        "Team".into(),
        "team@beispiel.invalid".into(),
    )]));
    s.apply(AppEvent::DistributionMembers {
        email: "team@beispiel.invalid".into(),
        members: Ok(vec![Contact::new(
            "sip:Max.Muster@beispiel.invalid",
            "Max Muster",
            Source::Directory,
        )]),
        partial: false,
    });
    s.apply(AppEvent::Presence {
        uri: "sip:max.muster@beispiel.invalid".into(),
        presence: Some(Presence::Busy),
        note: String::new(),
        name: None,
    });
    let m = s.distribution[0].members.as_ref().unwrap();
    assert_eq!(m[0].presence, Presence::Busy);
}

/// Issue #224: names sort alphabetically regardless of case and umlauts.
#[test]
fn names_sort_like_an_address_book() {
    use commscraft_ui::format::sort_key;
    let mut names = vec!["zora", "Ärger", "bob", "Anna", "Øyvind", "ölmann", "Bea"];
    names.sort_by_key(|n| sort_key(n));
    assert_eq!(
        names,
        ["Anna", "Ärger", "Bea", "bob", "ölmann", "Øyvind", "zora"]
    );
    assert_eq!(sort_key("Straße"), "strasse");
}

/// A formatted message keeps its runs (bounded and cleaned like plain text).
#[test]
fn formatted_messages_keep_their_runs() {
    use commscraft_ui::model::TextRun;
    let mut s = AppState::default();
    s.apply(AppEvent::ConversationStarted {
        session: 5,
        peer: "sip:eva@beispiel.invalid".into(),
        incoming: true,
    });
    s.apply(AppEvent::Text {
        session: 5,
        sender: None,
        text: "Fett normal".into(),
        runs: vec![
            TextRun {
                text: "Fett".into(),
                bold: true,
                ..TextRun::default()
            },
            TextRun {
                text: " nor\u{7}mal".into(),
                color: Some([0xC0, 0, 0]),
                ..TextRun::default()
            },
        ],
        at: 1,
    });
    let c = s.conversation(s.key_of_session(5).unwrap()).unwrap();
    let m = c.messages.back().unwrap();
    assert_eq!(m.runs.len(), 2);
    assert!(m.runs[0].bold);
    assert_eq!(m.runs[1].text, " normal", "control characters removed");
}
