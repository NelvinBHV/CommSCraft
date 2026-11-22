//! The icons of the interface: Lucide (https://lucide.dev, ISC licence; a subset of the
//! `lucide-static` icon font, `assets/icons/`, listed in ATTRIBUTION.md) when its font is
//! installed ([`install`]), otherwise CommSCraft's own line drawings with the painter.

use egui::{
    epaint::PathStroke, Color32, FontData, FontDefinitions, FontFamily, Painter, Pos2, Rect,
    Stroke, Vec2,
};
use std::sync::Arc;

/// The font family name of the Lucide icons.
pub const LUCIDE: &str = "lucide";
/// The Lucide subset (lucide-static 1.70.0, glyphs listed in `assets/icons/README.md`).
const LUCIDE_FONT: &[u8] = include_bytes!("../../../../assets/icons/lucide-subset.ttf");

/// Adds the Lucide icon font to font definitions as its own family (not as a fallback of the
/// text fonts: its glyphs lie in the private use area).
pub fn install(defs: &mut FontDefinitions) {
    defs.font_data
        .insert(LUCIDE.into(), Arc::new(FontData::from_static(LUCIDE_FONT)));
    defs.families
        .insert(FontFamily::Name(LUCIDE.into()), vec![LUCIDE.into()]);
}

/// The Lucide glyph of an icon (codepoints of lucide-static 1.70.0 `font/codepoints.json`).
fn lucide(icon: Icon) -> char {
    let cp = match icon {
        Icon::Person => 0xe19f,      // user
        Icon::Clock => 0xe087,       // clock
        Icon::Calendar => 0xe063,    // calendar
        Icon::Gear => 0xe154,        // settings
        Icon::Chat => 0xe117,        // message-square
        Icon::Camera => 0xe1a5,      // video
        Icon::Phone => 0xe133,       // phone
        Icon::Screen => 0xe11d,      // monitor
        Icon::People => 0xe1a4,      // users
        Icon::More => 0xe0b6,        // ellipsis
        Icon::Search => 0xe151,      // search
        Icon::AddPerson => 0xe1a2,   // user-plus
        Icon::ChevronDown => 0xe06d, // chevron-down
        Icon::Speaker => 0xe1ab,     // volume-2
        Icon::Mic => 0xe118,         // mic
        Icon::HangUp => 0xe138,      // phone-off
        Icon::Paperclip => 0xe12d,   // paperclip
        Icon::Smile => 0xe164,       // smile
        Icon::Send => 0xe152,        // send
        Icon::Type => 0xe198,        // type
        Icon::Close => 0xe1b2,       // x
        Icon::Warning => 0xe193,     // triangle-alert
    };
    char::from_u32(cp).unwrap_or('?')
}

