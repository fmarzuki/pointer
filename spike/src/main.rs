//! Fase 0 spike: overlay transparan per monitor (XWayland) dengan laser + trail.
//!
//! Menguji: transparansi, stacking di atas slideshow fullscreen, apakah overlay
//! mencuri fokus keyboard, dan apakah terlihat saat screen sharing.
//!
//! Pemakaian: spike [--managed] [--passthrough] [--secs N]
//!   default       : window override-redirect (tidak dikelola WM, tidak pernah difokus)
//!   --managed     : window biasa, type NOTIFICATION, always-on-top, WM_HINTS input=False
//!   --passthrough : click-through (pointer tidak tertangkap; di Wayland efek tidak akan bergerak)
//!   --secs N      : keluar otomatis setelah N detik (default 30)

use std::collections::{HashMap, VecDeque};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::x11::{EventLoopBuilderExtX11, WindowAttributesExtX11, WindowType};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowId, WindowLevel};

const TRAIL_MS: u128 = 450;
const RIPPLE_MS: u128 = 500;

struct Opts {
    managed: bool,
    passthrough: bool,
    secs: u64,
}

struct Overlay {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    trail: VecDeque<(f32, f32, Instant)>,
    ripples: Vec<(f32, f32, Instant)>,
    cursor: Option<(f32, f32)>,
    dirty: bool,
}

