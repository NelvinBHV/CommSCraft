//! The frame around the views: top bar, rail, list panel, main panel, context panel, status bar.

use crate::app::{CommsUi, UiIntent};
use crate::model::{Connection, NoticeKind, Presence, View};
use crate::theme::{self, size, Palette};
use crate::views;
use crate::widgets;
use egui::{
    Align, Align2, CornerRadius, FontId, Frame, Layout, Margin, RichText, Sense, Stroke, Ui, Vec2,
};

fn panel_frame(fill: egui::Color32, margin: Margin) -> Frame {
    Frame::new().fill(fill).inner_margin(margin)
}

pub fn topbar(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    egui::Panel::top("topbar")
        .exact_size(size::TOPBAR)
        .frame(
            panel_frame(pal.surface, Margin::symmetric(20, 10))
                .stroke(Stroke::new(1.0, pal.border)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                brand(app, ui, pal);
                let w = ui.available_width();
                let search_w = (w - 340.0).clamp(240.0, 560.0);
                ui.add_space(((w - search_w) / 2.0 - 230.0).max(16.0));
                if !matches!(app.state.connection, Connection::SignedOut) || app.demo {
                    search_box(app, ui, pal, search_w);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    profile(app, ui, pal);
                });
            });
        });
}

fn brand(app: &CommsUi, ui: &mut Ui, pal: &Palette) {
    match &app.logo {
        Some(tex) => {
            ui.add(egui::Image::new((tex.id(), Vec2::splat(38.0))).alt_text("CommSCraft"));
        }
        None => {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::hover());
            ui.painter()
                .rect_filled(r, CornerRadius::same(9), pal.primary_soft);
        }
    }
    ui.vertical(|ui| {
        ui.add_space(2.0);
        ui.label(
            RichText::new("CommSCraft")
                .size(20.0)
                .family(crate::theme::bold())
                .color(pal.ink),
        );
        ui.label(
            RichText::new(app.t.tagline)
                .size(10.0)
                .color(pal.muted)
                .family(crate::theme::bold()),
        );
    });
}

fn search_box(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, width: f32) {
    Frame::new()
        .fill(pal.surface_subtle)
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                let edit = egui::TextEdit::singleline(&mut app.search)
                    .hint_text(app.t.search_hint)
                    .frame(Frame::NONE)
                    .char_limit(128)
                    .desired_width(width - 70.0);
                let r = ui.add(edit);
                if app.focus_search {
                    r.request_focus();
                    app.focus_search = false;
                }
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "search"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    Frame::new()
                        .stroke(Stroke::new(1.0, pal.border))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(6, 1))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(app.t.search_shortcut)
                                    .size(theme::text::SMALL)
                                    .color(pal.muted),
                            );
                        });
                });
            });
        });
}

fn profile(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let name = if app.state.me.name.is_empty() {
        app.state.me.uri.clone()
    } else {
        app.state.me.name.clone()
    };
    if name.is_empty() {
        return;
    }
    let presence = app.state.me.presence;
    let r = ui
        .scope(|ui| {
            widgets::avatar(
                ui,
                &name,
                38.0,
                (app.state.connection.is_online() || app.demo).then_some(presence),
                pal,
            )
        })
        .response
        .interact(Sense::click());
    ui.vertical(|ui| {
        ui.add_space(4.0);
        ui.with_layout(Layout::top_down(Align::Max), |ui| {
            let org = if app.state.me.organisation.is_empty() {
                name.clone()
            } else {
                app.state.me.organisation.clone()
            };
            ui.label(
                RichText::new(org)
                    .family(crate::theme::bold())
                    .color(pal.ink),
            );
            ui.label(
                RichText::new(app.t.connection_short(&app.state.connection, app.demo))
                    .size(theme::text::SMALL)
                    .color(pal.muted),
            );
        });
    });
    let r = r.on_hover_text(app.t.profile_menu);
    egui::Popup::menu(&r).show(|ui| {
        ui.set_min_width(220.0);
        ui.label(RichText::new(app.t.my_status).family(crate::theme::bold()));
        let usable = app.state.capabilities.presence.usable();
        for p in [
            Presence::Available,
            Presence::Busy,
            Presence::DoNotDisturb,
            Presence::Away,
        ] {
            let selected = app.state.me.presence == p;
            let b = ui.add_enabled(
                usable,
                egui::Button::selectable(selected, app.t.presence(p)),
            );
            if b.clicked() {
                app.emit(UiIntent::SetPresence(p));
            }
        }
        ui.separator();
        if ui.button(app.t.sign_out).clicked() {
            app.emit(UiIntent::SignOut);
        }
    });
}

struct RailItem {
    view: View,
    mono: &'static str,
    label: &'static str,
}

