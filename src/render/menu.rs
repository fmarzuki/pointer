//! Menu radial: cincin 6 sektor di posisi kursor, tombol "Off" di tengah, dan flyout di
//! luar sektor Color (kipas warna) dan Shape (pilihan bentuk). Semua ukuran dalam piksel
//! logis, dikali `scale`.

use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Shader, Stroke,
    Transform,
};

use super::ink::{InkSettings, Tool};
use super::{Bounds, fill_circle, rgba, text};
use crate::app::Mode;

/// Jari-jari luar cincin.
const R_OUT: f32 = 150.0;
/// Jari-jari tombol tengah.
const R_IN: f32 = 52.0;
/// Flyout (kipas warna/bentuk): jari-jari dalam/luar dan rentang sudut (derajat, 0 = atas,
/// searah jarum jam). Celah antara cincin dan flyout tetap dianggap bagian sektor pemilik
/// agar flyout tidak hilang saat kursor menyeberanginya.
const FLY_IN: f32 = R_OUT + 8.0;
const FLY_OUT: f32 = R_OUT + 46.0;
const FLY_SPAN: f32 = 90.0;
/// Segmen flyout terpilih/disorot menonjol keluar sejauh ini.
const FLY_POP: f32 = 6.0;
/// Jari-jari terluar yang mungkin tergambar (segmen menonjol + garis tepi + anti-alias).
const R_MAX: f32 = FLY_OUT + FLY_POP + 4.0;
/// Jarak tepi menu dari tepi layar.
const MARGIN: f32 = R_MAX + 2.0;

const ACCENT: (u8, u8, u8) = (47, 212, 122);
/// Tombol "?" : sudut, jarak dari pusat, dan jari-jari (di luar cincin, jauh dari flyout).
const HELP_ANGLE: f32 = 45.0;
const HELP_DIST: f32 = R_OUT + 24.0;
const HELP_R: f32 = 15.0;

fn help_center(v: &MenuView) -> (f32, f32) {
    let a = HELP_ANGLE.to_radians();
    (v.center.0 + a.sin() * HELP_DIST * v.scale, v.center.1 - a.cos() * HELP_DIST * v.scale)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Laser,
    Draw,
    Color,
    Shape,
    Erase,
    Spotlight,
}

/// Urutan sektor searah jarum jam mulai dari atas; tiap sektor 60°.
const ITEMS: [Item; 6] = [
    Item::Laser,
    Item::Draw,
    Item::Color,
    Item::Shape,
    Item::Erase,
    Item::Spotlight,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Center,
    Item(Item),
    Swatch(usize),
    ShapeOption(Tool),
    /// Tombol "?" kecil di tepi kanan atas cincin: buka panel shortcut.
    Help,
    Outside,
}

/// Semua yang dibutuhkan untuk menggambar menu.
pub struct MenuView<'a> {
    pub center: (f32, f32),
    pub scale: f32,
    pub hover: Option<Hit>,
    pub mode: Mode,
    pub ink: InkSettings,
    pub palette: &'a [(u8, u8, u8)],
}

/// Pusat menu digeser agar seluruh menu (termasuk kipas warna) tetap di dalam layar.
pub fn clamp_center((x, y): (f32, f32), (w, h): (f32, f32), scale: f32) -> (f32, f32) {
    let m = MARGIN * scale;
    let fit = |v: f32, max: f32| if max <= 2.0 * m { max / 2.0 } else { v.clamp(m, max - m) };
    (fit(x, w), fit(y, h))
}

pub fn bounds(v: &MenuView) -> Bounds {
    let r = R_MAX * v.scale;
    (v.center.0 - r, v.center.1 - r, v.center.0 + r, v.center.1 + r)
}

/// Sudut titik relatif pusat: 0° di atas, bertambah searah jarum jam.
fn angle_of(dx: f32, dy: f32) -> f32 {
    dx.atan2(-dy).to_degrees().rem_euclid(360.0)
}

