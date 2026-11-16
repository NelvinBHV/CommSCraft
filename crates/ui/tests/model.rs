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
        presence: Presence::Busy,
        note: "Termin".into(),
    });
    s.apply(AppEvent::Contacts(vec![c]));
    assert_eq!(
        s.contacts["sip:f@beispiel.invalid"].presence,
        Presence::Busy
    );
}