fn lucide_installed(p: &Painter) -> bool {
    p.ctx().fonts(|f| {
        f.definitions()
            .families
            .contains_key(&FontFamily::Name(LUCIDE.into()))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Person,
    Clock,
    Calendar,
    Gear,
    Chat,
    Camera,
    Phone,
    Screen,
    People,
    More,
    Search,
    AddPerson,
    ChevronDown,
    Speaker,
    /// A microphone (struck through when muted is drawn by the caller).
    Mic,
    /// A handset lying down: hang up.
    HangUp,
    /// Attach a file (the composer).
    Paperclip,
    /// The emoticon picker (the composer).
    Smile,
    /// Send the message (the composer).
    Send,
    /// Text formatting (the composer).
    Type,
    /// Close a pane.
    Close,
    /// Warnings (the main window's footer).
    Warning,
}

/// Draws an icon into `rect` (square-ish): the Lucide glyph, or the own line drawing when the
/// icon font is not installed.
pub fn draw(p: &Painter, rect: Rect, icon: Icon, colour: Color32) {
    if lucide_installed(p) {
        let size = rect.width().min(rect.height()) * 0.9;
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            lucide(icon),
            egui::FontId::new(size, FontFamily::Name(LUCIDE.into())),
            colour,
        );
        return;
    }
    draw_lines(p, rect, icon, colour);
}

/// CommSCraft's own line drawing of an icon with a 1.6 px line.
fn draw_lines(p: &Painter, rect: Rect, icon: Icon, colour: Color32) {
    let s = Stroke::new(1.6, colour);
    let c = rect.center();
    let r = rect.width().min(rect.height()) / 2.0;
    let at = |x: f32, y: f32| Pos2::new(c.x + x * r, c.y + y * r);
    match icon {
        Icon::Person => {
            p.circle_stroke(at(0.0, -0.35), r * 0.32, s);
            arc(p, at(0.0, 0.75), r * 0.62, 200.0, 340.0, s);
        }
        Icon::People => {
            p.circle_stroke(at(-0.3, -0.3), r * 0.26, s);
            arc(p, at(-0.3, 0.7), r * 0.5, 200.0, 340.0, s);
            p.circle_stroke(at(0.38, -0.2), r * 0.22, s);
            arc(p, at(0.38, 0.72), r * 0.42, 210.0, 330.0, s);
        }
        Icon::AddPerson => {
            p.circle_stroke(at(-0.2, -0.35), r * 0.3, s);
            arc(p, at(-0.2, 0.75), r * 0.58, 200.0, 340.0, s);
            p.line_segment([at(0.55, -0.2), at(0.55, 0.3)], s);
            p.line_segment([at(0.3, 0.05), at(0.8, 0.05)], s);
        }
        Icon::Clock => {
            p.circle_stroke(c, r * 0.8, s);
            p.line_segment([c, at(0.0, -0.5)], s);
            p.line_segment([c, at(0.38, 0.12)], s);
        }
        Icon::Calendar => {
            let body = Rect::from_min_max(at(-0.75, -0.55), at(0.75, 0.8));
            p.rect_stroke(body, 2.0, s, egui::StrokeKind::Middle);
            p.line_segment([at(-0.75, -0.2), at(0.75, -0.2)], s);
            p.line_segment([at(-0.35, -0.8), at(-0.35, -0.4)], s);
            p.line_segment([at(0.35, -0.8), at(0.35, -0.4)], s);
        }
        Icon::Gear => {
            p.circle_stroke(c, r * 0.32, s);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = Vec2::angled(a);
                p.line_segment(
                    [c + d * r * 0.55, c + d * r * 0.85],
                    Stroke::new(2.4, colour),
                );
            }
            p.circle_stroke(c, r * 0.62, s);
        }
        Icon::Mic => {
            let body = Rect::from_min_max(at(-0.28, -0.85), at(0.28, 0.2));
            p.rect_stroke(body, r * 0.28, s, egui::StrokeKind::Middle);
            arc(p, at(0.0, -0.05), r * 0.55, 20.0, 160.0, s);
            p.line_segment([at(0.0, 0.5), at(0.0, 0.85)], s);
            p.line_segment([at(-0.35, 0.85), at(0.35, 0.85)], s);
        }
        Icon::HangUp => {
            // A handset on its back: the grip as a wide arc, the two pieces at its ends.
            arc(
                p,
                at(0.0, 0.55),
                r * 0.85,
                225.0,
                315.0,
                Stroke::new(3.0, colour),
            );
            for x in [-0.62f32, 0.62] {
                p.circle_filled(at(x, 0.0), r * 0.2, colour);
            }
        }
        Icon::Chat => {
            let body = Rect::from_min_max(at(-0.8, -0.65), at(0.8, 0.4));
            p.rect_stroke(body, 4.0, s, egui::StrokeKind::Middle);
            p.add(egui::Shape::line(
                vec![at(-0.35, 0.4), at(-0.5, 0.8), at(0.0, 0.4)],
                PathStroke::new(1.6, colour),
            ));
        }
        Icon::Camera => {
            let body = Rect::from_min_max(at(-0.85, -0.45), at(0.35, 0.45));
            p.rect_stroke(body, 3.0, s, egui::StrokeKind::Middle);
            p.add(egui::Shape::closed_line(
                vec![at(0.35, -0.1), at(0.85, -0.4), at(0.85, 0.4), at(0.35, 0.1)],
                PathStroke::new(1.6, colour),
            ));
        }
        Icon::Phone => {
            // A handset: the curved grip with the ear and mouth pieces at its ends.
            arc(
                p,
                at(0.35, 0.0),
                r * 0.7,
                120.0,
                240.0,
                Stroke::new(3.0, colour),
            );
            for a in [120.0f32, 240.0] {
                let e = at(0.35, 0.0) + Vec2::angled(a.to_radians()) * r * 0.7;
                p.circle_filled(e, r * 0.2, colour);
            }
        }
        Icon::Screen => {
            let body = Rect::from_min_max(at(-0.85, -0.6), at(0.85, 0.45));
            p.rect_stroke(body, 2.0, s, egui::StrokeKind::Middle);
            p.line_segment([at(-0.35, 0.8), at(0.35, 0.8)], s);
            p.line_segment([at(0.0, 0.45), at(0.0, 0.8)], s);
        }
        Icon::More => {
            for x in [-0.55, 0.0, 0.55] {
                p.circle_filled(at(x, 0.0), r * 0.14, colour);
            }
        }
        Icon::ChevronDown => {
            p.add(egui::Shape::line(
                vec![at(-0.5, -0.2), at(0.0, 0.3), at(0.5, -0.2)],
                PathStroke::new(1.8, colour),
            ));
        }
        Icon::Speaker => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(-0.8, -0.3),
                    at(-0.4, -0.3),
                    at(0.1, -0.75),
                    at(0.1, 0.75),
                    at(-0.4, 0.3),
                    at(-0.8, 0.3),
                ],
                PathStroke::new(1.6, colour),
            ));
            arc(p, at(0.1, 0.0), r * 0.4, -50.0, 50.0, s);
            arc(p, at(0.1, 0.0), r * 0.7, -50.0, 50.0, s);
        }
        Icon::Search => {
            p.circle_stroke(at(-0.15, -0.15), r * 0.5, s);
            p.line_segment([at(0.22, 0.22), at(0.75, 0.75)], Stroke::new(2.2, colour));
        }
        Icon::Paperclip => {
            arc(p, at(0.0, -0.35), r * 0.3, 180.0, 360.0, s);
            arc(p, at(0.0, 0.35), r * 0.3, 0.0, 180.0, s);
            p.line_segment([at(-0.3, -0.35), at(-0.3, 0.35)], s);
            p.line_segment([at(0.3, -0.35), at(0.3, 0.35)], s);
        }
        Icon::Smile => {
            p.circle_stroke(c, r * 0.8, s);
            p.circle_filled(at(-0.3, -0.2), r * 0.08, colour);
            p.circle_filled(at(0.3, -0.2), r * 0.08, colour);
            arc(p, at(0.0, 0.05), r * 0.4, 20.0, 160.0, s);
        }
        Icon::Send => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(-0.75, -0.6),
                    at(0.75, 0.0),
                    at(-0.75, 0.6),
                    at(-0.4, 0.0),
                ],
                s,
            ));
        }
        Icon::Type => {
            p.line_segment([at(-0.6, -0.6), at(0.6, -0.6)], s);
            p.line_segment([at(0.0, -0.6), at(0.0, 0.7)], s);
        }
        Icon::Close => {
            p.line_segment([at(-0.55, -0.55), at(0.55, 0.55)], s);
            p.line_segment([at(0.55, -0.55), at(-0.55, 0.55)], s);
        }
        Icon::Warning => {
            p.add(egui::Shape::closed_line(
                vec![at(0.0, -0.75), at(0.8, 0.65), at(-0.8, 0.65)],
                s,
            ));
            p.line_segment([at(0.0, -0.25), at(0.0, 0.2)], s);
            p.circle_filled(at(0.0, 0.42), r * 0.07, colour);
        }
    }
}

