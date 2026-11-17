//! CommSCraft's own line icons, drawn with the painter (no image files, no third-party art).

use egui::{epaint::PathStroke, Color32, Painter, Pos2, Rect, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Person,
    Clock,
    Keypad,
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
    ChevronRight,
    Speaker,
}

/// Draws an icon into `rect` (square-ish) with a 1.6 px line.
pub fn draw(p: &Painter, rect: Rect, icon: Icon, colour: Color32) {
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
        Icon::Keypad => {
            for y in [-0.6, 0.0, 0.6] {
                for x in [-0.6, 0.0, 0.6] {
                    p.circle_filled(at(x, y), r * 0.13, colour);
                }
            }
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
            arc(
                p,
                at(0.15, -0.15),
                r * 0.75,
                100.0,
                260.0,
                Stroke::new(3.0, colour),
            );
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
        Icon::ChevronRight => {
            p.add(egui::Shape::line(
                vec![at(-0.2, -0.5), at(0.3, 0.0), at(-0.2, 0.5)],
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
