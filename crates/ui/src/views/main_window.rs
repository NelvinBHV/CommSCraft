//! The main window: a narrow window with the user's presence on top, four tabs (contacts,
//! conversations, phone, meetings), the options menu and a footer. Conversations open in windows
//! of their own ([`crate::views::conv_window`]).

use crate::app::{CommsUi, ContactsView, ConvFilter, MainTab, UiIntent};
use crate::model::{CallDirection, Connection, Contact, ConvKey, NoticeKind, Presence};
use crate::theme::{self, Palette};
use crate::views::icons::{self, Icon};
use crate::widgets;
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, Sense, Stroke, Ui, Vec2};

/// The presences a user can choose (the session publishes them).
pub const CHOOSABLE: [Presence; 6] = [
    Presence::Available,
    Presence::Busy,
    Presence::DoNotDisturb,
    Presence::BeRightBack,
    Presence::OffWork,
    Presence::Away,
];

pub fn show(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    footer(app, ui, pal);
    rating(app, ui, pal);
    egui::Panel::top("main-head")
        .frame(
            Frame::new()
                .fill(pal.surface_subtle)
                .inner_margin(Margin::same(10)),
        )
        .show(ui, |ui| {
            header(app, ui, pal);
            ui.add_space(6.0);
            tabs(app, ui, pal);
        });
    alert(app, ui, pal);
    egui::CentralPanel::default()
        .frame(
            Frame::new()
                .fill(pal.surface)
                .inner_margin(Margin::symmetric(8, 6)),
        )
        .show(ui, |ui| match app.tab {
            MainTab::Contacts => contacts(app, ui, pal),
            MainTab::Conversations => conversations(app, ui, pal),
            MainTab::Meetings => crate::views::meetings::list(app, ui, pal),
            MainTab::Rooms => rooms(app, ui, pal),
        });
}

/// After a call: how was its quality? (sent to the server's quality service).
fn rating(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let Some((id, peer)) = app.state.rate_call.clone() else {
        return;
    };
    egui::Panel::bottom("call-rating")
        .frame(
            Frame::new()
                .fill(pal.primary_soft)
                .inner_margin(egui::Margin::symmetric(10, 8)),
        )
        .show(ui, |ui| {
            let name = app.state.name_of(&peer);
            ui.label(RichText::new(format!("{} {name}?", t.rate_call_question)).color(pal.ink));
            ui.horizontal(|ui| {
                // Five stars; the one under the pointer and those before it light up.
                let hovered = app.rating_hover;
                app.rating_hover = 0;
                for stars in 1u8..=5 {
                    let lit = stars <= hovered;
                    let r = icons::button(
                        ui,
                        Icon::Star,
                        &format!("{stars} {}", t.stars),
                        28.0,
                        lit,
                        true,
                        if lit { pal.primary } else { pal.muted },
                        pal.primary,
                    );
                    if r.hovered() {
                        app.rating_hover = stars;
                    }
                    if r.clicked() {
                        app.emit(UiIntent::RateCall { id, stars });
                        app.state.rate_call = None;
                    }
                }
                if ui.button(t.not_now).clicked() {
                    app.state.rate_call = None;
                }
            });
        });
}

fn header(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let online = app.state.connection.is_online() || app.demo;
    // The personal note: published when the field is left or Enter is pressed; a note set
    // elsewhere (another endpoint, the last session) comes into the field.
    if app.state.me.note != app.seen_note {
        app.seen_note = app.state.me.note.clone();
        app.note = app.seen_note.clone();
        app.published_note = app.seen_note.clone();
    }
    ui.horizontal(|ui| {
        let edit = egui::TextEdit::singleline(&mut app.note)
            .hint_text(t.note_hint)
            .char_limit(512)
            .frame(Frame::NONE)
            .desired_width(ui.available_width());
        let usable = app.state.capabilities.presence.usable();
        let r = ui.add_enabled(usable, edit);
        // Screen readers name the field like its hint.
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, usable, t.note_hint));
        if r.lost_focus() && app.note != app.published_note {
            app.published_note = app.note.clone();
            app.emit(UiIntent::SetNote(app.note.trim().to_string()));
        }
        let _ = online;
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let name = if app.state.me.name.is_empty() {
            app.state.me.uri.trim_start_matches("sip:").to_string()
        } else {
            app.state.me.name.clone()
        };
        let presence = app.state.me.presence;
        let me = app.state.me.uri.clone();
        app.avatar(
            ui,
            Some(me.as_str()),
            &name,
            52.0,
            online.then_some(presence),
            pal,
        );
        ui.vertical(|ui| {
            ui.label(
                RichText::new(&name)
                    .size(theme::text::NAME + 2.0)
                    .family(theme::bold())
                    .color(pal.ink),
            );
            let r = icons::menu_text(
                ui,
                RichText::new(t.presence(presence)).color(pal.ink),
                pal.muted,
            );
            let r = r.on_hover_text(t.change_status);
            egui::Popup::menu(&r).show(|ui| status_menu(app, ui));
            ui.label(
                RichText::new(t.connection_short(&app.state.connection, app.demo))
                    .size(theme::text::SMALL)
                    .color(pal.muted),
            );
        });
    });
}