/// Sektor yang flyout-nya sedang tampil (karena sektor itu atau salah satu opsinya disorot).
fn open_flyout(hover: Option<Hit>) -> Option<Item> {
    match hover? {
        Hit::Item(Item::Color) | Hit::Swatch(_) => Some(Item::Color),
        Hit::Item(Item::Shape) | Hit::ShapeOption(_) => Some(Item::Shape),
        _ => None,
    }
}

/// Sudut tengah flyout = sudut tengah sektor pemiliknya.
fn flyout_center(owner: Item) -> f32 {
    ITEMS.iter().position(|&i| i == owner).unwrap_or(0) as f32 * 60.0
}

fn flyout_len(owner: Item, v: &MenuView) -> usize {
    match owner {
        Item::Color => v.palette.len(),
        _ => Tool::SHAPES.len(),
    }
}

/// Rentang sudut segmen ke-`i` dari `n` segmen flyout `owner`.
fn flyout_segment(owner: Item, i: usize, n: usize) -> (f32, f32) {
    let start = flyout_center(owner) - FLY_SPAN / 2.0;
    let step = FLY_SPAN / n as f32;
    (start + i as f32 * step + 1.5, start + (i + 1) as f32 * step - 1.5)
}

pub fn hit(v: &MenuView, (px, py): (f32, f32)) -> Hit {
    let (dx, dy) = ((px - v.center.0) / v.scale, (py - v.center.1) / v.scale);
    let d = dx.hypot(dy);
    let angle = angle_of(dx, dy);
    if d <= R_IN {
        return Hit::Center;
    }
    if d <= R_OUT {
        return Hit::Item(ITEMS[(((angle + 30.0) % 360.0) / 60.0) as usize % 6]);
    }
    let (hx, hy) = help_center(v);
    if (px - hx).hypot(py - hy) <= (HELP_R + 3.0) * v.scale {
        return Hit::Help;
    }
    if let Some(owner) = open_flyout(v.hover) {
        let n = flyout_len(owner, v);
        let start = flyout_center(owner) - FLY_SPAN / 2.0;
        // Sudut relatif awal flyout, dinormalisasi agar rentang yang melewati 0° tetap benar.
        let rel = (angle - start).rem_euclid(360.0);
        if n > 0 && d <= FLY_OUT && rel < FLY_SPAN {
            if d < FLY_IN {
                return Hit::Item(owner);
            }
            let i = ((rel / FLY_SPAN * n as f32) as usize).min(n - 1);
            return match owner {
                Item::Color => Hit::Swatch(i),
                _ => Hit::ShapeOption(Tool::SHAPES[i]),
            };
        }
    }
    Hit::Outside
}

