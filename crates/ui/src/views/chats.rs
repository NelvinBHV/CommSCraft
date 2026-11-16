//! Screen 1: conversations – the list, the conversation with its composer, group details.

use crate::app::{ChatFilter, CommsUi, NewChat, UiIntent};
use crate::format;
use crate::model::{ConvKey, ConvState, Conversation, Delivery, Gate, Message, MAX_TEXT};
use crate::theme::{self, Palette};
use crate::views::shell::{context_block, context_title};
use crate::widgets;
use egui::{
    Align, CornerRadius, Frame, Key, KeyboardShortcut, Layout, Margin, RichText, Sense, Stroke, Ui,
    Vec2,
};

/// Seconds after the last key press until "typing" ends (the session repeats it itself).
const TYPING_IDLE: u64 = 5;

fn conv_matches(app: &CommsUi, c: &Conversation) -> bool {
    match app.chat_filter {
        ChatFilter::All => true,
        ChatFilter::Unread => c.unread > 0,
        ChatFilter::Groups => c.is_group(),
    }
}

pub fn list(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            RichText::new(t.messages_title)
                .size(theme::text::PANEL_TITLE + 2.0)
                .family(crate::theme::bold())
                .color(pal.ink),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let gate = app.state.capabilities.chat.clone();
            let r = widgets::gated(ui, "+", &gate, t, pal, |ui, l, en| {
                ui.add_enabled(
                    en,
                    egui::Button::new(RichText::new(l).color(pal.primary))
                        .min_size(Vec2::splat(34.0)),
                )
            });
            if let Some(r) = r {
                if r.on_hover_text(t.new_chat_tip).clicked() {
                    app.new_chat = Some(NewChat::default());
                }
            }
        });
    });
    ui.add_space(8.0);
    let unread = app.state.unread_total();
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        if widgets::chip(ui, t.filter_all, app.chat_filter == ChatFilter::All, pal) {
            app.chat_filter = ChatFilter::All;
        }
        let label = format!("{} {unread}", t.filter_unread);
        if widgets::chip(ui, &label, app.chat_filter == ChatFilter::Unread, pal) {
            app.chat_filter = ChatFilter::Unread;
        }
        if widgets::chip(
            ui,
            t.filter_groups,
            app.chat_filter == ChatFilter::Groups,
            pal,
        ) {
            app.chat_filter = ChatFilter::Groups;
        }
    });
    ui.add_space(6.0);
    let mut order: Vec<(bool, u64, ConvKey)> = app
        .state
        .conversations
        .iter()
        .filter(|c| conv_matches(app, c))
        .map(|c| {
            let fav = app
                .favorite_id(c.key)
                .is_some_and(|id| app.prefs.favorites.contains(&id));
            (fav, c.last_activity, c.key)
        })
        .collect();
    order.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    egui::ScrollArea::vertical()
        .id_salt("chat-list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if order.is_empty() {
                ui.add_space(20.0);
                ui.label(
                    RichText::new(if app.state.conversations.is_empty() {
                        t.no_conversations
                    } else {
                        t.no_conversations_filter
                    })
                    .color(pal.muted),
                );
                return;
            }
            let favs: Vec<ConvKey> = order.iter().filter(|o| o.0).map(|o| o.2).collect();
            let rest: Vec<ConvKey> = order.iter().filter(|o| !o.0).map(|o| o.2).collect();
            if !favs.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    widgets::section(ui, t.favorites, pal);
                });
                for k in favs {
                    row(app, ui, pal, k);
                }
                ui.add_space(10.0);
            }
            if !rest.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    widgets::section(ui, t.recent_conversations, pal);
                });
                for k in rest {
                    row(app, ui, pal, k);
                }
            }
        });
}

fn preview(app: &CommsUi, c: &Conversation) -> String {
    if c.typing {
        return app.t.typing_short.to_string();
    }
    match &c.state {
        ConvState::Failed(code) => return format!("{} ({code})", app.t.conversation_failed),
        ConvState::Opening if c.messages.is_empty() => return app.t.opening.to_string(),
        _ => {}
    }
    match c.last_text() {
        None => String::new(),
        Some(m) => {
            let who = match &m.sender {
                None => app.t.you.to_string(),
                Some(s) if c.is_group() => app
                    .state
                    .name_of(s)
                    .split(' ')
                    .next()
                    .unwrap_or("")
                    .to_string(),
                Some(_) => String::new(),
            };
            let line: String = m
                .text
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(80)
                .collect();
            if who.is_empty() {
                line
            } else {
                format!("{who}: {line}")
            }
        }
    }
}

