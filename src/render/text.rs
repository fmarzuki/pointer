//! Teks sederhana untuk menu: kontur glyph dari font tertanam (Noto Sans Bold, SIL OFL 1.1,
//! lihat `assets/fonts/OFL.txt`) diubah jadi path tiny-skia lalu diisi dengan anti-alias.
//! Tanpa shaping/kerning; cukup untuk label pendek berhuruf Latin.

use std::sync::OnceLock;

use tiny_skia::{Color, FillRule, Paint, Path, PathBuilder, Pixmap, Transform};
use ttf_parser::{Face, OutlineBuilder};

static FONT_DATA: &[u8] = include_bytes!("../../assets/fonts/NotoSans-Bold.ttf");

fn face() -> &'static Face<'static> {
    static FACE: OnceLock<Face<'static>> = OnceLock::new();
    FACE.get_or_init(|| Face::parse(FONT_DATA, 0).expect("font tertanam valid"))
}

/// Lebar teks dalam piksel pada ukuran `size` (piksel per em).
pub fn measure(text: &str, size: f32) -> f32 {
    let f = face();
    let scale = size / f.units_per_em() as f32;
    text.chars()
        .filter_map(|c| f.glyph_index(c))
        .filter_map(|g| f.glyph_hor_advance(g))
        .map(|a| a as f32 * scale)
        .sum()
}

/// Gambar teks dengan titik tengah (horizontal dan tinggi huruf kapital) di `(cx, cy)`.
pub fn draw_centered(pm: &mut Pixmap, text: &str, cx: f32, cy: f32, size: f32, color: Color) {
    let f = face();
    let scale = size / f.units_per_em() as f32;
    let cap = f.capital_height().unwrap_or(f.ascender()) as f32 * scale;
    let x = cx - measure(text, size) / 2.0;
    if let Some(path) = text_path(text, x, cy + cap / 2.0, size) {
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        paint.set_color(color);
        pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
}

/// Jarak antarbaris relatif ukuran font.
const LINE_HEIGHT: f32 = 1.25;

/// Lebar dan tinggi blok teks multi-baris (`\n`) pada ukuran `size`.
pub fn block_size(text: &str, size: f32) -> (f32, f32) {
    let lines = text.split('\n');
    let width = lines.clone().map(|l| measure(l, size)).fold(0.0, f32::max);
    (width, lines.count() as f32 * size * LINE_HEIGHT)
}

/// Gambar blok teks multi-baris rata kiri dengan sudut kiri atas di `(x, y)`.
pub fn draw_block(pm: &mut Pixmap, text: &str, x: f32, y: f32, size: f32, color: Color) {
    let f = face();
    let scale = size / f.units_per_em() as f32;
    // Garis dasar baris pertama: setengah sisa tinggi baris + ascender.
    let ascent = f.ascender() as f32 * scale;
    let descent = -(f.descender() as f32) * scale;
    let lead = (size * LINE_HEIGHT - (ascent + descent)) / 2.0;
    let mut paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    paint.set_color(color);
    for (i, line) in text.split('\n').enumerate() {
        let baseline = y + i as f32 * size * LINE_HEIGHT + lead + ascent;
        if let Some(path) = text_path(line, x, baseline, size) {
            pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
}

/// Path untuk `text` dengan awal di `x` dan garis dasar di `baseline`.
fn text_path(text: &str, x: f32, baseline: f32, size: f32) -> Option<Path> {
    let f = face();
    let scale = size / f.units_per_em() as f32;
    let mut b = GlyphPath {
        pb: PathBuilder::new(),
        x,
        baseline,
        scale,
    };
    for c in text.chars() {
        let Some(g) = f.glyph_index(c) else { continue };
        f.outline_glyph(g, &mut b);
        b.x += f.glyph_hor_advance(g).unwrap_or(0) as f32 * scale;
    }
    b.pb.finish()
}

/// Menerjemahkan koordinat font (y ke atas, satuan em) ke piksel layar (y ke bawah).
struct GlyphPath {
    pb: PathBuilder,
    x: f32,
    baseline: f32,
    scale: f32,
}

impl GlyphPath {
    fn p(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x + x * self.scale, self.baseline - y * self.scale)
    }
}

impl OutlineBuilder for GlyphPath {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.p(x, y);
        self.pb.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.p(x, y);
        self.pb.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x1, y1) = self.p(x1, y1);
        let (x, y) = self.p(x, y);
        self.pb.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (x1, y1) = self.p(x1, y1);
        let (x2, y2) = self.p(x2, y2);
        let (x, y) = self.p(x, y);
        self.pb.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.pb.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_grows_with_text_and_size() {
        assert!(measure("Laser", 16.0) > measure("L", 16.0));
        assert!((measure("Draw", 32.0) - 2.0 * measure("Draw", 16.0)).abs() < 0.01);
        assert_eq!(measure("", 16.0), 0.0);
    }

    #[test]
    fn draws_glyph_pixels_near_center() {
        let mut pm = Pixmap::new(60, 30).unwrap();
        draw_centered(&mut pm, "H", 30.0, 15.0, 20.0, Color::WHITE);
        let painted: Vec<(u32, u32)> = (0..30)
            .flat_map(|y| (0..60).map(move |x| (x, y)))
            .filter(|&(x, y)| pm.pixel(x, y).unwrap().alpha() > 128)
            .collect();
        assert!(!painted.is_empty());
        // Huruf berada di sekitar pusat, tidak menempel ke tepi.
        assert!(painted.iter().all(|&(x, y)| (20..40).contains(&x) && (5..25).contains(&y)));
    }

    #[test]
    fn block_draws_lines_inside_its_size() {
        let (w, h) = block_size("Hi\nThere", 20.0);
        assert!(w > measure("Hi", 20.0) && (h - 50.0).abs() < 0.01, "2 baris x 1.25 x 20");
        let mut pm = Pixmap::new(200, 80).unwrap();
        draw_block(&mut pm, "Hi\nThere", 10.0, 10.0, 20.0, Color::WHITE);
        for y in 0..80u32 {
            for x in 0..200u32 {
                if pm.pixel(x, y).unwrap().alpha() > 0 {
                    assert!(x as f32 >= 9.0 && x as f32 <= 11.0 + w && y as f32 >= 9.0 && y as f32 <= 11.0 + h, "({x},{y})");
                }
            }
        }
        let second_line = (35..60u32).any(|y| (10..100u32).any(|x| pm.pixel(x, y).unwrap().alpha() > 128));
        assert!(second_line, "baris kedua tergambar");
    }
}
