//! Screen 3: calls – the active call with its controls and the call history.

use crate::app::{CallFilter, CommsUi, UiIntent};
use crate::format;
use crate::model::{CallDirection, CallKind, CallState, Gate, VideoState};
use crate::theme::{self, Palette};
use crate::views::shell::{context_block, context_title};
use crate::widgets;
use egui::{Align, CornerRadius, Layout, RichText, Sense, Ui, Vec2};

pub fn list(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new(t.calls)
                    .size(theme::text::PANEL_TITLE + 2.0)
                    .family(crate::theme::bold())
                    .color(pal.ink),
            );
            ui.label(
                RichText::new(t.calls_sub)
                    .size(theme::text::SMALL + 0.5)
                    .color(pal.muted),
            );
        });
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        if widgets::chip(ui, t.filter_all, app.call_filter == CallFilter::All, pal) {
            app.call_filter = CallFilter::All;
        }
        if widgets::chip(
            ui,
            t.filter_missed,
            app.call_filter == CallFilter::Missed,
            pal,
        ) {
            app.call_filter = CallFilter::Missed;
        }
    });
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .id_salt("call-list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let active: Vec<_> = app
                .state
                .calls
                .iter()
                .filter(|c| !matches!(c.state, CallState::Ended { .. }))
                .cloned()
                .collect();
            if !active.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    widgets::section(ui, t.active_call, pal);
                });
                for c in active {
                    let name = app.state.name_of(&c.peer);
                    let state = state_text(app, c.state, c.held);
                    list_row(
                        ui,
                        pal,
                        &name,
                        &state,
                        &format::clock(c.started, app.utc_offset),
                        true,
                    );
                }
                ui.add_space(10.0);
            }
            let history: Vec<_> = app
                .state
                .call_history
                .iter()
                .filter(|r| {
                    app.call_filter == CallFilter::All || r.direction == CallDirection::Missed
                })
                .cloned()
                .collect();
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                widgets::section(ui, t.recent_calls, pal);
            });
            if history.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(t.no_calls).color(pal.muted));
                });
            }
            for r in history {
                let name = app.state.name_of(&r.peer);
                let dir = match r.direction {
                    CallDirection::Incoming => t.incoming,
                    CallDirection::Outgoing => t.outgoing,
                    CallDirection::Missed => t.missed,
                };
                let mut sub = format!("{dir} · {}", format::clock(r.at, app.utc_offset));
                if r.seconds > 0 {
                    sub.push_str(&format!(" · {}", format::duration(r.seconds)));
                } else if r.status >= 300 {
                    sub.push_str(&format!(" · {} {}", t.status_code, r.status));
                }
                let when =
                    if format::day(r.at, app.utc_offset) == format::day(app.now, app.utc_offset) {
                        t.today.to_string()
                    } else {
                        format::date(r.at, app.utc_offset)
                    };
                let resp = list_row(ui, pal, &name, &sub, &when, false);
                if resp.double_clicked()
                    && app.state.capabilities.audio_call.usable()
                    && app.state.active_call().is_none()
                {
                    app.emit(UiIntent::CallUser {
                        peer: r.peer.clone(),
                    });
                }
                resp.on_hover_text(t.call_back_tip);
            }
        });
}

fn list_row(
    ui: &mut Ui,
    pal: &Palette,
    name: &str,
    sub: &str,
    right: &str,
    selected: bool,
) -> egui::Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 68.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, CornerRadius::same(10), pal.primary_soft);
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(10), pal.surface_subtle);
    }
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(10.0, 12.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    widgets::avatar(&mut child, name, 42.0, None, pal);
    child.add_space(4.0);
    child.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.set_width(ui.available_width());
            ui.allocate_ui(Vec2::new(ui.available_width() - 60.0, 18.0), |ui| {
                widgets::ellipsis(
                    ui,
                    RichText::new(name)
                        .size(theme::text::NAME)
                        .family(crate::theme::bold())
                        .color(pal.ink),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(right)
                        .size(theme::text::SMALL)
                        .color(pal.muted),
                );
            });
        });
        widgets::ellipsis(
            ui,
            RichText::new(sub).size(theme::text::SMALL).color(pal.muted),
        );
    });
    let info = format!("{name}, {sub}");
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &info));
    resp
}