fn when(app: &CommsUi, at: u64) -> String {
    if at == 0 {
        return String::new();
    }
    let today = format::day(app.now, app.utc_offset);
    let d = format::day(at, app.utc_offset);
    if d == today {
        format::clock(at, app.utc_offset)
    } else if d + 1 == today {
        app.t.yesterday.to_string()
    } else {
        format::date(at, app.utc_offset)
    }
}

/// The name an avatar shows: a group titled after its members (addresses, a list) shows the
/// group mark instead of one member's initials.
fn group_avatar<'a>(app: &CommsUi, group: bool, title: &'a str) -> &'a str {
    if group && (title.contains('@') || title.contains(',') || title.trim().is_empty()) {
        app.t.group_chat
    } else {
        title
    }
}

fn row(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    let Some(c) = app.state.conversation(key) else {
        return;
    };
    let selected = app.selected_conv == Some(key);
    let presence = c
        .peer
        .as_ref()
        .and_then(|p| app.state.contacts.get(p))
        .map(|k| k.presence);
    let title = c.title.clone();
    let line = preview(app, c);
    let time = when(app, c.last_activity);
    let unread = c.unread;
    let is_group = c.is_group();
    let ended = matches!(c.state, ConvState::Ended | ConvState::Failed(_));
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 72.0), Sense::click());
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
    let avatar_name = group_avatar(app, is_group, &title);
    widgets::avatar(&mut child, avatar_name, 44.0, presence, pal);
    child.add_space(4.0);
    child.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.set_width(ui.available_width());
            let tw = ui.available_width() - 58.0;
            ui.allocate_ui(Vec2::new(tw, 18.0), |ui| {
                widgets::ellipsis(
                    ui,
                    RichText::new(&title)
                        .size(theme::text::NAME)
                        .family(crate::theme::bold())
                        .color(pal.ink),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(time)
                        .size(theme::text::SMALL)
                        .color(pal.muted),
                );
            });
        });
        ui.horizontal(|ui| {
            ui.set_width(ui.available_width());
            let tw = ui.available_width() - 30.0;
            ui.allocate_ui(Vec2::new(tw, 16.0), |ui| {
                let colour = if ended { pal.quiet } else { pal.muted };
                widgets::ellipsis(
                    ui,
                    RichText::new(line).size(theme::text::SMALL).color(colour),
                );
            });
            if unread > 0 {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 10.0, pal.primary);
                    ui.painter().text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        unread.min(99).to_string(),
                        egui::FontId::proportional(10.5),
                        pal.on_primary,
                    );
                });
            }
        });
    });
    let info = format!("{title}, {}", app.t.unread_count(unread));
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected, &info));
    if resp.clicked() {
        app.select_conversation(key);
    }
    // Arrow keys move through the list while a row has the focus.
    if resp.has_focus() {
        let down = ui.input(|i| i.key_pressed(Key::ArrowDown));
        let up = ui.input(|i| i.key_pressed(Key::ArrowUp));
        if down || up {
            ui.memory_mut(|m| {
                if down {
                    m.move_focus(egui::FocusDirection::Down)
                } else {
                    m.move_focus(egui::FocusDirection::Up)
                }
            });
        }
    }
    resp.context_menu(|ui| {
        if let Some(id) = app.favorite_id(key) {
            let fav = app.prefs.favorites.contains(&id);
            if ui
                .button(if fav {
                    app.t.unfavorite
                } else {
                    app.t.favorite
                })
                .clicked()
            {
                app.toggle_favorite(id);
                ui.close();
            }
        }
        if !ended && ui.button(app.t.end_conversation).clicked() {
            app.emit(UiIntent::EndConversation { key });
            ui.close();
        }
    });
}

pub fn main(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let Some(key) = app
        .selected_conv
        .filter(|k| app.state.conversation(*k).is_some())
    else {
        empty(app, ui, pal);
        return;
    };
    header(app, ui, pal, key);
    composer(app, ui, pal, key);
    history(app, ui, pal, key);
}