/// The status menu: the presences, reset, sign out and quit.
pub fn status_menu(app: &mut CommsUi, ui: &mut Ui) {
    let t = app.t;
    ui.set_min_width(200.0);
    let usable = app.state.capabilities.presence.usable();
    for p in CHOOSABLE {
        let selected = app.state.me.presence == p;
        if ui
            .add_enabled(
                usable,
                egui::Button::selectable(
                    selected,
                    if p == Presence::Away {
                        t.appear_away
                    } else {
                        t.presence(p)
                    },
                ),
            )
            .clicked()
        {
            app.emit(UiIntent::SetPresence(p));
        }
    }
    // Back to the state the server works out from activity and calendar.
    if ui
        .add_enabled(usable, egui::Button::new(t.reset_status))
        .clicked()
    {
        app.emit(UiIntent::ResetStatus);
        ui.close();
    }
    ui.separator();
    if ui.button(t.sign_out).clicked() {
        app.emit(UiIntent::SignOut);
    }
    if ui.button(t.quit).clicked() {
        app.emit(UiIntent::Quit);
    }
}

fn tabs(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let missed = app
        .state
        .call_history
        .iter()
        .filter(|r| r.direction == CallDirection::Missed)
        .count()
        + app.state.unread_total();
    ui.horizontal(|ui| {
        for (tab, icon, label, badge) in [
            (MainTab::Contacts, Icon::Person, t.tab_contacts, 0),
            (
                MainTab::Conversations,
                Icon::Clock,
                t.tab_conversations,
                missed,
            ),
            (
                MainTab::Meetings,
                Icon::Calendar,
                t.tab_meetings,
                app.state.invitations.len(),
            ),
        ] {
            let r = icons::button(
                ui,
                icon,
                label,
                44.0,
                app.tab == tab,
                true,
                pal.muted,
                pal.primary,
            )
            .on_hover_text(label);
            if badge > 0 {
                let c = r.rect.right_top() + Vec2::new(-8.0, 8.0);
                ui.painter().circle_filled(c, 8.0, pal.error);
                ui.painter().text(
                    c,
                    egui::Align2::CENTER_CENTER,
                    badge.min(99).to_string(),
                    egui::FontId::proportional(9.5),
                    pal.on_primary,
                );
            }
            if r.clicked() {
                app.tab = tab;
            }
        }
        // Chat rooms: a tab of their own where the server offers persistent chat.
        let rooms = app.state.capabilities.persistent_chat.clone();
        if rooms.usable() {
            let r = icons::button(
                ui,
                Icon::Chat,
                t.filter_rooms,
                44.0,
                app.tab == MainTab::Rooms,
                true,
                pal.muted,
                pal.primary,
            )
            .on_hover_text(t.filter_rooms);
            if r.clicked() {
                app.tab = MainTab::Rooms;
            }
        } else if app.tab == MainTab::Rooms {
            app.tab = MainTab::Contacts;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let menu = icons::button(
                ui,
                Icon::ChevronDown,
                t.main_menu,
                22.0,
                false,
                true,
                pal.muted,
                pal.primary,
            )
            .on_hover_text(t.main_menu);
            egui::Popup::menu(&menu).show(|ui| main_menu(app, ui));
            let gear = icons::button(
                ui,
                Icon::Gear,
                t.options,
                34.0,
                false,
                true,
                pal.muted,
                pal.primary,
            )
            .on_hover_text(t.options);
            if gear.clicked() {
                app.open_options(0);
            }
        });
    });
}

