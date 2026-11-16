//! Meetings: own scheduled meetings, invitations, joining by link or focus.

use crate::app::{CommsUi, UiIntent};
use crate::format;
use crate::model::Gate;
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
                RichText::new(t.meetings)
                    .size(theme::text::PANEL_TITLE + 2.0)
                    .family(crate::theme::bold())
                    .color(pal.ink),
            );
            ui.label(
                RichText::new(t.meetings_sub)
                    .size(theme::text::SMALL + 0.5)
                    .color(pal.muted),
            );
        });
    });
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let gate = app.state.capabilities.meetings.clone();
        if let Some(r) = widgets::gated(ui, t.refresh, &gate, t, pal, |ui, l, en| {
            ui.add_enabled(en, egui::Button::new(l))
        }) {
            if r.clicked() {
                app.emit(UiIntent::RefreshMeetings);
            }
        }
    });
    ui.add_space(10.0);
    egui::ScrollArea::vertical()
        .id_salt("meeting-list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if !app.state.invitations.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    widgets::section(ui, t.invitations, pal);
                });
                let invitations = app.state.invitations.clone();
                for (i, inv) in invitations.iter().enumerate() {
                    widgets::card(ui, pal, |ui| {
                        ui.set_width(ui.available_width());
                        let from = app.state.name_of(&inv.from);
                        ui.label(
                            RichText::new(if inv.subject.is_empty() {
                                t.group_chat
                            } else {
                                &inv.subject
                            })
                            .family(crate::theme::bold())
                            .color(pal.ink),
                        );
                        ui.label(
                            RichText::new(format!("{} {from}", t.invited_by))
                                .size(theme::text::SMALL)
                                .color(pal.muted),
                        );
                        ui.horizontal(|ui| {
                            if widgets::primary(ui, t.join, true, pal).clicked() {
                                app.emit(UiIntent::AcceptInvitation { index: i });
                            }
                            if widgets::secondary(ui, t.decline, true, pal).clicked() {
                                app.emit(UiIntent::DeclineInvitation { index: i });
                            }
                        });
                    });
                    ui.add_space(6.0);
                }
            }
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                widgets::section(ui, t.my_meetings, pal);
            });
            if app.state.meetings.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(t.no_meetings).color(pal.muted));
                });
            }
            let meetings = app.state.meetings.clone();
            for (i, m) in meetings.iter().enumerate() {
                let selected = app.selected_meeting == Some(i);
                let w = ui.available_width();
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 64.0), Sense::click());
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
                widgets::avatar(&mut child, &m.subject, 40.0, None, pal);
                child.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    widgets::ellipsis(
                        ui,
                        RichText::new(if m.subject.is_empty() {
                            t.untitled_meeting
                        } else {
                            &m.subject
                        })
                        .size(theme::text::NAME)
                        .family(crate::theme::bold())
                        .color(pal.ink),
                    );
                    let sub = match m.expires {
                        Some(e) => format!("{} {}", t.valid_until, format::date(e, app.utc_offset)),
                        None => m.conference_id.clone(),
                    };
                    widgets::ellipsis(
                        ui,
                        RichText::new(sub).size(theme::text::SMALL).color(pal.muted),
                    );
                });
                let info = m.subject.clone();
                resp.widget_info(|| {
                    egui::WidgetInfo::selected(egui::Role::Button, true, selected, &info)
                });
                if resp.clicked() {
                    app.selected_meeting = Some(i);
                }
            }
        });
}

pub fn main(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let gate = app.state.capabilities.meetings.clone();
    let selected = app
        .selected_meeting
        .and_then(|i| app.state.meetings.get(i))
        .cloned();
    egui::ScrollArea::vertical()
        .id_salt("meeting-main")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.allocate_ui(Vec2::new(ui.available_width().min(640.0), 10.0), |ui| {
                    match selected {
                        Some(m) => widgets::card(ui, pal, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                RichText::new(if m.subject.is_empty() {
                                    t.untitled_meeting
                                } else {
                                    &m.subject
                                })
                                .size(theme::text::HEADING)
                                .family(crate::theme::bold())
                                .color(pal.ink),
                            );
                            if m.demo {
                                ui.label(
                                    RichText::new(t.demo_meeting)
                                        .size(theme::text::SMALL)
                                        .color(pal.muted),
                                );
                            }
                            ui.add_space(8.0);
                            egui::Grid::new("meeting-facts")
                                .num_columns(2)
                                .spacing([30.0, 8.0])
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(t.conference_id)
                                            .size(theme::text::SMALL)
                                            .color(pal.muted),
                                    );
                                    ui.label(RichText::new(&m.conference_id).color(pal.ink));
                                    ui.end_row();
                                    if let Some(e) = m.expires {
                                        ui.label(
                                            RichText::new(t.valid_until)
                                                .size(theme::text::SMALL)
                                                .color(pal.muted),
                                        );
                                        ui.label(
                                            RichText::new(format::date(e, app.utc_offset))
                                                .color(pal.ink),
                                        );
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(12.0);
                            if let Some(r) =
                                widgets::gated(ui, t.join, &gate, t, pal, |ui, l, en| {
                                    widgets::primary(ui, l, en, pal)
                                })
                            {
                                if r.clicked() {
                                    app.emit(UiIntent::JoinMeeting {
                                        focus: m.focus.clone(),
                                    });
                                }
                            }
                        }),
                        None => {
                            ui.label(
                                RichText::new(t.meetings_title)
                                    .size(theme::text::PANEL_TITLE)
                                    .color(pal.ink),
                            );
                            ui.label(
                                RichText::new(match &gate {
                                    Gate::Unavailable(r) => t.reason(*r),
                                    _ => t.meetings_hint,
                                })
                                .color(pal.muted),
                            );
                        }
                    }
                    ui.add_space(16.0);
                    let start = app.state.capabilities.group_chat.clone();
                    if let Some(r) = widgets::gated(ui, t.meet_now, &start, t, pal, |ui, l, en| {
                        widgets::secondary(ui, l, en, pal)
                    }) {
                        if r.on_hover_text(t.meet_now_tip).clicked() {
                            app.emit(UiIntent::StartGroup { invite: Vec::new() });
                            app.set_view(crate::model::View::Chats);
                        }
                    }
                    ui.add_space(16.0);
                    widgets::hint(
                        ui,
                        pal,
                        Some(t.meeting_content_title),
                        t.meeting_content_body,
                    );
                });
            });
        });
}

pub fn context(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    context_title(app, ui, pal, t.meeting_details);
    context_block(ui, pal, t.meeting_audio, t.meeting_audio_body);
    context_block(ui, pal, t.meeting_sharing, t.meeting_sharing_body);
    context_block(
        ui,
        pal,
        t.persistent_chat,
        t.reason(match &app.state.capabilities.persistent_chat {
            Gate::Unavailable(r) => *r,
            _ => crate::model::Reason::NotImplemented,
        }),
    );
}