struct App {
    opts: Opts,
    deadline: Instant,
    overlays: HashMap<WindowId, Overlay>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if !self.overlays.is_empty() {
            return;
        }
        for mon in el.available_monitors() {
            let pos = mon.position();
            let size = mon.size();
            eprintln!(
                "[spike] monitor {:?} pos={:?} size={:?} scale={}",
                mon.name(),
                pos,
                size,
                mon.scale_factor()
            );
            let mut attrs = Window::default_attributes()
                .with_title("pointer-spike")
                .with_transparent(true)
                .with_decorations(false)
                .with_resizable(false)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_position(PhysicalPosition::new(pos.x, pos.y))
                .with_inner_size(PhysicalSize::new(size.width, size.height));
            if self.opts.managed {
                attrs = attrs.with_x11_window_type(vec![WindowType::Notification]);
            } else {
                attrs = attrs.with_override_redirect(true);
            }
            let window = Rc::new(el.create_window(attrs).expect("create window"));
            if self.opts.managed {
                if let Err(e) = set_no_input_hint(&window) {
                    eprintln!("[spike] gagal set WM_HINTS: {e}");
                }
            }
            if self.opts.passthrough {
                if let Err(e) = window.set_cursor_hittest(false) {
                    eprintln!("[spike] set_cursor_hittest gagal: {e}");
                }
            }
            let ctx = softbuffer::Context::new(window.clone()).expect("softbuffer ctx");
            let surface = softbuffer::Surface::new(&ctx, window.clone()).expect("surface");
            let id = window.id();
            window.request_redraw();
            self.overlays.insert(
                id,
                Overlay {
                    window,
                    surface,
                    trail: VecDeque::new(),
                    ripples: Vec::new(),
                    cursor: None,
                    dirty: true,
                },
            );
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(ov) = self.overlays.get_mut(&id) else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Focused(f) => eprintln!("[spike] FOCUS {f} (overlay {id:?})"),
            WindowEvent::KeyboardInput { event, .. } => {
                // Overlay seharusnya TIDAK menerima keyboard. Jika baris ini muncul,
                // fokus keyboard tercuri dari slideshow.
                eprintln!("[spike] KEY diterima overlay: {:?}", event.logical_key);
                if event.logical_key
                    == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
                {
                    el.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let p = (position.x as f32, position.y as f32);
                ov.cursor = Some(p);
                ov.trail.push_back((p.0, p.1, Instant::now()));
                ov.dirty = true;
                ov.window.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                ov.cursor = None;
                ov.dirty = true;
                ov.window.request_redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } => {
                if let Some((x, y)) = ov.cursor {
                    ov.ripples.push((x, y, Instant::now()));
                    ov.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let animating = draw(ov);
                if animating {
                    ov.window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if Instant::now() >= self.deadline {
            eprintln!("[spike] waktu habis, keluar");
            el.exit();
        } else {
            el.set_control_flow(ControlFlow::WaitUntil(self.deadline));
        }
    }
}

/// Gambar satu frame. Mengembalikan true bila masih ada animasi (perlu frame berikutnya).
fn draw(ov: &mut Overlay) -> bool {
    let now = Instant::now();
    ov.trail
        .retain(|(_, _, t)| now.duration_since(*t).as_millis() < TRAIL_MS);
    ov.ripples
        .retain(|(_, _, t)| now.duration_since(*t).as_millis() < RIPPLE_MS);
    let animating = !ov.trail.is_empty() || !ov.ripples.is_empty();
    if !animating && !ov.dirty {
        return false;
    }
    ov.dirty = false;

    let size = ov.window.inner_size();
    let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
        return false;
    };
    ov.surface.resize(w, h).expect("resize");
    let mut pixmap = Pixmap::new(size.width, size.height).expect("pixmap");

    // Trail: lingkaran mengecil dan memudar sesuai umur titik.
    for (x, y, t) in &ov.trail {
        let age = now.duration_since(*t).as_millis() as f32 / TRAIL_MS as f32;
        let k = 1.0 - age;
        circle(&mut pixmap, *x, *y, 2.0 + 6.0 * k, Color::from_rgba(1.0, 0.1, 0.1, 0.6 * k).unwrap());
    }
    // Kepala laser: glow berlapis + inti terang.
    if let Some((x, y)) = ov.cursor {
        for (r, a) in [(22.0, 0.08), (15.0, 0.15), (10.0, 0.3)] {
            circle(&mut pixmap, x, y, r, Color::from_rgba(1.0, 0.0, 0.0, a).unwrap());
        }
        circle(&mut pixmap, x, y, 6.0, Color::from_rgba(1.0, 0.25, 0.2, 1.0).unwrap());
        circle(&mut pixmap, x, y, 2.5, Color::from_rgba(1.0, 0.9, 0.9, 1.0).unwrap());
    }
    // Ripple klik.
    for (x, y, t) in &ov.ripples {
        let age = now.duration_since(*t).as_millis() as f32 / RIPPLE_MS as f32;
        ring(&mut pixmap, *x, *y, 10.0 + 40.0 * age, Color::from_rgba(1.0, 0.8, 0.0, 1.0 - age).unwrap());
    }

    // tiny-skia: RGBA premultiplied -> softbuffer: 0xAARRGGBB (depth 32 => byte atas = alpha).
    let mut buf = ov.surface.buffer_mut().expect("buffer");
    for (dst, px) in buf.iter_mut().zip(pixmap.pixels()) {
        *dst = (px.alpha() as u32) << 24
            | (px.red() as u32) << 16
            | (px.green() as u32) << 8
            | px.blue() as u32;
    }
    buf.present().expect("present");
    animating
}

fn circle(pm: &mut Pixmap, x: f32, y: f32, r: f32, c: Color) {
    if let Some(path) = PathBuilder::from_circle(x, y, r) {
        let mut paint = Paint::default();
        paint.set_color(c);
        paint.anti_alias = true;
        pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
}

fn ring(pm: &mut Pixmap, x: f32, y: f32, r: f32, c: Color) {
    if let Some(path) = PathBuilder::from_circle(x, y, r) {
        let mut paint = Paint::default();
        paint.set_color(c);
        paint.anti_alias = true;
        let stroke = tiny_skia::Stroke {
            width: 3.0,
            ..Default::default()
        };
        pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

/// Set WM_HINTS dengan input=False agar WM tidak memberi fokus keyboard ke overlay.
fn set_no_input_hint(window: &Window) -> Result<(), Box<dyn std::error::Error>> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, PropMode};
    use x11rb::wrapper::ConnectionExt as _;

    let xid = match window.window_handle()?.as_raw() {
        RawWindowHandle::Xlib(h) => h.window as u32,
        RawWindowHandle::Xcb(h) => h.window.get(),
        other => return Err(format!("bukan window X11: {other:?}").into()),
    };
    let (conn, _) = x11rb::connect(None)?;
    // flags = InputHint (1), input = 0 (False); sisa field 0.
    let hints: [u32; 9] = [1, 0, 0, 0, 0, 0, 0, 0, 0];
    conn.change_property32(PropMode::REPLACE, xid, AtomEnum::WM_HINTS, AtomEnum::WM_HINTS, &hints)?;
    conn.flush()?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let secs = args
        .iter()
        .position(|a| a == "--secs")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let opts = Opts {
        managed: args.iter().any(|a| a == "--managed"),
        passthrough: args.iter().any(|a| a == "--passthrough"),
        secs,
    };
    eprintln!(
        "[spike] mode={} passthrough={} secs={} (Esc juga keluar jika overlay menerima keyboard)",
        if opts.managed { "managed" } else { "override-redirect" },
        opts.passthrough,
        opts.secs
    );

    // Paksa backend X11 (XWayland di GNOME Wayland).
    let event_loop = EventLoop::builder().with_x11().build().expect("event loop");
    let mut app = App {
        deadline: Instant::now() + Duration::from_secs(opts.secs),
        opts,
        overlays: HashMap::new(),
    };
    event_loop.run_app(&mut app).expect("run");
}
