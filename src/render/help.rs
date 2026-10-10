//! Panel bantuan shortcut: kartu gelap di tengah layar, gaya sama dengan menu radial.

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

use super::{Bounds, rgba, text};
use crate::shortcuts::Section;

/// Lebar minimum panel; melebar bila ada keterangan yang lebih panjang.
const MIN_WIDTH: f32 = 620.0;
const PAD: f32 = 28.0;
const TITLE_H: f32 = 44.0;
const SECTION_H: f32 = 34.0;
const ROW_H: f32 = 25.0;
const KEY_COL: f32 = 210.0;
/// Baris dukungan di bawah panel (bisa diklik).
const FOOTER_H: f32 = 44.0;
const FOOTER_SIZE: f32 = 13.0;
pub const FOOTER_TEXT: &str = "Enjoying Pointer? Support it: buymeacoffee.com/fmarzuki";
const ACCENT: (u8, u8, u8) = (47, 212, 122);

/// Lebar panel (piksel logis): cukup untuk keterangan terpanjang.
fn width(sections: &[Section]) -> f32 {
    let longest = sections
        .iter()
        .flat_map(|s| s.rows.iter().map(|(_, d)| text::measure(d, 13.0)))
        .fold(0.0, f32::max);
    MIN_WIDTH.max(PAD * 2.0 + KEY_COL + longest + 8.0)
}

fn height(sections: &[Section]) -> f32 {
    let rows: usize = sections.iter().map(|s| s.rows.len()).sum();
    PAD * 2.0 + TITLE_H + sections.len() as f32 * SECTION_H + rows as f32 * ROW_H - 12.0 + FOOTER_H
}

/// Kotak panel bila dipusatkan di `center` (piksel fisik).
pub fn bounds(center: (f32, f32), scale: f32, sections: &[Section]) -> Bounds {
    let (w, h) = (width(sections) * scale / 2.0 + 4.0, height(sections) * scale / 2.0 + 4.0);
    (center.0 - w, center.1 - h, center.0 + w, center.1 + h)
}

/// Garis tengah vertikal baris dukungan.
fn footer_y(center: (f32, f32), s: f32, sections: &[Section]) -> f32 {
    center.1 + height(sections) * s / 2.0 - (PAD * 0.5 + FOOTER_H * 0.5) * s
}

/// Area teks dukungan yang bisa diklik.
pub fn link_bounds(center: (f32, f32), s: f32, sections: &[Section]) -> Bounds {
    let y = footer_y(center, s, sections);
    let half = text::measure(FOOTER_TEXT, FOOTER_SIZE * s) / 2.0 + 8.0 * s;
    (center.0 - half, y - 14.0 * s, center.0 + half, y + 14.0 * s)
}

