//! Hand-painted widgets that reproduce the Qt style sheet of the original GUI.

use eframe::egui::{
    self, epaint::Vertex, pos2, vec2, Align, Align2, Color32, CursorIcon, FontId, Frame, Layout, Margin,
    Mesh, Pos2, Rect, Response, RichText, Sense, Shape, Stroke, StrokeKind, Ui,
};

use super::theme::{card_shadow, Palette};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Link,
    Folder,
    FolderOpen,
    Clipboard,
    Log,
    Gear,
    Download,
}

/// Vector icons (fonts ship inconsistent emoji glyphs, so draw them).
pub fn paint_icon(pt: &egui::Painter, c: Pos2, s: f32, icon: Icon, col: Color32) {
    let st = Stroke::new(1.5, col);
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(c + vec2(x0, y0) * s, c + vec2(x1, y1) * s);
    let l = |a: (f32, f32), b: (f32, f32)| pt.line_segment([c + vec2(a.0, a.1) * s, c + vec2(b.0, b.1) * s], st);
    match icon {
        Icon::Folder | Icon::FolderOpen => {
            pt.rect_stroke(r(-0.52, -0.28, 0.52, 0.42), 2.0, st, StrokeKind::Middle);
            l((-0.52, -0.28), (-0.52, -0.44));
            l((-0.52, -0.44), (-0.12, -0.44));
            l((-0.12, -0.44), (0.0, -0.28));
            if icon == Icon::FolderOpen {
                pt.add(Shape::line(
                    vec![c + vec2(-0.52, 0.42) * s, c + vec2(-0.36, 0.02) * s, c + vec2(0.66, 0.02) * s, c + vec2(0.5, 0.42) * s],
                    st,
                ));
            }
        }
        Icon::Clipboard => {
            pt.rect_stroke(r(-0.4, -0.38, 0.4, 0.52), 2.0, st, StrokeKind::Middle);
            pt.rect_filled(r(-0.2, -0.54, 0.2, -0.26), 1.5, col);
            l((-0.2, 0.05), (0.2, 0.05));
            l((-0.2, 0.3), (0.2, 0.3));
        }
        Icon::Link => {
            pt.rect_stroke(r(-0.58, -0.22, 0.12, 0.22), 4.0, st, StrokeKind::Middle);
            pt.rect_stroke(r(-0.12, -0.22, 0.58, 0.22), 4.0, st, StrokeKind::Middle);
        }
        Icon::Log => {
            for (i, y) in [-0.36_f32, 0.0, 0.36].into_iter().enumerate() {
                pt.circle_filled(c + vec2(-0.5, y) * s, 1.3, col);
                l((-0.25, y), (if i == 1 { 0.35 } else { 0.55 }, y));
            }
        }
        Icon::Gear => {
            pt.text(c, Align2::CENTER_CENTER, "⚙", FontId::proportional(s * 1.15), col);
        }
        Icon::Download => {
            let st2 = Stroke::new(1.8, col);
            pt.line_segment([c + vec2(0.0, -0.5) * s, c + vec2(0.0, 0.25) * s], st2);
            pt.line_segment([c + vec2(-0.3, -0.05) * s, c + vec2(0.0, 0.28) * s], st2);
            pt.line_segment([c + vec2(0.3, -0.05) * s, c + vec2(0.0, 0.28) * s], st2);
            pt.line_segment([c + vec2(-0.42, 0.52) * s, c + vec2(0.42, 0.52) * s], st2);
        }
    }
}

pub fn elide(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let s: String = text.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", s.trim_end())
    }
}

