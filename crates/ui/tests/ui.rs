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
        ("Sign-in address", "mara@beispiel.invalid"),
        ("User name (optional)", "mara"),
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
    for tab in ["Kontakte", "Unterhaltungen", "Besprechungen", "Optionen"] {
        assert!(h.query_by_label(tab).is_some(), "{tab}");
    }
    // No telephony in this interface: no phone tab, no keypad for numbers.
    assert!(h.query_by_label("Telefon").is_none());
    assert!(h.query_by_label("Telefone").is_none());
    assert!(h.query_by_label_contains("DEMO").is_some());
    // (Anna is in a group and in the demo distribution list.)
    assert!(
        h.query_all_by_label_contains("Anna Weber, Beschäftigt")
            .count()
            > 0
    );
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
    for label in ["+ Datei", "Formatierung"] {
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
    // As in the Windows client: the call is on the stage of its conversation window.
    let key = h
        .state()
        .ui
        .state
        .conversation_with("sip:tim@beispiel.invalid")
        .expect("a conversation for the call");
    assert!(h.state().ui.windows().contains(&Win::Conv(key)));
    assert!(!h.state().ui.windows().contains(&Win::Call(7)));
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
    // Ended: the stage goes, the conversation stays.
    assert!(h.state().ui.windows().contains(&Win::Conv(key)));
    assert_eq!(h.query_all_by_label_contains("Audio verbunden").count(), 0);
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
    button(&h, "Anrufsteuerung").click();
    h.run();
    button(&h, "Übertragen").click();
    h.run();
    h.get_by_label("An eine Person …").click();
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

#[test]
fn the_sign_in_needs_only_the_address_and_the_password() {
    let mut h = harness(CommsUi::new(Lang::En, 0), (420.0, 760.0));
    h.run();
    for (label, text) in [
        ("Sign-in address", "mara@beispiel.invalid"),
        ("Password", "geheim"),
    ] {
        let f = field(&h, label);
        f.focus();
        f.type_text(text);
        h.run();
    }
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
    // The user name stays empty: the adapter signs in with the address itself.
    assert_eq!(sent.user, "");
    assert_eq!(sent.server, "");
}

#[test]
fn the_stored_certificate_signs_in_without_a_password() {
    let mut h = harness(CommsUi::new(Lang::En, 0), (420.0, 760.0));
    h.run();
    let f = field(&h, "Sign-in address");
    f.focus();
    f.type_text("mara@beispiel.invalid");
    h.run();
    // Without a password and without a certificate or Kerberos: not yet.
    assert!(button(&h, "Sign in").accesskit_node().is_disabled());
    h.get_by_label_contains("TLS-DSK").click();
    h.run();
    assert!(!button(&h, "Sign in").accesskit_node().is_disabled());
}

#[test]
fn appointments_and_automatic_replies_show_in_the_meetings_tab() {
    use commscraft_ui::model::{Appointment, Busy};
    let mut ui = online();
    ui.state.apply(AppEvent::Calendar {
        appointments: Some(vec![
            Appointment {
                start: NOW + 3600,
                end: NOW + 7200,
                busy: Busy::Busy,
                subject: "Planung".into(),
                location: "Raum 1".into(),
            },
            Appointment {
                start: NOW + 86_400,
                end: NOW + 90_000,
                busy: Busy::Tentative,
                subject: String::new(),
                location: String::new(),
            },
        ]),
        out_of_office: Some("Bis Montag im Urlaub".into()),
    });
    let mut h = harness(ui, (420.0, 760.0));
    h.run();
    button(&h, "Besprechungen").click();
    h.run();
    assert!(h.query_by_label_contains("Planung").is_some());
    assert!(h.query_by_label_contains("Gebucht · Raum 1").is_some());
    // A private appointment shows no subject.
    assert!(h.query_by_label_contains("Privater Termin").is_some());
    assert!(h
        .query_by_label_contains("Automatische Antworten sind an")
        .is_some());
    assert!(h.query_by_label_contains("Bis Montag im Urlaub").is_some());
}

#[test]
fn a_password_in_the_keyring_lets_the_field_stay_empty() {
    let mut ui = CommsUi::new(Lang::En, 0);
    ui.set_sign_in_address("mara@beispiel.invalid");
    ui.stored_password_for = Some("sip:mara@beispiel.invalid".into());
    let mut h = harness(ui, (420.0, 760.0));
    h.run();
    assert!(h
        .query_by_label_contains("Password kept in the keyring")
        .is_some());
    assert!(!button(&h, "Sign in").accesskit_node().is_disabled());
    // Another address: the stored password does not apply.
    let f = field(&h, "Sign-in address");
    f.focus();
    f.type_text("x");
    h.run();
    assert!(button(&h, "Sign in").accesskit_node().is_disabled());
}

#[test]
fn a_distribution_group_lists_its_members_when_open() {
    let mut h = harness(demo_quiet(), (420.0, 760.0));
    h.run();
    h.run();
    // The group is open: its members were asked for once and are shown.
    assert!(h.state().intents.iter().any(
        |i| matches!(i, UiIntent::ExpandGroup { email } if email.starts_with("projekt-nord@"))
    ));
    assert!(h.query_by_label_contains("PROJEKTTEAM NORD (3)").is_some());
    assert_eq!(
        h.state()
            .intents
            .iter()
            .filter(|i| matches!(i, UiIntent::ExpandGroup { .. }))
            .count(),
        1
    );
}

#[test]
fn the_new_view_adds_a_subscriber_to_the_contacts() {
    let mut h = harness(demo_quiet(), (420.0, 760.0));
    h.run();
    h.get_by_label("NEU").click();
    h.run();
    assert!(h.query_by_label_contains("Jana Koch").is_some());
    h.get_by_label("Zu Kontakten hinzufügen").click();
    h.run();
    let i = &h.state().intents;
    assert!(i.iter().any(
        |i| matches!(i, UiIntent::AddContact { uri, name, group: None }
        if uri.starts_with("sip:jana.koch@") && name == "Jana Koch")
    ));
    assert!(i
        .iter()
        .any(|i| matches!(i, UiIntent::AcknowledgeSubscriber { uri } if uri.starts_with("sip:jana.koch@"))));
    assert!(h.state().ui.state.subscribers.is_empty());
}

#[test]
fn the_note_is_published_when_the_field_is_left() {
    let mut h = harness(demo_quiet(), (420.0, 760.0));
    h.run();
    let field = h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Was gibt es Neues?");
    field.click();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Was gibt es Neues?")
        .type_text("Heute im Büro");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h
        .state()
        .intents
        .iter()
        .any(|i| matches!(i, UiIntent::SetNote(n) if n == "Heute im Büro")));
}

