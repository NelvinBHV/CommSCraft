//! Native demo. No packets are sent; contacts and conversations are examples.
use commscraft_core::challenge::parse_challenges;
use commscraft_core::discovery::DiscoveryPlan;
use commscraft_core::session::{SessionEvent, SessionMachine};
use commscraft_core::sip::{illustrative_register, parse_frame, SipMessage, StartLine};
use commscraft_core::stream::SipStreamDecoder;
use commscraft_core::transaction::{NonInviteClientTransaction, TransactionState};
use eframe::egui::{self, Color32, RichText};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Presence {
    Available,
    Busy,
    Away,
    Offline,
}
impl Presence {
    fn label(self) -> &'static str {
        match self {
            Self::Available => "Verfügbar",
            Self::Busy => "Beschäftigt",
            Self::Away => "Abwesend",
            Self::Offline => "Offline",
        }
    }
    fn color(self) -> Color32 {
        match self {
            Self::Available => Color32::from_rgb(28, 150, 82),
            Self::Busy => Color32::from_rgb(203, 68, 60),
            Self::Away => Color32::from_rgb(221, 160, 42),
            Self::Offline => Color32::GRAY,
        }
    }
}

struct ChatLine {
    author: &'static str,
    text: String,
    mine: bool,
}
struct Contact {
    name: &'static str,
    role: &'static str,
    sip: &'static str,
    presence: Presence,
    conversation: Vec<ChatLine>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Chat,
    Diagnostics,
}
struct SfbApp {
    page: Page,
    contacts: Vec<Contact>,
    active: usize,
    search: String,
    draft: String,
    own_presence: Presence,
    sip_identity: String,
    session: SessionMachine,
    demo_step: usize,
    events: Vec<String>,
    transaction: Option<NonInviteClientTransaction>,
    transaction_events: Vec<String>,
    transaction_step: u64,
}

impl Default for SfbApp {
    fn default() -> Self {
        Self {
            page: Page::Chat,
            contacts: vec![
                Contact {
                    name: "Alex Beispiel",
                    role: "Produktentwicklung",
                    sip: "sip:alex@example.com",
                    presence: Presence::Available,
                    conversation: vec![ChatLine {
                        author: "Alex (Demo)",
                        text: "Hallo! Das ist eine lokale Beispielunterhaltung.".into(),
                        mine: false,
                    }],
                },
                Contact {
                    name: "Sam Beispiel",
                    role: "IT / Collaboration",
                    sip: "sip:sam@example.com",
                    presence: Presence::Busy,
                    conversation: vec![ChatLine {
                        author: "Sam (Demo)",
                        text: "Die echte Serveranbindung ist noch nicht implementiert.".into(),
                        mine: false,
                    }],
                },
                Contact {
                    name: "Taylor Beispiel",
                    role: "Customer Success",
                    sip: "sip:taylor@example.com",
                    presence: Presence::Away,
                    conversation: vec![],
                },
            ],
            active: 0,
            search: String::new(),
            draft: String::new(),
            own_presence: Presence::Available,
            sip_identity: "sip:alice@example.com".to_string(),
            session: SessionMachine::default(),
            demo_step: 0,
            events: Vec::new(),
            transaction: None,
            transaction_events: Vec::new(),
            transaction_step: 0,
        }
    }
}

impl SfbApp {
    fn next_demo_step(&mut self) {
        let steps = [
            (SessionEvent::Start, "Discovery (simuliert)"),
            (
                SessionEvent::DiscoverySucceeded,
                "Pool gefunden (simuliert)",
            ),
            (SessionEvent::Connected, "TLS verbunden (simuliert)"),
            (SessionEvent::ChallengeReceived, "401 Challenge (simuliert)"),
            (
                SessionEvent::CredentialsPrepared,
                "Authentifizierung (simuliert)",
            ),
            (SessionEvent::RegistrationAccepted, "200 OK (simuliert)"),
        ];
        if let Some((event, label)) = steps.get(self.demo_step) {
            match self.session.dispatch(*event) {
                Ok(state) => self.events.push(format!("{label} → {state:?}")),
                Err(error) => self.events.push(error.to_string()),
            }
            self.demo_step += 1;
        }
    }

