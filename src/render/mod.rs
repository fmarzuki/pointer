//! Rendering CPU dengan tiny-skia. Hanya digambar saat ada perubahan atau animasi.

pub mod halo;
pub mod help;
pub mod ink;
pub mod laser;
pub mod menu;
pub mod ripple;
pub mod spotlight;
pub mod text;

use halo::HaloStyle;
use ink::InkStyle;
use laser::LaserStyle;
use ripple::RippleStyle;
use spotlight::SpotlightStyle;

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Shader, Transform};

pub fn fill_circle(pm: &mut Pixmap, x: f32, y: f32, r: f32, shader: Shader<'_>) {
    if let Some(path) = PathBuilder::from_circle(x, y, r) {
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        };
        pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
}

pub fn rgba(rgb: (u8, u8, u8), a: f32) -> Color {
    Color::from_rgba8(rgb.0, rgb.1, rgb.2, (a.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Semua gaya efek, dalam piksel logis (dari config) atau fisik (setelah `scaled`).
#[derive(Debug, Clone)]
pub struct Styles {
    pub laser: LaserStyle,
    pub spotlight: SpotlightStyle,
    pub halo: HaloStyle,
    /// `None` bila highlight klik dimatikan di config.
    pub ripple: Option<RippleStyle>,
    pub ink: InkStyle,
    /// Laser memakai warna tinta aktif, bukan `laser.color`.
    pub laser_follows_ink: bool,
    /// Kotak mode Cursor memakai warna tinta aktif, bukan `halo.color`.
    pub halo_follows_ink: bool,
}

impl Styles {
    /// Ubah ukuran logis ke piksel fisik untuk monitor dengan skala `s`.
    pub fn scaled(&self, s: f32) -> Self {
        Self {
            laser: LaserStyle { radius: self.laser.radius * s, ..self.laser },
            spotlight: SpotlightStyle { radius: self.spotlight.radius * s, ..self.spotlight },
            halo: HaloStyle { size: self.halo.size * s, width: self.halo.width * s, ..self.halo },
            ripple: self.ripple.map(|r| RippleStyle { radius: r.radius * s, ..r }),
            ink: InkStyle {
                pen_width: self.ink.pen_width * s,
                highlighter_width: self.ink.highlighter_width * s,
                rect_radius: self.ink.rect_radius * s,
                text_size: self.ink.text_size * s,
                ..self.ink.clone()
            },
            laser_follows_ink: self.laser_follows_ink,
            halo_follows_ink: self.halo_follows_ink,
        }
    }
}

pub type Bounds = (f32, f32, f32, f32);

pub fn union_bounds(a: Option<Bounds>, b: Option<Bounds>) -> Option<Bounds> {
    match (a, b) {
        (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))),
        (a, b) => a.or(b),
    }
}
