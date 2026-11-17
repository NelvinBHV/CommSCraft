//! Screen 1: conversations – the list, the conversation with its composer, group details.

use crate::app::{AddressFor, CommsUi, NewChat, UiIntent};
use crate::format;
use crate::model::{ConvKey, ConvState, Delivery, Gate, Message, MAX_TEXT};
use crate::theme::{self, Palette};
use crate::widgets;
use egui::{
    Align, CornerRadius, Frame, Key, KeyboardShortcut, Layout, Margin, RichText, Sense, Stroke, Ui,
    Vec2,
};

/// Seconds after the last key press until "typing" ends (the session repeats it itself).
const TYPING_IDLE: u64 = 5;

/// The name an avatar shows: a group titled after its members (addresses, a list) shows the
/// group mark instead of one member's initials.
fn group_avatar<'a>(app: &CommsUi, group: bool, title: &'a str) -> &'a str {
    if group && (title.contains('@') || title.contains(',') || title.trim().is_empty()) {
        app.t.group_chat
    } else {
        title
    }
}

/// A conversation in its own window: head, composer, history.
pub fn conversation_body(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    window_header(app, ui, pal, key);
    composer(app, ui, pal, key);
    history(app, ui, pal, key);
}

/// The head of a conversation window: picture, title, status or members (actions are in the bar
/// at the bottom).
fn window_header(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    let t = app.t;
    let Some(c) = app.state.conversation(key) else {
        return;
    };
    let title = c.title.clone();
    let peer = c.peer.clone();
    let subtitle = if let Some(r) = &c.room {
        if r.topic.is_empty() {
            t.chat_room.to_string()
        } else {
            format!("{} · {}", t.chat_room, r.topic)
        }
    } else if c.is_group() {
        format!(
            "{} · {}",
            t.group_chat,
            t.members_count(c.participants.len())
        )
    } else {
        match peer.as_deref().and_then(|p| app.state.contacts.get(p)) {
            Some(k) if !k.note.is_empty() => format!("{} – {}", t.presence(k.presence), k.note),
            Some(k) => t.presence(k.presence).to_string(),
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
    egui::Panel::top(("conv-head", key.0))
        .frame(
            Frame::new()
                .fill(pal.surface)
                .inner_margin(Margin::symmetric(16, 10))
                .stroke(Stroke::new(1.0, pal.border)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let shown = group_avatar(app, peer.is_none(), &title).to_string();
                app.avatar(ui, peer.as_deref(), &shown, 40.0, presence, pal);
                ui.vertical(|ui| {
                    widgets::ellipsis(
                        ui,
                        RichText::new(&title)
                            .size(theme::text::PANEL_TITLE - 1.0)
                            .family(crate::theme::bold())
                            .color(pal.ink),
                    );
                    widgets::ellipsis(
                        ui,
                        RichText::new(subtitle)
                            .size(theme::text::SMALL + 0.5)
                            .color(pal.muted),
                    );
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
            app.avatar(ui, Some(sender.as_str()), &name, 30.0, None, pal);
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
    let room = c.room.as_ref().map(|r| r.uri.clone());
    // A room takes messages once joined; a conversation already while it opens.
    let open = match room {
        Some(_) => c.state == ConvState::Open,
        None => matches!(c.state, ConvState::Open | ConvState::Opening),
    };
    let title = c.title.clone();
    let peer = c.peer.clone();
    let chat_gate = if room.is_some() {
        app.state.capabilities.persistent_chat.clone()
    } else {
        app.state.capabilities.chat.clone()
    };
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
                    if r.changed() && text_len > 0 && room.is_none() {
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
                                match room.clone() {
                                    Some(room) => app.emit(UiIntent::SendToRoom { room, text }),
                                    None => app.emit(UiIntent::SendMessage { key, text }),
                                }
                                app.focus_composer = true;
                            }
                            if peer.is_none() && room.is_none() && open {
                                let gate = app.state.capabilities.group_chat.clone();
                                if let Some(r) =
                                    widgets::gated(ui, t.invite, &gate, t, pal, |ui, l, en| {
                                        ui.add_enabled(en, egui::Button::new(l))
                                    })
                                {
                                    if r.clicked() {
                                        app.new_chat = Some(NewChat {
                                            text: String::new(),
                                            purpose: AddressFor::Invite(key),
                                        });
                                    }
                                }
                            }
                        });
                    });
                });
        });
}

/// The dialog for a new conversation (one address) or a group (several), also used to invite
/// someone into an open group.
pub fn new_chat_dialog(app: &mut CommsUi, ctx: &egui::Context, pal: &Palette) {
    let Some(mut dlg) = app.new_chat.take() else {
        return;
    };
    let t = app.t;
    let purpose = dlg.purpose;
    let mut keep = true;
    let modal = egui::Modal::new(egui::Id::unique("new-chat")).show(ctx, |ui| {
        ui.set_width(440.0);
        ui.label(
            RichText::new(match purpose {
                AddressFor::NewChat => t.new_chat_title,
                AddressFor::Invite(_) => t.invite_title,
                AddressFor::Transfer(_) => t.transfer_title,
                AddressFor::AddContact => t.add_contact,
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
                let label = match purpose {
                    AddressFor::NewChat => t.start_chat,
                    AddressFor::Invite(_) => t.invite,
                    AddressFor::Transfer(_) => t.transfer,
                    AddressFor::AddContact => t.add,
                };
                if (widgets::primary(ui, label, ok, pal).clicked() || (enter && ok)) && ok {
                    if let Some(a) = address.clone() {
                        match purpose {
                            AddressFor::Invite(k) => {
                                app.emit(UiIntent::InviteToGroup { key: k, peer: a })
                            }
                            AddressFor::Transfer(id) => {
                                app.emit(UiIntent::Transfer { id, peer: a })
                            }
                            AddressFor::NewChat => app.chat_with(&a),
                            AddressFor::AddContact => {
                                let name = app.state.known_name(&a);
                                app.emit(UiIntent::AddContact {
                                    uri: a,
                                    name,
                                    group: None,
                                })
                            }
                        }
                    }
                    keep = false;
                }
                if purpose == AddressFor::NewChat {
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