pub fn draw(pm: &mut Pixmap, v: &MenuView) {
    let s = v.scale;
    let (cx, cy) = v.center;
    let fill = |pm: &mut Pixmap, path: &Path, color: Color| {
        let mut paint = Paint { anti_alias: true, ..Paint::default() };
        paint.set_color(color);
        pm.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
    };

    // Latar cincin.
    fill_circle(pm, cx, cy, R_OUT * s, Shader::SolidColor(Color::from_rgba8(16, 24, 22, 225)));
    for (i, item) in ITEMS.iter().enumerate() {
        let from = i as f32 * 60.0 - 30.0;
        let active = is_active(*item, v);
        let hovered = v.hover == Some(Hit::Item(*item));
        if active || hovered {
            if let Some(p) = annular_sector(cx, cy, R_IN * s, R_OUT * s, from, from + 60.0) {
                let tint = if hovered { Color::from_rgba8(255, 255, 255, 30) } else { rgba(ACCENT, 0.13) };
                fill(pm, &p, tint);
            }
        }
        if hovered {
            // Garis aksen tebal di tepi luar sektor yang disorot.
            stroke_arc(pm, cx, cy, (R_OUT - 3.0) * s, from + 2.0, from + 58.0, 5.0 * s, rgba(ACCENT, 1.0));
        }
    }
    // Pemisah antarsektor.
    let mut seps = PathBuilder::new();
    for i in 0..6 {
        let a = (i as f32 * 60.0 - 30.0).to_radians();
        let (sx, sy) = (a.sin(), -a.cos());
        seps.move_to(cx + sx * R_IN * s, cy + sy * R_IN * s);
        seps.line_to(cx + sx * R_OUT * s, cy + sy * R_OUT * s);
    }
    if let Some(p) = seps.finish() {
        stroke(pm, &p, s, Color::from_rgba8(255, 255, 255, 22));
    }
    stroke_circle(pm, cx, cy, R_OUT * s, 2.0 * s, rgba(ACCENT, 0.85));

    // Tombol tengah.
    let center_fill = if v.hover == Some(Hit::Center) { Color::from_rgba8(48, 64, 58, 245) } else { Color::from_rgba8(28, 38, 35, 245) };
    fill_circle(pm, cx, cy, R_IN * s, Shader::SolidColor(center_fill));
    stroke_circle(pm, cx, cy, R_IN * s, 1.5 * s, rgba(ACCENT, 0.6));
    text::draw_centered(pm, "Off", cx, cy, 16.0 * s, Color::from_rgba8(230, 235, 233, 255));

    // Ikon, label, dan sublabel status tiap sektor.
    let r_mid = (R_IN + R_OUT) / 2.0 + 4.0;
    for (i, item) in ITEMS.iter().enumerate() {
        let a = (i as f32 * 60.0).to_radians();
        let (x, y) = (cx + a.sin() * r_mid * s, cy - a.cos() * r_mid * s);
        draw_icon(pm, *item, x, y - 14.0 * s, s, v);
        text::draw_centered(pm, label(*item), x, y + 10.0 * s, 14.0 * s, Color::WHITE);
        let sub = sublabel(*item, v);
        if !sub.is_empty() {
            text::draw_centered(pm, &sub, x, y + 26.0 * s, 9.5 * s, rgba(ACCENT, 1.0));
        }
    }

    // Tombol "?" (panel shortcut).
    let (hx, hy) = help_center(v);
    let help_fill = if v.hover == Some(Hit::Help) { Color::from_rgba8(48, 64, 58, 245) } else { Color::from_rgba8(16, 24, 22, 230) };
    fill_circle(pm, hx, hy, HELP_R * s, Shader::SolidColor(help_fill));
    stroke_circle(pm, hx, hy, HELP_R * s, 1.5 * s, rgba(ACCENT, 0.85));
    text::draw_centered(pm, "?", hx, hy, 15.0 * s, Color::WHITE);

    match open_flyout(v.hover) {
        Some(Item::Color) => draw_swatches(pm, v),
        Some(Item::Shape) => draw_shape_options(pm, v),
        _ => {}
    }
}

fn label(item: Item) -> &'static str {
    match item {
        Item::Laser => "Laser",
        Item::Draw => "Draw",
        Item::Color => "Color",
        Item::Shape => "Shape",
        Item::Erase => "Erase",
        Item::Spotlight => "Spotlight",
    }
}

fn is_active(item: Item, v: &MenuView) -> bool {
    match item {
        Item::Laser => v.mode == Mode::Laser,
        Item::Spotlight => v.mode == Mode::Spotlight,
        Item::Draw => v.mode == Mode::Draw && !v.ink.tool.is_shape(),
        Item::Shape => v.mode == Mode::Draw && v.ink.tool.is_shape(),
        Item::Color | Item::Erase => false,
    }
}

