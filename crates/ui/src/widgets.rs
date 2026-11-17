//! Small shared widgets: avatars, presence badges, filter chips, section titles, cards,
//! buttons with a disabled reason.

use crate::model::{Gate, Presence};
use crate::text::Texts;
use crate::theme::{self, Palette};
use egui::{
    Align2, Color32, CornerRadius, FontId, Response, RichText, Sense, Stroke, Ui, Vec2, Widget,
};

/// Soft backgrounds and the matching dark text for avatars (all reach 4.5:1).
const AVATAR: [(u32, u32); 6] = [
    (0xE8F0FF, 0x2456B6),
    (0xE3F5EE, 0x166E52),
    (0xFCE8EC, 0x9F2F45),
    (0xF8EFD9, 0x8A5A10),
    (0xEEE8FB, 0x5B3FA8),
    (0xE6F1F7, 0x1F5F80),
];

const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Up to two initials of a name ("Anna Weber" → "AW", "sip:x@y" → "X").
pub fn initials(name: &str) -> String {
    let name = name.strip_prefix("sip:").unwrap_or(name);
    let name = name.split('@').next().unwrap_or(name);
    let mut out: String = name
        .split(|c: char| c.is_whitespace() || c == '.' || c == '-' || c == '_')
        .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
        .take(2)
        .flat_map(char::to_uppercase)
        .collect();
    if out.is_empty() {
        out.push('?');
    }
    out
}

fn avatar_colours(name: &str) -> (Color32, Color32) {
    // A stable choice from the name (FNV-1a), so a person keeps their colour.
    let mut h: u32 = 0x811C_9DC5;
    for b in name.bytes() {
        h = (h ^ u32::from(b)).wrapping_mul(0x0100_0193);
    }
    let i = usize::try_from(h).unwrap_or(0) % AVATAR.len();
    let (bg, fg) = AVATAR.get(i).copied().unwrap_or((0xE8F0FF, 0x2456B6));
    (rgb(bg), rgb(fg))
}

pub fn presence_colour(p: Presence, pal: &Palette) -> Color32 {
    match p {
        Presence::Available => pal.success,
        Presence::Busy | Presence::InACall | Presence::DoNotDisturb => pal.error,
        Presence::BeRightBack | Presence::Away => pal.warning,
        Presence::Offline | Presence::Unknown => pal.quiet,
    }
}

/// A round avatar with initials; a presence dot when the presence is known.
pub fn avatar(ui: &mut Ui, name: &str, diameter: f32, presence: Option<Presence>, pal: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::hover());
    let (bg, fg) = avatar_colours(name);
    let p = ui.painter();
    p.circle_filled(rect.center(), diameter / 2.0, bg);
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        initials(name),
        FontId::proportional((diameter * 0.32).max(10.0)),
        fg,
    );
    if let Some(pr) = presence.filter(|p| *p != Presence::Unknown) {
        let r = (diameter * 0.12).max(4.0);
        let c = rect.right_bottom() - Vec2::splat(r + 1.0);
        p.circle(
            c,
            r,
            presence_colour(pr, pal),
            Stroke::new(2.0, pal.surface),
        );
    }
}

/// A contact's photo in a circle, with the presence dot like [`avatar`].
pub fn avatar_photo(
    ui: &mut Ui,
    texture: &egui::TextureHandle,
    diameter: f32,
    presence: Option<Presence>,
    pal: &Palette,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::hover());
    egui::Image::new(texture)
        .corner_radius(diameter / 2.0)
        .paint_at(ui, rect);
    if let Some(pr) = presence.filter(|p| *p != Presence::Unknown) {
        let r = (diameter * 0.12).max(4.0);
        let c = rect.right_bottom() - Vec2::splat(r + 1.0);
        ui.painter().circle(
            c,
            r,
            presence_colour(pr, pal),
            Stroke::new(2.0, pal.surface),
        );
    }
}

/// A presence word with its colour dot (never the colour alone).
pub fn presence_badge(ui: &mut Ui, p: Presence, t: &Texts, pal: &Palette) -> Response {
    let fill = match p {
        Presence::Available => Color32::from_rgb(0xE3, 0xF5, 0xEE),
        Presence::Busy | Presence::InACall | Presence::DoNotDisturb => {
            Color32::from_rgb(0xFC, 0xE8, 0xEC)
        }
        Presence::BeRightBack | Presence::Away => Color32::from_rgb(0xF8, 0xEF, 0xD9),
        _ => pal.message_incoming,
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::symmetric(14, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                ui.painter()
                    .circle_filled(r.center(), 5.0, presence_colour(p, pal));
                ui.label(RichText::new(t.presence(p)).color(pal.ink));
            });
        })
        .response
}

/// A filter chip; returns true when clicked.
pub fn chip(ui: &mut Ui, label: &str, selected: bool, pal: &Palette) -> bool {
    chip_response(ui, label, selected, pal).clicked()
}

