//! Overlay X11/XWayland: override-redirect (lolos uji spike: tampil di atas slideshow
//! fullscreen GNOME 50 dan tidak mencuri fokus keyboard), present via softbuffer.

use std::num::NonZeroU32;
use std::rc::Rc;

use tiny_skia::Pixmap;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::platform::x11::{WindowAttributesExtX11, WindowType};
use winit::window::{Window, WindowAttributes};

use super::{Damage, copy_damage};

pub fn window_attributes(attrs: WindowAttributes) -> WindowAttributes {
    attrs
        .with_transparent(true)
        .with_override_redirect(true)
        .with_x11_window_type(vec![WindowType::Notification])
}

/// Latar layer coretan. X11 menangkap input di seluruh window, jadi cukup transparan.
pub fn draw_backdrop() -> tiny_skia::Color {
    tiny_skia::Color::TRANSPARENT
}

pub struct Presenter {
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
}

impl Presenter {
    pub fn new(
        window: &Rc<Window>,
        _origin: PhysicalPosition<i32>,
        size: PhysicalSize<u32>,
        // Di X11/XWayland overlay selalu menangkap pointer (posisi global tidak tersedia).
        _click_through: bool,
    ) -> Result<Self, String> {
        let ctx = softbuffer::Context::new(window.clone()).map_err(|e| e.to_string())?;
        let mut surface = softbuffer::Surface::new(&ctx, window.clone()).map_err(|e| e.to_string())?;
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return Err(format!("overlay has zero size: {size:?}"));
        };
        surface.resize(w, h).map_err(|e| e.to_string())?;
        Ok(Self { surface })
    }

    /// X11/XWayland: overlay selalu menangkap pointer, tidak ada yang perlu diubah.
    pub fn set_click_through(&mut self, _on: bool) {}

    pub fn present(&mut self, pm: &Pixmap, damage: Damage) -> Result<(), String> {
        let mut buf = self.surface.buffer_mut().map_err(|e| e.to_string())?;
        // age 0 = isi buffer tidak terdefinisi (frame pertama atau tanpa XShm): salin penuh.
        let full = buf.age() == 0;
        let d = if full {
            Damage::full(PhysicalSize::new(pm.width(), pm.height()))
        } else {
            damage
        };
        copy_damage(&mut buf, pm, d);
        if full {
            return buf.present().map_err(|e| e.to_string());
        }
        let (Some(width), Some(height)) = (NonZeroU32::new(d.w), NonZeroU32::new(d.h)) else {
            return Ok(());
        };
        buf.present_with_damage(&[softbuffer::Rect { x: d.x, y: d.y, width, height }])
            .map_err(|e| e.to_string())
    }
}
