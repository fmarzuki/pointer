//! Highlight klik: cincin yang membesar lalu memudar di posisi klik.

use std::time::{Duration, Instant};

use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke, Transform};

use super::rgba;

#[derive(Debug, Clone, Copy)]
pub struct RippleStyle {
    pub color: (u8, u8, u8),
    /// Jari-jari akhir cincin dalam piksel fisik.
    pub radius: f32,
    pub duration: Duration,
}

impl RippleStyle {
    fn stroke_width(&self) -> f32 {
        (self.radius / 12.0).max(1.5)
    }
}

#[derive(Debug, Default)]
pub struct Ripples(Vec<(f32, f32, Instant)>);

impl Ripples {
    pub fn push(&mut self, x: f32, y: f32, now: Instant) {
        self.0.push((x, y, now));
    }

    pub fn prune(&mut self, now: Instant, style: &RippleStyle) {
        self.0
            .retain(|&(_, _, t)| now.saturating_duration_since(t) < style.duration);
    }

    pub fn is_animating(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn bounds(&self, style: &RippleStyle) -> Option<(f32, f32, f32, f32)> {
        let pad = style.radius + style.stroke_width() + 2.0;
        self.0.iter().fold(None, |acc, &(x, y, _)| {
            let (x0, y0, x1, y1) = acc.unwrap_or((x - pad, y - pad, x + pad, y + pad));
            Some((x0.min(x - pad), y0.min(y - pad), x1.max(x + pad), y1.max(y + pad)))
        })
    }

    pub fn draw(&self, pm: &mut Pixmap, now: Instant, style: &RippleStyle) {
        let stroke = Stroke {
            width: style.stroke_width(),
            ..Stroke::default()
        };
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        for &(x, y, t) in &self.0 {
            let p = (now.saturating_duration_since(t).as_secs_f32() / style.duration.as_secs_f32())
                .clamp(0.0, 1.0);
            // Ease-out: cepat membesar di awal, melambat di akhir.
            let grow = 1.0 - (1.0 - p) * (1.0 - p);
            let r = style.radius * (0.25 + 0.75 * grow);
            let Some(path) = PathBuilder::from_circle(x, y, r) else { continue };
            paint.set_color(rgba(style.color, 1.0 - p));
            pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLE: RippleStyle = RippleStyle {
        color: (255, 200, 0),
        radius: 24.0,
        duration: Duration::from_millis(500),
    };

    #[test]
    fn ripple_expires() {
        let t0 = Instant::now();
        let mut r = Ripples::default();
        r.push(10.0, 10.0, t0);
        r.prune(t0 + Duration::from_millis(499), &STYLE);
        assert!(r.is_animating());
        r.prune(t0 + Duration::from_millis(500), &STYLE);
        assert!(!r.is_animating());
        assert_eq!(r.bounds(&STYLE), None);
    }

    #[test]
    fn bounds_cover_full_radius() {
        let mut r = Ripples::default();
        r.push(100.0, 100.0, Instant::now());
        let (x0, y0, x1, y1) = r.bounds(&STYLE).unwrap();
        assert!(x0 <= 100.0 - 24.0 && y0 <= 100.0 - 24.0);
        assert!(x1 >= 100.0 + 24.0 && y1 >= 100.0 + 24.0);
    }
}