/// An arc from `from` to `to` degrees (0° = right, clockwise in screen coordinates).
fn arc(p: &Painter, centre: Pos2, radius: f32, from: f32, to: f32, s: Stroke) {
    let n = 16;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = (from + (to - from) * i as f32 / n as f32).to_radians();
            centre + Vec2::angled(a) * radius
        })
        .collect();
    p.add(egui::Shape::line(pts, PathStroke::new(s.width, s.color)));
}

/// A text button followed by a small chevron (a menu opens on click).
pub fn menu_text(ui: &mut egui::Ui, text: egui::RichText, colour: Color32) -> egui::Response {
    let r = ui.add(egui::Button::new(text).frame(false));
    let c = Rect::from_center_size(
        Pos2::new(r.rect.right() + 7.0, r.rect.center().y),
        Vec2::splat(10.0),
    );
    ui.allocate_exact_size(Vec2::new(14.0, 1.0), egui::Sense::hover());
    draw(ui.painter(), c, Icon::ChevronDown, colour);
    r
}

#[allow(clippy::too_many_arguments)]
/// A square icon button with a tooltip-capable response; `selected` draws the underline.
pub fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    size: f32,
    selected: bool,
    enabled: bool,
    colour: Color32,
    accent: Color32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());
    let resp = if enabled {
        resp
    } else {
        resp.on_hover_cursor(egui::CursorIcon::NotAllowed)
    };
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 4.0, accent.gamma_multiply(0.12));
        p.line_segment(
            [
                rect.left_bottom() + Vec2::new(6.0, -2.0),
                rect.right_bottom() + Vec2::new(-6.0, -2.0),
            ],
            Stroke::new(2.5, accent),
        );
    } else if resp.hovered() && enabled {
        p.rect_filled(rect, 4.0, accent.gamma_multiply(0.07));
    }
    if resp.has_focus() {
        p.rect_stroke(
            rect,
            4.0,
            Stroke::new(2.0, accent),
            egui::StrokeKind::Inside,
        );
    }
    let c = if enabled {
        if selected {
            accent
        } else {
            colour
        }
    } else {
        colour.gamma_multiply(0.35)
    };
    draw(p, rect.shrink(size * 0.24), icon, c);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, enabled, selected, label));
    resp
}
