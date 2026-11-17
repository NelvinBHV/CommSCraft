//! Interface tests through the accessibility tree (egui_kittest): no network, no GPU.

// Test helpers may fail loudly.
#![allow(clippy::expect_used)]

use commscraft_ui::app::{CommsUi, UiIntent};
use commscraft_ui::demo;
use commscraft_ui::model::{AppEvent, CallState, Connection, Delivery, View};
use commscraft_ui::text::Lang;
use egui::{FontDefinitions, FontFamily};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

const NOW: u64 = 1_779_000_000;

struct State {
    ui: CommsUi,
    intents: Vec<UiIntent>,
}

fn fonts(ctx: &egui::Context) {
    // egui's own fonts (tests only) and the bold family the application defines.
    let mut defs = FontDefinitions::default();
    let regular = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    defs.families
        .insert(FontFamily::Name(commscraft_ui::theme::BOLD.into()), regular);
    ctx.set_fonts(defs);
}

fn harness(ui: CommsUi, size: (f32, f32)) -> Harness<'static, State> {
    Harness::builder()
        .with_size(egui::vec2(size.0, size.1))
        .with_context_setup(fonts)
        .build_ui_state(
            |ui, s: &mut State| {
                let out = s.ui.show(ui, NOW);
                if s.ui.demo {
                    for i in &out {
                        demo::handle(&mut s.ui.state, i, NOW, Lang::De);
                    }
                }
                s.intents.extend(out);
            },
            State {
                ui,
                intents: Vec::new(),
            },
        )
}

fn demo_ui(lang: Lang) -> CommsUi {
    let mut ui = CommsUi::new(lang, 3600);
    demo::start(&mut ui, NOW);
    ui
}

/// The button with this label (a heading may carry the same words).
fn button<'a>(h: &'a Harness<'static, State>, label: &'a str) -> egui_kittest::Node<'a> {
    h.query_all_by_label(label)
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .expect("a button")
}

/// The text field with this accessible label (its caption has the same label).
fn field<'a>(h: &'a Harness<'static, State>, label: &'a str) -> egui_kittest::Node<'a> {
    h.query_all_by_label(label)
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::TextInput)
        .expect("a text field")
}

#[test]
fn the_demo_is_marked_and_has_no_alarm_navigation() {
    let mut h = harness(demo_ui(Lang::De), (1440.0, 900.0));
    h.run();
    assert!(h
        .query_by_label_contains("DEMO · Keine Verbindung")
        .is_some());
    for word in ["Alarm", "Lage", "Eskalation", "Quittierung"] {
        assert!(h.query_by_label_contains(word).is_none(), "{word}");
    }
    for view in ["Chats", "Kontakte", "Anrufe", "Meetings", "Einstellungen"] {
        assert!(h.query_by_label(view).is_some(), "{view}");
    }
}

#[test]
fn the_rail_switches_views() {
    let mut h = harness(demo_ui(Lang::De), (1440.0, 900.0));
    h.run();
    h.get_by_label("Kontakte").click();
    h.run();
    assert_eq!(h.state().ui.view(), View::Contacts);
    assert!(h.query_by_label("Kontaktübersicht").is_some());
    h.get_by_label("Anrufe").click();
    h.run();
    assert_eq!(h.state().ui.view(), View::Calls);
    assert!(h.query_by_label("Anruf beenden").is_some());
}

#[test]
fn enter_sends_and_the_demo_does_not_claim_delivery() {
    let mut h = harness(demo_ui(Lang::De), (1440.0, 900.0));
    h.run();
    let composer = h.get_by_label("composer");
    composer.focus();
    composer.type_text("Hallo zusammen");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    let key = h.state().ui.selected_conversation().unwrap();
    assert!(h.state().intents.contains(&UiIntent::SendMessage {
        key,
        text: "Hallo zusammen".into()
    }));
    let last = h
        .state()
        .ui
        .state
        .conversation(key)
        .unwrap()
        .messages
        .back()
        .unwrap()
        .clone();
    assert_eq!(last.text, "Hallo zusammen");
    assert_eq!(last.delivery, Delivery::Unknown);
    assert!(h.query_by_label_contains("Zustellung unbekannt").is_some());
}