/// The main menu: file, meeting, tools, help.
pub fn main_menu(app: &mut CommsUi, ui: &mut Ui) {
    let t = app.t;
    ui.set_min_width(200.0);
    ui.menu_button(t.menu_file, |ui| {
        if ui.button(t.sign_out).clicked() {
            app.emit(UiIntent::SignOut);
        }
        ui.menu_button(t.my_status, |ui| status_menu(app, ui));
        if ui.button(t.view_history).clicked() {
            app.tab = MainTab::Conversations;
            ui.close();
        }
        ui.separator();
        if ui.button(t.close_window).clicked() {
            app.emit(UiIntent::Minimize);
        }
        if ui.button(t.quit).clicked() {
            app.emit(UiIntent::Quit);
        }
    });
    let meet = app.state.capabilities.group_chat.clone();
    if ui
        .add_enabled(meet.usable(), egui::Button::new(t.meet_now))
        .clicked()
    {
        app.emit(UiIntent::StartGroup { invite: Vec::new() });
        ui.close();
    }
    ui.menu_button(t.menu_tools, |ui| {
        let mut top = app.prefs.always_on_top;
        if ui.checkbox(&mut top, t.always_on_top).changed() {
            app.prefs.always_on_top = top;
            app.emit(UiIntent::PrefsChanged);
        }
        ui.separator();
        if ui.button(t.audio_device_settings).clicked() {
            app.open_options(crate::views::options::AUDIO);
            ui.close();
        }
        if ui.button(t.video_device_settings).clicked() {
            app.open_options(crate::views::options::VIDEO);
            ui.close();
        }
        if ui.button(t.options).clicked() {
            app.open_options(0);
            ui.close();
        }
    });
    ui.menu_button(t.menu_help, |ui| {
        if ui.button(t.about).clicked() {
            app.open_window(crate::app::Win::About);
            ui.close();
        }
    });
}

/// The newest warning or error, until the user closes it.
fn alert(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let Some(n) = app
        .state
        .notices
        .iter()
        .rev()
        .find(|n| n.kind != NoticeKind::Info)
        .filter(|n| n.at > app.dismissed_alert)
        .cloned()
    else {
        return;
    };
    egui::Panel::top("main-alert")
        .frame(
            Frame::new()
                .fill(pal.surface.lerp_to_gamma(pal.warning, 0.2))
                .inner_margin(Margin::symmetric(10, 6)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let w = ui.available_width() - 28.0;
                ui.allocate_ui(Vec2::new(w, 0.0), |ui| {
                    ui.label(
                        RichText::new(&n.text)
                            .size(theme::text::SMALL)
                            .color(pal.ink),
                    );
                });
                if ui
                    .small_button("×")
                    .on_hover_text(app.t.close_alert)
                    .clicked()
                {
                    app.dismissed_alert = n.at;
                }
            });
        });
}

fn footer(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    egui::Panel::bottom("main-foot")
        .exact_size(30.0)
        .frame(
            Frame::new()
                .fill(pal.surface_subtle)
                .inner_margin(Margin::symmetric(10, 4))
                .stroke(Stroke::new(1.0, pal.border)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let r = icons::button(
                    ui,
                    Icon::Speaker,
                    t.audio_device_settings,
                    22.0,
                    false,
                    true,
                    pal.muted,
                    pal.primary,
                )
                .on_hover_text(t.audio_device_settings);
                if r.clicked() {
                    app.open_options(crate::views::options::AUDIO);
                }
                // The state of the connection, in words, where it is not simply connected.
                let state = if app.demo {
                    Some((pal.warning, t.status_demo.to_string()))
                } else {
                    match &app.state.connection {
                        Connection::Degraded(why) => {
                            Some((pal.warning, format!("{} – {why}", t.status_degraded)))
                        }
                        Connection::Disconnected(why) => {
                            Some((pal.error, format!("{} – {why}", t.status_disconnected)))
                        }
                        _ => None,
                    }
                };
                if let Some((c, text)) = state {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 4.0, c);
                    widgets::ellipsis(
                        ui,
                        RichText::new(text).size(theme::text::SMALL).color(pal.ink),
                    );
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if matches!(app.state.connection, Connection::Disconnected(_))
                        && !app.demo
                        && ui.small_button(t.sign_in_again).clicked()
                    {
                        app.emit(UiIntent::SignOut);
                    }
                    if let Some(n) = app.state.notices.back() {
                        widgets::ellipsis(
                            ui,
                            RichText::new(&n.text)
                                .size(theme::text::SMALL)
                                .color(pal.muted),
                        );
                    }
                });
            });
        });
}

fn search_field(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, hint: &str) {
    Frame::new()
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let edit = egui::TextEdit::singleline(&mut app.search)
                    .hint_text(hint)
                    .frame(Frame::NONE)
                    .char_limit(128)
                    .desired_width(ui.available_width() - 24.0);
                let r = ui.add(edit);
                if app.focus_search {
                    r.request_focus();
                    app.focus_search = false;
                }
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "search"));
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                icons::draw(ui.painter(), rect, Icon::Search, pal.muted);
            });
        });
}

fn sub_tabs<T: Copy + PartialEq>(ui: &mut Ui, pal: &Palette, current: &mut T, items: &[(T, &str)]) {
    ui.horizontal(|ui| {
        for (v, label) in items {
            let selected = *current == *v;
            let text = RichText::new(label.to_uppercase())
                .size(theme::text::SMALL)
                .color(if selected { pal.ink } else { pal.muted });
            let text = if selected {
                text.family(theme::bold())
            } else {
                text
            };
            let r = ui.add(egui::Button::selectable(selected, text));
            if r.clicked() {
                *current = *v;
            }
        }
    });
}