/// A filter chip, with its response (for a gated chip).
pub fn chip_response(ui: &mut Ui, label: &str, selected: bool, pal: &Palette) -> Response {
    let text = RichText::new(label).color(if selected { pal.primary } else { pal.muted });
    let b = egui::Button::new(text)
        .fill(if selected {
            pal.primary_soft
        } else {
            pal.surface
        })
        .stroke(Stroke::new(
            1.0,
            if selected { pal.primary } else { pal.border },
        ))
        .min_size(Vec2::new(52.0, 30.0))
        .selected(selected);
    ui.add(b)
}

/// An upper-case section title in the list panel.
pub fn section(ui: &mut Ui, title: &str, pal: &Palette) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .size(theme::text::SMALL)
            .color(pal.muted)
            .family(crate::theme::bold()),
    );
}

/// A white card with a border.
pub fn card<R>(ui: &mut Ui, pal: &Palette, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(pal.surface)
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), add)
                .inner
        })
        .inner
}

/// A soft blue hint card.
pub fn hint(ui: &mut Ui, pal: &Palette, title: Option<&str>, body: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(0xF2, 0xF6, 0xFE))
        .stroke(Stroke::new(1.0, Color32::from_rgb(0xD5, 0xE2, 0xFA)))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                if let Some(t) = title {
                    ui.label(
                        RichText::new(t)
                            .color(pal.primary_hover)
                            .family(crate::theme::bold()),
                    );
                }
                ui.add(egui::Label::new(RichText::new(body).color(pal.muted)).wrap());
            });
        });
}

/// The primary (filled blue) button.
pub fn primary(ui: &mut Ui, label: &str, enabled: bool, pal: &Palette) -> Response {
    let b = egui::Button::new(
        RichText::new(label)
            .color(pal.on_primary)
            .family(crate::theme::bold()),
    )
    .fill(if enabled { pal.primary } else { pal.quiet })
    .stroke(Stroke::NONE)
    .min_size(Vec2::new(110.0, 36.0));
    ui.add_enabled(enabled, b)
}

/// The red button for ending a call.
pub fn danger(ui: &mut Ui, label: &str, pal: &Palette) -> Response {
    let b = egui::Button::new(
        RichText::new(label)
            .color(pal.on_primary)
            .family(crate::theme::bold()),
    )
    .fill(pal.error)
    .stroke(Stroke::NONE)
    .min_size(Vec2::new(160.0, 40.0));
    ui.add(b)
}

/// A secondary (outlined) button.
pub fn secondary(ui: &mut Ui, label: &str, enabled: bool, pal: &Palette) -> Response {
    let b = egui::Button::new(RichText::new(label).color(if enabled {
        pal.primary
    } else {
        pal.quiet
    }))
    .fill(pal.surface)
    .stroke(Stroke::new(1.0, pal.border))
    .min_size(Vec2::new(110.0, 36.0));
    ui.add_enabled(enabled, b)
}

/// A button behind a capability gate: disabled with the reason as tooltip when unavailable,
/// marked as a demo when it is only a preview.
pub fn gated(
    ui: &mut Ui,
    label: &str,
    gate: &Gate,
    t: &Texts,
    _pal: &Palette,
    add: impl FnOnce(&mut Ui, &str, bool) -> Response,
) -> Option<Response> {
    let r = add(ui, label, gate.usable());
    match gate {
        Gate::Available => Some(r),
        Gate::Preview => Some(r.on_hover_text(t.preview_hint)),
        Gate::Unavailable(why) => {
            r.on_disabled_hover_text(t.reason(*why));
            None
        }
    }
}

/// A tile button of the call controls: monogram on top, the word below.
pub fn call_tile(
    ui: &mut Ui,
    mono: &str,
    word: &str,
    active: bool,
    enabled: bool,
    pal: &Palette,
) -> Response {
    let size = Vec2::new(100.0, 62.0);
    let (rect, resp) = ui.allocate_exact_size(
        size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let fill = if active {
        pal.primary_soft
    } else if enabled && resp.hovered() {
        pal.surface_subtle
    } else {
        pal.surface
    };
    let stroke = if resp.has_focus() {
        Stroke::new(2.0, pal.primary)
    } else {
        Stroke::new(1.0, pal.border)
    };
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(8),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
    let colour = if enabled {
        pal.primary_hover
    } else {
        pal.quiet
    };
    p.text(
        rect.center_top() + Vec2::new(0.0, 22.0),
        Align2::CENTER_CENTER,
        mono,
        FontId::proportional(17.0),
        colour,
    );
    p.text(
        rect.center_bottom() - Vec2::new(0.0, 16.0),
        Align2::CENTER_CENTER,
        word,
        FontId::proportional(theme::text::SMALL),
        if enabled { pal.ink } else { pal.quiet },
    );
    let info_word = word.to_string();
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Button, enabled, active, &info_word)
    });
    resp
}

/// A thin horizontal line in the border colour.
pub fn rule(ui: &mut Ui, pal: &Palette) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(Vec2::new(w, 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, pal.border);
}

/// A label that cuts long text with an ellipsis instead of growing the layout.
pub fn ellipsis(ui: &mut Ui, text: RichText) -> Response {
    egui::Label::new(text).truncate().ui(ui)
}
