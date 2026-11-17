//! Meetings: own scheduled meetings, invitations, joining by link or focus.

use crate::app::MeetingDialog;
use crate::app::{CommsUi, UiIntent};
use crate::format;
use crate::model::MeetingAccess;
use crate::theme::{self, Palette};
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
        // Plan a meeting (meet now is in the main window's menu).
        if let Some(r) = widgets::gated(ui, t.new_meeting_dots, &gate, t, pal, |ui, l, en| {
            ui.add_enabled(en, egui::Button::new(l))
        }) {
            if r.clicked() {
                app.meeting_dialog = Some(MeetingDialog {
                    change: None,
                    subject: String::new(),
                    minutes: 60,
                    access: MeetingAccess::Organisation,
                });
            }
        }
    });
    // Join by a meeting link (https, from an invitation mail or calendar entry).
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let usable = app.state.capabilities.meetings.usable();
        let link = app.meeting_link.trim().to_string();
        let ok = usable && link.starts_with("https://");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(8.0);
            let join = ui.add_enabled(ok, egui::Button::new(t.join)).clicked();
            let r = ui.add_enabled(
                usable,
                egui::TextEdit::singleline(&mut app.meeting_link)
                    .hint_text(t.meeting_link_hint)
                    .char_limit(2048)
                    .desired_width(ui.available_width()),
            );
            r.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::TextInput, usable, t.meeting_link_hint)
            });
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ok && (join || enter) {
                app.emit(UiIntent::JoinMeetingLink { link });
                app.meeting_link.clear();
            }
        });
    });
    ui.add_space(10.0);
    egui::ScrollArea::vertical()
        .id_salt("meeting-list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            calendar(app, ui, pal);
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
            // Planned meetings have a subject; ad-hoc conferences (from group conversations)
            // have none and are only counted.
            let meetings = app.state.meetings.clone();
            let untitled = meetings
                .iter()
                .filter(|m| m.subject.trim().is_empty())
                .count();
            if meetings.len() == untitled {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(t.no_meetings).color(pal.muted));
                });
            }
            if untitled > 0 {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("{untitled} {}", t.untitled_conferences))
                                .size(theme::text::SMALL)
                                .color(pal.muted),
                        )
                        .wrap(),
                    );
                });
            }
            for (i, m) in meetings.iter().enumerate() {
                if m.subject.trim().is_empty() {
                    continue;
                }
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
                if selected {
                    meeting_actions(app, ui, m);
                }
            }
        });
}

/// What can be done with the chosen meeting: join, copy its link, change, cancel.
fn meeting_actions(app: &mut CommsUi, ui: &mut Ui, m: &crate::model::Meeting) {
    let t = app.t;
    let usable = app.state.capabilities.meetings.usable();
    ui.horizontal(|ui| {
        ui.add_space(58.0);
        if ui.add_enabled(usable, egui::Button::new(t.join)).clicked() {
            app.emit(UiIntent::JoinMeeting {
                focus: m.focus.clone(),
            });
        }
        if let Some(url) = &m.join_url {
            if ui.button(t.copy_link).on_hover_text(url).clicked() {
                ui.ctx().copy_text(url.clone());
            }
        }
        if ui
            .add_enabled(usable, egui::Button::new(t.change_dots))
            .clicked()
        {
            app.meeting_dialog = Some(MeetingDialog {
                change: Some(m.conference_id.clone()),
                subject: m.subject.clone(),
                minutes: 60,
                access: MeetingAccess::Organisation,
            });
        }
        if ui
            .add_enabled(usable, egui::Button::new(t.cancel_meeting))
            .clicked()
        {
            app.emit(UiIntent::CancelMeeting {
                conference_id: m.conference_id.clone(),
            });
            app.selected_meeting = None;
        }
    });
    ui.add_space(6.0);
}

