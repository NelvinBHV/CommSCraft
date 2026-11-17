//! The sign-in screen (or the demo start). The password is handed over once and cleared, also
//! from the text field's undo memory.

use crate::app::{CommsUi, SignIn, UiIntent};
use crate::model::Connection;
use crate::theme::{self, Palette};
use crate::views::chats::normalise_address;
use crate::widgets;
use egui::{RichText, Ui, Vec2};

pub fn show(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let busy = app.state.connection == Connection::Connecting;
    egui::CentralPanel::no_frame().show(ui, |ui| {
        egui::Frame::new().fill(pal.surface_subtle).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() / 2.0 - 260.0).max(20.0));
                ui.allocate_ui(Vec2::new(440.0, 10.0), |ui| {
                    widgets::card(ui, pal, |ui| {
                        ui.set_width(408.0);
                        ui.label(
                            RichText::new(t.sign_in_title)
                                .size(theme::text::HEADING)
                                .family(crate::theme::bold())
                                .color(pal.ink),
                        );
                        ui.label(RichText::new(t.sign_in_body).color(pal.muted));
                        ui.add_space(10.0);
                        let f = &mut app.sign_in;
                        field(
                            ui,
                            pal,
                            t.sign_in_address,
                            &mut f.address,
                            "name@firma.example",
                            false,
                            busy,
                        );
                        field(
                            ui,
                            pal,
                            t.sign_in_user,
                            &mut f.user,
                            t.sign_in_user_hint,
                            false,
                            busy,
                        );
                        field(
                            ui,
                            pal,
                            t.sign_in_domain,
                            &mut f.domain,
                            t.sign_in_domain_hint,
                            false,
                            busy,
                        );
                        let pw_id =
                            field(ui, pal, t.sign_in_password, &mut f.password, "", true, busy);
                        field(
                            ui,
                            pal,
                            t.sign_in_server,
                            &mut f.server,
                            t.sign_in_server_hint,
                            false,
                            busy,
                        );
                        ui.checkbox(&mut app.sign_in.kerberos, t.sign_in_kerberos)
                            .on_hover_text(t.sign_in_kerberos_tip);
                        ui.checkbox(&mut app.sign_in.tls_dsk, t.sign_in_tls_dsk)
                            .on_hover_text(t.sign_in_tls_dsk_tip);
                        ui.checkbox(&mut app.sign_in.remember, t.sign_in_remember)
                            .on_hover_text(t.sign_in_remember_tip);
                        if app.stored_password_for.as_deref()
                            == normalise_address(&app.sign_in.address).as_deref()
                            && app.stored_password_for.is_some()
                        {
                            ui.label(
                                RichText::new(t.sign_in_password_stored)
                                    .size(theme::text::SMALL)
                                    .color(pal.muted),
                            );
                        }
                        if let Connection::Failed(why) = &app.state.connection {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(format!("{}: {why}", t.status_failed))
                                    .color(pal.error),
                            );
                        }
                        ui.add_space(10.0);
                        let address_ok = normalise_address(&app.sign_in.address).is_some();
                        let ok = !busy
                            && address_ok
                            // A password, or a way without one: the stored certificate
                            // (TLS-DSK) or the system's Kerberos ticket.
                            && (!app.sign_in.password.is_empty()
                                || app.sign_in.tls_dsk
                                || app.sign_in.kerberos
                                || app.stored_password_for.as_deref()
                                    == normalise_address(&app.sign_in.address).as_deref());
                        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        ui.horizontal(|ui| {
                            let label = if busy { t.signing_in } else { t.sign_in };
                            if (widgets::primary(ui, label, ok, pal).clicked() || (enter && ok))
                                && ok
                            {
                                let mut form = std::mem::take(&mut app.sign_in);
                                form.address = normalise_address(&form.address).unwrap_or_default();
                                // Keep the non-secret fields for the next attempt.
                                app.sign_in = SignIn {
                                    address: form.address.clone(),
                                    user: form.user.clone(),
                                    domain: form.domain.clone(),
                                    password: String::new(),
                                    server: form.server.clone(),
                                    kerberos: form.kerberos,
                                    tls_dsk: form.tls_dsk,
                                    remember: form.remember,
                                };
                                egui::text_edit::TextEditState::default().store(ui.ctx(), pw_id);
                                app.emit(UiIntent::SignIn(form));
                            }
                            if !busy
                                && widgets::secondary(ui, t.start_demo, true, pal)
                                    .on_hover_text(t.start_demo_tip)
                                    .clicked()
                            {
                                app.sign_in.password.clear();
                                egui::text_edit::TextEditState::default().store(ui.ctx(), pw_id);
                                app.emit(UiIntent::StartDemo);
                            }
                        });
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(t.sign_in_note)
                                .size(theme::text::SMALL)
                                .color(pal.muted),
                        );
                    });
                });
            });
        });
    });
}

fn field(
    ui: &mut Ui,
    pal: &Palette,
    label: &str,
    value: &mut String,
    hint: &str,
    password: bool,
    busy: bool,
) -> egui::Id {
    ui.label(RichText::new(label).size(theme::text::SMALL).color(pal.ink));
    let r = ui.add_enabled(
        !busy,
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .password(password)
            .char_limit(256)
            .desired_width(f32::INFINITY),
    );
    let name = label.to_string();
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, !busy, &name));
    ui.add_space(4.0);
    r.id
}

/// The trust question of discovery: a server outside the SIP domain, named over verified
/// HTTPS. Only an explicit yes trusts it (and remembers the domain); closing means no.
pub fn trust_dialog(app: &mut CommsUi, ctx: &egui::Context, pal: &Palette) {
    let Some(q) = app.state.trust_question.clone() else {
        return;
    };
    let t = app.t;
    let mut answer = None;
    let modal = egui::Modal::new(egui::Id::unique("trust")).show(ctx, |ui| {
        ui.set_width(480.0);
        ui.label(
            RichText::new(t.trust_title)
                .size(theme::text::PANEL_TITLE)
                .family(crate::theme::bold())
                .color(pal.ink),
        );
        ui.add_space(6.0);
        ui.add(
            egui::Label::new(
                RichText::new(t.trust_body(&q.host, &q.user_domain, &q.named_by, &q.domain))
                    .color(pal.ink),
            )
            .wrap(),
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if widgets::secondary(ui, t.trust_no, true, pal).clicked() {
                answer = Some(false);
            }
            if widgets::primary(ui, &t.trust_yes(&q.domain), true, pal).clicked() {
                answer = Some(true);
            }
        });
    });
    if modal.should_close() && answer.is_none() {
        answer = Some(false);
    }
    if let Some(a) = answer {
        app.state.trust_question = None;
        app.emit(UiIntent::AnswerTrust(a));
    }
}
