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
