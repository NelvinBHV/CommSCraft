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

/// A conversation in its own window: head, composer, history.
pub fn conversation_body(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    window_header(app, ui, pal, key);
    composer(app, ui, pal, key);
    history(app, ui, pal, key);
}

/// The IM of a conversation without its head: the pane beside a call's stage.
pub fn conversation_im(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
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
        // A fixed height: name, status and participants link (the panel does not grow with
        // its content inside the tabbed window).
        .exact_size(96.0)
        .frame(
            Frame::new()
                .fill(pal.surface)
                .inner_margin(Margin::symmetric(16, 10))
                .stroke(Stroke::new(1.0, pal.border)),
        )
        .show(ui, |ui| {
            // As in the Windows client: a presence dot and the name large, the status below,
            // the participants as a link, the add-people button at the right.
            let count = app.state.conversation(key).map_or(0, |c| {
                c.participants.len().max(if c.is_group() { 1 } else { 2 })
            });
            ui.horizontal(|ui| {
                if let Some(p) = presence {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                    ui.painter()
                        .circle_filled(r.center(), 5.0, widgets::presence_colour(p, pal));
                }
                ui.add(
                    egui::Label::new(
                        RichText::new(&title)
                            .size(theme::text::PANEL_TITLE + 2.0)
                            .color(pal.ink),
                    )
                    .truncate(),
                );
                // The add-people button at the right edge.
                ui.add_space((ui.available_width() - 38.0).max(0.0));
                let gate = app.state.capabilities.group_chat.clone();
                let add = crate::views::icons::round(
                    ui,
                    crate::views::icons::Icon::AddPerson,
                    t.invite,
                    34.0,
                    pal.primary,
                    false,
                    gate.usable(),
                );
                if add.clicked() && gate.usable() {
                    app.new_chat = Some(NewChat {
                        text: String::new(),
                        purpose: AddressFor::Invite(key),
                    });
                }
            });
            ui.add(
                egui::Label::new(
                    RichText::new(subtitle)
                        .size(theme::text::SMALL + 0.5)
                        .color(pal.ink),
                )
                .truncate(),
            );
            let open = app.participants_open.contains(&key);
            let link = ui
                .add(
                    egui::Button::new(
                        RichText::new(t.participants_count(count))
                            .size(theme::text::SMALL + 0.5)
                            .color(pal.primary),
                    )
                    .frame(false),
                )
                .on_hover_text(t.participants);
            if link.clicked() {
                if open {
                    app.participants_open.remove(&key);
                } else {
                    app.participants_open.insert(key);
                }
            }
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
    // A line of the client's own (a call connected or ended), centred as in the Windows client.
    if m.sender.as_deref() == Some(crate::model::SYSTEM_SENDER) {
        let t = app.t;
        let text = match m.text.split_once(' ') {
            Some(("ended", secs)) => {
                let minutes = secs.parse::<u64>().unwrap_or(0).div_ceil(60);
                t.call_ended_line(minutes)
            }
            _ => t.call_connected_line.to_string(),
        };
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(format!("{text}  {}", format::clock(m.at, app.utc_offset)))
                    .size(theme::text::SMALL)
                    .color(pal.muted),
            );
        });
        return;
    }
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
                    message_body(app, ui, m, pal, max_w);
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
                    message_body(app, ui, m, pal, max_w);
                });
        });
    });
    ui.add_space(4.0);
}