#[test]
fn missing_functions_are_disabled_with_a_reason() {
    let mut h = harness(demo_ui(Lang::De), (1440.0, 900.0));
    h.run();
    for label in ["+ Datei", "Formatierung", "Im Chat suchen"] {
        let node = h.get_by_label(label);
        assert!(node.accesskit_node().is_disabled(), "{label}");
    }
}

#[test]
fn a_narrow_window_closes_the_context_panel() {
    let mut h = harness(demo_ui(Lang::De), (1440.0, 900.0));
    h.run();
    assert!(h.query_by_label("Gruppeninformationen").is_some());
    let mut h = harness(demo_ui(Lang::De), (1050.0, 800.0));
    h.run();
    assert!(h.query_by_label("Gruppeninformationen").is_none());
}

#[test]
fn the_sign_in_clears_the_password() {
    let ui = CommsUi::new(Lang::En, 0);
    let mut h = harness(ui, (1280.0, 800.0));
    h.run();
    for (label, text) in [
        ("SIP address", "mara@beispiel.invalid"),
        ("User name", "mara"),
        ("Password", "geheim-123"),
    ] {
        let f = field(&h, label);
        f.focus();
        f.type_text(text);
        h.run();
    }
    h.run();
    button(&h, "Sign in").click();
    h.run();
    let sent = h
        .state()
        .intents
        .iter()
        .find_map(|i| match i {
            UiIntent::SignIn(f) => Some(f.clone()),
            _ => None,
        })
        .expect("a sign-in intent");
    assert_eq!(sent.address, "sip:mara@beispiel.invalid");
    assert_eq!(sent.password, "geheim-123");
    // The debug form never shows the password.
    assert!(!format!("{sent:?}").contains("geheim"));
    // The field is empty again; the address stays for the next attempt.
    assert_eq!(
        field(&h, "Password").accesskit_node().value(),
        Some(String::new())
    );
}

#[test]
fn ringing_is_not_connected() {
    let mut ui = CommsUi::new(Lang::De, 0);
    ui.state.apply(AppEvent::Connection(Connection::Connected));
    ui.state.apply(AppEvent::CallStarted {
        id: 7,
        peer: "sip:tim@beispiel.invalid".into(),
        incoming: false,
        at: NOW,
    });
    ui.state.apply(AppEvent::CallRinging { id: 7 });
    ui.set_view(View::Calls);
    let mut h = harness(ui, (1440.0, 900.0));
    h.run();
    assert!(h.query_all_by_label_contains("Es klingelt").count() > 0);
    assert_eq!(h.query_all_by_label_contains("verbunden").count(), 0);
    h.state_mut()
        .ui
        .state
        .apply(AppEvent::CallConnected { id: 7, at: NOW });
    h.run();
    assert!(matches!(
        h.state().ui.state.active_call().unwrap().state,
        CallState::Connected { .. }
    ));
    assert!(h.query_all_by_label_contains("Audio verbunden").count() > 0);
}

#[test]
fn search_names_its_sources() {
    let mut ui = demo_ui(Lang::De);
    ui.set_view(View::Chats);
    let mut h = harness(ui, (1440.0, 900.0));
    h.run();
    let search = h.get_by_label("search");
    search.focus();
    search.type_text("weber");
    h.run();
    h.run();
    assert!(h
        .query_by_label_contains("Nachrichtentexte werden nicht durchsucht")
        .is_some());
    assert!(h.query_all_by_label_contains("Anna Weber").count() > 0);
    // Asked once for the settled text, not once per key.
    let asked: Vec<&UiIntent> = h
        .state()
        .intents
        .iter()
        .filter(|i| matches!(i, UiIntent::SearchDirectory { .. }))
        .collect();
    assert_eq!(
        asked,
        vec![&UiIntent::SearchDirectory {
            text: "weber".into()
        }]
    );
}

#[test]
fn english_texts_render() {
    let mut h = harness(demo_ui(Lang::En), (1280.0, 800.0));
    h.run();
    assert!(h.query_by_label("Contacts").is_some());
    assert!(h.query_by_label_contains("DEMO · Not connected").is_some());
}

#[test]
fn a_failed_sign_in_says_why() {
    let mut ui = CommsUi::new(Lang::De, 0);
    ui.state.apply(AppEvent::Connection(Connection::Failed(
        "no server reachable".into(),
    )));
    let mut h = harness(ui, (1280.0, 800.0));
    h.run();
    assert!(h
        .query_by_label_contains("Anmeldung fehlgeschlagen: no server reachable")
        .is_some());
    // Still the sign-in form, no conversation view behind it.
    assert!(h.query_by_label("Nachrichten").is_none());
}

