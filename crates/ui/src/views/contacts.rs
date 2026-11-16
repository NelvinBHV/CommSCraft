//! Screen 2: contacts and presence.

use crate::app::{CommsUi, ContactFilter, UiIntent};
use crate::model::{Contact, Presence, Source, View};
use crate::theme::{self, Palette};
use crate::views::shell::{context_block, context_title};
use crate::widgets;
use egui::{Align, CornerRadius, Key, Layout, RichText, Sense, Stroke, Ui, Vec2};
use std::collections::BTreeMap;

fn online(p: Presence) -> bool {
    !matches!(p, Presence::Offline | Presence::Unknown)
}

pub fn list(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new(t.contacts)
                    .size(theme::text::PANEL_TITLE + 2.0)
                    .family(crate::theme::bold())
                    .color(pal.ink),
            );
            ui.label(
                RichText::new(t.contacts_sub)
                    .size(theme::text::SMALL + 0.5)
                    .color(pal.muted),
            );
        });
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        if widgets::chip(
            ui,
            t.filter_all,
            app.contact_filter == ContactFilter::All,
            pal,
        ) {
            app.contact_filter = ContactFilter::All;
        }
        if widgets::chip(
            ui,
            t.filter_online,
            app.contact_filter == ContactFilter::Online,
            pal,
        ) {
            app.contact_filter = ContactFilter::Online;
        }
        if widgets::chip(
            ui,
            t.filter_favorites,
            app.contact_filter == ContactFilter::Favorites,
            pal,
        ) {
            app.contact_filter = ContactFilter::Favorites;
        }
    });
    ui.add_space(6.0);
    // Groups of the contact list in order, contacts by name inside.
    let mut groups: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for c in app.state.contacts.values() {
        let keep = match app.contact_filter {
            ContactFilter::All => true,
            ContactFilter::Online => online(c.presence),
            ContactFilter::Favorites => app.prefs.favorites.contains(&c.uri),
        };
        if keep {
            let g = if c.group.is_empty() {
                t.other_contacts.to_string()
            } else {
                c.group.clone()
            };
            groups
                .entry(g)
                .or_default()
                .push((c.display_name().to_lowercase(), c.uri.clone()));
        }
    }
    egui::ScrollArea::vertical()
        .id_salt("contact-list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if groups.is_empty() {
                ui.add_space(20.0);
                let text = if app.state.contacts.is_empty() {
                    if app.state.connection.is_online() || app.demo {
                        t.no_contacts
                    } else {
                        t.contacts_offline
                    }
                } else {
                    t.no_contacts_filter
                };
                ui.label(RichText::new(text).color(pal.muted));
                return;
            }
            for (g, mut members) in groups {
                members.sort();
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    widgets::section(ui, &g, pal);
                });
                for (_, uri) in members {
                    row(app, ui, pal, &uri);
                }
                ui.add_space(8.0);
            }
        });
}

fn subline(app: &CommsUi, c: &Contact) -> String {
    let mut s = app.t.presence(c.presence).to_string();
    let extra = if !c.department.is_empty() {
        &c.department
    } else {
        &c.title
    };
    if !extra.is_empty() {
        s.push_str(" · ");
        s.push_str(extra);
    }
    s
}

pub fn row(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, uri: &str) {
    let Some(c) = app.state.contacts.get(uri).cloned().or_else(|| {
        app.state
            .search_results
            .iter()
            .find(|c| c.uri == uri)
            .cloned()
    }) else {
        return;
    };
    let selected = app.selected_contact.as_deref() == Some(uri);
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 68.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, CornerRadius::same(10), pal.primary_soft);
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(10), pal.surface_subtle);
    }
    if resp.has_focus() {
        p.rect_stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(2.0, pal.primary),
            egui::StrokeKind::Inside,
        );
    }
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(10.0, 12.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    widgets::avatar(&mut child, c.display_name(), 42.0, Some(c.presence), pal);
    child.add_space(4.0);
    child.vertical(|ui| {
        ui.set_width(ui.available_width());
        widgets::ellipsis(
            ui,
            RichText::new(c.display_name())
                .size(theme::text::NAME)
                .family(crate::theme::bold())
                .color(pal.ink),
        );
        widgets::ellipsis(
            ui,
            RichText::new(subline(app, &c))
                .size(theme::text::SMALL)
                .color(pal.muted),
        );
    });
    let info = format!("{}, {}", c.display_name(), app.t.presence(c.presence));
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected, &info));
    if resp.clicked() {
        app.select_contact(uri);
    }
    if resp.double_clicked() && app.state.capabilities.chat.usable() {
        app.chat_with(uri);
    }
    if resp.has_focus() {
        let down = ui.input(|i| i.key_pressed(Key::ArrowDown));
        let up = ui.input(|i| i.key_pressed(Key::ArrowUp));
        if down || up {
            ui.memory_mut(|m| {
                m.move_focus(if down {
                    egui::FocusDirection::Down
                } else {
                    egui::FocusDirection::Up
                })
            });
        }
    }
    resp.context_menu(|ui| {
        let fav = app.prefs.favorites.contains(uri);
        if ui
            .button(if fav {
                app.t.unfavorite
            } else {
                app.t.favorite
            })
            .clicked()
        {
            app.toggle_favorite(uri.to_string());
            ui.close();
        }
    });
}

fn source_text(app: &CommsUi, s: Source) -> &'static str {
    match s {
        Source::ContactList => app.t.source_contact_list,
        Source::AddressBook => app.t.source_address_book,
        Source::Directory => app.t.source_directory,
        Source::Demo => app.t.source_demo,
    }
}