pub fn draw(pm: &mut Pixmap, center: (f32, f32), s: f32, sections: &[Section]) {
    let (w, h) = (width(sections) * s, height(sections) * s);
    let (x0, y0) = (center.0 - w / 2.0, center.1 - h / 2.0);
    if let Some(card) = rounded_rect(x0, y0, w, h, 18.0 * s) {
        let mut paint = Paint { anti_alias: true, ..Paint::default() };
        paint.set_color(Color::from_rgba8(16, 24, 22, 240));
        pm.fill_path(&card, &paint, FillRule::Winding, Transform::identity(), None);
        paint.set_color(rgba(ACCENT, 0.85));
        let stroke = Stroke { width: 2.0 * s, ..Stroke::default() };
        pm.stroke_path(&card, &paint, &stroke, Transform::identity(), None);
    }

    let white = Color::from_rgba8(235, 240, 238, 255);
    let muted = Color::from_rgba8(170, 184, 178, 255);
    let left = x0 + PAD * s;
    let mut y = y0 + PAD * s + 10.0 * s;
    draw_left(pm, "Shortcuts", left, y, 22.0 * s, white);
    text::draw_centered(pm, "click anywhere to close", x0 + w - PAD * s - text::measure("click anywhere to close", 11.0 * s) / 2.0, y, 11.0 * s, muted);
    y += TITLE_H * s;
    for section in sections {
        draw_left(pm, section.title, left, y, 13.0 * s, rgba(ACCENT, 1.0));
        y += SECTION_H * s * 0.75;
        for (key, desc) in &section.rows {
            draw_left(pm, key, left + 8.0 * s, y, 13.0 * s, white);
            draw_left(pm, desc, left + KEY_COL * s, y, 13.0 * s, muted);
            y += ROW_H * s;
        }
        y += SECTION_H * s * 0.25;
    }

    // Baris dukungan: garis pemisah lalu teks aksen yang bisa diklik.
    let fy = footer_y(center, s, sections);
    let mut pb = PathBuilder::new();
    pb.move_to(left, fy - FOOTER_H * s * 0.5);
    pb.line_to(x0 + w - PAD * s, fy - FOOTER_H * s * 0.5);
    if let Some(p) = pb.finish() {
        let mut paint = Paint { anti_alias: true, ..Paint::default() };
        paint.set_color(Color::from_rgba8(255, 255, 255, 30));
        pm.stroke_path(&p, &paint, &Stroke { width: s, ..Stroke::default() }, Transform::identity(), None);
    }
    text::draw_centered(pm, FOOTER_TEXT, center.0, fy, FOOTER_SIZE * s, rgba(ACCENT, 1.0));
}

/// Teks rata kiri dengan tengah tinggi huruf di `y`.
fn draw_left(pm: &mut Pixmap, t: &str, x: f32, y: f32, size: f32, color: Color) {
    text::draw_centered(pm, t, x + text::measure(t, size) / 2.0, y, size, color);
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let rect = Rect::from_xywh(x, y, w, h)?;
    let mut pb = PathBuilder::new();
    let (x1, y1) = (rect.right(), rect.bottom());
    pb.move_to(x + r, y);
    pb.line_to(x1 - r, y);
    pb.quad_to(x1, y, x1, y + r);
    pb.line_to(x1, y1 - r);
    pb.quad_to(x1, y1, x1 - r, y1);
    pb.line_to(x + r, y1);
    pb.quad_to(x, y1, x, y1 - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HotkeyConfig;
    use crate::shortcuts::sections;

    #[test]
    fn draws_inside_bounds() {
        let secs = sections(&HotkeyConfig::default());
        let mut pm = Pixmap::new(900, 800).unwrap();
        let c = (450.0, 400.0);
        draw(&mut pm, c, 1.0, &secs);
        let (x0, y0, x1, y1) = bounds(c, 1.0, &secs);
        for y in 0..800u32 {
            for x in 0..900u32 {
                if pm.pixel(x, y).unwrap().alpha() > 0 {
                    assert!((x0..=x1).contains(&(x as f32)) && (y0..=y1).contains(&(y as f32)), "({x},{y})");
                }
            }
        }
        assert!(pm.pixel(450, 400).unwrap().alpha() > 200);
        let (lx0, ly0, lx1, ly1) = link_bounds(c, 1.0, &secs);
        assert!(lx0 > x0 && lx1 < x1 && ly0 > y0 && ly1 < y1, "link di dalam panel");
        let link_pixels = (ly0 as u32..ly1 as u32)
            .flat_map(|y| (lx0 as u32..lx1 as u32).map(move |x| (x, y)))
            .filter(|&(x, y)| pm.pixel(x, y).unwrap().green() > 150)
            .count();
        assert!(link_pixels > 50, "teks link tergambar");
    }

    /// Pratinjau: POINTER_HELP_PNG=/tmp/help.png cargo test -- --ignored help_preview
    #[test]
    #[ignore]
    fn help_preview() {
        let Some(out) = std::env::var_os("POINTER_HELP_PNG") else { return };
        let secs = sections(&HotkeyConfig::default());
        let mut pm = Pixmap::new(800, 700).unwrap();
        pm.fill(Color::from_rgba8(236, 240, 238, 255));
        draw(&mut pm, (400.0, 350.0), 1.0, &secs);
        pm.save_png(out).unwrap();
    }
}
