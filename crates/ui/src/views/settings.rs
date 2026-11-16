//! Settings: display (contrast, scale), account, the capability overview.

use crate::app::{CommsUi, UiIntent};
use crate::model::Gate;
use crate::theme::{self, Palette};
use crate::widgets;
use egui::{RichText, Ui};

pub fn list(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            RichText::new(t.settings)
                .size(theme::text::PANEL_TITLE + 2.0)
                .family(crate::theme::bold())
                .color(pal.ink),
        );
    });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.vertical(|ui| {
            widgets::section(ui, t.display, pal);
            let mut hc = app.prefs.high_contrast;
            if ui.checkbox(&mut hc, t.high_contrast).changed() {
                app.prefs.high_contrast = hc;
                app.emit(UiIntent::PrefsChanged);
            }
            ui.label(RichText::new(t.scale).color(pal.ink));
            let mut s = app.prefs.scale;
            let r = ui.add(
                egui::Slider::new(&mut s, 1.0..=2.0)
                    .step_by(0.25)
                    .custom_formatter(|v, _| format!("{:.0} %", v * 100.0)),
            );
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                app.prefs.scale = s;
                app.emit(UiIntent::PrefsChanged);
            } else if r.changed() {
                app.prefs.scale = s;
            }
            let mut ctx_open = app.prefs.context_open;
            if ui.checkbox(&mut ctx_open, t.show_context_setting).changed() {
                app.prefs.context_open = ctx_open;
                app.emit(UiIntent::PrefsChanged);
            }
            ui.add_space(12.0);
            widgets::section(ui, t.account, pal);
            if !app.state.me.uri.is_empty() {
                ui.label(RichText::new(app.state.me.uri.trim_start_matches("sip:")).color(pal.ink));
            }
            if ui.button(t.sign_out).clicked() {
                app.emit(UiIntent::SignOut);
            }
            ui.add_space(12.0);
            widgets::section(ui, t.shortcuts, pal);
            for line in t.shortcut_lines {
                ui.label(
                    RichText::new(*line)
                        .size(theme::text::SMALL)
                        .color(pal.muted),
                );
            }
        });
    });
}

pub fn main(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let c = app.state.capabilities.clone();
    egui::ScrollArea::vertical()
        .id_salt("settings-main")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::same(28))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(t.capabilities_title)
                            .size(theme::text::PANEL_TITLE)
                            .family(crate::theme::bold())
                            .color(pal.ink),
                    );
                    ui.label(RichText::new(t.capabilities_body).color(pal.muted));
                    ui.add_space(10.0);
                    let rows: [(&str, &Gate); 14] = [
                        (t.cap_chat, &c.chat),
                        (t.cap_group_chat, &c.group_chat),
                        (t.cap_presence, &c.presence),
                        (t.cap_directory, &c.directory_search),
                        (t.cap_audio, &c.audio_call),
                        (t.cap_hold, &c.hold),
                        (t.cap_transfer, &c.transfer),
                        (t.cap_video, &c.video_call),
                        (t.cap_meetings, &c.meetings),
                        (t.cap_files, &c.file_transfer),
                        (t.cap_rich_text, &c.rich_text),
                        (t.cap_persistent_chat, &c.persistent_chat),
                        (t.cap_history_search, &c.history_search),
                        (t.cap_read_receipts, &c.read_receipts),
                    ];
                    egui::Grid::new("caps")
                        .num_columns(3)
                        .striped(true)
                        .spacing([24.0, 8.0])
                        .show(ui, |ui| {
                            for (name, gate) in rows {
                                ui.label(RichText::new(name).color(pal.ink));
                                let (word, colour) = match gate {
                                    Gate::Available => (t.gate_available, pal.success_text),
                                    Gate::Preview => (t.gate_preview, pal.primary_hover),
                                    Gate::Unavailable(_) => (t.gate_unavailable, pal.muted),
                                };
                                ui.label(
                                    RichText::new(word)
                                        .family(crate::theme::bold())
                                        .color(colour),
                                );
                                let why = match gate {
                                    Gate::Unavailable(r) => t.reason(*r),
                                    Gate::Preview => t.preview_hint,
                                    Gate::Available => "",
                                };
                                ui.label(
                                    RichText::new(why).size(theme::text::SMALL).color(pal.muted),
                                );
                                ui.end_row();
                            }
                        });
                    ui.add_space(14.0);
                    widgets::hint(ui, pal, Some(t.security_title), t.security_body);
                });
        });
}