fn tool_name(t: Tool) -> &'static str {
    match t {
        Tool::Pen => "PEN",
        Tool::Highlighter => "MARKER",
        Tool::Rect => "BOX",
        Tool::Arrow => "ARROW",
        Tool::Ellipse => "ELLIPSE",
        Tool::Line => "LINE",
        Tool::Text => "TEXT",
    }
}

fn sublabel(item: Item, v: &MenuView) -> String {
    match item {
        Item::Laser | Item::Spotlight if is_active(item, v) => "ON".into(),
        Item::Laser | Item::Spotlight => String::new(),
        Item::Draw => tool_name(v.ink.last_freehand).into(),
        Item::Shape => tool_name(v.ink.last_shape).into(),
        Item::Color => current_color(v).map(color_name).unwrap_or("").into(),
        Item::Erase => "ALL".into(),
    }
}

fn current_color(v: &MenuView) -> Option<(u8, u8, u8)> {
    v.palette.get(v.ink.color % v.palette.len().max(1)).copied()
}

/// Nama kasar warna untuk sublabel (warna palet berasal dari config hex).
pub fn color_name((r, g, b): (u8, u8, u8)) -> &'static str {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let chroma = max - min;
    if chroma < 0.15 {
        return if max > 0.8 { "WHITE" } else if max < 0.25 { "BLACK" } else { "GRAY" };
    }
    let hue = if max == r {
        60.0 * ((g - b) / chroma).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / chroma + 2.0)
    } else {
        60.0 * ((r - g) / chroma + 4.0)
    };
    match hue {
        h if h < 15.0 || h >= 335.0 => "RED",
        h if h < 40.0 => "ORANGE",
        h if h < 70.0 => "YELLOW",
        h if h < 165.0 => "GREEN",
        h if h < 255.0 => "BLUE",
        h if h < 300.0 => "PURPLE",
        _ => "PINK",
    }
}

fn draw_swatches(pm: &mut Pixmap, v: &MenuView) {
    let s = v.scale;
    let n = v.palette.len();
    if n == 0 {
        return;
    }
    for (i, &rgb) in v.palette.iter().enumerate() {
        let selected = i == v.ink.color % n;
        let hovered = v.hover == Some(Hit::Swatch(i));
        let outer = if hovered || selected { FLY_OUT + FLY_POP } else { FLY_OUT };
        let (from, to) = flyout_segment(Item::Color, i, n);
        if let Some(p) = annular_sector(v.center.0, v.center.1, FLY_IN * s, outer * s, from, to) {
            let mut paint = Paint { anti_alias: true, ..Paint::default() };
            paint.set_color(rgba(rgb, 1.0));
            pm.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            // Tepi gelap tipis agar swatch terang (putih/kuning) tetap terlihat di latar terang.
            let edge = if selected { (Color::WHITE, 2.5) } else { (Color::from_rgba8(0, 0, 0, 90), 1.0) };
            stroke(pm, &p, edge.1 * s, edge.0);
        }
    }
}

/// Flyout bentuk: segmen gelap dengan ikon tiap bentuk; bentuk terakhir dipakai diberi
/// garis aksen.
fn draw_shape_options(pm: &mut Pixmap, v: &MenuView) {
    let s = v.scale;
    let (cx, cy) = v.center;
    let n = Tool::SHAPES.len();
    for (i, &tool) in Tool::SHAPES.iter().enumerate() {
        let selected = tool == v.ink.last_shape;
        let hovered = v.hover == Some(Hit::ShapeOption(tool));
        let outer = if hovered || selected { FLY_OUT + FLY_POP } else { FLY_OUT };
        let (from, to) = flyout_segment(Item::Shape, i, n);
        let Some(p) = annular_sector(cx, cy, FLY_IN * s, outer * s, from, to) else { continue };
        let fill = if hovered { Color::from_rgba8(52, 70, 63, 245) } else { Color::from_rgba8(16, 24, 22, 235) };
        let mut paint = Paint { anti_alias: true, ..Paint::default() };
        paint.set_color(fill);
        pm.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        let edge = if selected { (rgba(ACCENT, 1.0), 2.5) } else { (rgba(ACCENT, 0.45), 1.0) };
        stroke(pm, &p, edge.1 * s, edge.0);
        let mid = ((from + to) / 2.0).to_radians();
        let r = (FLY_IN + outer) / 2.0 * s;
        let icon = if hovered || selected { Color::WHITE } else { Color::from_rgba8(200, 210, 206, 255) };
        draw_shape_icon(pm, tool, cx + mid.sin() * r, cy - mid.cos() * r, s * 0.8, icon);
    }
}