fn contacts(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    ui.horizontal(|ui| {
        ui.allocate_ui(Vec2::new(ui.available_width() - 40.0, 30.0), |ui| {
            search_field(app, ui, pal, t.find_someone);
        });
        let can_edit = app.state.capabilities.presence.usable();
        let add = icons::button(
            ui,
            Icon::AddPerson,
            t.add_contact,
            30.0,
            false,
            can_edit,
            pal.muted,
            pal.primary,
        )
        .on_hover_text(t.add_contact);
        egui::Popup::menu(&add).show(|ui| {
            if ui
                .add_enabled(can_edit, egui::Button::new(t.add_contact_dots))
                .clicked()
            {
                app.new_chat = Some(crate::app::NewChat {
                    text: String::new(),
                    purpose: crate::app::AddressFor::AddContact,
                });
                ui.close();
            }
            if ui
                .add_enabled(can_edit, egui::Button::new(t.new_group_dots))
                .clicked()
            {
                app.group_dialog = Some((None, String::new()));
                ui.close();
            }
        });
    });
    if !app.search.trim().is_empty() {
        egui::ScrollArea::vertical()
            .id_salt("search")
            .auto_shrink([false, false])
            .show(ui, |ui| crate::views::search::results(app, ui, pal));
        return;
    }
    let mut view = app.contacts_view;
    sub_tabs(
        ui,
        pal,
        &mut view,
        &[
            (ContactsView::Groups, t.contacts_groups),
            (ContactsView::Status, t.contacts_status),
            (ContactsView::Relationships, t.contacts_relationships),
            (ContactsView::New, t.contacts_new),
        ],
    );
    app.contacts_view = view;
    ui.add_space(4.0);
    let mut all: Vec<Contact> = app.state.contacts.values().cloned().collect();
    if app.prefs.sort_by_availability {
        all.sort_by(|a, b| {
            a.presence
                .cmp(&b.presence)
                .then(a.display_name().cmp(b.display_name()))
        });
    } else {
        all.sort_by(|a, b| {
            a.display_name()
                .to_lowercase()
                .cmp(&b.display_name().to_lowercase())
        });
    }
    let groups: Vec<(String, Vec<Contact>)> = match app.contacts_view {
        ContactsView::Groups => {
            let mut out: Vec<(String, Vec<Contact>)> = Vec::new();
            if app.prefs.favorites_group {
                let favs: Vec<Contact> = all
                    .iter()
                    .filter(|c| app.prefs.favorites.contains(&c.uri) || c.pinned)
                    .cloned()
                    .collect();
                out.push((t.favorites.to_string(), favs));
            }
            let mut by_group: std::collections::BTreeMap<String, Vec<Contact>> = Default::default();
            let away = |p: Presence| {
                matches!(
                    p,
                    Presence::Away
                        | Presence::BeRightBack
                        | Presence::Inactive
                        | Presence::OffWork
                        | Presence::Offline
                        | Presence::Unknown
                )
            };
            let mut absent = Vec::new();
            for c in &all {
                if app.prefs.away_group && away(c.presence) {
                    absent.push(c.clone());
                    continue;
                }
                let g = if c.group.is_empty() {
                    t.other_contacts.to_string()
                } else {
                    c.group.clone()
                };
                by_group.entry(g).or_default().push(c.clone());
            }
            out.extend(by_group);
            if app.prefs.away_group {
                out.push((t.status_away_offline.to_string(), absent));
            }
            out
        }
        ContactsView::Status => {
            let mut out: Vec<(String, Vec<Contact>)> = Vec::new();
            for (label, f) in [
                (
                    t.status_online,
                    &(|p: Presence| matches!(p, Presence::Available)) as &dyn Fn(Presence) -> bool,
                ),
                (t.status_busy, &|p| {
                    matches!(
                        p,
                        Presence::Busy | Presence::InACall | Presence::DoNotDisturb
                    )
                }),
                (t.status_away, &|p| {
                    matches!(
                        p,
                        Presence::Away | Presence::BeRightBack | Presence::OffWork
                    )
                }),
                (t.status_offline, &|p| {
                    matches!(p, Presence::Offline | Presence::Unknown)
                }),
            ] {
                out.push((
                    label.to_string(),
                    all.iter().filter(|c| f(c.presence)).cloned().collect(),
                ));
            }
            out
        }
        ContactsView::New => {
            subscribers(app, ui, pal);
            return;
        }
        ContactsView::Relationships => {
            ui.label(RichText::new(t.relationships_unavailable).color(pal.muted));
            return;
        }
    };
    egui::ScrollArea::vertical()
        .id_salt("contacts")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if all.is_empty() && app.state.distribution.is_empty() {
                ui.label(RichText::new(t.no_contacts).color(pal.muted));
            }
            for (name, members) in groups {
                let open = !app.collapsed.contains(&name);
                let head = format!("{} ({})", name.to_uppercase(), members.len());
                let header = ui
                    .horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                        icons::draw(
                            ui.painter(),
                            rect,
                            if open {
                                Icon::ChevronDown
                            } else {
                                Icon::ChevronRight
                            },
                            pal.muted,
                        );
                        ui.add(
                            egui::Button::new(
                                RichText::new(head)
                                    .size(theme::text::SMALL)
                                    .color(pal.muted),
                            )
                            .frame(false),
                        )
                    })
                    .inner;
                if header.clicked() {
                    if open {
                        app.collapsed.insert(name.clone());
                    } else {
                        app.collapsed.remove(&name);
                    }
                }
                if app.state.groups.contains(&name) {
                    group_menu(app, &header, &name, members.is_empty());
                }
                if open {
                    for c in members {
                        contact_row(app, ui, pal, &c);
                    }
                }
                ui.add_space(4.0);
            }
            if app.contacts_view == ContactsView::Groups {
                dist_groups(app, ui, pal);
            }
        });
}