    fn begin_transaction_demo(&mut self) {
        self.transaction_events.clear();
        self.transaction_step = 0;
        self.transaction = None;
        match illustrative_register(&self.sip_identity) {
            Ok(request) => {
                let wire = request.encode().expect("validated demonstration message");
                let mut decoder = SipStreamDecoder::new();
                let split = wire.len() / 2;
                let count = decoder
                    .push(&wire[..split])
                    .expect("first half is valid")
                    .len()
                    + decoder
                        .push(&wire[split..])
                        .expect("second half is valid")
                        .len();
                self.transaction_events.push(format!(
                    "Fragmentierter SIP-Stream: {count} vollständige Nachricht(en)"
                ));
                match NonInviteClientTransaction::new(&request, 0) {
                    Ok(txn) => {
                        self.transaction = Some(txn);
                        self.transaction_events
                            .push("REGISTER-Transaktion vorbereitet (NICHT gesendet)".into());
                    }
                    Err(e) => self.transaction_events.push(format!("Transaktion: {e}")),
                }
            }
            Err(e) => self.transaction_events.push(format!("Fehler: {e}")),
        }
    }

    fn simulate_transaction_reply(&mut self, code: u16) {
        let Some(txn) = self.transaction.as_mut() else {
            return;
        };
        // Build a synthetic reply correlated to the illustrative request.
        // Credentials and real network operations are deliberately absent.
        if let Ok(request) = illustrative_register(&self.sip_identity) {
            let response = SipMessage {
                start_line: StartLine::Response {
                    code,
                    reason: "Demo".into(),
                },
                headers: ["Via", "Call-ID", "CSeq"]
                    .iter()
                    .map(|key| {
                        (
                            key.to_string(),
                            request.header(key).unwrap_or_default().to_owned(),
                        )
                    })
                    .collect(),
                body: Vec::new(),
            };
            self.transaction_step += 1;
            let result = txn.receive_response(&response, self.transaction_step * 1000);
            let label = match result {
                Ok(event) => format!("Simulierte Antwort {code}: {event:?}"),
                Err(e) => format!("Simulierte Antwort {code}: {e}"),
            };
            self.transaction_events.push(label);
            if self.transaction_events.len() > 12 {
                self.transaction_events.remove(0);
            }
        }
    }