/// Ikon kecil (±11 px logis) untuk alat bentuk.
fn draw_shape_icon(pm: &mut Pixmap, tool: Tool, x: f32, y: f32, s: f32, color: Color) {
    let line = |pm: &mut Pixmap, pts: &[(f32, f32)]| {
        let mut pb = PathBuilder::new();
        for (i, &(px, py)) in pts.iter().enumerate() {
            let (px, py) = (x + px * s, y + py * s);
            if i == 0 { pb.move_to(px, py) } else { pb.line_to(px, py) }
        }
        if let Some(p) = pb.finish() {
            stroke(pm, &p, 2.5 * s, color);
        }
    };
    match tool {
        Tool::Arrow => {
            line(pm, &[(-9.0, 7.0), (8.0, -7.0)]);
            line(pm, &[(0.0, -7.0), (8.0, -7.0), (8.0, 1.0)]);
        }
        Tool::Ellipse => {
            if let Some(r) = tiny_skia::Rect::from_xywh(x - 11.0 * s, y - 7.0 * s, 22.0 * s, 14.0 * s) {
                if let Some(p) = PathBuilder::from_oval(r) {
                    stroke(pm, &p, 2.5 * s, color);
                }
            }
        }
        Tool::Line => line(pm, &[(-10.0, 7.0), (10.0, -7.0)]),
        Tool::Text => text::draw_centered(pm, "T", x, y, 20.0 * s, color),
        _ => line(pm, &[(-10.0, -7.0), (10.0, -7.0), (10.0, 7.0), (-10.0, 7.0), (-10.0, -7.0)]),
    }
}

fn draw_icon(pm: &mut Pixmap, item: Item, x: f32, y: f32, s: f32, v: &MenuView) {
    let white = Color::from_rgba8(235, 240, 238, 255);
    let line = |pm: &mut Pixmap, pts: &[(f32, f32)], w: f32, color: Color| {
        let mut pb = PathBuilder::new();
        for (i, &(px, py)) in pts.iter().enumerate() {
            let (px, py) = (x + px * s, y + py * s);
            if i == 0 { pb.move_to(px, py) } else { pb.line_to(px, py) }
        }
        if let Some(p) = pb.finish() {
            stroke(pm, &p, w * s, color);
        }
    };
    match item {
        Item::Laser => {
            stroke_circle(pm, x, y, 9.0 * s, 1.5 * s, Color::from_rgba8(255, 70, 70, 140));
            fill_circle(pm, x, y, 5.0 * s, Shader::SolidColor(Color::from_rgba8(255, 50, 50, 255)));
            fill_circle(pm, x, y, 2.0 * s, Shader::SolidColor(Color::from_rgba8(255, 225, 225, 255)));
        }
        Item::Spotlight => {
            fill_circle(pm, x, y, 10.0 * s, Shader::SolidColor(Color::from_rgba8(0, 0, 0, 150)));
            fill_circle(pm, x, y, 6.0 * s, Shader::SolidColor(Color::from_rgba8(255, 250, 220, 255)));
            stroke_circle(pm, x, y, 10.0 * s, 1.5 * s, white);
        }
        Item::Draw => {
            if v.ink.last_freehand == Tool::Highlighter {
                line(pm, &[(-8.0, 6.0), (8.0, -6.0)], 7.0, Color::from_rgba8(255, 220, 0, 170));
            } else {
                line(pm, &[(-8.0, 8.0), (6.0, -6.0)], 3.5, white);
                line(pm, &[(6.0, -6.0), (9.0, -9.0)], 3.5, Color::from_rgba8(255, 80, 80, 255));
            }
        }
        Item::Color => {
            let rgb = current_color(v).unwrap_or((255, 255, 255));
            fill_circle(pm, x, y, 8.0 * s, Shader::SolidColor(rgba(rgb, 1.0)));
            stroke_circle(pm, x, y, 8.0 * s, 1.5 * s, white);
        }
        Item::Shape => draw_shape_icon(pm, v.ink.last_shape, x, y, s, white),
        Item::Erase => {
            // Penghapus miring: badan putih dengan ujung merah muda.
            let body = [(-9.0, 3.0), (1.0, -7.0), (9.0, 1.0), (-1.0, 11.0)];
            let tip = [(-9.0, 3.0), (-4.0, -2.0), (4.0, 6.0), (-1.0, 11.0)];
            for (pts, color) in [(&body[..], white), (&tip[..], Color::from_rgba8(255, 140, 170, 255))] {
                let mut pb = PathBuilder::new();
                pb.move_to(x + pts[0].0 * s, y + pts[0].1 * s);
                for &(px, py) in &pts[1..] {
                    pb.line_to(x + px * s, y + py * s);
                }
                pb.close();
                if let Some(p) = pb.finish() {
                    let mut paint = Paint { anti_alias: true, ..Paint::default() };
                    paint.set_color(color);
                    pm.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
                }
            }
        }
    }
}