pub fn rail(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    egui::Panel::left("rail")
        .exact_size(size::RAIL)
        .resizable(false)
        .frame(
            panel_frame(pal.navigation_bg, Margin::symmetric(8, 18))
                .stroke(Stroke::new(1.0, pal.border)),
        )
        .show(ui, |ui| {
            let items = [
                RailItem {
                    view: View::Chats,
                    mono: t.mono_chats,
                    label: t.chats,
                },
                RailItem {
                    view: View::Contacts,
                    mono: t.mono_contacts,
                    label: t.contacts,
                },
                RailItem {
                    view: View::Calls,
                    mono: t.mono_calls,
                    label: t.calls,
                },
                RailItem {
                    view: View::Meetings,
                    mono: t.mono_meetings,
                    label: t.meetings,
                },
            ];
            let unread = app.state.unread_total();
            for item in items {
                let badge = match item.view {
                    View::Chats => unread,
                    View::Calls => usize::from(app.state.active_call().is_some()),
                    _ => 0,
                };
                if rail_button(ui, &item, app.view == item.view, badge, pal).clicked() {
                    app.view = item.view;
                }
                ui.add_space(14.0);
            }
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                let s = RailItem {
                    view: View::Settings,
                    mono: t.mono_settings,
                    label: t.settings,
                };
                if rail_button(ui, &s, app.view == View::Settings, 0, pal).clicked() {
                    app.view = View::Settings;
                }
            });
        });
}

fn rail_button(
    ui: &mut Ui,
    item: &RailItem,
    active: bool,
    badge: usize,
    pal: &Palette,
) -> egui::Response {
    let size = Vec2::new(size::RAIL - 16.0, 80.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter();
    if active {
        p.rect_filled(rect, CornerRadius::same(10), pal.primary_soft);
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(10), pal.surface);
    }
    if resp.has_focus() {
        p.rect_stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(2.0, pal.primary),
            egui::StrokeKind::Inside,
        );
    }
    let icon =
        egui::Rect::from_center_size(rect.center_top() + Vec2::new(0.0, 27.0), Vec2::splat(32.0));
    p.rect_filled(
        icon,
        CornerRadius::same(9),
        if active {
            pal.surface
        } else {
            pal.primary_soft
        },
    );
    p.text(
        icon.center(),
        Align2::CENTER_CENTER,
        item.mono,
        FontId::proportional(11.0),
        pal.primary_hover,
    );
    p.text(
        rect.center_bottom() - Vec2::new(0.0, 16.0),
        Align2::CENTER_CENTER,
        item.label,
        FontId::proportional(11.5),
        if active { pal.primary_hover } else { pal.ink },
    );
    if badge > 0 {
        let c = icon.right_top() + Vec2::new(2.0, 2.0);
        p.circle_filled(c, 9.0, pal.primary);
        p.text(
            c,
            Align2::CENTER_CENTER,
            if badge > 99 {
                "99+".to_string()
            } else {
                badge.to_string()
            },
            FontId::proportional(10.0),
            pal.on_primary,
        );
    }
    let label = item.label.to_string();
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, &label));
    resp
}

pub fn list_panel(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    egui::Panel::left("list")
        .exact_size(size::LIST)
        .resizable(false)
        .frame(
            panel_frame(pal.surface, Margin::symmetric(10, 18))
                .stroke(Stroke::new(1.0, pal.border)),
        )
        .show(ui, |ui| {
            if !app.search.trim().is_empty() {
                views::search::results(app, ui, pal);
                return;
            }
            match app.view {
                View::Chats => views::chats::list(app, ui, pal),
                View::Contacts => views::contacts::list(app, ui, pal),
                View::Calls => views::calls::list(app, ui, pal),
                View::Meetings => views::meetings::list(app, ui, pal),
                View::Settings => views::settings::list(app, ui, pal),
            }
        });
}

pub fn main_panel(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    egui::CentralPanel::no_frame().show(ui, |ui| {
        Frame::new().fill(pal.surface).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            match app.view {
                View::Chats => views::chats::main(app, ui, pal),
                View::Contacts => views::contacts::main(app, ui, pal),
                View::Calls => views::calls::main(app, ui, pal),
                View::Meetings => views::meetings::main(app, ui, pal),
                View::Settings => views::settings::main(app, ui, pal),
            }
        });
    });
}

pub fn context_panel(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    egui::Panel::right("context")
        .exact_size(size::CONTEXT)
        .resizable(false)
        .frame(
            panel_frame(pal.surface, Margin::symmetric(22, 22))
                .stroke(Stroke::new(1.0, pal.border)),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("context-scroll")
                .show(ui, |ui| match app.view {
                    View::Chats => views::chats::context(app, ui, pal),
                    View::Contacts => views::contacts::context(app, ui, pal),
                    View::Calls => views::calls::context(app, ui, pal),
                    View::Meetings => views::meetings::context(app, ui, pal),
                    View::Settings => {}
                });
        });
}

/// A titled block of the context panel with a rule above it.
pub fn context_block(ui: &mut Ui, pal: &Palette, title: &str, body: &str) {
    widgets::rule(ui, pal);
    ui.add_space(10.0);
    ui.label(
        RichText::new(title)
            .family(crate::theme::bold())
            .color(pal.ink),
    );
    ui.label(RichText::new(body).color(pal.muted));
    ui.add_space(14.0);
}

/// The panel title of the context panel with a close button.
pub fn context_title(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, title: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .size(theme::text::PANEL_TITLE - 2.0)
                .family(crate::theme::bold())
                .color(pal.ink),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .small_button("×")
                .on_hover_text(app.t.hide_context)
                .clicked()
            {
                app.prefs.context_open = false;
                app.emit(UiIntent::PrefsChanged);
            }
        });
    });
    ui.add_space(12.0);
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