#[test]
fn a_presenter_admits_from_the_lobby_and_promotes() {
    use commscraft_ui::model::{ConfCommand, ConvState, Participant};
    let mut ui = demo_quiet();
    let me = ui.state.me.uri.clone();
    let p = |uri: &str, presenter, waiting, muted| Participant {
        uri: uri.into(),
        name: String::new(),
        presenter,
        waiting,
        muted,
        in_chat: true,
        video: false,
        sharing: false,
    };
    let c = ui
        .state
        .conversations
        .iter_mut()
        .find(|c| c.title == "Projekt Nord")
        .unwrap();
    c.state = ConvState::Open;
    c.participants = vec![
        p(&me, true, false, Some(false)),
        p("sip:gast@beispiel.invalid", false, true, None),
        p("sip:tim@b.de", false, false, Some(false)),
    ];
    let mut h = harness(ui, BIG);
    h.run();
    open_conversation(&mut h, "Projekt Nord");
    h.get_by_label("Teilnehmer").click();
    h.run();
    assert!(h.query_by_label_contains("Im Wartebereich (1)").is_some());
    h.get_by_label("Zulassen").click();
    h.run();
    assert!(h.state().intents.iter().any(|i| matches!(i,
        UiIntent::ConferenceCommand { command: ConfCommand::Admit(u), .. } if u == &vec!["sip:gast@beispiel.invalid".to_string()])));
    h.get_by_label_contains("tim@b.de").click_secondary();
    h.run();
    h.get_by_label("Zum Referenten machen").click();
    h.run();
    assert!(h.state().intents.iter().any(|i| matches!(i,
        UiIntent::ConferenceCommand { command: ConfCommand::Role { user, presenter: true }, .. } if user == "sip:tim@b.de")));
}

#[test]
fn a_meeting_is_planned_joined_and_cancelled_from_the_meetings_tab() {
    use commscraft_ui::model::MeetingAccess;
    let mut h = harness(demo_quiet(), BIG);
    h.run();
    button(&h, "Besprechungen").click();
    h.run();
    h.get_by_label("Neue Besprechung …").click();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Betreff")
        .click();
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Betreff")
        .type_text("Planung Q1");
    h.run();
    h.get_by_label("Nur eingeladene Personen").click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().intents.iter().any(|i| matches!(i,
        UiIntent::ScheduleMeeting { subject, minutes: 60, access: MeetingAccess::Invited, join: false } if subject == "Planung Q1")));
    // A planned meeting of the demo: join and cancel.
    h.get_by_role_and_label(
        egui::accesskit::Role::Button,
        "Wochenabstimmung Projekt Nord",
    )
    .click();
    h.run();
    h.get_by_label("Absagen").click();
    h.run();
    assert!(h.state().intents.iter().any(
        |i| matches!(i, UiIntent::CancelMeeting { conference_id } if conference_id == "DEMO1")
    ));
}