#[test]
fn empty_and_offline_states_have_words() {
    let mut ui = CommsUi::new(Lang::De, 0);
    ui.state.apply(AppEvent::Connection(Connection::Connected));
    ui.set_view(View::Contacts);
    let mut h = harness(ui, (1440.0, 900.0));
    h.run();
    assert!(h.query_by_label("Die Kontaktliste ist leer.").is_some());
    h.state_mut()
        .ui
        .state
        .apply(AppEvent::Connection(Connection::Disconnected("TLS".into())));
    h.run();
    assert!(h
        .query_by_label_contains("Verbindung getrennt – TLS")
        .is_some());
    // The way back is offered.
    button(&h, "Neu anmelden").click();
    h.run();
    assert!(h.state().intents.contains(&UiIntent::SignOut));
}

#[test]
fn double_scale_still_lays_out() {
    let mut ui = demo_ui(Lang::De);
    ui.prefs.scale = 2.0;
    let mut h = harness(ui, (1440.0, 900.0));
    h.run();
    assert!(h.query_by_label("Chats").is_some());
}

#[test]
fn a_connected_call_can_be_transferred() {
    use commscraft_ui::model::{Capabilities, Gate};
    let mut ui = CommsUi::new(Lang::De, 0);
    ui.state.apply(AppEvent::Connection(Connection::Connected));
    let mut caps = Capabilities::offline();
    caps.audio_call = Gate::Available;
    caps.transfer = Gate::Available;
    ui.state.apply(AppEvent::Capabilities(caps));
    ui.state.apply(AppEvent::CallStarted {
        id: 3,
        peer: "sip:tim@beispiel.invalid".into(),
        incoming: false,
        at: NOW,
    });
    ui.state.apply(AppEvent::CallConnected { id: 3, at: NOW });
    ui.set_view(View::Calls);
    let mut h = harness(ui, (1440.0, 900.0));
    h.run();
    button(&h, "Übertragen").click();
    h.run();
    assert!(h.query_by_label("Anruf übertragen").is_some());
    // The dialog's field has the focus: type the target and confirm.
    h.query_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().is_focused())
        .expect("the focused address field")
        .type_text("erin@beispiel.invalid");
    h.run();
    let confirm = h
        .query_all_by_label("Übertragen")
        .rfind(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .expect("the confirm button");
    confirm.click();
    h.run();
    assert!(h.state().intents.contains(&UiIntent::Transfer {
        id: 3,
        peer: "sip:erin@beispiel.invalid".into()
    }));
}

#[test]
fn a_foreign_server_needs_an_explicit_yes() {
    use commscraft_ui::model::TrustQuestion;
    let mut ui = CommsUi::new(Lang::De, 0);
    ui.state.apply(AppEvent::Connection(Connection::Connecting));
    ui.state.apply(AppEvent::TrustQuestion(TrustQuestion {
        host: "sip.hoster.invalid".into(),
        domain: "hoster.invalid".into(),
        user_domain: "beispiel.invalid".into(),
        named_by: "lyncdiscover.beispiel.invalid".into(),
    }));
    let mut h = harness(ui, (1280.0, 800.0));
    h.run();
    assert!(h
        .query_by_label_contains("sip.hoster.invalid gehört nicht zu Ihrer SIP-Domäne")
        .is_some());
    button(&h, "Nicht vertrauen").click();
    h.run();
    assert!(h.state().intents.contains(&UiIntent::AnswerTrust(false)));
    assert!(h.state().ui.state.trust_question.is_none());
}

