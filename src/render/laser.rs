//! Titik laser bercahaya + trail yang memudar berdasarkan waktu.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tiny_skia::{
    GradientStop, LineCap, Paint, PathBuilder, Pixmap, Point, RadialGradient, Shader, SpreadMode,
    Stroke, Transform,
};

use super::{fill_circle, rgba};

/// Batas titik trail agar mouse ber-polling tinggi (1000 Hz) tidak membengkakkan buffer.
const MAX_POINTS: usize = 512;

#[derive(Debug, Clone, Copy)]
pub struct LaserStyle {
    pub color: (u8, u8, u8),
    /// Jari-jari dalam piksel fisik (sudah dikali skala monitor).
    pub radius: f32,
    pub trail: Duration,
}

#[derive(Debug, Default)]
pub struct Laser {
    trail: VecDeque<(f32, f32, Instant)>,
    cursor: Option<(f32, f32)>,
}

impl Laser {
    pub fn move_to(&mut self, x: f32, y: f32, now: Instant) {
        if self.cursor == Some((x, y)) {
            return;
        }
        self.cursor = Some((x, y));
        self.trail.push_back((x, y, now));
        if self.trail.len() > MAX_POINTS {
            self.trail.pop_front();
        }
    }

    /// Kursor keluar dari overlay ini (pindah monitor atau mode dimatikan).
    pub fn leave(&mut self) {
        self.cursor = None;
    }

    /// Buang titik trail yang sudah lebih tua dari `ttl`.
    pub fn prune(&mut self, now: Instant, ttl: Duration) {
        while let Some(&(_, _, t)) = self.trail.front() {
            if now.saturating_duration_since(t) >= ttl {
                self.trail.pop_front();
            } else {
                break;
            }
        }
    }

    /// Masih ada trail yang sedang memudar (perlu frame berikutnya).
    pub fn is_animating(&self) -> bool {
        // Satu titik tersisa di posisi kursor tidak menghasilkan segmen yang terlihat.
        self.trail.len() > 1
    }

    /// Kotak pembatas semua yang akan digambar `draw`, dalam piksel (x0, y0, x1, y1).
    pub fn bounds(&self, style: &LaserStyle) -> Option<(f32, f32, f32, f32)> {
        // Glow kepala (3.5r) lebih besar dari stroke trail terlebar (1.6r / 2).
        let pad = style.radius * 3.5 + 2.0;
        let pts = self
            .trail
            .iter()
            .map(|&(x, y, _)| (x, y))
            .chain(self.cursor);
        pts.fold(None, |acc, (x, y)| {
            let (x0, y0, x1, y1) = acc.unwrap_or((x, y, x, y));
            Some((x0.min(x), y0.min(y), x1.max(x), y1.max(y)))
        })
        .map(|(x0, y0, x1, y1)| (x0 - pad, y0 - pad, x1 + pad, y1 + pad))
    }

    pub fn draw(&self, pm: &mut Pixmap, now: Instant, style: &LaserStyle) {
        let ttl = style.trail.as_secs_f32();
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        for (a, b) in self.trail.iter().zip(self.trail.iter().skip(1)) {
            let age = now.saturating_duration_since(b.2).as_secs_f32() / ttl;
            let k = (1.0 - age).clamp(0.0, 1.0);
            if k <= 0.0 {
                continue;
            }
            let mut pb = PathBuilder::new();
            pb.move_to(a.0, a.1);
            pb.line_to(b.0, b.1);
            let Some(path) = pb.finish() else { continue };
            paint.set_color(rgba(style.color, 0.75 * k));
            let stroke = Stroke {
                width: style.radius * (0.4 + 1.2 * k),
                line_cap: LineCap::Round,
                ..Stroke::default()
            };
            pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        if let Some((x, y)) = self.cursor {
            let r = style.radius;
            let glow = RadialGradient::new(
                Point::from_xy(x, y),
                Point::from_xy(x, y),
                r * 3.5,
                vec![
                    GradientStop::new(0.0, rgba(style.color, 0.55)),
                    GradientStop::new(0.4, rgba(style.color, 0.2)),
                    GradientStop::new(1.0, rgba(style.color, 0.0)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            );
            if let Some(glow) = glow {
                fill_circle(pm, x, y, r * 3.5, glow);
            }
            fill_circle(pm, x, y, r, Shader::SolidColor(rgba(style.color, 1.0)));
            // Inti terang agar terlihat di latar berwarna sama dengan laser.
            let core = lighten(style.color, 0.75);
            fill_circle(pm, x, y, r * 0.45, Shader::SolidColor(rgba(core, 1.0)));
        }
    }
}

fn lighten((r, g, b): (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let f = |c: u8| (c as f32 + (255.0 - c as f32) * t).round() as u8;
    (f(r), f(g), f(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TTL: Duration = Duration::from_millis(400);

    #[test]
    fn trail_fades_out_after_ttl() {
        let t0 = Instant::now();
        let mut l = Laser::default();
        l.move_to(0.0, 0.0, t0);
        l.move_to(10.0, 0.0, t0 + Duration::from_millis(100));
        assert!(l.is_animating());

        l.prune(t0 + Duration::from_millis(450), TTL);
        assert!(!l.is_animating(), "titik pertama sudah kedaluwarsa");
        l.prune(t0 + Duration::from_millis(600), TTL);
        assert!(!l.is_animating());
        assert!(l.cursor.is_some(), "kepala laser tetap tampil meski trail habis");
    }

    #[test]
    fn duplicate_positions_ignored_and_buffer_capped() {
        let t0 = Instant::now();
        let mut l = Laser::default();
        l.move_to(1.0, 1.0, t0);
        l.move_to(1.0, 1.0, t0);
        assert_eq!(l.trail.len(), 1);
        for i in 0..(MAX_POINTS * 2) {
            l.move_to(i as f32, 0.0, t0);
        }
        assert_eq!(l.trail.len(), MAX_POINTS);
    }

    #[test]
    fn bounds_cover_trail_and_glow() {
        let t0 = Instant::now();
        let mut l = Laser::default();
        let style = LaserStyle {
            color: (255, 0, 0),
            radius: 4.0,
            trail: TTL,
        };
        assert_eq!(l.bounds(&style), None);
        l.move_to(10.0, 50.0, t0);
        l.move_to(100.0, 20.0, t0);
        let pad = 4.0 * 3.5 + 2.0;
        assert_eq!(l.bounds(&style), Some((10.0 - pad, 20.0 - pad, 100.0 + pad, 50.0 + pad)));
    }

    #[test]
    fn draw_paints_pixels_at_cursor() {
        let t0 = Instant::now();
        let mut l = Laser::default();
        l.move_to(20.0, 20.0, t0);
        let mut pm = Pixmap::new(40, 40).unwrap();
        let style = LaserStyle {
            color: (255, 0, 0),
            radius: 6.0,
            trail: TTL,
        };
        l.draw(&mut pm, t0, &style);
        assert_eq!(pm.pixel(20, 20).unwrap().alpha(), 255);
        assert_eq!(pm.pixel(0, 0).unwrap().alpha(), 0);
    }
}