fn empty(app: &CommsUi, ui: &mut Ui, pal: &Palette) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() / 3.0);
        ui.label(
            RichText::new(app.t.select_conversation)
                .size(theme::text::PANEL_TITLE)
                .color(pal.ink),
        );
        ui.label(RichText::new(app.t.select_conversation_hint).color(pal.muted));
    });
}

fn header(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    let t = app.t;
    let Some(c) = app.state.conversation(key) else {
        return;
    };
    let title = c.title.clone();
    let peer = c.peer.clone();
    let subtitle = if c.is_group() {
        let n = c.participants.len();
        format!("{} · {}", t.group_chat, t.members_count(n))
    } else {
        let p = peer.as_deref().and_then(|p| app.state.contacts.get(p));
        match p {
            Some(k) => format!(
                "{} · {}",
                t.presence(k.presence),
                k.uri.trim_start_matches("sip:")
            ),
            None => peer
                .clone()
                .unwrap_or_default()
                .trim_start_matches("sip:")
                .to_string(),
        }
    };
    let presence = peer
        .as_ref()
        .and_then(|p| app.state.contacts.get(p))
        .map(|k| k.presence);
    let avatar_name = group_avatar(app, peer.is_none(), &title).to_string();
    egui::Panel::top(("chat-head", key.0))
        .exact_size(74.0)
        .frame(
            Frame::new()
                .fill(pal.surface)
                .inner_margin(Margin::symmetric(24, 12))
                .stroke(Stroke::new(1.0, pal.border)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                widgets::avatar(ui, &avatar_name, 44.0, presence, pal);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(&title)
                            .size(theme::text::PANEL_TITLE - 1.0)
                            .family(crate::theme::bold())
                            .color(pal.ink),
                    );
                    ui.label(
                        RichText::new(subtitle)
                            .size(theme::text::SMALL + 0.5)
                            .color(pal.muted),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let caps = app.state.capabilities.clone();
                    let history = caps.history_search.clone();
                    widgets::gated(ui, t.search_in_chat, &history, t, pal, |ui, l, en| {
                        ui.add_enabled(en, egui::Button::new(l))
                    });
                    if let Some(p) = peer.clone() {
                        if let Some(r) =
                            widgets::gated(ui, t.call, &caps.audio_call, t, pal, |ui, l, en| {
                                ui.add_enabled(
                                    en && app.state.active_call().is_none(),
                                    egui::Button::new(l),
                                )
                            })
                        {
                            if r.clicked() {
                                app.emit(UiIntent::CallUser { peer: p });
                                app.view = crate::model::View::Calls;
                            }
                        }
                    }
                    if let Some(id) = app.favorite_id(key) {
                        let fav = app.prefs.favorites.contains(&id);
                        let label = if fav { t.unfavorite } else { t.favorite };
                        if ui.button(label).clicked() {
                            app.toggle_favorite(id);
                        }
                    }
                });
            });
        });
}

fn delivery_text(app: &CommsUi, d: &Delivery) -> (String, bool) {
    let t = app.t;
    match d {
        Delivery::Sending => (t.sending.to_string(), false),
        Delivery::Delivered => (t.delivered.to_string(), false),
        Delivery::Failed(code) => (format!("{} ({code})", t.not_delivered), true),
        Delivery::PartlyFailed(list) => {
            let names: Vec<String> = list
                .iter()
                .map(|(u, c)| format!("{} ({c})", app.state.name_of(u)))
                .collect();
            (
                format!("{}: {}", t.not_delivered_to, names.join(", ")),
                true,
            )
        }
        Delivery::Unknown => (t.delivery_unknown.to_string(), false),
    }
}