/// Scheduling or changing a meeting: subject, how long it stays usable, who may join.
pub fn meeting_dialog(app: &mut CommsUi, ctx: &egui::Context) {
    let t = app.t;
    let Some(mut d) = app.meeting_dialog.take() else {
        return;
    };
    let mut keep = true;
    egui::Modal::new(egui::Id::unique("meeting-dialog")).show(ctx, |ui| {
        ui.set_width(340.0);
        ui.label(
            RichText::new(if d.change.is_some() {
                t.change_meeting
            } else {
                t.new_meeting
            })
            .family(crate::theme::bold()),
        );
        ui.add_space(6.0);
        ui.label(t.meeting_subject);
        let r = ui.add(
            egui::TextEdit::singleline(&mut d.subject)
                .char_limit(256)
                .desired_width(320.0),
        );
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, t.meeting_subject));
        ui.horizontal(|ui| {
            ui.label(t.meeting_lifetime);
            ui.add(
                egui::DragValue::new(&mut d.minutes)
                    .range(15..=525_600)
                    .suffix(" min"),
            );
        });
        ui.label(t.meeting_access);
        for (a, label) in [
            (MeetingAccess::Invited, t.access_invited),
            (MeetingAccess::Organisation, t.access_organisation),
            (MeetingAccess::Anyone, t.access_anyone),
        ] {
            ui.radio_value(&mut d.access, a, label);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let ok = !d.subject.trim().is_empty() && !d.subject.chars().any(char::is_control);
            if ui.add_enabled(ok, egui::Button::new(t.ok)).clicked() {
                let subject = d.subject.trim().to_string();
                match &d.change {
                    Some(id) => app.emit(UiIntent::ChangeMeeting {
                        conference_id: id.clone(),
                        subject,
                        minutes: d.minutes,
                        access: d.access,
                    }),
                    None => app.emit(UiIntent::ScheduleMeeting {
                        subject,
                        minutes: d.minutes,
                        access: d.access,
                        join: false,
                    }),
                }
                keep = false;
            }
            if ui.button(t.cancel).clicked() {
                keep = false;
            }
        });
    });
    if keep {
        app.meeting_dialog = Some(d);
    }
}

/// The user's appointments from Exchange, by day, and the automatic reply when it is on.
fn calendar(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    if let Some(text) = app.state.out_of_office.clone() {
        widgets::hint(
            ui,
            pal,
            Some(t.oof_on),
            if text.is_empty() {
                t.oof_no_text
            } else {
                &text
            },
        );
        ui.add_space(8.0);
    }
    let Some(list) = app.state.appointments.clone() else {
        return;
    };
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        widgets::section(ui, t.calendar_title, pal);
    });
    let today = crate::format::day(app.now, app.utc_offset);
    let upcoming: Vec<_> = list
        .iter()
        .filter(|a| a.end > app.now || crate::format::day(a.start, app.utc_offset) == today)
        .collect();
    if upcoming.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new(t.calendar_empty).color(pal.muted));
        });
    }
    let mut last_day = None;
    for a in upcoming {
        let day = crate::format::day(a.start, app.utc_offset);
        if last_day != Some(day) {
            last_day = Some(day);
            let word = match day - today {
                0 => t.today.to_string(),
                1 => t.tomorrow.to_string(),
                _ => crate::format::date(a.start, app.utc_offset),
            };
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(word)
                        .size(theme::text::SMALL)
                        .family(crate::theme::bold())
                        .color(pal.muted),
                );
            });
        }
        let subject = if a.subject.is_empty() {
            t.private_appointment.to_string()
        } else {
            a.subject.clone()
        };
        let busy = match a.busy {
            crate::model::Busy::Free => t.busy_free,
            crate::model::Busy::Tentative => t.busy_tentative,
            crate::model::Busy::Busy => t.busy_busy,
            crate::model::Busy::OutOfOffice => t.busy_oof,
        };
        let now_running = a.start <= app.now && app.now < a.end;
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.vertical(|ui| {
                let head = format!(
                    "{}–{}  {subject}",
                    crate::format::clock(a.start, app.utc_offset),
                    crate::format::clock(a.end, app.utc_offset)
                );
                let text = RichText::new(head).color(pal.ink);
                ui.label(if now_running {
                    text.family(crate::theme::bold())
                } else {
                    text
                });
                let mut sub = busy.to_string();
                if !a.location.is_empty() {
                    sub = format!("{sub} · {}", a.location);
                }
                if now_running {
                    sub = format!("{} · {sub}", t.now_running);
                }
                ui.label(RichText::new(sub).size(theme::text::SMALL).color(pal.muted));
            });
        });
    }
    ui.add_space(10.0);
}