/// Sektor cincin dari sudut `from` ke `to` (derajat, 0 di atas, searah jarum jam).
fn annular_sector(cx: f32, cy: f32, r0: f32, r1: f32, from: f32, to: f32) -> Option<Path> {
    let steps = (((to - from).abs() / 3.0).ceil() as usize).max(2);
    let pt = |r: f32, deg: f32| {
        let a = deg.to_radians();
        (cx + a.sin() * r, cy - a.cos() * r)
    };
    let mut pb = PathBuilder::new();
    let (x, y) = pt(r1, from);
    pb.move_to(x, y);
    for i in 1..=steps {
        let (x, y) = pt(r1, from + (to - from) * i as f32 / steps as f32);
        pb.line_to(x, y);
    }
    for i in (0..=steps).rev() {
        let (x, y) = pt(r0, from + (to - from) * i as f32 / steps as f32);
        pb.line_to(x, y);
    }
    pb.close();
    pb.finish()
}

fn stroke_arc(pm: &mut Pixmap, cx: f32, cy: f32, r: f32, from: f32, to: f32, width: f32, color: Color) {
    let steps = (((to - from).abs() / 3.0).ceil() as usize).max(2);
    let mut pb = PathBuilder::new();
    for i in 0..=steps {
        let a = (from + (to - from) * i as f32 / steps as f32).to_radians();
        let (x, y) = (cx + a.sin() * r, cy - a.cos() * r);
        if i == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
    }
    if let Some(p) = pb.finish() {
        stroke(pm, &p, width, color);
    }
}

fn stroke_circle(pm: &mut Pixmap, x: f32, y: f32, r: f32, width: f32, color: Color) {
    if let Some(p) = PathBuilder::from_circle(x, y, r) {
        stroke(pm, &p, width, color);
    }
}

fn stroke(pm: &mut Pixmap, path: &Path, width: f32, color: Color) {
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    paint.set_color(color);
    let st = Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    pm.stroke_path(path, &paint, &st, Transform::identity(), None);
}

#[cfg(test)]
mod tests {
    use super::*;

    const PALETTE: [(u8, u8, u8); 5] = [(255, 32, 32), (255, 212, 0), (32, 192, 64), (32, 128, 255), (255, 255, 255)];