fn history(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    let Some(c) = app.state.conversation(key).cloned() else {
        return;
    };
    let t = app.t;
    egui::CentralPanel::no_frame().show(ui, |ui| {
        Frame::new()
            .fill(pal.surface_subtle)
            .inner_margin(Margin::symmetric(28, 12))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                egui::ScrollArea::vertical()
                    .id_salt(("history", key.0))
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if c.demo {
                            widgets::hint(ui, pal, Some(t.demo_content_title), t.demo_content);
                            ui.add_space(8.0);
                        }
                        if c.messages.is_empty() {
                            ui.add_space(30.0);
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new(t.no_messages).color(pal.muted));
                            });
                        }
                        let mut last_day = None;
                        let mut last_sender: Option<Option<String>> = None;
                        for m in &c.messages {
                            let d = format::day(m.at, app.utc_offset);
                            if last_day != Some(d) {
                                day_separator(app, ui, pal, m.at);
                                last_day = Some(d);
                                last_sender = None;
                            }
                            let first = last_sender.as_ref() != Some(&m.sender);
                            bubble(app, ui, pal, m, c.is_group(), first);
                            last_sender = Some(m.sender.clone());
                        }
                        if c.typing {
                            ui.label(
                                RichText::new(t.is_typing(&c.title))
                                    .italics()
                                    .color(pal.muted),
                            );
                        }
                        match c.state {
                            ConvState::Ended => {
                                ui.add_space(8.0);
                                ui.label(RichText::new(t.conversation_ended).color(pal.muted));
                            }
                            ConvState::Failed(code) => {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(format!("{} ({code})", t.conversation_failed))
                                        .color(pal.error),
                                );
                            }
                            _ => {}
                        }
                    });
            });
    });
}

fn day_separator(app: &CommsUi, ui: &mut Ui, pal: &Palette, at: u64) {
    let today = format::day(app.now, app.utc_offset);
    let d = format::day(at, app.utc_offset);
    let text = if d == today {
        app.t.today.to_uppercase()
    } else if d + 1 == today {
        app.t.yesterday.to_uppercase()
    } else {
        format::date(at, app.utc_offset)
    };
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let w = ui.available_width();
        let label_w = 90.0;
        let side = ((w - label_w) / 2.0).max(10.0);
        let (r, _) = ui.allocate_exact_size(Vec2::new(side, 12.0), Sense::hover());
        ui.painter()
            .hline(r.x_range(), r.center().y, Stroke::new(1.0, pal.border));
        ui.allocate_ui_with_layout(
            Vec2::new(label_w - 16.0, 14.0),
            Layout::top_down(Align::Center),
            |ui| {
                ui.label(
                    RichText::new(text)
                        .size(theme::text::SMALL)
                        .color(pal.muted),
                );
            },
        );
        let (r, _) = ui.allocate_exact_size(Vec2::new(side - 8.0, 12.0), Sense::hover());
        ui.painter()
            .hline(r.x_range(), r.center().y, Stroke::new(1.0, pal.border));
    });
    ui.add_space(4.0);
}

fn bubble(app: &CommsUi, ui: &mut Ui, pal: &Palette, m: &Message, group: bool, first: bool) {
    let own = m.sender.is_none();
    let max_w = (ui.available_width() * 0.68).max(240.0);
    let time = format::clock(m.at, app.utc_offset);
    if own {
        ui.with_layout(Layout::top_down(Align::Max), |ui| {
            Frame::new()
                .fill(pal.message_outgoing)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    ui.set_max_width(max_w);
                    ui.add(
                        egui::Label::new(
                            RichText::new(&m.text)
                                .size(theme::text::BODY)
                                .color(pal.ink),
                        )
                        .wrap(),
                    );
                });
            let (status, bad) = delivery_text(app, &m.delivery);
            ui.label(
                RichText::new(format!("{time} · {status}"))
                    .size(theme::text::SMALL)
                    .color(if bad { pal.error } else { pal.muted }),
            );
        });
        ui.add_space(4.0);
        return;
    }
    let sender = m.sender.clone().unwrap_or_default();
    let name = app.state.name_of(&sender);
    ui.horizontal_top(|ui| {
        if first {
            widgets::avatar(ui, &name, 30.0, None, pal);
        } else {
            ui.add_space(30.0);
        }
        ui.add_space(4.0);
        ui.vertical(|ui| {
            if first {
                ui.horizontal(|ui| {
                    if group || first {
                        ui.label(
                            RichText::new(&name)
                                .size(theme::text::NAME - 0.5)
                                .family(crate::theme::bold())
                                .color(pal.ink),
                        );
                    }
                    ui.label(
                        RichText::new(&time)
                            .size(theme::text::SMALL)
                            .color(pal.muted),
                    );
                });
            }
            Frame::new()
                .fill(pal.message_incoming)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    ui.set_max_width(max_w);
                    ui.add(
                        egui::Label::new(
                            RichText::new(&m.text)
                                .size(theme::text::BODY)
                                .color(pal.ink),
                        )
                        .wrap(),
                    );
                });
        });
    });
    ui.add_space(4.0);
}