/// A message's body as the Windows client shows it: a high-priority message's red "!" at the
/// start of the line, the sender's formatting, the emoticons as coloured pictures.
fn message_body(app: &CommsUi, ui: &mut Ui, m: &Message, pal: &Palette, max_w: f32) {
    use crate::emoticons::Piece;
    let plain_run;
    let runs: &[crate::model::TextRun] = if m.runs.is_empty() {
        plain_run = [crate::model::TextRun {
            text: m.text.clone(),
            ..Default::default()
        }];
        &plain_run
    } else {
        &m.runs
    };
    let has_emoticon = runs.iter().any(|r| {
        crate::emoticons::pieces(&r.text)
            .iter()
            .any(|p| matches!(p, Piece::Emoticon(_)))
    });
    if !has_emoticon && !m.important {
        ui.add(egui::Label::new(formatted(runs, max_w, pal)).wrap());
        return;
    }
    let wrap = Layout::left_to_right(Align::Center).with_main_wrap(true);
    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 0.0), wrap, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if m.important {
            ui.label(
                RichText::new("! ")
                    .size(theme::text::BODY + 1.0)
                    .family(theme::bold())
                    .color(pal.error),
            );
        }
        for r in runs {
            let colour = r
                .color
                .map(|[cr, cg, cb]| egui::Color32::from_rgb(cr, cg, cb))
                .filter(|c| readable(*c, pal.message_incoming))
                .unwrap_or(pal.ink);
            let styled = |t: &str| {
                let mut rt = RichText::new(t).size(theme::text::BODY).color(colour);
                if r.bold {
                    rt = rt.family(theme::bold());
                }
                if r.italic {
                    rt = rt.italics();
                }
                if r.underline {
                    rt = rt.underline();
                }
                if r.strike {
                    rt = rt.strikethrough();
                }
                rt
            };
            for p in crate::emoticons::pieces(&r.text) {
                match p {
                    Piece::Text(t) => {
                        for (i, line) in t.split('\n').enumerate() {
                            if i > 0 {
                                ui.end_row();
                            }
                            for word in line.split_inclusive(' ') {
                                ui.label(styled(word));
                            }
                        }
                    }
                    Piece::Emoticon(e) => emoticon_picture(app, ui, e, theme::text::BODY, colour),
                }
            }
        }
    });
}

/// One line (a preview in a list) with the emoticons as pictures, cut at the available width.
pub(crate) fn emoticon_line(
    app: &CommsUi,
    ui: &mut Ui,
    text: &str,
    size: f32,
    colour: egui::Color32,
) {
    use crate::emoticons::Piece;
    let line = text.replace('\n', " ");
    let pieces = crate::emoticons::pieces(&line);
    if !pieces.iter().any(|p| matches!(p, Piece::Emoticon(_))) {
        widgets::ellipsis(ui, RichText::new(line).size(size).color(colour));
        return;
    }
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let clip = ui.max_rect();
        ui.set_clip_rect(clip.intersect(ui.clip_rect()));
        for p in pieces {
            match p {
                Piece::Text(t) => {
                    ui.add(egui::Label::new(RichText::new(t).size(size).color(colour)).truncate());
                }
                Piece::Emoticon(e) => emoticon_picture(app, ui, e, size, colour),
            }
        }
    });
}

/// An emoticon's coloured picture at the text's size, or its emoji without a picture.
fn emoticon_picture(app: &CommsUi, ui: &mut Ui, emoji: &str, size: f32, colour: egui::Color32) {
    match app.emoticon_texture(ui.ctx(), emoji) {
        Some(tex) => {
            let side = size * 1.35;
            ui.add(egui::Image::new(&tex).fit_to_exact_size(egui::vec2(side, side)));
        }
        None => {
            ui.label(RichText::new(emoji).size(size).color(colour));
        }
    }
}

/// A formatted message as the sender wrote it: bold, italic, underline, strike-through and its
/// colour (a colour too close to the background falls back to the text colour).
fn formatted(runs: &[crate::model::TextRun], width: f32, pal: &Palette) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    for r in runs {
        let family = if r.bold {
            crate::theme::bold()
        } else {
            egui::FontFamily::Proportional
        };
        let color = r
            .color
            .map(|[cr, cg, cb]| egui::Color32::from_rgb(cr, cg, cb))
            .filter(|c| readable(*c, pal.message_incoming))
            .unwrap_or(pal.ink);
        let stroke = |on: bool| {
            if on {
                egui::Stroke::new(1.0, color)
            } else {
                egui::Stroke::NONE
            }
        };
        job.append(
            &r.text,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(theme::text::BODY, family),
                color,
                italics: r.italic,
                underline: stroke(r.underline),
                strikethrough: stroke(r.strike),
                ..Default::default()
            },
        );
    }
    job
}