pub fn card<R>(ui: &mut Ui, p: &Palette, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(p.card_bg)
        .stroke(Stroke::new(1.0, p.card_border))
        .corner_radius(16.0)
        .inner_margin(Margin::symmetric(16, 13))
        .shadow(card_shadow(p))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

pub fn section_label(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(RichText::new(text).size(12.5).color(p.muted).strong());
}

pub fn hint(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(RichText::new(text).size(12.0).color(p.muted));
}

pub fn mini_icon(ui: &mut Ui, p: &Palette, icon: Icon) {
    let (rect, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
    ui.painter().rect_filled(rect, 8.0, p.tonal_bg);
    paint_icon(ui.painter(), rect.center(), 15.0, icon, p.primary);
}

/// Icon + title (+ subtitle) on the left, `trailing` right-aligned on the same row.
pub fn card_header(ui: &mut Ui, p: &Palette, icon: Icon, title: &str, subtitle: Option<&str>, trailing: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        mini_icon(ui, p, icon);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(RichText::new(title).size(14.0).color(p.on_surface).strong());
            if let Some(s) = subtitle {
                ui.label(RichText::new(s).size(11.5).color(p.muted));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), trailing);
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BtnKind {
    Tonal,
    Cta,
    Nav,
    Danger,
}

pub fn button(ui: &mut Ui, p: &Palette, icon: Option<Icon>, text: &str, kind: BtnKind, checked: bool, enabled: bool) -> Response {
    let (size_pt, pad, radius, h_min) = match kind {
        BtnKind::Tonal => (12.5, vec2(16.0, 8.0), 10.0, 0.0),
        BtnKind::Cta => (14.0, vec2(20.0, 13.0), 23.0, 46.0),
        BtnKind::Nav | BtnKind::Danger => (12.0, vec2(16.0, 7.0), 16.0, 0.0),
    };
    let font = FontId::proportional(size_pt);
    let galley = ui.fonts(|f| f.layout_no_wrap(text.to_string(), font, Color32::WHITE));
    let icon_w = if icon.is_some() { 14.0 + 8.0 } else { 0.0 };
    let mut size = galley.size() + vec2(icon_w, 0.0) + pad * 2.0;
    if kind == BtnKind::Cta {
        size.x = ui.available_width();
    }
    size.y = size.y.max(h_min);
    let (rect, resp) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    let (hover, down) = (resp.hovered() && enabled, resp.is_pointer_button_down_on() && enabled);

    let (fill, stroke, text_color) = match kind {
        BtnKind::Tonal => (
            if down { p.tonal_border } else { p.tonal_bg },
            Stroke::new(1.0, if hover { p.primary } else { p.tonal_border }),
            if enabled { p.on_surface } else { p.muted },
        ),
        BtnKind::Cta => (
            if !enabled { p.tonal_bg } else if down { p.accent_end } else if hover { p.accent_start } else { p.primary },
            Stroke::NONE,
            if enabled { p.on_primary } else { p.muted },
        ),
        BtnKind::Nav => {
            if checked {
                (p.primary, Stroke::new(1.0, p.primary), p.on_primary)
            } else {
                (p.tonal_bg, Stroke::new(1.0, if hover { p.primary } else { p.tonal_border }), p.on_surface)
            }
        }
        BtnKind::Danger => (
            if hover { p.error_bg } else { Color32::TRANSPARENT },
            Stroke::new(1.0, if enabled { p.error } else { p.tonal_border }),
            if enabled { p.error } else { p.muted },
        ),
    };
    ui.painter().rect(rect, radius, fill, stroke, StrokeKind::Inside);
    let g = ui.fonts(|f| f.layout_no_wrap(text.to_string(), FontId::proportional(size_pt), text_color));
    let group_w = icon_w + g.size().x;
    let x0 = rect.center().x - group_w / 2.0;
    if let Some(ic) = icon {
        paint_icon(ui.painter(), pos2(x0 + 7.0, rect.center().y), 14.0, ic, text_color);
    }
    ui.painter().galley(pos2(x0 + icon_w, rect.center().y - g.size().y / 2.0), g, text_color);
    if enabled {
        return resp.on_hover_cursor(CursorIcon::PointingHand);
    }
    resp
}

/// Pill-shaped on/off switch followed by its label. Returns true when toggled.
pub fn switch(ui: &mut Ui, p: &Palette, value: &mut bool, label: &str) -> bool {
    let galley = ui.fonts(|f| f.layout_no_wrap(label.to_string(), FontId::proportional(13.0), p.on_surface));
    let size = vec2(34.0 + 10.0 + galley.size().x, 22.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let mut changed = false;
    if resp.clicked() {
        *value = !*value;
        changed = true;
    }
    let track = Rect::from_min_size(rect.min + vec2(0.0, 2.0), vec2(34.0, 18.0));
    let (fill, border) = if *value {
        (p.primary, p.primary)
    } else if resp.hovered() {
        (p.tonal_bg, p.muted)
    } else {
        (p.tonal_bg, p.tonal_border)
    };
    let pt = ui.painter();
    pt.rect(track, 9.0, fill, Stroke::new(2.0, border), StrokeKind::Inside);
    let kx = if *value { track.right() - 9.0 } else { track.left() + 9.0 };
    pt.circle_filled(pos2(kx, track.center().y), 5.5, if *value { p.on_primary } else { p.muted });
    pt.galley(pos2(rect.left() + 44.0, rect.center().y - galley.size().y / 2.0), galley, p.on_surface);
    resp.on_hover_cursor(CursorIcon::PointingHand);
    changed
}

/// Rounded progress bar. `frac` in 0..=1.
pub fn progress(ui: &mut Ui, p: &Palette, frac: f32, height: f32, text: Option<&str>) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let r = height / 2.0;
    ui.painter().rect_filled(rect, r, p.track);
    let frac = frac.clamp(0.0, 1.0);
    if frac > 0.0 {
        let w = (rect.width() * frac).max(height.min(rect.width()));
        let fill = Rect::from_min_size(rect.min, vec2(w, height));
        ui.painter().rect_filled(fill, r, p.primary);
    }
    if let Some(t) = text {
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, t, FontId::proportional(11.0), p.on_surface);
    }
}

/// Rounded rectangle filled with a diagonal gradient (egui has no gradient primitive).
pub fn gradient_rounded(ui: &mut Ui, rect: Rect, radius: f32, c0: Color32, c1: Color32) {
    let mut pts: Vec<Pos2> = Vec::new();
    let corners = [
        (rect.right_top() + vec2(-radius, radius), -90.0_f32),
        (rect.right_bottom() + vec2(-radius, -radius), 0.0),
        (rect.left_bottom() + vec2(radius, -radius), 90.0),
        (rect.left_top() + vec2(radius, radius), 180.0),
    ];
    for (c, start) in corners {
        for i in 0..=6 {
            let a = (start + 15.0 * i as f32).to_radians();
            pts.push(c + vec2(a.cos(), a.sin()) * radius);
        }
    }
    let color_at = |q: Pos2| {
        let t = (((q.x - rect.left()) / rect.width() + (q.y - rect.top()) / rect.height()) / 2.0).clamp(0.0, 1.0);
        c0.lerp_to_gamma(c1, t)
    };
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex { pos: rect.center(), uv: egui::epaint::WHITE_UV, color: color_at(rect.center()) });
    for q in &pts {
        mesh.vertices.push(Vertex { pos: *q, uv: egui::epaint::WHITE_UV, color: color_at(*q) });
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.indices.extend_from_slice(&[0, 1 + i, 1 + (i + 1) % n]);
    }
    ui.painter().add(Shape::mesh(mesh));
}