#[test]
fn the_meeting_audio_is_joined_from_a_group_conversation() {
    use commscraft_ui::model::ConvState;
    let mut ui = demo_quiet();
    ui.state.capabilities.audio_call = Gate::Available;
    ui.state.capabilities.video_call = Gate::Available;
    let c = ui
        .state
        .conversations
        .iter_mut()
        .find(|c| c.title == "Projekt Nord")
        .unwrap();
    c.state = ConvState::Open;
    c.session = Some(7);
    let key = c.key;
    let mut h = harness(ui, BIG);
    h.run();
    open_conversation(&mut h, "Projekt Nord");
    h.get_by_label("Der Besprechung mit Video beitreten")
        .click();
    h.run();
    assert!(h
        .state()
        .intents
        .iter()
        .any(|i| matches!(i, UiIntent::JoinMeetingVideo { key: k } if *k == key)));
    h.get_by_label("Audio der Besprechung beitreten").click();
    h.run();
    // Asked first, as in the Windows client: the computer's audio is the preset choice.
    h.get_by_label("OK").click();
    h.run();
    assert!(h
        .state()
        .intents
        .iter()
        .any(|i| matches!(i, UiIntent::JoinMeetingAudio { key: k } if *k == key)));
    // The call that comes back belongs to the meeting and is named like it.
    h.state_mut().ui.state.apply(AppEvent::CallStarted {
        id: 3,
        peer: String::new(),
        incoming: false,
        at: NOW,
    });
    h.state_mut()
        .ui
        .state
        .apply(AppEvent::ConferenceCall { id: 3, session: 7 });
    assert_eq!(h.state().ui.state.calls[0].conference, Some(key));
}

#[test]
fn a_shared_screen_in_a_meeting_is_offered_for_viewing() {
    use commscraft_ui::model::ConvState;
    let mut ui = demo_quiet();
    let c = ui
        .state
        .conversations
        .iter_mut()
        .find(|c| c.title == "Projekt Nord")
        .unwrap();
    c.state = ConvState::Open;
    c.session = Some(9);
    let key = c.key;
    ui.state.apply(AppEvent::ConferenceSharing {
        session: 9,
        sharer: Some("Anna Weber".into()),
    });
    let mut h = harness(ui, BIG);
    h.run();
    open_conversation(&mut h, "Projekt Nord");
    assert!(h.query_by_label_contains("Anna Weber gibt frei").is_some());
    h.get_by_label("Anzeigen").click();
    h.run();
    assert!(h
        .state()
        .intents
        .iter()
        .any(|i| matches!(i, UiIntent::ViewMeetingScreen { key: k } if *k == key)));
    // Sharing ends: the offer goes.
    h.state_mut().ui.state.apply(AppEvent::ConferenceSharing {
        session: 9,
        sharer: None,
    });
    h.run();
    assert!(h.query_by_label_contains("gibt frei").is_none());
}

#[test]
fn an_ended_call_can_be_rated_when_the_server_has_a_quality_service() {
    let mut ui = demo_quiet();
    ui.state.apply(AppEvent::QualityRating(true));
    ui.state.apply(AppEvent::CallStarted {
        id: 5,
        peer: "sip:anna@beispiel.invalid".into(),
        incoming: false,
        at: NOW - 60,
    });
    ui.state.apply(AppEvent::CallConnected {
        id: 5,
        at: NOW - 50,
    });
    ui.state.apply(AppEvent::CallEnded {
        id: 5,
        status: 200,
        at: NOW,
    });
    let mut h = harness(ui, (420.0, 760.0));
    h.run();
    h.get_by_label("4 von 5").click();
    h.run();
    assert!(h
        .state()
        .intents
        .iter()
        .any(|i| matches!(i, UiIntent::RateCall { id: 5, stars: 4 })));
    assert!(h.state().ui.state.rate_call.is_none());
}