/// The call state in words; "connected" only after the session reported the answer.
pub fn state_text(app: &CommsUi, s: CallState, held: bool) -> String {
    let t = app.t;
    match s {
        CallState::Dialing => t.call_dialing.to_string(),
        CallState::Ringing => t.call_ringing.to_string(),
        CallState::Incoming => t.call_incoming.to_string(),
        CallState::Connected { since } if held => format!(
            "{} · {}",
            t.call_on_hold,
            format::duration(app.now.saturating_sub(since))
        ),
        CallState::Connected { since } => format!(
            "{} · {}",
            t.call_connected,
            format::duration(app.now.saturating_sub(since))
        ),
        CallState::Ended { status: 0 } => t.call_ended.to_string(),
        CallState::Ended { status } => format!("{} ({status})", t.call_ended),
    }
}

pub fn main(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let Some(call) = app.state.active_call().cloned() else {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);
            ui.label(
                RichText::new(t.no_active_call)
                    .size(theme::text::PANEL_TITLE)
                    .color(pal.ink),
            );
            ui.label(
                RichText::new(match &app.state.capabilities.audio_call {
                    Gate::Unavailable(r) => t.reason(*r),
                    _ => t.no_active_call_hint,
                })
                .color(pal.muted),
            );
        });
        return;
    };
    let name = app.state.name_of(&call.peer);
    let caps = app.state.capabilities.clone();
    let connected = matches!(call.state, CallState::Connected { .. });
    egui::ScrollArea::vertical()
        .id_salt("call-main")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(28.0);
            ui.vertical_centered(|ui| {
                ui.allocate_ui(Vec2::new(ui.available_width().min(640.0), 10.0), |ui| {
                    widgets::card(ui, pal, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            ui.add_space(18.0);
                            widgets::avatar(ui, &name, 150.0, None, pal);
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(&name)
                                    .size(theme::text::HEADING)
                                    .family(crate::theme::bold())
                                    .color(pal.ink),
                            );
                            let colour = if connected && !call.held {
                                pal.success_text
                            } else {
                                pal.muted
                            };
                            let state = if call.kind == CallKind::Screen
                                && call.state == CallState::Incoming
                            {
                                t.screen_offered.to_string()
                            } else {
                                state_text(app, call.state, call.held)
                            };
                            ui.label(RichText::new(state).size(theme::text::NAME).color(colour));
                            let video = match &call.video {
                                VideoState::None => None,
                                VideoState::Starting => Some(t.video_starting.to_string()),
                                VideoState::Window => Some(t.video_window.to_string()),
                                VideoState::Failed(why) => {
                                    Some(format!("{}: {why}", t.video_failed))
                                }
                            };
                            if let Some(v) = video {
                                ui.label(
                                    RichText::new(v)
                                        .size(theme::text::SMALL + 0.5)
                                        .color(pal.muted),
                                );
                            }
                            if call.demo {
                                ui.label(
                                    RichText::new(t.demo_call)
                                        .size(theme::text::SMALL)
                                        .color(pal.muted),
                                );
                            }
                            ui.add_space(14.0);
                            widgets::rule(ui, pal);
                            ui.add_space(14.0);
                            if call.state == CallState::Incoming {
                                let screen = call.kind == CallKind::Screen;
                                let with_video = !screen && call.offers_video;
                                let buttons = if with_video { 3.0 } else { 2.0 };
                                ui.horizontal(|ui| {
                                    ui.add_space(
                                        ((ui.available_width()
                                            - buttons * 160.0
                                            - (buttons - 1.0) * 12.0)
                                            / 2.0)
                                            .max(0.0),
                                    );
                                    // As large as the red button next to it.
                                    let green = |label: &str| {
                                        egui::Button::new(
                                            RichText::new(label)
                                                .color(pal.on_primary)
                                                .family(crate::theme::bold()),
                                        )
                                        .fill(pal.success_text)
                                        .stroke(egui::Stroke::NONE)
                                        .min_size(Vec2::new(160.0, 40.0))
                                    };
                                    if screen {
                                        let r = ui.add_enabled(
                                            caps.video_call.usable(),
                                            green(t.view_screen),
                                        );
                                        if gate_tip(r, &caps.video_call, app).clicked() {
                                            app.emit(UiIntent::ViewScreen { id: call.id });
                                        }
                                    } else {
                                        let answer = green(t.answer);
                                        if ui
                                            .add_enabled(caps.audio_call.usable(), answer)
                                            .clicked()
                                        {
                                            app.emit(UiIntent::AnswerCall { id: call.id });
                                        }
                                    }
                                    if with_video {
                                        let r = ui.add_enabled(
                                            caps.video_call.usable(),
                                            green(t.answer_video),
                                        );
                                        if gate_tip(r, &caps.video_call, app).clicked() {
                                            app.emit(UiIntent::AnswerVideo { id: call.id });
                                        }
                                    }
                                    if widgets::danger(ui, t.decline, pal).clicked() {
                                        app.emit(UiIntent::HangUp { id: call.id });
                                    }
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    let total = 4.0 * 100.0 + 3.0 * 24.0;
                                    ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                                    ui.spacing_mut().item_spacing.x = 24.0;
                                    let m = widgets::call_tile(
                                        ui,
                                        t.mono_mute,
                                        if call.muted { t.unmute } else { t.mute },
                                        call.muted,
                                        connected,
                                        pal,
                                    );
                                    if m.clicked() {
                                        app.emit(UiIntent::Mute {
                                            id: call.id,
                                            muted: !call.muted,
                                        });
                                    }
                                    let hold_ok = connected && caps.hold.usable();
                                    let h = widgets::call_tile(
                                        ui,
                                        t.mono_hold,
                                        if call.held { t.resume } else { t.hold },
                                        call.held,
                                        hold_ok,
                                        pal,
                                    );
                                    let h = gate_tip(h, &caps.hold, app);
                                    if h.clicked() && hold_ok {
                                        app.emit(UiIntent::Hold {
                                            id: call.id,
                                            hold: !call.held,
                                        });
                                    }
                                    // Video is chosen when the call starts or is answered.
                                    let v = widgets::call_tile(
                                        ui,
                                        t.mono_video,
                                        t.video,
                                        call.kind != CallKind::Audio
                                            && call.video == VideoState::Window,
                                        false,
                                        pal,
                                    );
                                    gate_tip(v, &caps.video_call, app);
                                    let transfer_ok = connected && caps.transfer.usable();
                                    let x = widgets::call_tile(
                                        ui,
                                        t.mono_transfer,
                                        t.transfer,
                                        false,
                                        transfer_ok,
                                        pal,
                                    );
                                    let x = gate_tip(x, &caps.transfer, app);
                                    if x.clicked() && transfer_ok {
                                        app.new_chat = Some(crate::app::NewChat {
                                            text: String::new(),
                                            purpose: crate::app::AddressFor::Transfer(call.id),
                                        });
                                    }
                                });
                            }
                            ui.add_space(18.0);
                            if call.state != CallState::Incoming
                                && widgets::danger(ui, t.hang_up, pal).clicked()
                            {
                                app.emit(UiIntent::HangUp { id: call.id });
                            }
                            ui.add_space(10.0);
                        });
                    });
                    ui.add_space(18.0);
                    widgets::hint(
                        ui,
                        pal,
                        Some(t.call_hint_title),
                        if app.demo {
                            t.call_hint_demo
                        } else {
                            t.call_hint
                        },
                    );
                });
            });
        });
}