/// App icon: gradient badge with a download arrow.
pub fn icon_badge(ui: &mut Ui, p: &Palette, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    gradient_rounded(ui, rect, 12.0, p.accent_start, p.accent_end);
    paint_download_arrow(ui, rect.center(), size * 0.34, p.on_primary);
}

pub fn paint_download_arrow(ui: &Ui, c: Pos2, s: f32, color: Color32) {
    let st = Stroke::new(2.4, color);
    let pt = ui.painter();
    pt.line_segment([c + vec2(0.0, -s), c + vec2(0.0, s * 0.55)], st);
    pt.line_segment([c + vec2(-s * 0.6, s * 0.0), c + vec2(0.0, s * 0.6)], st);
    pt.line_segment([c + vec2(s * 0.6, s * 0.0), c + vec2(0.0, s * 0.6)], st);
    pt.line_segment([c + vec2(-s * 0.75, s * 1.0), c + vec2(s * 0.75, s * 1.0)], st);
}

pub fn chevron(ui: &Ui, center: Pos2, expanded: bool, color: Color32) {
    let st = Stroke::new(1.6, color);
    let pt = ui.painter();
    if expanded {
        pt.line_segment([center + vec2(-4.0, -2.0), center + vec2(0.0, 2.5)], st);
        pt.line_segment([center + vec2(4.0, -2.0), center + vec2(0.0, 2.5)], st);
    } else {
        pt.line_segment([center + vec2(-2.0, -4.0), center + vec2(2.5, 0.0)], st);
        pt.line_segment([center + vec2(-2.0, 4.0), center + vec2(2.5, 0.0)], st);
    }
}