#[test]
fn a_call_is_transferred_to_the_party_of_another_call() {
    use commscraft_ui::model::CallState;
    let mut ui = online();
    for (id, peer) in [(1, "sip:anna@b.de"), (2, "sip:tim@b.de")] {
        ui.state.apply(AppEvent::CallStarted {
            id,
            peer: peer.into(),
            incoming: false,
            at: NOW - 60,
        });
        ui.state.apply(AppEvent::CallConnected { id, at: NOW - 50 });
    }
    assert!(ui
        .state
        .calls
        .iter()
        .all(|c| matches!(c.state, CallState::Connected { .. })));
    let mut h = harness(ui, BIG);
    h.run();
    h.run();
    // Both call windows are open; the first one's transfer menu offers the other call.
    // (Embedded windows overlap in tests: the top one, drawn last, takes the click.)
    h.get_all_by_label("Anrufsteuerung").last().unwrap().click();
    h.run();
    h.get_all_by_label("Übertragen").last().unwrap().click();
    h.run();
    h.get_by_label_contains("Mit Rückfrage an").click();
    h.run();
    assert!(h.state().intents.iter().any(|i| matches!(
        i,
        UiIntent::TransferToCall { id: 1, target: 2 }
            | UiIntent::TransferToCall { id: 2, target: 1 }
    )));
}

#[test]
fn the_conversation_history_from_exchange_is_listed_and_read() {
    use commscraft_ui::model::HistoryEntry;
    let mut h = harness(online(), (420.0, 760.0));
    h.run();
    button(&h, "Unterhaltungen").click();
    h.run();
    h.get_by_label("Laden").click();
    h.run();
    assert!(h
        .state()
        .intents
        .iter()
        .any(|i| matches!(i, UiIntent::LoadHistory)));
    h.state_mut()
        .ui
        .state
        .apply(AppEvent::History(Ok(vec![HistoryEntry {
            id: "AAMkA1=".into(),
            subject: "Hallo".into(),
            at: Some(NOW),
            with: "Erin Test".into(),
        }])));
    h.run();
    h.get_by_label_contains("Erin Test").click();
    h.run();
    assert!(h
        .state()
        .intents
        .iter()
        .any(|i| matches!(i, UiIntent::OpenHistory { id } if id == "AAMkA1=")));
    h.state_mut().ui.state.apply(AppEvent::HistoryText {
        id: "AAMkA1=".into(),
        text: Ok("17:00 Alice: Hallo\n17:01 Erin: Hallo zurück".into()),
    });
    h.run();
    assert!(h.query_by_label_contains("Hallo zurück").is_some());
    h.get_by_label("Schließen").click();
    h.run();
    assert!(h.state().ui.state.history_open.is_none());
}

#[test]
fn conversations_are_tabs_of_one_window() {
    let mut h = harness(demo_quiet(), BIG);
    h.run();
    open_conversation(&mut h, "Projekt Nord");
    // (The second one directly: embedded windows cover the list in tests.)
    let max = h
        .state()
        .ui
        .state
        .conversations
        .iter()
        .find(|c| c.title == "Max Richter")
        .unwrap()
        .key;
    h.state_mut().ui.select_conversation(max);
    h.run();
    h.run();
    // Two conversations: one window, the tab list names both.
    let tabs: Vec<_> = h
        .query_all_by_role(egui::accesskit::Role::Tab)
        .map(|n| n.accesskit_node().label().unwrap_or_default().to_string())
        .collect();
    assert!(
        tabs.iter().any(|t| t.contains("Projekt Nord")),
        "{tabs:?} {:?}",
        h.state().ui.windows()
    );
    assert!(tabs.iter().any(|t| t.contains("Max Richter")), "{tabs:?}");
    // Another tab chosen: its conversation is shown.
    h.query_all_by_role(egui::accesskit::Role::Tab)
        .find(|n| {
            n.accesskit_node()
                .label()
                .is_some_and(|l| l.contains("Projekt Nord"))
        })
        .unwrap()
        .click();
    h.run();
    let k = h
        .state()
        .ui
        .state
        .conversations
        .iter()
        .find(|c| c.title == "Projekt Nord")
        .unwrap()
        .key;
    assert_eq!(h.state().ui.selected_conversation(), Some(k));
}