/// The row of a contact known by address (from the contact list or a search).
pub fn contact_row_of(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, uri: &str) {
    let c = app.state.contacts.get(uri).cloned().or_else(|| {
        app.state
            .search_results
            .iter()
            .find(|c| c.uri == uri)
            .cloned()
    });
    if let Some(c) = c {
        contact_row(app, ui, pal, &c);
    }
}

fn contact_row(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, c: &Contact) {
    let t = app.t;
    let two_lines = !app.prefs.compact_contacts;
    let h = if two_lines { 48.0 } else { 28.0 };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 4.0, pal.surface_subtle);
    }
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(4.0, 2.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    app.avatar(
        &mut child,
        Some(c.uri.as_str()),
        c.display_name(),
        if two_lines { 36.0 } else { 20.0 },
        Some(c.presence),
        pal,
    );
    child.vertical(|ui| {
        widgets::ellipsis(ui, RichText::new(c.display_name()).color(pal.ink));
        if two_lines {
            let sub = if c.note.is_empty() {
                t.presence(c.presence).to_string()
            } else {
                format!("{} – {}", t.presence(c.presence), c.note)
            };
            widgets::ellipsis(
                ui,
                RichText::new(sub).size(theme::text::SMALL).color(pal.muted),
            );
        }
    });
    let info = format!("{}, {}", c.display_name(), t.presence(c.presence));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &info));
    if resp.double_clicked() {
        app.chat_with(&c.uri);
    }
    resp.context_menu(|ui| contact_menu(app, ui, &c.uri));
}

/// The distribution groups of the contact list; a group's members are asked for from the
/// address book the first time it is open.
fn dist_groups(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    for g in app.state.distribution.clone() {
        let key = format!("dg:{}", g.email);
        let open = !app.collapsed.contains(&key);
        let count = g
            .members
            .as_ref()
            .map_or(String::new(), |m| format!(" ({})", m.len()));
        let clicked = ui
            .horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                icons::draw(
                    ui.painter(),
                    rect,
                    if open {
                        Icon::ChevronDown
                    } else {
                        Icon::ChevronRight
                    },
                    pal.muted,
                );
                let r = ui
                    .add(
                        egui::Button::new(
                            RichText::new(format!("{}{count}", g.name.to_uppercase()))
                                .size(theme::text::SMALL)
                                .color(pal.muted),
                        )
                        .frame(false),
                    )
                    .on_hover_text(format!("{} · {}", t.distribution_group, g.email));
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                icons::draw(ui.painter(), rect, Icon::People, pal.muted);
                r.clicked()
            })
            .inner;
        if clicked {
            if open {
                app.collapsed.insert(key.clone());
            } else {
                app.collapsed.remove(&key);
            }
        }
        if open {
            if g.members.is_none()
                && g.error.is_none()
                && app.expanded_groups.insert(g.email.clone())
            {
                app.emit(UiIntent::ExpandGroup {
                    email: g.email.clone(),
                });
            }
            match (&g.members, &g.error) {
                (_, Some(e)) => {
                    ui.label(
                        RichText::new(format!("{} ({e})", t.dg_failed))
                            .size(theme::text::SMALL)
                            .color(pal.error),
                    );
                }
                (None, None) => {
                    ui.label(
                        RichText::new(t.dg_loading)
                            .size(theme::text::SMALL)
                            .color(pal.muted),
                    );
                }
                (Some(m), None) => {
                    if m.is_empty() {
                        ui.label(
                            RichText::new(t.dg_empty)
                                .size(theme::text::SMALL)
                                .color(pal.muted),
                        );
                    }
                    for c in m {
                        // A member known in the contact list shows its presence there.
                        let c = app
                            .state
                            .contacts
                            .get(&c.uri)
                            .cloned()
                            .unwrap_or_else(|| c.clone());
                        contact_row(app, ui, pal, &c);
                    }
                    if g.partial {
                        ui.label(
                            RichText::new(t.dg_partial)
                                .size(theme::text::SMALL)
                                .color(pal.muted),
                        );
                    }
                }
            }
        }
        ui.add_space(4.0);
    }
}