    fn view(hover: Option<Hit>) -> MenuView<'static> {
        MenuView {
            center: (300.0, 300.0),
            scale: 1.0,
            hover,
            mode: Mode::Idle,
            ink: InkSettings::default(),
            palette: &PALETTE,
        }
    }

    #[test]
    fn hit_test_sectors_center_and_outside() {
        let v = view(None);
        assert_eq!(hit(&v, (300.0, 300.0)), Hit::Center);
        assert_eq!(hit(&v, (300.0, 200.0)), Hit::Item(Item::Laser), "atas");
        assert_eq!(hit(&v, (390.0, 250.0)), Hit::Item(Item::Draw), "kanan atas");
        assert_eq!(hit(&v, (390.0, 350.0)), Hit::Item(Item::Color), "kanan bawah");
        assert_eq!(hit(&v, (300.0, 400.0)), Hit::Item(Item::Shape), "bawah");
        assert_eq!(hit(&v, (210.0, 350.0)), Hit::Item(Item::Erase), "kiri bawah");
        assert_eq!(hit(&v, (210.0, 250.0)), Hit::Item(Item::Spotlight), "kiri atas");
        assert_eq!(hit(&v, (300.0, 100.0)), Hit::Outside);
    }

    #[test]
    fn swatches_only_hit_when_visible() {
        // Titik di kipas warna: sudut 120° (kanan bawah), jarak 175.
        let a = 120f32.to_radians();
        let p = (300.0 + a.sin() * 175.0, 300.0 - a.cos() * 175.0);
        assert_eq!(hit(&view(None), p), Hit::Outside);
        assert_eq!(hit(&view(Some(Hit::Item(Item::Color))), p), Hit::Swatch(2), "warna tengah dari 5");
    }

    fn at(angle: f32, d: f32) -> (f32, f32) {
        let a = angle.to_radians();
        (300.0 + a.sin() * d, 300.0 - a.cos() * d)
    }

    #[test]
    fn help_button_hit() {
        assert_eq!(hit(&view(None), at(45.0, R_OUT + 24.0)), Hit::Help);
        assert_eq!(hit(&view(None), at(45.0, R_OUT + 60.0)), Hit::Outside);
        assert_eq!(hit(&view(None), at(45.0, R_OUT - 10.0)), Hit::Item(Item::Draw), "cincin tetap prioritas");
    }

    #[test]
    fn gap_between_ring_and_flyout_keeps_flyout_open() {
        // Celah 150..158 px: tetap sektor pemilik selama flyout terbuka.
        assert_eq!(hit(&view(Some(Hit::Item(Item::Color))), at(120.0, 154.0)), Hit::Item(Item::Color));
        assert_eq!(hit(&view(Some(Hit::Item(Item::Shape))), at(180.0, 154.0)), Hit::Item(Item::Shape));
        assert_eq!(hit(&view(None), at(120.0, 154.0)), Hit::Outside, "flyout tertutup");
    }

    #[test]
    fn shape_flyout_options() {
        let v = view(Some(Hit::Item(Item::Shape)));
        // Rentang 135°..225°, 5 segmen 18°.
        assert_eq!(hit(&v, at(140.0, 175.0)), Hit::ShapeOption(Tool::Rect));
        assert_eq!(hit(&v, at(160.0, 175.0)), Hit::ShapeOption(Tool::Arrow));
        assert_eq!(hit(&v, at(180.0, 175.0)), Hit::ShapeOption(Tool::Ellipse));
        assert_eq!(hit(&v, at(197.0, 175.0)), Hit::ShapeOption(Tool::Line));
        assert_eq!(hit(&v, at(220.0, 175.0)), Hit::ShapeOption(Tool::Text));
        assert_eq!(hit(&v, at(240.0, 175.0)), Hit::Outside);
        // Pindah dari satu opsi ke opsi lain tetap di flyout.
        let v = view(Some(Hit::ShapeOption(Tool::Rect)));
        assert_eq!(hit(&v, at(220.0, 175.0)), Hit::ShapeOption(Tool::Text));
        assert_eq!(hit(&view(None), at(190.0, 175.0)), Hit::Outside, "flyout tertutup");
    }

    #[test]
    fn shape_flyout_draws_within_bounds() {
        let v = view(Some(Hit::ShapeOption(Tool::Line)));
        let mut pm = Pixmap::new(600, 600).unwrap();
        draw(&mut pm, &v);
        let (x0, y0, x1, y1) = bounds(&v);
        for y in 0..600u32 {
            for x in 0..600u32 {
                if pm.pixel(x, y).unwrap().alpha() > 0 {
                    assert!((x0..=x1).contains(&(x as f32)) && (y0..=y1).contains(&(y as f32)));
                }
            }
        }
        let (fx, fy) = at(180.0, 175.0);
        assert!(pm.pixel(fx as u32, fy as u32).unwrap().alpha() > 200, "flyout bentuk tergambar");
    }

    #[test]
    fn clamp_keeps_menu_on_screen() {
        let (x, y) = clamp_center((5.0, 1000.0), (1920.0, 1080.0), 1.0);
        assert_eq!(x, MARGIN);
        assert_eq!(y, 1080.0 - MARGIN);
        assert_eq!(clamp_center((960.0, 540.0), (1920.0, 1080.0), 1.0), (960.0, 540.0));
    }

    #[test]
    fn color_names() {
        assert_eq!(color_name((255, 32, 32)), "RED");
        assert_eq!(color_name((255, 212, 0)), "YELLOW");
        assert_eq!(color_name((32, 192, 64)), "GREEN");
        assert_eq!(color_name((32, 128, 255)), "BLUE");
        assert_eq!(color_name((255, 255, 255)), "WHITE");
    }

    #[test]
    fn draws_within_bounds() {
        let v = view(Some(Hit::Item(Item::Color)));
        let mut pm = Pixmap::new(600, 600).unwrap();
        draw(&mut pm, &v);
        let (x0, y0, x1, y1) = bounds(&v);
        for y in 0..600u32 {
            for x in 0..600u32 {
                if pm.pixel(x, y).unwrap().alpha() > 0 {
                    assert!((x0..=x1).contains(&(x as f32)) && (y0..=y1).contains(&(y as f32)), "({x},{y}) di luar bounds");
                }
            }
        }
        assert!(pm.pixel(300, 300).unwrap().alpha() > 200, "tombol tengah opak");
    }
}