#[test]
fn a_presenter_changes_the_meeting_options() {
    use commscraft_ui::model::{
        ConfCommand, ConvState, MeetingAccess, MeetingView, Participant, Presenters,
    };
    let mut ui = demo_quiet();
    let me = ui.state.me.uri.clone();
    let c = ui
        .state
        .conversations
        .iter_mut()
        .find(|c| c.title == "Projekt Nord")
        .unwrap();
    c.state = ConvState::Open;
    c.session = Some(5);
    c.participants = vec![Participant {
        uri: me,
        name: String::new(),
        presenter: true,
        waiting: false,
        muted: Some(false),
        in_chat: true,
        video: false,
        sharing: false,
    }];
    ui.state.apply(AppEvent::MeetingSettings {
        session: 5,
        view: MeetingView {
            access: MeetingAccess::Organisation,
            presenters: Presenters::Organisation,
            ..MeetingView::default()
        },
    });
    let mut h = harness(ui, BIG);
    h.run();
    open_conversation(&mut h, "Projekt Nord");
    h.get_by_label("Teilnehmer").click();
    h.run();
    // Grouped as in the Windows client.
    assert!(h.query_by_label_contains("Referenten (1)").is_some());
    h.get_by_label("Weitere Optionen").click();
    h.run();
    h.get_by_label("Besprechungsoptionen …").click();
    h.run();
    h.get_by_label("Nur eingeladene Personen").click();
    h.get_by_label("Nur der Organisator").click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().intents.iter().any(|i| matches!(
        i,
        UiIntent::ConferenceCommand {
            command: ConfCommand::Options {
                access: MeetingAccess::Invited,
                presenters: Presenters::Nobody,
                phone_bypass: false
            },
            ..
        }
    )));
}

#[test]
fn a_tagged_contact_becoming_available_raises_an_alert() {
    use commscraft_ui::model::Presence;
    let mut ui = demo_quiet();
    let (uri, name) = ui
        .state
        .contacts
        .values()
        .find(|c| c.presence != Presence::Available)
        .map(|c| (c.uri.clone(), c.name.clone()))
        .unwrap();
    ui.prefs.status_alerts.insert(uri.clone());
    let before = ui.state.notices.len();
    ui.apply(AppEvent::Presence {
        uri: uri.clone(),
        presence: Some(Presence::Available),
        note: String::new(),
        name: None,
    });
    assert_eq!(ui.state.notices.len(), before + 1);
    assert!(ui.state.notices.back().unwrap().text.contains(&name));
    // Already available: no second alert.
    ui.apply(AppEvent::Presence {
        uri,
        presence: Some(Presence::Available),
        note: String::new(),
        name: None,
    });
    assert_eq!(ui.state.notices.len(), before + 1);
}

#[test]
fn the_meeting_entry_info_names_link_and_organizer() {
    use commscraft_ui::model::ConvState;
    let mut ui = demo_quiet();
    let c = ui
        .state
        .conversations
        .iter_mut()
        .find(|c| c.title == "Projekt Nord")
        .unwrap();
    c.state = ConvState::Open;
    c.session = Some(6);
    ui.state.apply(AppEvent::ConferenceFocus {
        session: 6,
        focus: "sip:anna@beispiel.invalid;gruu;opaque=app:conf:focus:id:AB12".into(),
    });
    let mut h = harness(ui, BIG);
    h.run();
    open_conversation(&mut h, "Projekt Nord");
    h.get_by_label("Weitere Optionen").click();
    h.run();
    h.get_by_label("Besprechungsinfos").click();
    h.run();
    assert!(h.query_by_label("Organisator").is_some());
    assert!(h
        .query_by_label_contains("opaque=app:conf:focus:id:AB12")
        .is_some());
    h.get_by_label("Schließen").click();
    h.run();
    assert!(h.query_by_label("Organisator").is_none());
}

#[test]
fn joining_the_meeting_audio_asks_how() {
    use commscraft_ui::model::{ConfCommand, ConvState};
    let mut ui = demo_quiet();
    let c = ui
        .state
        .conversations
        .iter_mut()
        .find(|c| c.title == "Projekt Nord")
        .unwrap();
    c.state = ConvState::Open;
    c.session = Some(7);
    let mut h = harness(ui, BIG);
    h.run();
    open_conversation(&mut h, "Projekt Nord");
    button(&h, "Audio der Besprechung beitreten").click();
    h.run();
    h.get_by_label("Mich anrufen unter:").click();
    h.run();
    // The number field of the dialog (the conversation's message field is there too).
    h.query_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().placeholder() == Some("+49 …"))
        .expect("number field")
        .click();
    h.run();
    h.query_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().placeholder() == Some("+49 …"))
        .expect("number field")
        .type_text("+49 40 123456");
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().intents.iter().any(|i| matches!(i,
        UiIntent::ConferenceCommand { command: ConfCommand::DialOut(n), .. } if n == "+49 40 123456")), "{:?}", h.state().intents);
    assert!(h.query_by_label("Mich anrufen unter:").is_none());
}