/// Renaming and deleting a group of the contact list (only an empty one can be deleted).
fn group_menu(app: &mut CommsUi, header: &egui::Response, name: &str, empty: bool) {
    let t = app.t;
    header.context_menu(|ui| {
        if ui.button(t.rename_group).clicked() {
            app.group_dialog = Some((Some(name.to_string()), name.to_string()));
            ui.close();
        }
        if ui
            .add_enabled(empty, egui::Button::new(t.delete_group))
            .on_disabled_hover_text(t.delete_group_not_empty)
            .clicked()
        {
            app.emit(UiIntent::DeleteGroup {
                name: name.to_string(),
            });
            ui.close();
        }
    });
}

/// People who added the user: add them, or let them be.
fn subscribers(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let list = app.state.subscribers.clone();
    if list.is_empty() {
        ui.label(RichText::new(t.no_new_subscribers).color(pal.muted));
    }
    for s in list {
        ui.horizontal(|ui| {
            let name = if s.name.is_empty() {
                app.state.name_of(&s.uri)
            } else {
                s.name.clone()
            };
            app.avatar(ui, Some(s.uri.as_str()), &name, 30.0, None, pal);
            ui.vertical(|ui| {
                ui.label(RichText::new(&name).color(pal.ink));
                ui.label(
                    RichText::new(t.added_you)
                        .size(theme::text::SMALL)
                        .color(pal.muted),
                );
                ui.horizontal(|ui| {
                    if ui.button(t.add_to_contacts).clicked() {
                        app.emit(UiIntent::AddContact {
                            uri: s.uri.clone(),
                            name: s.name.clone(),
                            group: None,
                        });
                        app.emit(UiIntent::AcknowledgeSubscriber { uri: s.uri.clone() });
                        app.state.subscribers.retain(|x| x.uri != s.uri);
                    }
                    if ui.button(t.ignore).clicked() {
                        app.emit(UiIntent::AcknowledgeSubscriber { uri: s.uri.clone() });
                        app.state.subscribers.retain(|x| x.uri != s.uri);
                    }
                });
            });
        });
        ui.add_space(6.0);
    }
}

/// The name of a new group, or the new name of a group.
pub fn group_dialog(app: &mut CommsUi, ctx: &egui::Context) {
    let t = app.t;
    let Some((old, mut text)) = app.group_dialog.take() else {
        return;
    };
    let mut keep = true;
    egui::Modal::new(egui::Id::unique("group-dialog")).show(ctx, |ui| {
        ui.label(
            RichText::new(if old.is_some() {
                t.rename_group
            } else {
                t.new_group
            })
            .family(theme::bold()),
        );
        let r = ui.add(
            egui::TextEdit::singleline(&mut text)
                .char_limit(64)
                .desired_width(260.0),
        );
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "group name"));
        r.request_focus();
        ui.horizontal(|ui| {
            let ok = !text.trim().is_empty() && !text.chars().any(char::is_control);
            if ui.add_enabled(ok, egui::Button::new(t.ok)).clicked() {
                let name = text.trim().to_string();
                match &old {
                    Some(o) => app.emit(UiIntent::RenameGroup {
                        name: o.clone(),
                        to: name,
                    }),
                    None => app.emit(UiIntent::AddGroup { name }),
                }
                keep = false;
            }
            if ui.button(t.cancel).clicked() {
                keep = false;
            }
        });
    });
    if keep {
        app.group_dialog = Some((old, text));
    }
}