fn composer(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    let t = app.t;
    let Some(c) = app.state.conversation(key) else {
        return;
    };
    let open = matches!(c.state, ConvState::Open | ConvState::Opening);
    let title = c.title.clone();
    let peer = c.peer.clone();
    let chat_gate = app.state.capabilities.chat.clone();
    let can_send = open && chat_gate.usable();
    let file_gate = app.state.capabilities.file_transfer.clone();
    let rich_gate = app.state.capabilities.rich_text.clone();
    egui::Panel::bottom(("composer", key.0))
        .exact_size(108.0)
        .frame(
            Frame::new()
                .fill(pal.surface)
                .inner_margin(Margin::symmetric(24, 14))
                .stroke(Stroke::new(1.0, pal.border)),
        )
        .show(ui, |ui| {
            Frame::new()
                .stroke(Stroke::new(1.0, pal.border))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let draft = app.drafts.entry(key).or_default();
                    let hint = if can_send {
                        t.write_to(&title)
                    } else if !open {
                        t.conversation_closed_hint.to_string()
                    } else {
                        t.reason(match &chat_gate {
                            Gate::Unavailable(r) => *r,
                            _ => crate::model::Reason::NotConnected,
                        })
                        .to_string()
                    };
                    let edit = egui::TextEdit::multiline(draft)
                        .hint_text(hint)
                        .frame(Frame::NONE)
                        .desired_rows(1)
                        .char_limit(MAX_TEXT)
                        .return_key(KeyboardShortcut::new(egui::Modifiers::SHIFT, Key::Enter))
                        .desired_width(f32::INFINITY);
                    let r = ui.add_enabled(can_send, edit);
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::TextInput, true, "composer")
                    });
                    if app.focus_composer && can_send {
                        r.request_focus();
                        app.focus_composer = false;
                    }
                    let enter = r.has_focus()
                        && ui.input(|i| i.key_pressed(Key::Enter) && i.modifiers.is_none());
                    let text_len = app.drafts.get(&key).map_or(0, |d| d.chars().count());
                    // Typing notifications: on the first key press, off after a pause.
                    if r.changed() && text_len > 0 {
                        if !app.typing_since.contains_key(&key) {
                            app.emit(UiIntent::Typing { key, typing: true });
                        }
                        app.typing_since.insert(key, app.now);
                    }
                    if let Some(since) = app.typing_since.get(&key).copied() {
                        if app.now.saturating_sub(since) >= TYPING_IDLE || text_len == 0 {
                            app.typing_since.remove(&key);
                            app.emit(UiIntent::Typing { key, typing: false });
                        }
                    }
                    ui.horizontal(|ui| {
                        widgets::gated(ui, t.attach, &file_gate, t, pal, |ui, l, en| {
                            ui.add_enabled(en, egui::Button::new(l).frame(false))
                        });
                        widgets::gated(ui, t.formatting, &rich_gate, t, pal, |ui, l, en| {
                            ui.add_enabled(en, egui::Button::new(l).frame(false))
                        });
                        if text_len * 10 > MAX_TEXT * 9 {
                            ui.label(
                                RichText::new(format!("{text_len}/{MAX_TEXT}"))
                                    .size(theme::text::SMALL)
                                    .color(pal.error),
                            );
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let has_text =
                                app.drafts.get(&key).is_some_and(|d| !d.trim().is_empty());
                            let send = widgets::primary(ui, t.send, can_send && has_text, pal)
                                .on_hover_text(t.send_tip);
                            if (send.clicked() || enter) && can_send && has_text {
                                let text = app.drafts.remove(&key).unwrap_or_default();
                                let text = text.trim_end().to_string();
                                app.typing_since.remove(&key);
                                app.state.apply(crate::model::AppEvent::OwnText {
                                    key,
                                    text: text.clone(),
                                    at: app.now,
                                });
                                app.emit(UiIntent::SendMessage { key, text });
                                app.focus_composer = true;
                            }
                            if peer.is_none() && open {
                                let gate = app.state.capabilities.group_chat.clone();
                                if let Some(r) =
                                    widgets::gated(ui, t.invite, &gate, t, pal, |ui, l, en| {
                                        ui.add_enabled(en, egui::Button::new(l))
                                    })
                                {
                                    if r.clicked() {
                                        app.new_chat = Some(NewChat {
                                            text: String::new(),
                                            members: vec![format!("conv:{}", key.0)],
                                        });
                                    }
                                }
                            }
                        });
                    });
                });
        });
}