pub fn main(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let uri = app.selected_contact.clone();
    let Some(c) = uri.as_ref().and_then(|u| {
        app.state.contacts.get(u).cloned().or_else(|| {
            app.state
                .search_results
                .iter()
                .find(|c| &c.uri == u)
                .cloned()
        })
    }) else {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);
            ui.label(
                RichText::new(t.select_contact)
                    .size(theme::text::PANEL_TITLE)
                    .color(pal.ink),
            );
            ui.label(RichText::new(t.select_contact_hint).color(pal.muted));
        });
        return;
    };
    let caps = app.state.capabilities.clone();
    egui::ScrollArea::vertical()
        .id_salt("contact-main")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(34.0);
                widgets::avatar(ui, c.display_name(), 112.0, Some(c.presence), pal);
                ui.add_space(10.0);
                ui.label(
                    RichText::new(c.display_name())
                        .size(theme::text::HEADING + 2.0)
                        .family(crate::theme::bold())
                        .color(pal.ink),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    // Centre the badge: about 40 px for dot and margins plus the word.
                    let word = t.presence(c.presence).chars().count() as f32 * 7.5 + 44.0;
                    ui.add_space(((ui.available_width() - word) / 2.0).max(0.0));
                    widgets::presence_badge(ui, c.presence, t, pal);
                });
                let mut parts = Vec::new();
                for v in [&c.title, &c.company] {
                    if !v.is_empty() {
                        parts.push(v.as_str());
                    }
                }
                if !parts.is_empty() {
                    ui.label(RichText::new(parts.join(" · ")).color(pal.muted));
                }
                if !c.note.is_empty() {
                    ui.label(
                        RichText::new(format!("„{}“", c.note))
                            .italics()
                            .color(pal.muted),
                    );
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let total = 2.0 * 150.0 + 12.0;
                    ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                    if let Some(r) =
                        widgets::gated(ui, t.start_chat, &caps.chat, t, pal, |ui, l, en| {
                            ui.add_enabled(
                                en,
                                egui::Button::new(
                                    RichText::new(l)
                                        .color(pal.on_primary)
                                        .family(crate::theme::bold()),
                                )
                                .fill(pal.primary)
                                .min_size(Vec2::new(150.0, 42.0)),
                            )
                        })
                    {
                        if r.clicked() {
                            app.chat_with(&c.uri);
                        }
                    }
                    if let Some(r) =
                        widgets::gated(ui, t.audio_call, &caps.audio_call, t, pal, |ui, l, en| {
                            ui.add_enabled(
                                en && app.state.active_call().is_none(),
                                egui::Button::new(RichText::new(l).color(pal.primary))
                                    .fill(pal.surface)
                                    .min_size(Vec2::new(150.0, 42.0)),
                            )
                        })
                    {
                        if r.clicked() {
                            app.emit(UiIntent::CallUser {
                                peer: c.uri.clone(),
                            });
                            app.view = View::Calls;
                        }
                    }
                });
                ui.add_space(20.0);
                ui.allocate_ui(Vec2::new(ui.available_width().min(640.0), 10.0), |ui| {
                    widgets::card(ui, pal, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            RichText::new(t.contact_overview)
                                .size(theme::text::NAME + 1.0)
                                .family(crate::theme::bold())
                                .color(pal.ink),
                        );
                        widgets::rule(ui, pal);
                        egui::Grid::new("contact-facts")
                            .num_columns(2)
                            .spacing([40.0, 10.0])
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(t.status)
                                        .size(theme::text::SMALL)
                                        .color(pal.muted),
                                );
                                ui.label(RichText::new(t.presence(c.presence)).color(pal.ink));
                                ui.end_row();
                                ui.label(
                                    RichText::new(t.sip_address)
                                        .size(theme::text::SMALL)
                                        .color(pal.muted),
                                );
                                ui.label(
                                    RichText::new(c.uri.trim_start_matches("sip:")).color(pal.ink),
                                );
                                ui.end_row();
                                if !c.phone.is_empty() {
                                    ui.label(
                                        RichText::new(t.phone)
                                            .size(theme::text::SMALL)
                                            .color(pal.muted),
                                    );
                                    ui.label(RichText::new(&c.phone).color(pal.ink));
                                    ui.end_row();
                                }
                                ui.label(
                                    RichText::new(t.origin)
                                        .size(theme::text::SMALL)
                                        .color(pal.muted),
                                );
                                ui.label(RichText::new(source_text(app, c.source)).color(pal.ink));
                                ui.end_row();
                            });
                    });
                    ui.add_space(14.0);
                    widgets::hint(ui, pal, None, t.presence_hint);
                });
            });
        });
}

pub fn context(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    context_title(app, ui, pal, t.contact_info);
    let Some(c) = app
        .selected_contact
        .as_ref()
        .and_then(|u| app.state.contacts.get(u))
        .cloned()
    else {
        ui.label(RichText::new(t.select_contact_hint).color(pal.muted));
        return;
    };
    let function: Vec<&str> = [c.title.as_str(), c.department.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    context_block(
        ui,
        pal,
        t.function,
        &if function.is_empty() {
            t.not_known.to_string()
        } else {
            function.join(" · ")
        },
    );
    context_block(
        ui,
        pal,
        t.organisation,
        if c.company.is_empty() {
            t.not_known
        } else {
            &c.company
        },
    );
    context_block(ui, pal, t.reachability, t.reachability_body);
    context_block(ui, pal, t.availability, t.availability_body);
    if c.source == Source::Demo {
        context_block(ui, pal, t.note, t.demo_person);
    }
}
