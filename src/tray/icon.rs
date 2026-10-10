//! Ikon tray digambar saat runtime: titik laser merah, menyala saat ada mode aktif.

use tiny_skia::{Color, GradientStop, Pixmap, Point, RadialGradient, Shader, SpreadMode, Transform};

use crate::render::fill_circle;

/// RGBA non-premultiplied, `size` x `size`.
pub fn rgba(size: u32, active: bool) -> Vec<u8> {
    let mut pm = Pixmap::new(size, size).expect("ukuran ikon > 0");
    let s = size as f32;
    let c = s / 2.0;
    if active {
        let glow = RadialGradient::new(
            Point::from_xy(c, c),
            Point::from_xy(c, c),
            c,
            vec![
                GradientStop::new(0.0, Color::from_rgba8(255, 40, 40, 200)),
                GradientStop::new(1.0, Color::from_rgba8(255, 40, 40, 0)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        );
        if let Some(glow) = glow {
            fill_circle(&mut pm, c, c, c, glow);
        }
        fill_circle(&mut pm, c, c, s * 0.26, Shader::SolidColor(Color::from_rgba8(255, 40, 40, 255)));
        fill_circle(&mut pm, c, c, s * 0.12, Shader::SolidColor(Color::from_rgba8(255, 225, 225, 255)));
    } else {
        // Idle: titik redup tanpa glow, tetap terbaca di panel gelap maupun terang.
        fill_circle(&mut pm, c, c, s * 0.3, Shader::SolidColor(Color::from_rgba8(200, 60, 60, 255)));
        fill_circle(&mut pm, c, c, s * 0.14, Shader::SolidColor(Color::from_rgba8(110, 30, 30, 255)));
    }
    pm.pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_has_opaque_center_and_clear_corner() {
        for active in [false, true] {
            let px = rgba(32, active);
            assert_eq!(px.len(), 32 * 32 * 4);
            let at = |x: usize, y: usize| &px[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4];
            assert_eq!(at(16, 16)[3], 255);
            assert_eq!(at(0, 0)[3], 0);
        }
    }
}
