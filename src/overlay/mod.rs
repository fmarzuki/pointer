//! Satu window overlay transparan per monitor.
//!
//! Linux (X11/XWayland): window override-redirect yang menangkap pointer saat aktif
//! (di Wayland posisi kursor global tidak tersedia), tidak pernah menerima fokus keyboard.
//! Windows: layered window click-through; posisi kursor di-poll dengan `GetCursorPos`.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as imp;
#[cfg(windows)]
pub use self::windows::{cursor_pos, left_button_down};

use std::rc::Rc;
use std::time::{Duration, Instant};

use tiny_skia::{BlendMode, Paint, Pixmap, Rect, Shader, Transform};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::monitor::MonitorHandle;
use winit::event::MouseButton;
use winit::window::{CursorIcon, Window};

use crate::app::Mode;
use crate::render::laser::{Laser, LaserStyle};
use crate::render::ink::{Ink, InkSettings, Tool};
use crate::render::help;
use crate::render::menu::{self, Hit, MenuView};
use crate::shortcuts::Section;
use crate::render::ripple::Ripples;
use crate::render::{Styles, halo, spotlight, union_bounds};

/// Jarak minimum antar frame (~60 fps) agar mouse 1000 Hz tidak memicu render berlebih.
const FRAME: Duration = Duration::from_millis(16);