/// Enough contrast between a sender's colour and the bubble (relative luminance distance).
fn readable(fg: egui::Color32, bg: egui::Color32) -> bool {
    let lum = |c: egui::Color32| {
        0.2126 * f32::from(c.r()) + 0.7152 * f32::from(c.g()) + 0.0722 * f32::from(c.b())
    };
    (lum(fg) - lum(bg)).abs() > 60.0
}

/// Right click on a misspelled word in the composer: its suggestions; a click replaces it.
fn spelling_menu(app: &mut CommsUi, ui: &mut Ui, r: &egui::Response, key: ConvKey) {
    let Some(speller) = app.speller.clone().filter(|_| app.prefs.spell_check) else {
        return;
    };
    let draft = app.drafts.get(&key).cloned().unwrap_or_default();
    let cursor = egui::TextEdit::load_state(ui.ctx(), r.id)
        .and_then(|s| s.cursor.char_range())
        .map(|c| c.primary.index);
    let at = cursor.map(|c| {
        draft
            .char_indices()
            .nth(c.0)
            .map_or(draft.len(), |(i, _)| i)
    });
    let target = at
        .and_then(|a| crate::spell::word_at(&draft, a))
        .filter(|w| draft.get(w.clone()).is_some_and(|x| !speller.check(x)));
    let t = app.t;
    r.context_menu(|ui| {
        let Some(range) = target.clone() else {
            ui.label(t.spelling_no_suggestion);
            return;
        };
        let word = draft.get(range.clone()).unwrap_or_default().to_string();
        if app.spell_menu.as_ref().is_none_or(|(w, _)| *w != word) {
            app.spell_menu = Some((word.clone(), speller.suggest(&word)));
        }
        let suggestions = app
            .spell_menu
            .as_ref()
            .map(|(_, s)| s.clone())
            .unwrap_or_default();
        if suggestions.is_empty() {
            ui.label(t.spelling_no_suggestion);
        }
        for s in suggestions.into_iter().take(8) {
            if ui.button(&s).clicked() {
                if let Some(d) = app.drafts.get_mut(&key) {
                    if d.get(range.clone()) == Some(word.as_str()) {
                        d.replace_range(range.clone(), &s);
                    }
                }
                app.spell_menu = None;
                ui.close();
            }
        }
    });
}

