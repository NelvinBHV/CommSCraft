//! The frame around the views: top bar, rail, list panel, main panel, context panel, status bar.

use crate::app::{CommsUi, UiIntent};
use crate::model::{Connection, NoticeKind, View};
use crate::theme::{self, size, Palette};
use crate::widgets;
use egui::{Align, Frame, Layout, Margin, RichText, Sense, Stroke, Ui, Vec2};

fn panel_frame(fill: egui::Color32, margin: Margin) -> Frame {
    Frame::new().fill(fill).inner_margin(margin)
}

pub fn statusbar(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    egui::Panel::bottom("statusbar")
        .exact_size(size::STATUSBAR)
        .frame(
            panel_frame(pal.surface_subtle, Margin::symmetric(16, 4))
                .stroke(Stroke::new(1.0, pal.border)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let (colour, text) = if app.demo {
                    (pal.warning, app.t.status_demo.to_string())
                } else {
                    match &app.state.connection {
                        Connection::Connected => (pal.success, app.t.status_connected.to_string()),
                        Connection::Degraded(why) => {
                            (pal.warning, format!("{} – {why}", app.t.status_degraded))
                        }
                        Connection::Connecting => {
                            (pal.warning, app.t.status_connecting.to_string())
                        }
                        Connection::Disconnected(why) => {
                            (pal.error, format!("{} – {why}", app.t.status_disconnected))
                        }
                        Connection::Failed(why) => {
                            (pal.error, format!("{} – {why}", app.t.status_failed))
                        }
                        Connection::SignedOut | Connection::Demo => {
                            (pal.quiet, app.t.status_signed_out.to_string())
                        }
                    }
                };
                let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 4.5, colour);
                widgets::ellipsis(
                    ui,
                    RichText::new(text).size(theme::text::SMALL).color(pal.ink),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("CommSCraft {}", env!("CARGO_PKG_VERSION")))
                            .size(theme::text::SMALL)
                            .color(pal.muted),
                    );
                    if !app.prefs.context_open
                        && app.view != View::Settings
                        && ui.small_button(app.t.show_context).clicked()
                    {
                        app.prefs.context_open = true;
                        app.emit(UiIntent::PrefsChanged);
                    }
                    // After a lost connection: back to the sign-in (the state is cleared).
                    if matches!(app.state.connection, Connection::Disconnected(_))
                        && !app.demo
                        && ui.small_button(app.t.sign_in_again).clicked()
                    {
                        app.emit(UiIntent::SignOut);
                    }
                    if let Some(n) = app.state.notices.back() {
                        let c = match n.kind {
                            NoticeKind::Info => pal.muted,
                            NoticeKind::Warning => pal.ink,
                            NoticeKind::Error => pal.error,
                        };
                        widgets::ellipsis(
                            ui,
                            RichText::new(&n.text).size(theme::text::SMALL).color(c),
                        );
                    }
                });
            });
        });
}