#[cfg(test)]
mod preview {
    use super::*;
    use crate::render::ink::InkSettings;

    /// Simpan pratinjau menu ke PNG untuk dicek mata.
    /// Jalankan: POINTER_MENU_PNG=/tmp/menu.png cargo test -- --ignored menu_preview
    #[test]
    #[ignore]
    fn menu_preview() {
        let Some(out) = std::env::var_os("POINTER_MENU_PNG") else { return };
        let palette = [(255, 32, 32), (255, 212, 0), (32, 192, 64), (32, 128, 255), (255, 255, 255)];
        let mut pm = Pixmap::new(1000, 520).unwrap();
        // Latar mirip dokumen terang agar kontras menu terlihat.
        pm.fill(Color::from_rgba8(236, 240, 238, 255));
        let ink = InkSettings { color: 0, ..InkSettings::default() };
        for (cx, hover, mode) in [
            (250.0, Some(Hit::Item(Item::Draw)), Mode::Laser),
            (750.0, Some(Hit::ShapeOption(Tool::Ellipse)), Mode::Draw),
        ] {
            let v = MenuView { center: (cx, 260.0), scale: 1.0, hover, mode, ink, palette: &palette };
            draw(&mut pm, &v);
        }
        pm.save_png(out).unwrap();
    }
}