/// The emoticons, as the Windows client's picker: a click writes the code into the draft.
fn emoticon_picker(app: &mut CommsUi, ui: &mut Ui, key: ConvKey, enabled: bool) {
    let t = app.t;
    let pal = app.palette();
    let r = crate::views::icons::button(
        ui,
        crate::views::icons::Icon::Smile,
        t.emoticons,
        24.0,
        false,
        enabled,
        pal.muted,
        pal.primary,
    )
    .on_hover_text(t.emoticons);
    let font = egui::FontId::proportional(18.0);
    egui::Popup::menu(&r).show(|ui| {
        egui::Grid::new(("emoticons", key.0)).show(ui, |ui| {
            for (i, (code, emoji, de, en)) in crate::emoticons::EMOTICONS.iter().enumerate() {
                let name = match app.prefs.lang {
                    crate::text::Lang::De => de,
                    crate::text::Lang::En => en,
                };
                // The coloured picture as in the Windows client; the emoji or the code without.
                let b = match app.emoticon_texture(ui.ctx(), emoji) {
                    Some(tex) => ui.add(
                        egui::Button::image(
                            egui::Image::new(&tex).fit_to_exact_size(egui::vec2(22.0, 22.0)),
                        )
                        .frame(false),
                    ),
                    None => {
                        let has = ui.ctx().fonts_mut(|f| f.has_glyphs(&font, emoji));
                        let face = if has { *emoji } else { *code };
                        ui.add(
                            egui::Button::new(RichText::new(face).font(font.clone())).frame(false),
                        )
                    }
                }
                .on_hover_text(format!("{name}  {code}"));
                b.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, *name));
                if b.clicked() {
                    let d = app.drafts.entry(key).or_default();
                    if !d.is_empty() && !d.ends_with(' ') {
                        d.push(' ');
                    }
                    d.push_str(code);
                    d.push(' ');
                    app.focus_composer = true;
                    ui.close();
                }
                // Ten in a row, as the Windows client's picker.
                if i % 10 == 9 {
                    ui.end_row();
                }
            }
        });
    });
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
            // As in the Windows client: the typing area without a box, the symbols at the right.
            Frame::new()
                .inner_margin(Margin::symmetric(4, 2))
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
                    // Misspelled words underlined (the word being typed is left alone).
                    let speller = app
                        .speller
                        .clone()
                        .filter(|_| app.prefs.spell_check && can_send);
                    let ink = pal.ink;
                    let wrong = pal.error;
                    let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, width: f32| {
                        let text = buf.as_str();
                        let bad = speller
                            .as_deref()
                            .map(|s| crate::spell::misspelled(s, text))
                            .unwrap_or_default();
                        let font = egui::FontId::proportional(theme::text::BODY);
                        let mut job = egui::text::LayoutJob::default();
                        job.wrap.max_width = width;
                        let mut at = 0;
                        for r in bad.iter().chain(std::iter::once(&(text.len()..text.len()))) {
                            if let Some(part) = text.get(at..r.start) {
                                job.append(part, 0.0, egui::TextFormat::simple(font.clone(), ink));
                            }
                            if let Some(word) = text.get(r.clone()) {
                                job.append(
                                    word,
                                    0.0,
                                    egui::TextFormat {
                                        font_id: font.clone(),
                                        color: ink,
                                        underline: egui::Stroke::new(1.5, wrong),
                                        ..Default::default()
                                    },
                                );
                            }
                            at = r.end;
                        }
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    let edit = egui::TextEdit::multiline(draft)
                        .layouter(&mut layouter)
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
                    spelling_menu(app, ui, &r, key);
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
                        // As in the Windows client: the counter at the left, the symbols at the
                        // right ending with the round send button (Lucide), names as tool tips.
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
                            let send = crate::views::icons::round(
                                ui,
                                crate::views::icons::Icon::Send,
                                t.send,
                                30.0,
                                pal.primary,
                                true,
                                can_send && has_text,
                            )
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
                            ui.add_space(6.0);
                            let symbol = |ui: &mut Ui, icon, l: &str, en: bool| {
                                crate::views::icons::button(
                                    ui,
                                    icon,
                                    l,
                                    24.0,
                                    false,
                                    en,
                                    pal.muted,
                                    pal.primary,
                                )
                                .on_hover_text(l)
                            };
                            emoticon_picker(app, ui, key, can_send);
                            // High priority, sticky as the Windows client's red "!".
                            let urgent = app.state.conversation(key).is_some_and(|c| c.urgent);
                            let bang = ui
                                .add_enabled(
                                    can_send && room.is_none(),
                                    egui::Button::selectable(
                                        urgent,
                                        RichText::new("!")
                                            .size(18.0)
                                            .family(theme::bold())
                                            .color(pal.error),
                                    ),
                                )
                                .on_hover_text(t.high_priority);
                            if bang.clicked() {
                                if let Some(c) = app.state.conversation_mut(key) {
                                    c.urgent = !urgent;
                                }
                                app.emit(UiIntent::SetUrgent {
                                    key,
                                    urgent: !urgent,
                                });
                            }
                            widgets::gated(ui, t.formatting, &rich_gate, t, pal, |ui, l, en| {
                                symbol(ui, crate::views::icons::Icon::Type, l, en)
                            });
                            widgets::gated(ui, t.attach, &file_gate, t, pal, |ui, l, en| {
                                symbol(ui, crate::views::icons::Icon::Paperclip, l, en)
                            });
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
                                // A 1:1 conversation becomes a group with both, as adding
                                // people does in the Windows client.
                                match app.state.conversation(k).and_then(|c| c.peer.clone()) {
                                    Some(peer) => app.emit(UiIntent::StartGroup {
                                        invite: vec![peer, a],
                                    }),
                                    None => app.emit(UiIntent::InviteToGroup { key: k, peer: a }),
                                }
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