pub fn context(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let Some(c) = app
        .selected_conv
        .and_then(|k| app.state.conversation(k))
        .cloned()
    else {
        context_title(app, ui, pal, t.details);
        ui.label(RichText::new(t.select_conversation_hint).color(pal.muted));
        return;
    };
    if c.is_group() {
        context_title(app, ui, pal, t.group_info);
    } else {
        context_title(app, ui, pal, t.contact_info);
    }
    ui.vertical_centered(|ui| {
        let presence = c
            .peer
            .as_ref()
            .and_then(|p| app.state.contacts.get(p))
            .map(|k| k.presence);
        widgets::avatar(ui, &c.title, 56.0, presence, pal);
        ui.label(
            RichText::new(&c.title)
                .size(theme::text::NAME + 2.0)
                .family(crate::theme::bold())
                .color(pal.ink),
        );
        let sub = if c.is_group() {
            t.members_count(c.participants.len())
        } else {
            c.peer
                .clone()
                .unwrap_or_default()
                .trim_start_matches("sip:")
                .to_string()
        };
        ui.label(RichText::new(sub).size(theme::text::SMALL).color(pal.muted));
    });
    ui.add_space(14.0);
    if c.is_group() {
        widgets::rule(ui, pal);
        ui.add_space(10.0);
        ui.label(
            RichText::new(t.members_roles)
                .family(crate::theme::bold())
                .color(pal.ink),
        );
        if c.participants.is_empty() {
            ui.label(RichText::new(t.no_roster).color(pal.muted));
        }
        for p in c.participants.iter().take(50) {
            ui.horizontal(|ui| {
                let name = if p.name.is_empty() {
                    app.state.name_of(&p.uri)
                } else {
                    p.name.clone()
                };
                widgets::avatar(
                    ui,
                    &name,
                    24.0,
                    app.state.contacts.get(&p.uri).map(|k| k.presence),
                    pal,
                );
                widgets::ellipsis(ui, RichText::new(name).color(pal.ink));
                if p.presenter {
                    ui.label(
                        RichText::new(t.presenter)
                            .size(theme::text::SMALL)
                            .color(pal.muted),
                    );
                }
            });
        }
        if c.participants.len() > 50 {
            ui.label(RichText::new(t.more_members(c.participants.len() - 50)).color(pal.muted));
        }
        ui.add_space(14.0);
    } else if let Some(k) = c.peer.as_ref().and_then(|p| app.state.contacts.get(p)) {
        let mut parts = Vec::new();
        for v in [&k.title, &k.department, &k.company] {
            if !v.is_empty() {
                parts.push(v.as_str());
            }
        }
        if !parts.is_empty() {
            context_block(ui, pal, t.function, &parts.join(" · "));
        }
        context_block(
            ui,
            pal,
            t.availability,
            &format!(
                "{}{}",
                t.presence(k.presence),
                if k.note.is_empty() {
                    String::new()
                } else {
                    format!(" – {}", k.note)
                }
            ),
        );
    }
    context_block(
        ui,
        pal,
        t.files,
        t.reason(match &app.state.capabilities.file_transfer {
            Gate::Unavailable(r) => *r,
            _ => crate::model::Reason::NotImplemented,
        }),
    );
    context_block(ui, pal, t.history_title, t.history_body);
    context_block(ui, pal, t.delivery_title, t.delivery_body);
}

