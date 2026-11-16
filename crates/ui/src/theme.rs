//! The colour tokens and measures of the interface (blue and slate), in one place.
//!
//! The brand logo keeps its own colours; buttons, selection and status use these tokens.

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals};

const fn hex(rgb: u32) -> Color32 {
    // The three bytes of 0xRRGGBB; `as u8` keeps the low byte on purpose.
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// Colour tokens. High contrast replaces the quiet greys with darker ones.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub primary: Color32,
    pub primary_hover: Color32,
    pub primary_soft: Color32,
    pub message_outgoing: Color32,
    pub message_incoming: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub quiet: Color32,
    pub surface: Color32,
    pub surface_subtle: Color32,
    pub navigation_bg: Color32,
    pub border: Color32,
    pub success: Color32,
    /// Success as text (the dot colour is too light for 4.5:1 on tinted surfaces).
    pub success_text: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub on_primary: Color32,
}

pub const LIGHT: Palette = Palette {
    primary: hex(0x3267CC),
    primary_hover: hex(0x2456B6),
    primary_soft: hex(0xE8F0FF),
    message_outgoing: hex(0xE5EEFF),
    message_incoming: hex(0xF0F3F8),
    ink: hex(0x24344F),
    // The package names #64748B; it reaches only 4.2:1 on the selection and message colours,
    // so secondary text is a little darker to meet 4.5:1 everywhere.
    muted: hex(0x586A80),
    quiet: hex(0x94A3B8),
    surface: hex(0xFFFFFF),
    surface_subtle: hex(0xF8FAFC),
    navigation_bg: hex(0xF4F6FA),
    border: hex(0xDFE6F0),
    success: hex(0x1E8E69),
    success_text: hex(0x177A59),
    warning: hex(0xB7791F),
    error: hex(0xBA3D48),
    on_primary: hex(0xFFFFFF),
};

/// High contrast: secondary text and lines darker, the rest unchanged.
pub const HIGH_CONTRAST: Palette = Palette {
    muted: hex(0x334155),
    quiet: hex(0x475569),
    border: hex(0x64748B),
    ..LIGHT
};

/// Layout measures in points (egui scales them with the display).
pub mod size {
    pub const TOPBAR: f32 = 66.0;
    pub const RAIL: f32 = 92.0;
    pub const LIST: f32 = 300.0;
    pub const CONTEXT: f32 = 290.0;
    pub const STATUSBAR: f32 = 28.0;
    /// Below this window width the context pane closes by itself.
    pub const CONTEXT_MIN_WINDOW: f32 = 1100.0;
    pub const RADIUS: u8 = 8;
    pub const GAP: f32 = 8.0;
}

/// Text sizes (points): heading, panel title, names, messages, secondary.
pub mod text {
    pub const HEADING: f32 = 22.0;
    pub const PANEL_TITLE: f32 = 19.0;
    pub const NAME: f32 = 14.5;
    pub const BODY: f32 = 13.5;
    pub const SMALL: f32 = 11.5;
}

/// The name of the bold font family. The application must define it (the system's bold face,
/// or the regular one when there is none); egui does not embolden by itself.
pub const BOLD: &str = "bold";

pub fn bold() -> FontFamily {
    FontFamily::Name(BOLD.into())
}

/// WCAG relative luminance of a colour (sRGB).
pub fn luminance(c: Color32) -> f64 {
    let lin = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// WCAG contrast ratio of two colours (1 … 21).
pub fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Applies the palette to egui's style: light theme, visible focus, 8 px spacing.
pub fn apply(ctx: &egui::Context, p: &Palette) {
    let mut v = Visuals::light();
    v.override_text_color = Some(p.ink);
    v.panel_fill = p.surface;
    v.window_fill = p.surface;
    v.extreme_bg_color = p.surface;
    v.faint_bg_color = p.surface_subtle;
    v.hyperlink_color = p.primary;
    v.selection.bg_fill = p.primary_soft;
    v.selection.stroke = Stroke::new(1.0, p.primary);
    v.window_stroke = Stroke::new(1.0, p.border);
    let radius = CornerRadius::same(size::RADIUS);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = radius;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.ink);
    // Slider rails, check boxes and scroll bars use `bg_fill`; buttons use `weak_bg_fill`.
    v.widgets.inactive.bg_fill = p.border;
    v.widgets.inactive.weak_bg_fill = p.surface;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.ink);
    v.widgets.hovered.bg_fill = p.primary_soft;
    v.widgets.hovered.weak_bg_fill = p.primary_soft;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.primary_hover);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, p.primary_hover);
    v.widgets.active.bg_fill = p.primary_soft;
    v.widgets.active.weak_bg_fill = p.primary_soft;
    v.widgets.active.bg_stroke = Stroke::new(2.0, p.primary);
    v.widgets.active.fg_stroke = Stroke::new(2.0, p.primary);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(size::GAP, size::GAP);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        s.spacing.window_margin = Margin::same(12);
        s.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(text::HEADING, FontFamily::Proportional),
            ),
            (
                TextStyle::Body,
                FontId::new(text::BODY, FontFamily::Proportional),
            ),
            (
                TextStyle::Button,
                FontId::new(text::BODY, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(text::SMALL, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(text::BODY, FontFamily::Monospace),
            ),
        ]
        .into();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_match_the_specification() {
        assert_eq!(LIGHT.primary, Color32::from_rgb(0x32, 0x67, 0xCC));
        assert_eq!(LIGHT.ink, Color32::from_rgb(0x24, 0x34, 0x4F));
        assert_eq!(LIGHT.error, Color32::from_rgb(0xBA, 0x3D, 0x48));
    }

    /// Normal-size text needs at least 4.5:1 on the surfaces it is drawn on.
    #[test]
    fn text_contrast_is_at_least_aa() {
        for p in [LIGHT, HIGH_CONTRAST] {
            for bg in [
                p.surface,
                p.surface_subtle,
                p.navigation_bg,
                p.primary_soft,
                p.message_outgoing,
                p.message_incoming,
            ] {
                assert!(contrast(p.ink, bg) >= 4.5, "ink on {bg:?}");
                assert!(contrast(p.muted, bg) >= 4.5, "muted on {bg:?}");
            }
            assert!(contrast(p.on_primary, p.primary) >= 4.5);
            assert!(contrast(p.on_primary, p.error) >= 4.5);
            assert!(contrast(p.primary, p.surface) >= 4.5);
        }
    }

    /// Status colours are used as text (labels with words, never colour alone).
    #[test]
    fn status_colours_are_readable_as_text() {
        assert!(contrast(LIGHT.success_text, LIGHT.surface) >= 4.5);
        assert!(contrast(LIGHT.success_text, LIGHT.primary_soft) >= 4.5);
        assert!(contrast(LIGHT.error, LIGHT.surface) >= 4.5);
        // Status dots (non-text graphics) need 3:1; they always come with a word.
        for c in [LIGHT.success, LIGHT.warning, LIGHT.error] {
            assert!(contrast(c, LIGHT.surface_subtle) >= 3.0);
        }
    }
}