/// Area piksel yang berubah antar frame; hanya area ini yang dibersihkan, dikonversi,
/// dan dikirim ke compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Damage {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Damage {
    pub fn full(size: PhysicalSize<u32>) -> Self {
        Self { x: 0, y: 0, w: size.width, h: size.height }
    }

    /// Bulatkan keluar ke piksel dan potong ke ukuran layar; `None` bila di luar layar.
    fn from_bounds((x0, y0, x1, y1): (f32, f32, f32, f32), size: PhysicalSize<u32>) -> Option<Self> {
        let clamp = |v: f32, max: u32| v.clamp(0.0, max as f32) as u32;
        let (x0, y0) = (clamp(x0.floor(), size.width), clamp(y0.floor(), size.height));
        let (x1, y1) = (clamp(x1.ceil(), size.width), clamp(y1.ceil(), size.height));
        (x1 > x0 && y1 > y0).then_some(Self { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
    }

    fn union(self, o: Self) -> Self {
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        let x1 = (self.x + self.w).max(o.x + o.w);
        let y1 = (self.y + self.h).max(o.y + o.h);
        Self { x, y, w: x1 - x, h: y1 - y }
    }
}

/// Salin area `d` dari pixmap tiny-skia (RGBA premultiplied) ke buffer 0xAARRGGBB
/// premultiplied dengan lebar baris yang sama.
pub fn copy_damage(dst: &mut [u32], pm: &Pixmap, d: Damage) {
    let width = pm.width() as usize;
    let src = pm.pixels();
    for row in d.y as usize..(d.y + d.h) as usize {
        let start = row * width + d.x as usize;
        let end = start + d.w as usize;
        for (out, px) in dst[start..end].iter_mut().zip(&src[start..end]) {
            *out = (px.alpha() as u32) << 24
                | (px.red() as u32) << 16
                | (px.green() as u32) << 8
                | px.blue() as u32;
        }
    }
}

/// Hasil tombol mouse di overlay yang perlu ditangani aplikasi.
pub enum ButtonOutcome {
    Nothing,
    /// Alat/warna berubah: sebarkan ke monitor lain.
    Ink(InkSettings),
    /// Klik dengan alat Text: buka kotak input teks di titik ini (koordinat lokal).
    StartText((f32, f32)),
}

struct OverlayMenu {
    center: (f32, f32),
    hover: Option<Hit>,
}

pub struct Overlay {
    pub window: Rc<Window>,
    presenter: imp::Presenter,
    pixmap: Pixmap,
    mode: Mode,
    /// Gaya efek yang sudah diskalakan untuk monitor ini.
    styles: Styles,
    cursor: Option<(f32, f32)>,
    laser: Laser,
    ripples: Ripples,
    ink: Ink,
    ink_settings: InkSettings,
    /// Menu radial yang sedang tampil di monitor ini.
    menu: Option<OverlayMenu>,
    /// Kursor sistem disembunyikan (mode laser dengan `hide_cursor`).
    cursor_hidden: bool,
    /// Panel shortcut yang sedang tampil di monitor ini.
    help: Option<Vec<Section>>,
    /// Menu atau panel shortcut terbuka (di monitor mana pun): overlay harus menangkap klik.
    menu_capture: bool,
    scale: f32,
    /// Mode Draw: layer coretan yang sudah selesai (latar + semua coretan).
    base: Option<Pixmap>,
    /// Layer base perlu digambar ulang penuh (undo/clear).
    base_stale: bool,
    origin: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    dirty: bool,
    last_frame: Option<Instant>,
    /// Area yang digambar pada frame sebelumnya (harus dihapus di frame ini).
    prev: Option<Damage>,
    /// Frame berikutnya harus digambar penuh (frame pertama, undo, clear).
    full_redraw: bool,
    /// Statistik untuk `POINTER_DEBUG=1`: jumlah frame dan total waktu render.
    pub frames: u32,
    pub busy: Duration,
}

impl Overlay {
    pub fn new(
        el: &ActiveEventLoop,
        monitor: &MonitorHandle,
        mode: Mode,
        styles: &Styles,
        ink_settings: InkSettings,
        hide_cursor: bool,
    ) -> Result<Self, String> {
        let origin = monitor.position();
        let size = monitor.size();
        let pixmap = Pixmap::new(size.width, size.height)
            .ok_or_else(|| format!("invalid monitor size: {size:?}"))?;
        let attrs = imp::window_attributes(
            Window::default_attributes()
                .with_title("pointer")
                .with_decorations(false)
                .with_resizable(false)
                .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
                .with_position(origin)
                .with_inner_size(size),
        );
        let window = Rc::new(el.create_window(attrs).map_err(|e| e.to_string())?);
        // Mode Draw harus menerima klik/drag; mode lain click-through bila platform mendukung.
        let presenter = imp::Presenter::new(&window, origin, size, mode != Mode::Draw)?;
        if hide_cursor {
            window.set_cursor_visible(false);
        }
        let base = if mode == Mode::Draw {
            Some(pixmap.clone())
        } else {
            None
        };
        let scale = monitor.scale_factor() as f32;
        let mut ink = Ink::default();
        ink.tool = ink_settings.tool;
        ink.color = ink_settings.color;
        let overlay = Self {
            window,
            presenter,
            pixmap,
            mode,
            styles: styles.scaled(scale),
            cursor: None,
            laser: Laser::default(),
            ripples: Ripples::default(),
            ink,
            ink_settings,
            menu: None,
            help: None,
            cursor_hidden: hide_cursor,
            menu_capture: false,
            scale,
            base,
            base_stale: true,
            origin,
            size,
            dirty: true,
            last_frame: None,
            prev: None,
            full_redraw: true,
            frames: 0,
            busy: Duration::ZERO,
        };
        overlay.update_draw_cursor();
        Ok(overlay)
    }

    /// Koordinat layar -> koordinat lokal overlay, bila titik berada di monitor ini.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn to_local(&self, x: i32, y: i32) -> Option<(f32, f32)> {
        let lx = x - self.origin.x;
        let ly = y - self.origin.y;
        let inside = lx >= 0 && ly >= 0 && (lx as u32) < self.size.width && (ly as u32) < self.size.height;
        inside.then_some((lx as f32, ly as f32))
    }

    pub fn pointer_moved(&mut self, x: f32, y: f32, now: Instant) {
        if self.cursor == Some((x, y)) {
            return;
        }
        self.cursor = Some((x, y));
        match self.mode {
            Mode::Laser => self.laser.move_to(x, y, now),
            Mode::Draw if self.ink.is_dragging() => self.ink.drag_to((x, y)),
            Mode::Draw if self.ink.is_drawing() => {
                self.ink.extend(x, y);
            }
            Mode::Draw if !self.menu_capture => {
                let hit = self.ink.hit((x, y), &self.styles.ink);
                if self.ink.set_hover(hit) {
                    self.update_draw_cursor();
                }
            }
            _ => {}
        }
        if self.help.is_some() {
            let icon = if self.help_link_hit() { CursorIcon::Pointer } else { CursorIcon::Default };
            self.window.set_cursor(icon);
        }
        // Saat Idle (hanya menu), gambar ulang hanya bila sektor yang disorot berubah.
        self.update_menu_hover();
        if self.mode != Mode::Idle {
            self.dirty = true;
        }
    }

    pub fn cursor(&self) -> Option<(f32, f32)> {
        self.cursor
    }

    /// Terapkan alat/warna dari aplikasi (menu radial atau monitor lain).
    pub fn set_ink(&mut self, s: InkSettings) {
        self.ink_settings = s;
        self.ink.tool = s.tool;
        self.ink.color = s.color;
        self.update_draw_cursor();
        self.dirty = true;
    }

    // ---- Menu radial ----

    /// Selama menu terbuka, overlay menangkap klik (Windows: matikan click-through dan
    /// pakai latar alpha 1/255 agar klik di luar menu juga tertangkap untuk menutupnya).
    pub fn set_menu_capture(&mut self, on: bool) {
        if self.menu_capture == on {
            return;
        }
        self.menu_capture = on;
        if !on {
            self.menu = None;
        } else if self.ink.set_hover(None) {
            self.update_draw_cursor();
        }
        self.presenter.set_click_through(!on && self.mode != Mode::Draw);
        if imp::draw_backdrop().alpha() > 0.0 && self.mode != Mode::Draw && self.mode != Mode::Spotlight {
            self.full_redraw = true;
        }
        self.dirty = true;
    }

    pub fn open_menu(&mut self, at: (f32, f32)) {
        let size = (self.size.width as f32, self.size.height as f32);
        self.menu = Some(OverlayMenu {
            center: menu::clamp_center(at, size, self.scale),
            hover: None,
        });
        self.update_menu_hover();
        self.dirty = true;
    }

    pub fn show_help(&mut self, sections: Vec<Section>) {
        self.help = Some(sections);
        // Kursor perlu terlihat untuk mengklik link (mode laser menyembunyikannya).
        self.window.set_cursor_visible(true);
        self.dirty = true;
    }

    pub fn hide_help(&mut self) {
        if self.help.take().is_some() {
            self.window.set_cursor(CursorIcon::Default);
            self.update_draw_cursor();
            self.window.set_cursor_visible(!self.cursor_hidden);
            self.dirty = true;
        }
    }

    /// Kursor berada di atas link dukungan pada panel shortcut.
    pub fn help_link_hit(&self) -> bool {
        let (Some(h), Some((x, y))) = (&self.help, self.cursor) else { return false };
        let (x0, y0, x1, y1) = help::link_bounds(self.screen_center(), self.scale, h);
        (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
    }

    /// Pusat layar monitor ini (posisi menu bila kursor belum diketahui).
    pub fn screen_center(&self) -> (f32, f32) {
        (self.size.width as f32 / 2.0, self.size.height as f32 / 2.0)
    }

    pub fn has_menu(&self) -> bool {
        self.menu.is_some()
    }

    /// Bagian menu di bawah kursor; `Outside` bila menu tidak tampil di monitor ini.
    pub fn menu_hit(&self) -> Hit {
        match (self.menu_view(), self.cursor) {
            (Some(v), Some(c)) => menu::hit(&v, c),
            _ => Hit::Outside,
        }
    }

    fn update_menu_hover(&mut self) {
        let hit = self.menu_hit();
        if let Some(m) = &mut self.menu {
            let hover = (hit != Hit::Outside).then_some(hit);
            if m.hover != hover {
                m.hover = hover;
                self.dirty = true;
            }
        }
    }

    fn menu_view(&self) -> Option<MenuView<'_>> {
        let m = self.menu.as_ref()?;
        Some(MenuView {
            center: m.center,
            scale: self.scale,
            hover: m.hover,
            mode: self.mode,
            ink: self.ink_settings,
            palette: &self.styles.ink.palette,
        })
    }

    pub fn pointer_left(&mut self) {
        // Coretan yang sedang dibuat diselesaikan bila kursor keluar monitor. Geser objek
        // tetap berlanjut: X11 meneruskan event ke window ini selama tombol masih ditekan.
        self.commit_stroke();
        if self.ink.set_hover(None) {
            self.update_draw_cursor();
        }
        if self.cursor.take().is_some() {
            self.laser.leave();
            self.dirty = true;
        }
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn has_cursor(&self) -> bool {
        self.cursor.is_some()
    }

    /// Klik mouse di posisi kursor terakhir: cincin highlight (bukan di mode Draw).
    pub fn clicked(&mut self, now: Instant) {
        if self.mode == Mode::Draw {
            return;
        }
        if let (Some((x, y)), Some(_)) = (self.cursor, self.styles.ripple) {
            self.ripples.push(x, y, now);
            self.dirty = true;
        }
    }

    /// Tombol mouse di overlay. Mode Draw: kiri = coret, kanan = warna berikutnya,
    /// tengah = alat berikutnya. Di atas garis objek: tekan kiri = geser objek itu,
    /// klik kanan = hapus objek itu.
    /// Mode lain: tekan = highlight klik.
    ///
    /// Mengembalikan pengaturan tinta baru bila alat/warna berubah, untuk disebarkan ke
    /// monitor lain.
    pub fn button(&mut self, button: MouseButton, pressed: bool, now: Instant, seq: &mut u64) -> ButtonOutcome {
        if self.mode != Mode::Draw {
            if pressed {
                self.clicked(now);
            }
            return ButtonOutcome::Nothing;
        }
        let mut s = self.ink_settings;
        match (button, pressed) {
            (MouseButton::Left, true) => {
                if let Some(p) = self.cursor {
                    match self.ink.hit(p, &self.styles.ink) {
                        Some(id) if self.ink.start_drag(id, p) => {
                            // Objek keluar dari layer base selama digeser.
                            self.invalidate_base();
                            self.update_draw_cursor();
                        }
                        _ if self.ink.tool == Tool::Text => return ButtonOutcome::StartText(p),
                        _ => {
                            *seq += 1;
                            self.ink.begin(p.0, p.1, *seq, &self.styles.ink);
                        }
                    }
                }
            }
            (MouseButton::Left, false) if self.ink.is_dragging() => {
                *seq += 1;
                self.ink.end_drag(*seq);
                self.invalidate_base();
                let hit = self.cursor.and_then(|p| self.ink.hit(p, &self.styles.ink));
                self.ink.set_hover(hit);
                self.update_draw_cursor();
            }
            (MouseButton::Left, false) => self.commit_stroke(),
            (MouseButton::Right, true) => {
                let target = self.cursor.and_then(|p| self.ink.hit(p, &self.styles.ink));
                match target {
                    Some(id) if !self.ink.is_drawing() && !self.ink.is_dragging() => {
                        *seq += 1;
                        self.ink.delete(id, *seq);
                        self.invalidate_base();
                        self.update_draw_cursor();
                    }
                    _ => s.next_color(self.styles.ink.palette.len()),
                }
            }
            (MouseButton::Middle, true) => s.next_tool(),
            _ => return ButtonOutcome::Nothing,
        }
        self.dirty = true;
        if s != self.ink_settings {
            self.set_ink(s);
            return ButtonOutcome::Ink(s);
        }
        ButtonOutcome::Nothing
    }

    // ---- Teks ----

    /// Pratinjau teks yang sedang diketik di posisi `at` (`None` = hapus pratinjau).
    pub fn set_pending_text(&mut self, pending: Option<((f32, f32), String)>) {
        self.ink.set_pending_text(pending);
        self.dirty = true;
    }

    /// Tambahkan objek teks (dengan urutan riwayat `seq`).
    pub fn commit_text(&mut self, at: (f32, f32), content: &str, seq: u64) -> bool {
        self.ink.set_pending_text(None);
        self.dirty = true;
        self.ink.add_text(at, content, seq, self.base.as_mut(), &self.styles.ink)
    }

    /// Posisi monitor ini di layar (piksel fisik), untuk menempatkan kotak input teks.
    pub fn origin(&self) -> PhysicalPosition<i32> {
        self.origin
    }

    pub fn text_size(&self) -> f32 {
        self.styles.ink.text_size
    }

    /// Kursor mode Draw: tangan menggenggam saat menggeser, panah geser di atas objek,
    /// crosshair untuk menggambar.
    fn update_draw_cursor(&self) {
        if self.mode != Mode::Draw {
            return;
        }
        let icon = if self.ink.is_dragging() {
            CursorIcon::Grabbing
        } else if self.ink.hover().is_some() {
            CursorIcon::Move
        } else if self.ink.tool == Tool::Text {
            CursorIcon::Text
        } else {
            CursorIcon::Crosshair
        };
        self.window.set_cursor(icon);
    }

    /// Gambar coretan aktif ke layer base lalu simpan.
    fn commit_stroke(&mut self) {
        if !self.ink.is_drawing() {
            return;
        }
        self.ink.end(self.base.as_mut(), &self.styles.ink);
        self.dirty = true;
    }

    pub fn last_stroke_seq(&self) -> Option<u64> {
        self.ink.last_seq()
    }

    pub fn undo(&mut self) {
        if self.ink.undo() {
            self.invalidate_base();
        }
    }

    pub fn clear(&mut self) {
        if self.ink.clear() {
            self.invalidate_base();
        }
    }

    fn invalidate_base(&mut self) {
        self.base_stale = true;
        self.full_redraw = true;
        self.dirty = true;
    }

    /// Posisi pratinjau kuas; disembunyikan saat kursor di atas objek atau sedang menggeser
    /// (kursor "move" sudah memberi tanda).
    fn brush_preview_at(&self) -> Option<(f32, f32)> {
        let busy = self.ink.is_dragging() || self.ink.hover().is_some();
        self.cursor.filter(|_| !busy)
    }

    /// Kapan frame berikutnya perlu digambar; `None` bila tidak ada yang berubah.
    pub fn next_frame_at(&self, now: Instant) -> Option<Instant> {
        if !self.dirty {
            return None;
        }
        Some(self.last_frame.map_or(now, |t| (t + FRAME).max(now)))
    }

    pub fn render(&mut self, now: Instant) -> Result<(), String> {
        let st = &self.styles;
        let start = Instant::now();
        // Warna laser mengikuti warna aktif (bisa diganti kapan saja lewat menu radial).
        let active = st.ink.palette.get(self.ink_settings.color).copied();
        let laser_style = LaserStyle {
            color: active.filter(|_| st.laser_follows_ink).unwrap_or(st.laser.color),
            ..st.laser
        };
        let halo_style = halo::HaloStyle {
            color: active.filter(|_| st.halo_follows_ink).unwrap_or(st.halo.color),
            ..st.halo
        };
        self.laser.prune(now, st.laser.trail);
        if let Some(r) = &st.ripple {
            self.ripples.prune(now, r);
        }
        if self.base_stale {
            if let Some(base) = &mut self.base {
                base.fill(imp::draw_backdrop());
                self.ink.draw_committed(base, &st.ink);
            }
            self.base_stale = false;
        }

        let effect = match self.mode {
            Mode::Laser => self.laser.bounds(&laser_style),
            Mode::Spotlight => spotlight::bounds(self.cursor, &st.spotlight),
            Mode::Cursor => halo::bounds(self.cursor, &halo_style),
            Mode::Draw => union_bounds(
                self.ink.active_bounds(&st.ink),
                self.brush_preview_at().map(|c| self.ink.preview_bounds(c, &st.ink)),
            ),
            Mode::Idle => None,
        };
        let ripple = st.ripple.and_then(|r| self.ripples.bounds(&r));
        // Dibangun dari field langsung (bukan `self.menu_view()`) agar peminjaman tidak
        // bentrok dengan `&mut self.pixmap` di bawah.
        let menu_view = self.menu.as_ref().map(|m| MenuView {
            center: m.center,
            scale: self.scale,
            hover: m.hover,
            mode: self.mode,
            ink: self.ink_settings,
            palette: &st.ink.palette,
        });
        let menu_bounds = menu_view.as_ref().map(menu::bounds);
        let help_center = self.screen_center();
        let help_bounds = self.help.as_ref().map(|h| help::bounds(help_center, self.scale, h));
        let cur = union_bounds(union_bounds(union_bounds(effect, ripple), menu_bounds), help_bounds)
            .and_then(|b| Damage::from_bounds(b, self.size));
        let damage = if self.full_redraw {
            Some(Damage::full(self.size))
        } else {
            match (self.prev, cur) {
                (Some(a), Some(b)) => Some(a.union(b)),
                (a, b) => a.or(b),
            }
        };
        if let Some(d) = damage {
            let area = Rect::from_xywh(d.x as f32, d.y as f32, d.w as f32, d.h as f32)
                .ok_or("invalid damage rect")?;
            match (self.mode, &self.base) {
                (Mode::Spotlight, _) => spotlight::draw(&mut self.pixmap, area, self.cursor, &st.spotlight),
                (Mode::Draw, Some(base)) => copy_region(&mut self.pixmap, base, d),
                _ => {
                    let backdrop = if self.menu_capture {
                        imp::draw_backdrop()
                    } else {
                        tiny_skia::Color::TRANSPARENT
                    };
                    let clear = Paint {
                        shader: Shader::SolidColor(backdrop),
                        blend_mode: BlendMode::Source,
                        anti_alias: false,
                        ..Paint::default()
                    };
                    self.pixmap.fill_rect(area, &clear, Transform::identity(), None);
                }
            }
            match self.mode {
                Mode::Laser => self.laser.draw(&mut self.pixmap, now, &laser_style),
                Mode::Cursor => halo::draw(&mut self.pixmap, self.cursor, &halo_style),
                Mode::Draw => {
                    self.ink.draw_current(&mut self.pixmap, &st.ink);
                    if let Some(c) = self.brush_preview_at() {
                        self.ink.draw_preview(&mut self.pixmap, c, &st.ink);
                    }
                }
                _ => {}
            }
            if let Some(r) = &st.ripple {
                self.ripples.draw(&mut self.pixmap, now, r);
            }
            if let Some(v) = &menu_view {
                menu::draw(&mut self.pixmap, v);
            }
            if let Some(h) = &self.help {
                help::draw(&mut self.pixmap, help_center, self.scale, h);
            }
            self.presenter.present(&self.pixmap, d)?;
            self.full_redraw = false;
        }
        self.prev = cur;
        self.last_frame = Some(now);
        self.frames += 1;
        self.busy += start.elapsed();
        self.dirty = self.laser.is_animating() || self.ripples.is_animating();
        Ok(())
    }
}

/// Salin area `d` dari `src` ke `dst` (ukuran sama, format sama).
fn copy_region(dst: &mut Pixmap, src: &Pixmap, d: Damage) {
    let stride = dst.width() as usize * 4;
    let (x0, x1) = (d.x as usize * 4, (d.x + d.w) as usize * 4);
    let src = src.data();
    let dst = dst.data_mut();
    for row in d.y as usize..(d.y + d.h) as usize {
        let r = row * stride;
        dst[r + x0..r + x1].copy_from_slice(&src[r + x0..r + x1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: PhysicalSize<u32> = PhysicalSize::new(100, 50);

    #[test]
    fn damage_from_bounds_rounds_out_and_clips() {
        assert_eq!(
            Damage::from_bounds((10.4, 5.6, 20.2, 9.1), SCREEN),
            Some(Damage { x: 10, y: 5, w: 11, h: 5 })
        );
        assert_eq!(
            Damage::from_bounds((-30.0, 40.0, 130.0, 80.0), SCREEN),
            Some(Damage { x: 0, y: 40, w: 100, h: 10 })
        );
        assert_eq!(Damage::from_bounds((200.0, 0.0, 300.0, 10.0), SCREEN), None);
    }

    #[test]
    fn damage_union() {
        let a = Damage { x: 10, y: 10, w: 5, h: 5 };
        let b = Damage { x: 30, y: 2, w: 10, h: 4 };
        assert_eq!(a.union(b), Damage { x: 10, y: 2, w: 30, h: 13 });
    }

    #[test]
    fn copy_region_only_touches_rect() {
        let mut src = Pixmap::new(4, 3).unwrap();
        src.fill(tiny_skia::Color::from_rgba8(0, 255, 0, 255));
        let mut dst = Pixmap::new(4, 3).unwrap();
        copy_region(&mut dst, &src, Damage { x: 2, y: 0, w: 2, h: 2 });
        let alpha: Vec<u8> = dst.pixels().iter().map(|p| p.alpha()).collect();
        assert_eq!(alpha, [0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn copy_damage_only_touches_rect() {
        let mut pm = Pixmap::new(4, 3).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        let mut dst = vec![0u32; 12];
        copy_damage(&mut dst, &pm, Damage { x: 1, y: 1, w: 2, h: 1 });
        let red = 0xffff0000;
        assert_eq!(dst, [0, 0, 0, 0, 0, red, red, 0, 0, 0, 0, 0]);
    }
}

#[cfg(test)]
mod bench {
    use super::*;

    /// Perkiraan biaya render saat mouse bergerak cepat terus-menerus (tanpa present).
    /// Jalankan: cargo test --release -- --ignored --nocapture render_cost
    #[test]
    #[ignore]
    fn render_cost() {
        let size = PhysicalSize::new(2560, 1440);
        let mut pm = Pixmap::new(size.width, size.height).unwrap();
        let mut buf = vec![0u32; (size.width * size.height) as usize];
        let style = LaserStyle { color: (255, 32, 32), radius: 6.0, trail: Duration::from_millis(450) };
        let mut laser = Laser::default();
        let t0 = Instant::now();
        let mut prev: Option<Damage> = None;
        let frames = 600;
        let mut busy = Duration::ZERO;
        for i in 0..frames {
            // Simulasi 60 fps, kursor bergerak ~1500 px/detik dengan 16 event per frame.
            let now = t0 + FRAME * i;
            for k in 0..16 {
                let t = i as f32 + k as f32 / 16.0;
                laser.move_to(1280.0 + 900.0 * (t / 40.0).sin(), 720.0 + 500.0 * (t / 23.0).cos(), now);
            }
            let start = Instant::now();
            laser.prune(now, style.trail);
            let cur = laser.bounds(&style).and_then(|b| Damage::from_bounds(b, size));
            let d = match (prev, cur) { (Some(a), Some(b)) => a.union(b), (a, b) => a.or(b).unwrap() };
            let r = Rect::from_xywh(d.x as f32, d.y as f32, d.w as f32, d.h as f32).unwrap();
            let clear = Paint { shader: Shader::SolidColor(tiny_skia::Color::TRANSPARENT), blend_mode: BlendMode::Source, ..Paint::default() };
            pm.fill_rect(r, &clear, Transform::identity(), None);
            laser.draw(&mut pm, now, &style);
            copy_damage(&mut buf, &pm, d);
            busy += start.elapsed();
            prev = cur;
        }
        let avg = busy / frames;
        eprintln!("average {avg:?}/frame, ~{:.1}% of one CPU core at 60 fps", avg.as_secs_f64() * 60.0 * 100.0);
    }
}