/// The dialog for a new conversation (one address) or a group (several), also used to invite
/// someone into an open group.
pub fn new_chat_dialog(app: &mut CommsUi, ctx: &egui::Context, pal: &Palette) {
    let Some(mut dlg) = app.new_chat.take() else {
        return;
    };
    let t = app.t;
    let invite_into: Option<ConvKey> = dlg
        .members
        .first()
        .and_then(|m| m.strip_prefix("conv:"))
        .and_then(|k| k.parse().ok())
        .map(ConvKey);
    let mut keep = true;
    let modal = egui::Modal::new(egui::Id::unique("new-chat")).show(ctx, |ui| {
        ui.set_width(440.0);
        ui.label(
            RichText::new(if invite_into.is_some() {
                t.invite_title
            } else {
                t.new_chat_title
            })
            .size(theme::text::PANEL_TITLE)
            .family(crate::theme::bold())
            .color(pal.ink),
        );
        ui.label(RichText::new(t.new_chat_body).color(pal.muted));
        ui.add_space(8.0);
        let r = ui.add(
            egui::TextEdit::singleline(&mut dlg.text)
                .hint_text(t.address_hint)
                .char_limit(256)
                .desired_width(f32::INFINITY),
        );
        r.request_focus();
        // Matching contacts as suggestions.
        let needle = dlg.text.trim().to_lowercase();
        if needle.len() >= 2 {
            let hits: Vec<(String, String)> = app
                .state
                .contacts
                .values()
                .filter(|c| {
                    c.uri.to_lowercase().contains(&needle)
                        || c.name.to_lowercase().contains(&needle)
                })
                .take(6)
                .map(|c| (c.uri.clone(), c.display_name().to_string()))
                .collect();
            for (uri, name) in hits {
                if ui
                    .button(format!("{name} – {}", uri.trim_start_matches("sip:")))
                    .clicked()
                {
                    dlg.text = uri;
                }
            }
        }
        let address = normalise_address(&dlg.text);
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(t.cancel).clicked() {
                keep = false;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let ok = address.is_some();
                let label = if invite_into.is_some() {
                    t.invite
                } else {
                    t.start_chat
                };
                if (widgets::primary(ui, label, ok, pal).clicked() || (enter && ok)) && ok {
                    if let Some(a) = address.clone() {
                        match invite_into {
                            Some(k) => app.emit(UiIntent::InviteToGroup { key: k, peer: a }),
                            None => app.chat_with(&a),
                        }
                    }
                    keep = false;
                }
                if invite_into.is_none() {
                    let gate = app.state.capabilities.group_chat.clone();
                    if let Some(r) =
                        widgets::gated(ui, t.start_group, &gate, t, pal, |ui, l, en| {
                            ui.add_enabled(en && ok, egui::Button::new(l))
                        })
                    {
                        if r.on_hover_text(t.start_group_tip).clicked() {
                            if let Some(a) = address.clone() {
                                app.emit(UiIntent::StartGroup { invite: vec![a] });
                            }
                            keep = false;
                        }
                    }
                }
            });
        });
    });
    if modal.should_close() {
        keep = false;
    }
    if keep {
        app.new_chat = Some(dlg);
    }
}

/// `name@domain` or `sip:name@domain` → `sip:name@domain`; anything else is refused.
pub fn normalise_address(text: &str) -> Option<String> {
    let s = text.trim();
    let s = s.strip_prefix("sip:").unwrap_or(s);
    let (user, domain) = s.split_once('@')?;
    let ok_user = !user.is_empty()
        && user.len() <= 128
        && user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-+'".contains(c));
    let ok_domain = domain.len() >= 3
        && domain.len() <= 253
        && domain.contains('.')
        && domain
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && !domain.starts_with('.')
        && !domain.ends_with('.');
    (ok_user && ok_domain).then(|| format!("sip:{}@{}", user, domain.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::normalise_address;

    #[test]
    fn addresses() {
        assert_eq!(
            normalise_address(" anna.weber@Beispiel.invalid "),
            Some("sip:anna.weber@beispiel.invalid".into())
        );
        assert_eq!(
            normalise_address("sip:tim@beispiel.invalid"),
            Some("sip:tim@beispiel.invalid".into())
        );
        assert_eq!(normalise_address("tim"), None);
        assert_eq!(normalise_address("tim@local"), None);
        assert_eq!(normalise_address("a b@beispiel.invalid"), None);
        assert_eq!(normalise_address("a@beispiel.invalid>;x"), None);
    }
}
