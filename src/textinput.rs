//! Kotak input teks untuk alat Text.
//!
//! Overlay sengaja tidak pernah menerima keyboard (agar PageDown/clicker tetap menggerakkan
//! slide), jadi untuk mengetik dibuat window kecil biasa yang menerima fokus. Teks yang diketik
//! ditampilkan langsung sebagai pratinjau di overlay; kotak ini hanya menampilkan petunjuk.
//! Setelah Enter/Esc window ditutup dan fokus kembali ke aplikasi sebelumnya.

use std::num::NonZeroU32;
use std::rc::Rc;

use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Ime, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId, WindowLevel};

use crate::render::{rgba, text};

const WIDTH: f32 = 470.0;
const HEIGHT: f32 = 44.0;
const ACCENT: (u8, u8, u8) = (47, 212, 122);

pub enum InputResult {
    Continue,
    /// Teks berubah: perbarui pratinjau di overlay.
    Changed,
    Commit,
    Cancel,
}

pub struct TextInput {
    pub window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    size: PhysicalSize<u32>,
    scale: f32,
    /// Overlay tempat teks akan ditaruh, dan posisi (koordinat lokal overlay).
    pub overlay: WindowId,
    pub at: (f32, f32),
    pub text: String,
    focused: bool,
    shift: bool,
}

impl TextInput {
    /// Buka kotak input tepat di bawah titik `at` pada overlay `overlay` (asal layar `origin`).
    pub fn open(
        el: &ActiveEventLoop,
        overlay: WindowId,
        origin: PhysicalPosition<i32>,
        at: (f32, f32),
        text_size: f32,
        scale: f32,
    ) -> Result<Self, String> {
        let size = PhysicalSize::new((WIDTH * scale) as u32, (HEIGHT * scale) as u32);
        // Di bawah baris pertama teks, sedikit ke kiri agar sejajar dengan kotak seleksi.
        let pos = PhysicalPosition::new(
            origin.x + at.0 as i32 - (8.0 * scale) as i32,
            origin.y + (at.1 + text_size * 1.25 + 12.0 * scale) as i32,
        );
        let attrs = Window::default_attributes()
            .with_title("Pointer text")
            .with_decorations(false)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_inner_size(size)
            .with_position(pos)
            .with_active(true);
        #[cfg(target_os = "linux")]
        let attrs = {
            use winit::platform::x11::{WindowAttributesExtX11, WindowType};
            attrs.with_x11_window_type(vec![WindowType::Dialog])
        };
        #[cfg(windows)]
        let attrs = {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs.with_skip_taskbar(true)
        };
        let window = Rc::new(el.create_window(attrs).map_err(|e| e.to_string())?);
        window.set_ime_allowed(true);
        window.focus_window();
        let ctx = softbuffer::Context::new(window.clone()).map_err(|e| e.to_string())?;
        let mut surface = softbuffer::Surface::new(&ctx, window.clone()).map_err(|e| e.to_string())?;
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return Err("text input has zero size".into());
        };
        surface.resize(w, h).map_err(|e| e.to_string())?;
        let mut input = Self {
            window,
            surface,
            size,
            scale,
            overlay,
            at,
            text: String::new(),
            focused: false,
            shift: false,
        };
        input.render();
        Ok(input)
    }

    pub fn handle(&mut self, event: &WindowEvent) -> InputResult {
        let result = match event {
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                InputResult::Continue
            }
            WindowEvent::Focused(f) => {
                self.focused = *f;
                InputResult::Continue
            }
            WindowEvent::CloseRequested => InputResult::Cancel,
            // Klik di kotak (mis. bila fokus tidak otomatis didapat) meminta fokus lagi.
            WindowEvent::MouseInput { state: ElementState::Pressed, .. } => {
                self.window.focus_window();
                InputResult::Continue
            }
            WindowEvent::Ime(Ime::Commit(s)) => {
                self.push(s);
                InputResult::Changed
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match &event.logical_key {
                    Key::Named(NamedKey::Enter) if self.shift => {
                        self.text.push('\n');
                        InputResult::Changed
                    }
                    Key::Named(NamedKey::Enter) => InputResult::Commit,
                    Key::Named(NamedKey::Escape) => InputResult::Cancel,
                    Key::Named(NamedKey::Backspace) => {
                        self.text.pop();
                        InputResult::Changed
                    }
                    // Tombol tanpa teks (F11, panah, ...) diabaikan.
                    _ => match &event.text {
                        Some(t) if self.push(t) => InputResult::Changed,
                        _ => InputResult::Continue,
                    },
                }
            }
            WindowEvent::RedrawRequested => InputResult::Continue,
            _ => return InputResult::Continue,
        };
        self.render();
        result
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Tambah karakter yang bisa dicetak; `true` bila ada yang ditambahkan.
    fn push(&mut self, s: &str) -> bool {
        let before = self.text.len();
        self.text.extend(s.chars().filter(|c| !c.is_control()));
        self.text.len() != before
    }

    fn render(&mut self) {
        let s = self.scale;
        let (w, h) = (self.size.width as f32, self.size.height as f32);
        let Some(mut pm) = Pixmap::new(self.size.width, self.size.height) else { return };
        pm.fill(Color::from_rgba8(16, 24, 22, 255));
        if let Some(r) = Rect::from_xywh(1.0, 1.0, w - 2.0, h - 2.0) {
            let mut paint = Paint { anti_alias: true, ..Paint::default() };
            paint.set_color(rgba(ACCENT, if self.focused { 1.0 } else { 0.45 }));
            let path = PathBuilder::from_rect(r);
            pm.stroke_path(&path, &paint, &Stroke { width: 2.0 * s, ..Stroke::default() }, Transform::identity(), None);
        }
        let muted = Color::from_rgba8(170, 184, 178, 255);
        let hint = if !self.focused {
            "Click here to type"
        } else if self.text.is_empty() {
            "Type text  ·  Enter = add  ·  Shift+Enter = new line  ·  Esc = cancel"
        } else {
            "Enter = add  ·  Shift+Enter = new line  ·  Esc = cancel"
        };
        let size = 12.5 * s;
        text::draw_centered(&mut pm, hint, w / 2.0, h / 2.0, size, if self.focused { muted } else { rgba(ACCENT, 1.0) });

        let Ok(mut buf) = self.surface.buffer_mut() else { return };
        for (dst, px) in buf.iter_mut().zip(pm.pixels()) {
            *dst = 0xff00_0000 | (px.red() as u32) << 16 | (px.green() as u32) << 8 | px.blue() as u32;
        }
        let _ = buf.present();
    }
}
