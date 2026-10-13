//! Mode Cursor: kotak bersudut bulat dengan pendar neon di sekeliling kursor. Bagian
//! tengahnya bening agar kursor sistem tetap terlihat.

use tiny_skia::{LineJoin, Paint, Pixmap, Stroke, Transform};

use super::ink::rounded_rect_path;
use super::{Bounds, rgba};

/// Jari-jari sudut, proporsi dari sisi kotak.
const CORNER: f32 = 0.3;
/// Lapisan pendar dari luar ke dalam: (lebar garis, opasitas), proporsi dari `width`.
/// tiny-skia tidak punya blur, jadi pendar dibuat dari garis lebar yang makin tipis.
const GLOW: [(f32, f32); 4] = [(7.0, 0.07), (5.0, 0.1), (3.2, 0.18), (2.0, 0.3)];

#[derive(Debug, Clone, Copy)]
pub struct HaloStyle {
    pub color: (u8, u8, u8),
    /// Sisi kotak dalam piksel fisik.
    pub size: f32,
    /// Tebal garis inti dalam piksel fisik.
    pub width: f32,
}

/// Kotak pembatas kotak beserta pendarnya.
pub fn bounds(cursor: Option<(f32, f32)>, style: &HaloStyle) -> Option<Bounds> {
    let (x, y) = cursor?;
    let r = style.size / 2.0 + style.width * GLOW[0].0 / 2.0 + 2.0;
    Some((x - r, y - r, x + r, y + r))
}

pub fn draw(pm: &mut Pixmap, cursor: Option<(f32, f32)>, style: &HaloStyle) {
    let Some((x, y)) = cursor else { return };
    let h = style.size / 2.0;
    let Some(path) = rounded_rect_path((x - h, y - h), (x + h, y + h), style.size * CORNER) else { return };
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    let layers = GLOW.iter().map(|&(w, a)| (style.width * w, rgba(style.color, a)));
    // Garis inti tegas, lalu garis tipis lebih terang di tengahnya.
    let core = [
        (style.width, rgba(style.color, 1.0)),
        (style.width * 0.35, rgba(lighten(style.color), 0.9)),
    ];
    for (width, color) in layers.chain(core) {
        paint.set_color(color);
        let stroke = Stroke { width, line_join: LineJoin::Round, ..Stroke::default() };
        pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn lighten((r, g, b): (u8, u8, u8)) -> (u8, u8, u8) {
    let f = |c: u8| (c as f32 + (255.0 - c as f32) * 0.6).round() as u8;
    (f(r), f(g), f(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLE: HaloStyle = HaloStyle { color: (47, 212, 122), size: 60.0, width: 3.0 };

    #[test]
    fn ring_is_hollow_and_within_bounds() {
        let mut pm = Pixmap::new(200, 200).unwrap();
        draw(&mut pm, Some((100.0, 100.0)), &STYLE);
        assert_eq!(pm.pixel(100, 100).unwrap().alpha(), 0, "tengah bening");
        assert!(pm.pixel(130, 100).unwrap().alpha() > 200, "garis inti di tepi kanan");
        let (x0, y0, x1, y1) = bounds(Some((100.0, 100.0)), &STYLE).unwrap();
        for y in 0..200u32 {
            for x in 0..200u32 {
                if pm.pixel(x, y).unwrap().alpha() > 0 {
                    assert!((x0..=x1).contains(&(x as f32)) && (y0..=y1).contains(&(y as f32)), "({x},{y}) di luar bounds");
                }
            }
        }
    }

    #[test]
    fn no_cursor_draws_nothing() {
        let mut pm = Pixmap::new(10, 10).unwrap();
        draw(&mut pm, None, &STYLE);
        assert!(pm.pixels().iter().all(|p| p.alpha() == 0));
        assert_eq!(bounds(None, &STYLE), None);
    }

    /// Simpan pratinjau ke PNG untuk dicek mata.
    /// Jalankan: POINTER_HALO_PNG=/tmp/halo.png cargo test -- --ignored halo_preview
    #[test]
    #[ignore]
    fn halo_preview() {
        let Some(out) = std::env::var_os("POINTER_HALO_PNG") else { return };
        let mut pm = Pixmap::new(320, 160).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(24, 26, 28, 255));
        let style = HaloStyle { size: 72.0, width: 3.0, ..STYLE };
        draw(&mut pm, Some((80.0, 80.0)), &style);
        let mut light = Pixmap::new(160, 160).unwrap();
        light.fill(tiny_skia::Color::from_rgba8(236, 240, 238, 255));
        draw(&mut light, Some((80.0, 80.0)), &style);
        pm.draw_pixmap(160, 0, light.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        pm.save_png(out).unwrap();
    }
}
