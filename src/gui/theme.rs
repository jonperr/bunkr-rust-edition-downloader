//! Palette copied from the PySide6 GUI (gui/theme.py) + egui style setup.

use eframe::egui::{self, epaint::Shadow, Color32, CornerRadius, Stroke, Visuals};

pub const fn rgb(h: u32) -> Color32 {
    Color32::from_rgb((h >> 16) as u8, (h >> 8) as u8, h as u8)
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub page_bg: Color32,
    pub card_bg: Color32,
    pub card_border: Color32,
    pub input_bg: Color32,
    pub input_border: Color32,
    pub on_surface: Color32,
    pub muted: Color32,
    pub placeholder: Color32,
    pub primary: Color32,
    pub accent_start: Color32,
    pub accent_end: Color32,
    pub on_primary: Color32,
    pub tonal_bg: Color32,
    pub tonal_border: Color32,
    pub track: Color32,
    pub success: Color32,
    pub error: Color32,
    pub error_bg: Color32,
    pub warning: Color32,
    pub divider: Color32,
}

pub const DARK: Palette = Palette {
    dark: true,
    page_bg: rgb(0x131B24),
    card_bg: rgb(0x1A2430),
    card_border: rgb(0x2B3946),
    input_bg: rgb(0x101820),
    input_border: rgb(0x2B3946),
    on_surface: rgb(0xEDF1F5),
    muted: rgb(0x8B98A8),
    placeholder: rgb(0x5C6B7A),
    primary: rgb(0x0092F9),
    accent_start: rgb(0x3FB6FF),
    accent_end: rgb(0x0072D6),
    on_primary: rgb(0xFFFFFF),
    tonal_bg: rgb(0x212C38),
    tonal_border: rgb(0x33404F),
    track: rgb(0x233A4B),
    success: rgb(0x2FD16B),
    error: rgb(0xFF5A5F),
    error_bg: rgb(0x3A171A),
    warning: rgb(0xFFB020),
    divider: rgb(0x28333F),
};

pub const LIGHT: Palette = Palette {
    dark: false,
    page_bg: rgb(0xEAF1FA),
    card_bg: rgb(0xFFFFFF),
    card_border: rgb(0xD8E2EC),
    input_bg: rgb(0xF3F7FB),
    input_border: rgb(0xCBD8E4),
    on_surface: rgb(0x101826),
    muted: rgb(0x5B6B7C),
    placeholder: rgb(0x94A3B4),
    primary: rgb(0x0092F9),
    accent_start: rgb(0x3FB6FF),
    accent_end: rgb(0x0072D6),
    on_primary: rgb(0xFFFFFF),
    tonal_bg: rgb(0xEEF3F9),
    tonal_border: rgb(0xD8E2EC),
    track: rgb(0xDCE9F4),
    success: rgb(0x1E8E3E),
    error: rgb(0xD6353B),
    error_bg: rgb(0xFBE2E2),
    warning: rgb(0x8A5300),
    divider: rgb(0xE1E9F1),
};

pub fn card_shadow(p: &Palette) -> Shadow {
    Shadow { offset: [0, 3], blur: 24, spread: 0, color: Color32::from_black_alpha(if p.dark { 70 } else { 22 }) }
}

pub fn apply(ctx: &egui::Context, p: &Palette) {
    let mut v = if p.dark { Visuals::dark() } else { Visuals::light() };
    let r10 = CornerRadius::same(10);

    v.panel_fill = p.page_bg;
    v.window_fill = p.card_bg;
    v.window_stroke = Stroke::new(1.0, p.card_border);
    v.window_corner_radius = CornerRadius::same(16);
    v.extreme_bg_color = p.input_bg; // text-edit background
    v.faint_bg_color = p.tonal_bg;
    v.override_text_color = None;
    v.selection.bg_fill = p.primary.gamma_multiply(0.55);
    v.selection.stroke = Stroke::new(1.5, p.primary); // focus ring of text edits
    v.hyperlink_color = p.primary;

    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.on_surface);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.divider);
    for (w, fill) in [
        (&mut v.widgets.inactive, p.input_bg),
        (&mut v.widgets.hovered, p.input_bg),
        (&mut v.widgets.active, p.input_bg),
        (&mut v.widgets.open, p.input_bg),
    ] {
        w.corner_radius = r10;
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.fg_stroke = Stroke::new(1.0, p.on_surface);
    }
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.input_border);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.primary);
    v.widgets.active.bg_stroke = Stroke::new(1.5, p.primary);
    v.popup_shadow = card_shadow(p);
    v.window_shadow = card_shadow(p);
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        use egui::{FontFamily, FontId, TextStyle::*};
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        s.spacing.interact_size.y = 28.0;
        s.spacing.scroll = egui::style::ScrollStyle { bar_width: 8.0, floating: true, ..Default::default() };
        s.text_styles.insert(Heading, FontId::new(19.0, FontFamily::Proportional));
        s.text_styles.insert(Body, FontId::new(13.0, FontFamily::Proportional));
        s.text_styles.insert(Button, FontId::new(12.5, FontFamily::Proportional));
        s.text_styles.insert(Small, FontId::new(11.5, FontFamily::Proportional));
        s.text_styles.insert(Monospace, FontId::new(11.5, FontFamily::Monospace));
    });
}
