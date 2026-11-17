//! Interface tests through the accessibility tree (egui_kittest): no network, no GPU.

// Test helpers may fail loudly.
#![allow(clippy::expect_used)]

use commscraft_ui::app::Win;
use commscraft_ui::app::{CommsUi, UiIntent};
use commscraft_ui::demo;
use commscraft_ui::model::{AppEvent, CallKind, Capabilities, Connection, Delivery, Gate};
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
    assert!(h.query_by_label("Kontakte").is_none());
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

const BIG: (f32, f32) = (1400.0, 900.0);

/// The demo without its sample call and unread messages: no window covers the main window
/// (embedded windows lie on top of it in tests).
fn demo_quiet() -> CommsUi {
    let mut ui = demo_ui(Lang::De);
    ui.state.calls.clear();
    for c in &mut ui.state.conversations {
        c.unread = 0;
    }
    ui
}

/// Opens a conversation of the conversations tab by its title.
fn open_conversation(h: &mut Harness<'static, State>, title: &str) {
    button(h, "Unterhaltungen").click();
    h.run();
    h.get_by_label_contains(&format!("{title},")).click();
    h.run();
    h.run();
}

fn online() -> CommsUi {
    let mut ui = CommsUi::new(Lang::De, 0);
    ui.state.apply(AppEvent::Connection(Connection::Connected));
    let mut caps = Capabilities::demo();
    caps.audio_call = Gate::Available;
    caps.video_call = Gate::Available;
    caps.transfer = Gate::Available;
    caps.hold = Gate::Available;
    ui.state.apply(AppEvent::Capabilities(caps));
    ui
}

#[test]
fn the_main_window_has_the_four_tabs_and_marks_the_demo() {
    let mut h = harness(demo_quiet(), (420.0, 760.0));
    h.run();
    for tab in [
        "Kontakte",
        "Unterhaltungen",
        "Telefon",
        "Besprechungen",
        "Optionen",
    ] {
        assert!(h.query_by_label(tab).is_some(), "{tab}");
    }
    assert!(h.query_by_label_contains("DEMO").is_some());
    assert!(h
        .query_by_label_contains("Anna Weber, Beschäftigt")
        .is_some());
    // No alarm or situation views (out of scope).
    assert!(h.query_by_label_contains("Alarm").is_none());
    // No conversation window opens by itself.
    assert!(!h
        .state()
        .ui
        .windows()
        .iter()
        .any(|w| matches!(w, Win::Conv(_))));
}

#[test]
fn a_conversation_opens_in_its_own_window_and_enter_sends() {
    let mut h = harness(demo_quiet(), BIG);
    h.run();
    open_conversation(&mut h, "Projekt Nord");
    let key = h.state().ui.selected_conversation().unwrap();
    assert!(h.state().ui.windows().contains(&Win::Conv(key)));
    let composer = h.get_by_label("composer");
    composer.focus();
    composer.type_text("Hallo zusammen");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
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
    assert_eq!(last.delivery, Delivery::Unknown);
    // Not built: disabled, with the reason.
    for label in ["+ Datei", "Formatierung", "Im Chat suchen"] {
        assert!(
            h.get_by_label(label).accesskit_node().is_disabled(),
            "{label}"
        );
    }
}

#[test]
fn ringing_is_not_connected_and_the_call_has_its_window() {
    let mut ui = online();
    ui.state.apply(AppEvent::CallStarted {
        id: 7,
        peer: "sip:tim@beispiel.invalid".into(),
        incoming: false,
        at: NOW,
    });
    ui.state.apply(AppEvent::CallRinging { id: 7 });
    let mut h = harness(ui, BIG);
    h.run();
    h.run();
    assert!(h.state().ui.windows().contains(&Win::Call(7)));
    assert!(h.query_all_by_label_contains("Es klingelt").count() > 0);
    assert_eq!(h.query_all_by_label_contains("verbunden").count(), 0);
    h.state_mut()
        .ui
        .state
        .apply(AppEvent::CallConnected { id: 7, at: NOW });
    h.run();
    assert!(h.query_all_by_label_contains("Audio verbunden").count() > 0);
    // Ended: the window goes.
    h.state_mut().ui.state.apply(AppEvent::CallEnded {
        id: 7,
        status: 0,
        at: NOW,
    });
    h.run();
    assert!(!h.state().ui.windows().contains(&Win::Call(7)));
}

#[test]
fn a_connected_call_can_be_transferred_from_its_window() {
    let mut ui = online();
    ui.state.apply(AppEvent::CallStarted {
        id: 3,
        peer: "sip:tim@beispiel.invalid".into(),
        incoming: false,
        at: NOW,
    });
    ui.state.apply(AppEvent::CallConnected { id: 3, at: NOW });
    let mut h = harness(ui, BIG);
    h.run();
    h.run();
    button(&h, "Übertragen").click();
    h.run();
    assert!(h.query_by_label_contains("Anruf übertragen").is_some());
}

#[test]
fn the_search_finds_contacts() {
    let mut h = harness(demo_quiet(), (420.0, 760.0));
    h.run();
    let f = field(&h, "search");
    f.focus();
    f.type_text("anna");
    h.run();
    assert!(h.query_all_by_label_contains("Anna Weber").count() > 0);
    assert_eq!(h.query_all_by_label_contains("Lena Vogt").count(), 0);
}