    fn show_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("contacts")
            .default_width(290.0)
            .min_width(240.0)
            .max_width(360.0)
            .show(ctx, |ui| {
                ui.add_space(9.0);
                ui.heading("Meine Kontakte");
                ui.label(
                    RichText::new("Beispieldaten · Keine Serververbindung")
                        .color(Color32::DARK_GRAY)
                        .small(),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.colored_label(self.own_presence.color(), "●");
                    egui::ComboBox::from_id_salt("own-presence")
                        .selected_text(self.own_presence.label())
                        .show_ui(ui, |ui| {
                            for p in [
                                Presence::Available,
                                Presence::Busy,
                                Presence::Away,
                                Presence::Offline,
                            ] {
                                ui.selectable_value(&mut self.own_presence, p, p.label());
                            }
                        });
                });
                ui.label(
                    RichText::new("Status nur lokal")
                        .small()
                        .color(Color32::GRAY),
                );
                ui.separator();
                ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Kontakte suchen …"));
                ui.add_space(9.0);
                let needle = self.search.to_lowercase();
                for (idx, contact) in self.contacts.iter().enumerate() {
                    if !contact.name.to_lowercase().contains(&needle)
                        && !contact.sip.to_lowercase().contains(&needle)
                    {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        ui.colored_label(contact.presence.color(), "●");
                        if ui
                            .selectable_label(self.active == idx, contact.name)
                            .clicked()
                        {
                            self.active = idx;
                            self.page = Page::Chat;
                        }
                    });
                    ui.label(RichText::new(contact.role).small().color(Color32::GRAY));
                    ui.add_space(10.0);
                }
                ui.separator();
                if ui.button("⚙ Diagnose / Protokolle").clicked() {
                    self.page = Page::Diagnostics;
                }
                if ui.button("💬 Unterhaltungen").clicked() {
                    self.page = Page::Chat;
                }
            });
    }

    fn show_chat(&mut self, ui: &mut egui::Ui) {
        let contact = &self.contacts[self.active];
        ui.heading(contact.name);
        ui.horizontal(|ui| {
            ui.colored_label(
                contact.presence.color(),
                format!("● {}", contact.presence.label()),
            );
            ui.label(RichText::new(contact.sip).small().color(Color32::GRAY));
        });
        ui.separator();
        let scroll_height = (ui.available_height() - 110.0).max(160.0);
        egui::ScrollArea::vertical()
            .max_height(scroll_height)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.contacts[self.active].conversation.is_empty() {
                    ui.label("Noch keine lokalen Nachrichten in dieser Demo-Unterhaltung.");
                }
                for line in &self.contacts[self.active].conversation {
                    ui.group(|ui| {
                        ui.label(RichText::new(line.author).strong().color(if line.mine {
                            Color32::from_rgb(38, 96, 172)
                        } else {
                            Color32::from_rgb(27, 98, 83)
                        }));
                        ui.label(&line.text);
                    });
                    ui.add_space(4.0);
                }
            });
        ui.separator();
        ui.label(
            RichText::new("Nur lokale Vorschau – nichts wird versendet")
                .small()
                .color(Color32::DARK_GRAY),
        );
        ui.horizontal(|ui| {
            ui.add_sized(
                [ui.available_width() - 116.0, 70.0],
                egui::TextEdit::multiline(&mut self.draft).hint_text("Nachricht eingeben …"),
            );
            if ui
                .add_sized([105.0, 35.0], egui::Button::new("Lokal hinzufügen"))
                .clicked()
            {
                let body = self.draft.trim();
                if !body.is_empty() {
                    self.contacts[self.active].conversation.push(ChatLine {
                        author: "Du (lokal)",
                        text: body.to_owned(),
                        mine: true,
                    });
                    self.draft.clear();
                }
            }
        });
    }

    fn show_diagnostics(&mut self, ui: &mut egui::Ui) {
        ui.heading("Protokolldiagnose · Offline-Sandbox");
        ui.label("Diese Ansicht erzeugt ausschließlich Beispieldaten. Es erfolgen keine DNS-, HTTP-, TLS- oder SIP-Anfragen.");
        ui.add_space(7.0);
        ui.horizontal(|ui| {
            ui.label("SIP-Identität:");
            ui.text_edit_singleline(&mut self.sip_identity);
        });
        egui::CollapsingHeader::new("Discovery-Kandidaten").default_open(true).show(ui, |ui| {
            match DiscoveryPlan::from_sip_uri(&self.sip_identity) {
                Ok(plan) => {
                    ui.label("HTTPS-Hostnamen:");
                    for host in &plan.https_hosts { ui.monospace(host); }
                    ui.label("Microsoft SIP SRV (MS-CONMGMT §3.1.5):");
                    for (service, name) in plan.srv_queries() {
                        ui.monospace(format!("{}  [{}]", name,
                            if service.is_tls() { "TLS" } else { "TCP: nicht erlaubt" }));
                    }
                    ui.label("TLS-Fallback-Hosts (Port 443):");
                    for host in plan.fallback_hosts { ui.monospace(format!("{host}:443")); }
                    ui.label("Live DNS/TLS ausschließlich im separaten commscraft-probe CLI mit Opt-in.");
                }
                Err(error) => { ui.colored_label(Color32::RED, format!("Ungültige SIP-Identität: {error:?}")); }
            }
        });
        egui::CollapsingHeader::new("REGISTER-Template (nicht sendbar)").show(ui, |ui| {
            match illustrative_register(&self.sip_identity) {
                Ok(message) => {
                    let wire = message.encode().expect("safe illustration");
                    if let Ok(Some((_, bytes))) = parse_frame(&wire) {
                        ui.label(format!("Parser: {} Bytes gerahmt", bytes));
                    }
                    let mut preview = String::from_utf8_lossy(&wire).to_string();
                    ui.add(
                        egui::TextEdit::multiline(&mut preview)
                            .code_editor()
                            .desired_rows(14)
                            .interactive(false),
                    );
                }
                Err(err) => {
                    ui.colored_label(Color32::RED, format!("{err}"));
                }
            }
        });
        egui::CollapsingHeader::new("MS-SIPAE Authentifizierungs-Challenges (nur Parser)")
            .show(ui, |ui| {
                ui.label("Beispielhafte 401-Challenges; keine Tokens, Passwörter oder Anmeldungen.");
                let synthetic = SipMessage {
                    start_line: StartLine::Response { code: 401, reason: "Unauthorized".into() },
                    headers: vec![
                        ("WWW-Authenticate".into(), "NTLM realm=\"SIP Communications Service\", targetname=\"server.example.com\", version=3".into()),
                        ("WWW-Authenticate".into(), "Kerberos realm=\"SIP Communications Service\", targetname=\"sip/server.example.com\", version=3".into()),
                    ], body: Vec::new(),
                };
                match parse_challenges(&synthetic) {
                    Ok(offers) => for offer in offers {
                        ui.monospace(format!("{:?} · {:?} · Protokollversion {:?}",
                            offer.scheme, offer.source, offer.version));
                    },
                    Err(err) => { ui.colored_label(Color32::RED, format!("Ungültige Test-Challenge: {err}")); }
                }
            });
        ui.separator();
        ui.heading("RFC 3261 – REGISTER-Transaktion (Offline)");
        ui.label("Antworten werden anhand von Via-Branch, Call-ID und CSeq abgeglichen. Eine 401/407 Challenge beendet die aktuelle Transaktion; sie bedeutet keine erfolgreiche Anmeldung.");
        if ui.button("Neue Demo-Transaktion vorbereiten").clicked() {
            self.begin_transaction_demo();
        }
        if let Some(txn) = &self.transaction {
            ui.monospace(format!("Transaktion: {:?} (kein Netzwerk)", txn.state()));
        }
        let active_txn = self
            .transaction
            .as_ref()
            .is_some_and(|txn| txn.state() != TransactionState::Terminated);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(active_txn, egui::Button::new("100 Trying simulieren"))
                .clicked()
            {
                self.simulate_transaction_reply(100);
            }
            if ui
                .add_enabled(active_txn, egui::Button::new("401 Challenge simulieren"))
                .clicked()
            {
                self.simulate_transaction_reply(401);
            }
            if ui
                .add_enabled(active_txn, egui::Button::new("200 OK simulieren"))
                .clicked()
            {
                self.simulate_transaction_reply(200);
            }
        });
        for line in &self.transaction_events {
            ui.monospace(line);
        }
        ui.separator();
        ui.heading("Registrierungs-Zustandsmaschine");
        ui.monospace(format!("{:?} (nur simuliert)", self.session.state()));
        if ui
            .add_enabled(
                self.demo_step < 6,
                egui::Button::new("Nächsten Demo-Schritt ausführen"),
            )
            .clicked()
        {
            self.next_demo_step();
        }
        if ui.button("Demo zurücksetzen").clicked() {
            self.session = SessionMachine::default();
            self.demo_step = 0;
            self.events.clear();
        }
        for line in &self.events {
            ui.monospace(line);
        }
    }
}

impl eframe::App for SfbApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("banner").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new("CommSCraft · Community Prototype")
                        .color(Color32::from_rgb(27, 88, 143)),
                );
                ui.separator();
                ui.colored_label(Color32::from_rgb(160, 76, 29), "OFFLINE-DEMO");
            });
        });
        self.show_sidebar(ctx);
        egui::CentralPanel::default().show(ctx, |ui| match self.page {
            Page::Chat => self.show_chat(ui),
            Page::Diagnostics => self.show_diagnostics(ui),
        });
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.label(
                RichText::new("Open Source · Clean Room · Nicht mit Microsoft verbunden")
                    .small()
                    .color(Color32::GRAY),
            );
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1050.0, 700.0])
            .with_min_inner_size([760.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CommSCraft – Community Prototype",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Ok(Box::new(SfbApp::default()))
        }),
    )
}