#[test]
fn a_room_is_listed_apart_and_its_messages_go_to_the_room() {
    let mut h = harness(demo_ui(Lang::De), (1440.0, 900.0));
    h.run();
    button(&h, "Räume").click();
    h.run();
    assert!(h.query_by_label("Chatrooms laden").is_some());
    // Only the room is listed under "Räume", not the group or the one-to-one chats.
    // (A list row is labelled "<title>, <unread>"; the open group's header stays.)
    assert!(h.query_by_label_contains("Projekt Nord,").is_none());
    h.get_by_label_contains("Wissensaustausch,").click();
    h.run();
    assert!(h.query_by_label_contains("Chatroom · 3").is_some());
    let composer = h.get_by_label("composer");
    composer.focus();
    composer.type_text("Danke für die Vorlage");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    let room = h
        .state()
        .ui
        .state
        .conversation(h.state().ui.selected_conversation().unwrap())
        .unwrap()
        .room
        .clone()
        .unwrap()
        .uri;
    assert!(h.state().intents.contains(&UiIntent::SendToRoom {
        room,
        text: "Danke für die Vorlage".into()
    }));
    // No typing notification and no "end" for a room.
    assert!(!h
        .state()
        .intents
        .iter()
        .any(|i| matches!(i, UiIntent::Typing { .. })));
    assert!(h.query_by_label_contains("Zustellung unbekannt").is_some());
}

fn online_with_video() -> CommsUi {
    use commscraft_ui::model::{Capabilities, Gate};
    let mut ui = CommsUi::new(Lang::De, 0);
    ui.state.apply(AppEvent::Connection(Connection::Connected));
    let mut caps = Capabilities::demo();
    caps.audio_call = Gate::Available;
    caps.video_call = Gate::Available;
    ui.state.apply(AppEvent::Capabilities(caps));
    ui.set_view(View::Calls);
    ui
}

#[test]
fn a_call_with_video_can_be_answered_with_video() {
    use commscraft_ui::model::VideoState;
    let mut ui = online_with_video();
    ui.state.apply(AppEvent::CallStarted {
        id: 3,
        peer: "sip:tim@beispiel.invalid".into(),
        incoming: true,
        at: NOW,
    });
    ui.state.apply(AppEvent::CallOffersVideo { id: 3 });
    let mut h = harness(ui, (1440.0, 900.0));
    h.run();
    button(&h, "Mit Video annehmen").click();
    h.run();
    assert!(h.state().intents.contains(&UiIntent::AnswerVideo { id: 3 }));
    h.state_mut()
        .ui
        .state
        .apply(AppEvent::CallConnected { id: 3, at: NOW });
    h.state_mut().ui.state.apply(AppEvent::CallVideo {
        id: 3,
        state: VideoState::Window,
    });
    h.run();
    assert!(h
        .query_by_label_contains("eigenen, abgeschotteten Fenster")
        .is_some());
}

#[test]
fn a_shared_screen_is_offered_for_viewing() {
    use commscraft_ui::model::CallKind;
    let mut ui = online_with_video();
    ui.state.apply(AppEvent::CallStarted {
        id: 4,
        peer: "sip:tim@beispiel.invalid".into(),
        incoming: true,
        at: NOW,
    });
    ui.state.apply(AppEvent::CallKind {
        id: 4,
        kind: CallKind::Screen,
    });
    let mut h = harness(ui, (1440.0, 900.0));
    h.run();
    assert!(h
        .query_by_label_contains("teilt einen Bildschirm")
        .is_some());
    // Not "answer": viewing is all there is.
    assert!(h.query_by_label("Annehmen").is_none());
    button(&h, "Ansehen").click();
    h.run();
    assert!(h.state().intents.contains(&UiIntent::ViewScreen { id: 4 }));
}

#[test]
fn meeting_content_shows_polls_with_voting() {
    let mut h = harness(demo_ui(Lang::De), (1440.0, 900.0));
    h.run();
    // The demo group is selected; its details offer the meeting content.
    button(&h, "Inhalte anzeigen").click();
    h.run();
    assert!(h.query_by_label_contains("Welcher Termin passt?").is_some());
    assert!(h
        .query_by_label_contains("Präsentiert von: Anna Weber")
        .is_some());
    assert!(h.query_by_label_contains("4 Anmerkungen").is_some());
    // Empty choices are no choice.
    assert_eq!(h.query_all_by_label_contains(" · 0").count(), 0);
    button(&h, "Mittwoch · 1").click();
    h.run();
    let key = h.state().ui.selected_conversation().unwrap();
    assert!(h.state().intents.contains(&UiIntent::Vote {
        key,
        content: 2,
        choice: 1
    }));
    button(&h, "Inhalte schließen").click();
    h.run();
    assert!(h.query_by_label("Inhalte anzeigen").is_some());
}