#[test]
fn english_texts_render() {
    let mut h = harness(demo_ui(Lang::En), (420.0, 760.0));
    h.run();
    assert!(h.query_by_label("Contacts").is_some());
    assert!(h.query_by_label_contains("DEMO").is_some());
}

#[test]
fn empty_and_offline_states_have_words() {
    let mut ui = CommsUi::new(Lang::De, 0);
    ui.state.apply(AppEvent::Connection(Connection::Connected));
    let mut h = harness(ui, (420.0, 760.0));
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
    button(&h, "Neu anmelden").click();
    h.run();
    assert!(h.state().intents.contains(&UiIntent::SignOut));
}

#[test]
fn double_scale_still_lays_out() {
    let mut ui = demo_ui(Lang::De);
    ui.prefs.scale = 2.0;
    let mut h = harness(ui, (840.0, 1500.0));
    h.run();
    assert!(h.query_by_label("Kontakte").is_some());
}

#[test]
fn the_status_menu_sets_the_presence() {
    let mut h = harness(demo_quiet(), (420.0, 760.0));
    h.run();
    button(&h, "Verfügbar").click();
    h.run();
    h.query_all_by_label("Beschäftigt")
        .next()
        .expect("the menu item")
        .click();
    h.run();
    assert!(h
        .state()
        .intents
        .contains(&UiIntent::SetPresence(commscraft_ui::model::Presence::Busy)));
}

#[test]
fn the_options_keep_changes_only_with_ok() {
    let mut h = harness(demo_ui(Lang::De), BIG);
    h.run();
    button(&h, "Optionen").click();
    h.run();
    h.run();
    assert!(h.state().ui.windows().contains(&Win::Options));
    h.get_by_label("Kontaktliste").click();
    h.run();
    h.get_by_label("Nur Name (einzeilig)").click();
    h.run();
    button(&h, "Abbrechen").click();
    h.run();
    assert!(!h.state().ui.prefs.compact_contacts);
    button(&h, "Optionen").click();
    h.run();
    h.run();
    h.get_by_label("Kontaktliste").click();
    h.run();
    h.get_by_label("Nur Name (einzeilig)").click();
    h.run();
    button(&h, "OK").click();
    h.run();
    assert!(h.state().ui.prefs.compact_contacts);
    assert!(h.state().intents.contains(&UiIntent::PrefsChanged));
    assert!(!h.state().ui.windows().contains(&Win::Options));
}

#[test]
fn a_room_has_its_tab_and_its_messages_go_to_the_room() {
    let mut h = harness(demo_quiet(), BIG);
    h.run();
    button(&h, "Räume").click();
    h.run();
    assert!(h.query_by_label("Chatrooms laden").is_some());
    h.get_by_label_contains("Wissensaustausch,").click();
    h.run();
    h.run();
    let composer = h.get_by_label("composer");
    composer.focus();
    composer.type_text("Danke");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h
        .state()
        .intents
        .iter()
        .any(|i| matches!(i, UiIntent::SendToRoom { text, .. } if text == "Danke")));
    assert!(!h
        .state()
        .intents
        .iter()
        .any(|i| matches!(i, UiIntent::Typing { .. })));
}

#[test]
fn an_incoming_video_call_is_offered_with_video() {
    let mut ui = online();
    ui.state.apply(AppEvent::CallStarted {
        id: 3,
        peer: "sip:tim@beispiel.invalid".into(),
        incoming: true,
        at: NOW,
    });
    ui.state.apply(AppEvent::CallOffersVideo { id: 3 });
    let mut h = harness(ui, BIG);
    h.run();
    h.run();
    assert!(h.query_by_label_contains("ruft mit Video an").is_some());
    button(&h, "Mit Video annehmen").click();
    h.run();
    assert!(h.state().intents.contains(&UiIntent::AnswerVideo { id: 3 }));
}

#[test]
fn a_shared_screen_is_offered_for_viewing() {
    let mut ui = online();
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
    let mut h = harness(ui, BIG);
    h.run();
    h.run();
    assert!(h
        .query_by_label_contains("teilt einen Bildschirm")
        .is_some());
    assert!(h.query_by_label("Annehmen").is_none());
    button(&h, "Ansehen").click();
    h.run();
    assert!(h.state().intents.contains(&UiIntent::ViewScreen { id: 4 }));
}

#[test]
fn meeting_content_shows_polls_with_voting() {
    let mut h = harness(demo_quiet(), BIG);
    h.run();
    open_conversation(&mut h, "Projekt Nord");
    button(&h, "Präsentieren").click();
    h.run();
    button(&h, "Inhalte anzeigen").click();
    h.run();
    h.run();
    assert!(h.query_by_label_contains("Welcher Termin passt?").is_some());
    assert!(h
        .query_by_label_contains("Präsentiert von: Anna Weber")
        .is_some());
    assert!(h.query_by_label_contains("4 Anmerkungen").is_some());
    button(&h, "Mittwoch · 1").click();
    h.run();
    let key = h.state().ui.selected_conversation().unwrap();
    assert!(h.state().intents.contains(&UiIntent::Vote {
        key,
        content: 2,
        choice: 1
    }));
}

#[test]
fn sharing_our_own_screen_is_disabled_with_a_reason() {
    let mut h = harness(demo_quiet(), BIG);
    h.run();
    open_conversation(&mut h, "Projekt Nord");
    button(&h, "Präsentieren").click();
    h.run();
    assert!(h
        .get_by_label("Bildschirm freigeben")
        .accesskit_node()
        .is_disabled());
}