fn gate_tip(r: egui::Response, gate: &Gate, app: &CommsUi) -> egui::Response {
    match gate {
        Gate::Unavailable(why) => r.on_hover_text(app.t.reason(*why)),
        Gate::Preview => r.on_hover_text(app.t.preview_hint),
        Gate::Available => r,
    }
}

pub fn context(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    context_title(app, ui, pal, t.call_details);
    match app.state.active_call().cloned() {
        Some(c) => {
            context_block(
                ui,
                pal,
                t.call_kind,
                match (c.conference.is_some(), c.kind) {
                    (true, _) => t.call_kind_conference,
                    (false, CallKind::Video) => t.call_kind_video,
                    (false, CallKind::Screen) => t.call_kind_screen,
                    (false, CallKind::Audio) => t.call_kind_audio,
                },
            );
            context_block(
                ui,
                pal,
                t.participants,
                &format!("{} · {}", app.state.name_of(&c.peer), t.you),
            );
            context_block(
                ui,
                pal,
                t.connection_state,
                &state_text(app, c.state, c.held),
            );
        }
        None => {
            ui.label(RichText::new(t.no_active_call).color(pal.muted));
            ui.add_space(12.0);
        }
    }
    context_block(ui, pal, t.audio_devices, t.audio_devices_body);
    context_block(ui, pal, t.privacy, t.privacy_body);
}