/// What can be done with a contact (also from search results).
pub fn contact_menu(app: &mut CommsUi, ui: &mut Ui, uri: &str) {
    let t = app.t;
    let caps = app.state.capabilities.clone();
    if ui
        .add_enabled(caps.chat.usable(), egui::Button::new(t.send_im))
        .clicked()
    {
        app.chat_with(uri);
        ui.close();
    }
    let idle = app.state.active_call().is_none();
    if ui
        .add_enabled(caps.audio_call.usable() && idle, egui::Button::new(t.call))
        .clicked()
    {
        app.call(uri, false);
        ui.close();
    }
    if ui
        .add_enabled(
            caps.video_call.usable() && idle,
            egui::Button::new(t.video_call),
        )
        .clicked()
    {
        app.call(uri, true);
        ui.close();
    }
    ui.separator();
    let can_edit = app.state.capabilities.presence.usable();
    let groups = app.state.groups.clone();
    if app.state.contacts.contains_key(uri) {
        ui.add_enabled_ui(can_edit, |ui| {
            ui.menu_button(t.move_to_group, |ui| {
                for g in &groups {
                    if ui.button(g).clicked() {
                        app.emit(UiIntent::MoveContact {
                            uri: uri.to_string(),
                            group: g.clone(),
                        });
                        ui.close();
                    }
                }
            });
            if ui.button(t.remove_contact).clicked() {
                app.emit(UiIntent::RemoveContact {
                    uri: uri.to_string(),
                });
                ui.close();
            }
        });
    } else {
        ui.add_enabled_ui(can_edit, |ui| {
            ui.menu_button(t.add_to_contacts, |ui| {
                let name = app.state.known_name(uri);
                if ui.button(t.other_contacts).clicked() {
                    app.emit(UiIntent::AddContact {
                        uri: uri.to_string(),
                        name: name.clone(),
                        group: None,
                    });
                    ui.close();
                }
                for g in &groups {
                    if ui.button(g).clicked() {
                        app.emit(UiIntent::AddContact {
                            uri: uri.to_string(),
                            name: name.clone(),
                            group: Some(g.clone()),
                        });
                        ui.close();
                    }
                }
            });
        });
    }
    ui.add_enabled_ui(can_edit, |ui| {
        ui.menu_button(t.relationship, |ui| {
            use crate::model::Relation as R;
            for (r, label) in [
                (Some(R::Family), t.rel_family),
                (Some(R::Workgroup), t.rel_workgroup),
                (Some(R::Colleagues), t.rel_colleagues),
                (Some(R::External), t.rel_external),
                (Some(R::Blocked), t.rel_blocked),
                (None, t.rel_auto),
            ] {
                if ui.button(label).clicked() {
                    app.emit(UiIntent::SetRelation {
                        uri: uri.to_string(),
                        relation: r,
                    });
                    ui.close();
                }
            }
        });
    });
    ui.separator();
    let fav = app.prefs.favorites.contains(uri);
    if ui
        .button(if fav { t.unfavorite } else { t.favorite })
        .clicked()
    {
        app.toggle_favorite(uri.to_string());
        ui.close();
    }
    if ui.button(t.contact_card).clicked() {
        app.open_card(uri);
        ui.close();
    }
}

fn conversations(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    search_field(app, ui, pal, t.find_someone);
    let mut f = app.conv_filter;
    sub_tabs(
        ui,
        pal,
        &mut f,
        &[
            (ConvFilter::All, t.filter_all),
            (ConvFilter::Missed, t.filter_missed),
            (ConvFilter::Calls, t.filter_calls),
        ],
    );
    app.conv_filter = f;
    let needle = app.search.trim().to_lowercase();
    egui::ScrollArea::vertical()
        .id_salt("conversations")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut any = false;
            if app.conv_filter != ConvFilter::Calls {
                let mut keys: Vec<(u64, ConvKey)> = app
                    .state
                    .conversations
                    .iter()
                    .filter(|c| app.conv_filter == ConvFilter::All || c.unread > 0)
                    .filter(|c| needle.is_empty() || c.title.to_lowercase().contains(&needle))
                    .map(|c| (c.last_activity, c.key))
                    .collect();
                keys.sort_by(|a, b| b.cmp(a));
                for (_, k) in keys {
                    any = true;
                    conversation_row(app, ui, pal, k);
                }
            }
            if app.conv_filter != ConvFilter::All || app.state.conversations.is_empty() {
                for r in app.state.call_history.clone() {
                    if app.conv_filter == ConvFilter::Missed && r.direction != CallDirection::Missed
                    {
                        continue;
                    }
                    any = true;
                    let name = app.state.name_of(&r.peer);
                    let sub = format!(
                        "{} · {}",
                        t.call_direction(r.direction),
                        crate::format::clock(r.at, app.utc_offset)
                    );
                    let resp = ui.add(
                        egui::Button::new(RichText::new(format!("{name}\n{sub}")).color(
                            if r.direction == CallDirection::Missed {
                                pal.error
                            } else {
                                pal.ink
                            },
                        ))
                        .frame(false),
                    );
                    resp.context_menu(|ui| contact_menu(app, ui, &r.peer));
                }
            }
            if !any {
                ui.label(RichText::new(t.no_conversations_yet).color(pal.muted));
            }
            if app.conv_filter == ConvFilter::All {
                history_section(app, ui, pal, &needle);
            }
        });
}

