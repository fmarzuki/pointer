//! Spotlight: layar digelapkan kecuali lingkaran lembut di sekitar kursor.

use tiny_skia::{
    BlendMode, Color, FillRule, GradientStop, Paint, PathBuilder, Pixmap, Point, RadialGradient,
    Rect, Shader, SpreadMode, Transform,
};

/// Bagian tepi lingkaran yang dibuat lembut (proporsi dari jari-jari).
const FEATHER: f32 = 0.2;

#[derive(Debug, Clone, Copy)]
pub struct SpotlightStyle {
    /// Jari-jari lingkaran terang dalam piksel fisik.
    pub radius: f32,
    /// Opasitas area gelap, 0..1.
    pub dim: f32,
}

/// Kotak pembatas lingkaran terang; area di luarnya tidak berubah saat kursor bergerak.
pub fn bounds(cursor: Option<(f32, f32)>, style: &SpotlightStyle) -> Option<(f32, f32, f32, f32)> {
    let (x, y) = cursor?;
    let r = style.radius + 2.0;
    Some((x - r, y - r, x + r, y + r))
}

/// Gambar ulang area `area`: isi gelap, lalu lubangi lingkaran di posisi kursor.
pub fn draw(pm: &mut Pixmap, area: Rect, cursor: Option<(f32, f32)>, style: &SpotlightStyle) {
    let dark = Color::from_rgba(0.0, 0.0, 0.0, style.dim.clamp(0.0, 1.0)).unwrap_or(Color::BLACK);
    // BlendMode::Source: ganti piksel, jangan menumpuk dengan isi frame sebelumnya.
    let fill = Paint {
        shader: Shader::SolidColor(dark),
        blend_mode: BlendMode::Source,
        anti_alias: false,
        ..Paint::default()
    };
    pm.fill_rect(area, &fill, Transform::identity(), None);

    let Some((x, y)) = cursor else { return };
    let r = style.radius;
    let Some(path) = PathBuilder::from_circle(x, y, r) else { return };
    let hole = RadialGradient::new(
        Point::from_xy(x, y),
        Point::from_xy(x, y),
        r,
        vec![
            GradientStop::new(0.0, Color::TRANSPARENT),
            GradientStop::new(1.0 - FEATHER, Color::TRANSPARENT),
            GradientStop::new(1.0, dark),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    );
    let Some(hole) = hole else { return };
    let paint = Paint {
        shader: hole,
        blend_mode: BlendMode::Source,
        anti_alias: true,
        ..Paint::default()
    };
    pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLE: SpotlightStyle = SpotlightStyle { radius: 20.0, dim: 0.6 };

    #[test]
    fn hole_is_clear_and_outside_is_dim() {
        let mut pm = Pixmap::new(100, 100).unwrap();
        let all = Rect::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap();
        draw(&mut pm, all, Some((50.0, 50.0)), &STYLE);
        assert_eq!(pm.pixel(50, 50).unwrap().alpha(), 0, "pusat lingkaran terang");
        assert_eq!(pm.pixel(5, 5).unwrap().alpha(), (0.6f32 * 255.0).round() as u8);
    }

    #[test]
    fn no_cursor_dims_everything() {
        let mut pm = Pixmap::new(10, 10).unwrap();
        let all = Rect::from_xywh(0.0, 0.0, 10.0, 10.0).unwrap();
        draw(&mut pm, all, None, &STYLE);
        assert!(pm.pixels().iter().all(|p| p.alpha() > 0));
        assert_eq!(bounds(None, &STYLE), None);
    }
}