/// Earlier conversations from the Conversation History folder in Exchange.
fn history_section(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, needle: &str) {
    let t = app.t;
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        widgets::section(ui, t.history_exchange, pal);
        let usable = app.state.connection.is_online();
        if ui
            .add_enabled(usable, egui::Button::new(t.load).small())
            .clicked()
        {
            app.emit(UiIntent::LoadHistory);
        }
    });
    match app.state.history.clone() {
        None => {}
        Some(Err(e)) => {
            ui.label(RichText::new(format!("{}: {e}", t.history_failed)).color(pal.muted));
        }
        Some(Ok(list)) if list.is_empty() => {
            ui.label(RichText::new(t.history_empty).color(pal.muted));
        }
        Some(Ok(list)) => {
            for e in list.iter().filter(|e| {
                needle.is_empty()
                    || e.subject.to_lowercase().contains(needle)
                    || e.with.to_lowercase().contains(needle)
            }) {
                let when =
                    e.at.map(|a| crate::format::date(a, app.utc_offset))
                        .unwrap_or_default();
                let title = if e.with.is_empty() {
                    &e.subject
                } else {
                    &e.with
                };
                let r = ui.add(
                    egui::Button::new(RichText::new(format!("{title}  ·  {when}")).color(pal.ink))
                        .frame(false),
                );
                if r.clicked() {
                    app.state.history_open = Some((e.id.clone(), None));
                    app.emit(UiIntent::OpenHistory { id: e.id.clone() });
                }
            }
        }
    }
}

/// The text of a history entry, read-only.
pub fn history_reader(app: &mut CommsUi, ctx: &egui::Context) {
    let t = app.t;
    let Some((_, text)) = app.state.history_open.clone() else {
        return;
    };
    let mut keep = true;
    // As wide as fits the window (the main window can be narrow).
    let width = (ctx.content_rect().width() - 60.0).clamp(200.0, 460.0);
    egui::Modal::new(egui::Id::unique("history-reader")).show(ctx, |ui| {
        ui.set_width(width);
        ui.label(RichText::new(t.history_exchange).family(theme::bold()));
        egui::ScrollArea::vertical()
            .max_height(380.0)
            .show(ui, |ui| match &text {
                None => {
                    ui.label(t.loading);
                }
                Some(Err(e)) => {
                    ui.label(format!("{}: {e}", t.history_failed));
                }
                Some(Ok(body)) => {
                    ui.add(egui::Label::new(body.as_str()).wrap().selectable(true));
                }
            });
        if ui.button(t.close).clicked() {
            keep = false;
        }
    });
    if !keep {
        app.state.history_open = None;
    }
}

fn conversation_row(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    let Some(c) = app.state.conversation(key).cloned() else {
        return;
    };
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 4.0, pal.surface_subtle);
    }
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(4.0, 2.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    let presence = c
        .peer
        .as_ref()
        .and_then(|p| app.state.contacts.get(p))
        .map(|k| k.presence);
    app.avatar(&mut child, c.peer.as_deref(), &c.title, 34.0, presence, pal);
    child.vertical(|ui| {
        let title = if c.unread > 0 {
            RichText::new(&c.title).family(theme::bold()).color(pal.ink)
        } else {
            RichText::new(&c.title).color(pal.ink)
        };
        widgets::ellipsis(ui, title);
        let last = c
            .last_text()
            .map(|m| m.text.replace('\n', " "))
            .unwrap_or_default();
        widgets::ellipsis(
            ui,
            RichText::new(last)
                .size(theme::text::SMALL)
                .color(pal.muted),
        );
    });
    let info = format!("{}, {}", c.title, app.t.unread_count(c.unread));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &info));
    if resp.clicked() || resp.double_clicked() {
        app.select_conversation(key);
    }
}

fn rooms(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    if ui
        .button(t.load_rooms)
        .on_hover_text(t.load_rooms_tip)
        .clicked()
    {
        app.emit(UiIntent::LoadRooms);
    }
    ui.add_space(6.0);
    let mut keys: Vec<(String, ConvKey)> = app
        .state
        .conversations
        .iter()
        .filter(|c| c.is_room())
        .map(|c| (c.title.to_lowercase(), c.key))
        .collect();
    keys.sort();
    egui::ScrollArea::vertical()
        .id_salt("rooms")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if keys.is_empty() {
                ui.label(RichText::new(t.no_rooms).color(pal.muted));
            }
            for (_, k) in keys {
                conversation_row(app, ui, pal, k);
            }
        });
}
