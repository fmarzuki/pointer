//! Event loop, state machine mode (Idle, Laser, Spotlight, Cursor, Draw), dan menu radial.

use std::sync::mpsc::SyncSender;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::WindowId;

use crate::command::Command;
use crate::config::{Config, parse_hex_color};
use crate::overlay::{ButtonOutcome, Overlay};
use crate::textinput::{InputResult, TextInput};
use crate::render::Styles;
use crate::render::halo::HaloStyle;
use crate::render::ink::{InkSettings, InkStyle};
use crate::render::laser::LaserStyle;
use crate::render::menu::{Hit, Item};
use crate::render::ripple::RippleStyle;
use crate::render::spotlight::SpotlightStyle;
use crate::shortcuts::{self, Section};
use crate::tray::Tray;

/// Event dari thread lain (hotkey, IPC). `reply` diisi bila pengirim menunggu jawaban.
pub struct AppEvent {
    pub cmd: Command,
    pub reply: Option<SyncSender<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Idle,
    Laser,
    Spotlight,
    /// Kotak bercahaya di sekitar kursor sistem.
    Cursor,
    Draw,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::Idle => "idle",
            Mode::Laser => "laser",
            Mode::Spotlight => "spotlight",
            Mode::Cursor => "cursor",
            Mode::Draw => "draw",
        }
    }
}

/// Bila kursor belum diketahui (overlay baru dibuat), menu muncul di tengah layar setelah ini.
const MENU_PLACE_TIMEOUT: Duration = Duration::from_millis(150);

pub struct App {
    cfg: Config,
    styles: Styles,
    mode: Mode,
    overlays: Vec<Overlay>,
    /// Nomor urut coretan lintas monitor, agar undo membatalkan coretan terbaru.
    stroke_seq: u64,
    /// Alat dan warna tinta, bertahan saat overlay dibuat ulang.
    ink: InkSettings,
    /// Waktu menu radial dibuka; `None` bila menu tertutup.
    menu_opened: Option<Instant>,
    /// Panel shortcut sedang tampil.
    help_open: bool,
    /// Kotak input teks yang sedang terbuka (alat Text).
    text_input: Option<TextInput>,
    shortcut_sections: Vec<Section>,
    tray: Option<Tray>,
    /// Windows: status tombol kiri pada polling sebelumnya, untuk mendeteksi klik baru.
    #[cfg(windows)]
    button_was_down: bool,
}

impl App {
    pub fn new(cfg: Config, tray: Option<Tray>) -> Self {
        let color = |s: &str| parse_hex_color(s).expect("config sudah divalidasi");
        let styles = Styles {
            laser: LaserStyle {
                color: color(&cfg.laser.color),
                radius: cfg.laser.radius,
                trail: Duration::from_millis(cfg.laser.trail_ms),
            },
            spotlight: SpotlightStyle {
                radius: cfg.spotlight.radius,
                dim: cfg.spotlight.dim,
            },
            halo: HaloStyle {
                color: color(&cfg.cursor.color),
                size: cfg.cursor.size,
                width: cfg.cursor.width,
            },
            ripple: cfg.click.enabled.then(|| RippleStyle {
                color: color(&cfg.click.color),
                radius: cfg.click.radius,
                duration: Duration::from_millis(cfg.click.duration_ms),
            }),
            ink: InkStyle {
                palette: cfg.draw.colors.iter().map(|c| color(c)).collect(),
                pen_width: cfg.draw.pen_width,
                highlighter_width: cfg.draw.highlighter_width,
                highlighter_alpha: cfg.draw.highlighter_alpha,
                rect_radius: cfg.draw.rect_radius,
                text_size: cfg.draw.text_size,
            },
            laser_follows_ink: cfg.laser.follow_color,
            halo_follows_ink: cfg.cursor.follow_color,
        };
        let shortcut_sections = shortcuts::sections(&cfg.hotkeys);
        Self {
            cfg,
            styles,
            mode: Mode::Idle,
            overlays: Vec::new(),
            stroke_seq: 0,
            ink: InkSettings::default(),
            menu_opened: None,
            help_open: false,
            text_input: None,
            shortcut_sections,
            tray,
            #[cfg(windows)]
            button_was_down: false,
        }
    }

    /// Buat satu overlay per monitor untuk `mode` (Idle dipakai saat hanya menu yang tampil).
    fn create_overlays(&mut self, el: &ActiveEventLoop, mode: Mode) {
        // Kursor sistem hanya disembunyikan di mode laser; mode lain dan menu memakainya.
        let hide_cursor = self.cfg.hide_cursor && mode == Mode::Laser;
        for monitor in el.available_monitors() {
            match Overlay::new(el, &monitor, mode, &self.styles, self.ink, hide_cursor) {
                Ok(o) => self.overlays.push(o),
                Err(e) => eprintln!("[overlay] failed for monitor {:?}: {e}", monitor.name()),
            }
        }
        if self.overlays.is_empty() {
            eprintln!("[overlay] no overlay could be created");
        }
    }

    fn set_mode(&mut self, el: &ActiveEventLoop, mode: Mode) {
        if mode == self.mode {
            return;
        }
        if std::env::var_os("POINTER_DEBUG").is_some() {
            for o in &self.overlays {
                let avg = o.busy.checked_div(o.frames).unwrap_or_default();
                eprintln!("[debug] overlay: {} frames, total {:?}, average {avg:?}/frame", o.frames, o.busy);
            }
        }
        // Overlay dibuat ulang tiap kali mode berganti agar perubahan monitor (hotplug,
        // resolusi) langsung terbawa; saat Idle tidak ada window sama sekali. Coretan ikut
        // terhapus saat keluar dari mode Draw.
        self.finish_text(true);
        self.close_menu();
        self.close_help();
        self.overlays.clear();
        if mode != Mode::Idle {
            self.create_overlays(el, mode);
            if self.overlays.is_empty() {
                self.mode = Mode::Idle;
                return;
            }
        }
        self.mode = mode;
        eprintln!("[pointer] mode: {}", mode.as_str());
    }

    fn toggle(&mut self, el: &ActiveEventLoop, mode: Mode) {
        let next = if self.mode == mode { Mode::Idle } else { mode };
        self.set_mode(el, next);
    }

    fn handle(&mut self, el: &ActiveEventLoop, cmd: Command) -> String {
        match cmd {
            Command::ToggleLaser => self.toggle(el, Mode::Laser),
            Command::ToggleSpotlight => self.toggle(el, Mode::Spotlight),
            Command::ToggleCursor => self.toggle(el, Mode::Cursor),
            Command::ToggleDraw => self.toggle(el, Mode::Draw),
            Command::Shortcuts => {
                if self.help_open {
                    self.close_help();
                } else {
                    self.open_help(el);
                }
            }
            Command::Menu => {
                if self.menu_opened.is_some() {
                    self.close_menu();
                } else {
                    self.open_menu(el);
                }
            }
            Command::Clear => {
                self.finish_text(false);
                self.overlays.iter_mut().for_each(Overlay::clear);
            }
            Command::Undo => {
                // Batalkan coretan terbaru di monitor mana pun.
                if let Some(o) = self
                    .overlays
                    .iter_mut()
                    .filter(|o| o.last_stroke_seq().is_some())
                    .max_by_key(|o| o.last_stroke_seq())
                {
                    o.undo();
                }
            }
            Command::Off => self.set_mode(el, Mode::Idle),
            Command::Status => {}
            Command::Quit => {
                self.set_mode(el, Mode::Idle);
                el.exit();
            }
        }
        // Sinkronkan tray setelah setiap perintah (termasuk dari menu tray sendiri).
        self.sync_tray();
        format!("ok {}", self.mode.as_str())
    }

    fn sync_tray(&self) {
        if let Some(t) = &self.tray {
            t.set_mode(self.mode);
        }
    }

    fn sync_ink(&mut self) {
        let ink = self.ink;
        self.overlays.iter_mut().for_each(|o| o.set_ink(ink));
    }

    // ---- Menu radial ----

    fn open_menu(&mut self, el: &ActiveEventLoop) {
        self.finish_text(true);
        self.close_help();
        if self.overlays.is_empty() {
            // Idle: buat overlay khusus untuk menu; dihapus lagi saat menu ditutup.
            self.create_overlays(el, Mode::Idle);
            if self.overlays.is_empty() {
                return;
            }
        }
        self.menu_opened = Some(Instant::now());
        for o in &mut self.overlays {
            o.set_menu_capture(true);
        }
        // Muncul di posisi kursor bila sudah diketahui; selain itu menunggu gerakan pertama.
        if let Some(o) = self.overlays.iter_mut().find(|o| o.cursor().is_some()) {
            let at = o.cursor().expect("difilter di atas");
            o.open_menu(at);
        }
    }

    fn close_menu(&mut self) {
        if self.menu_opened.take().is_none() {
            return;
        }
        for o in &mut self.overlays {
            o.set_menu_capture(false);
        }
        if self.mode == Mode::Idle {
            self.overlays.clear();
        }
    }

    // ---- Teks ----

    /// Buka kotak input teks untuk overlay ke-`i` di titik `at` (koordinat lokal).
    fn start_text(&mut self, el: &ActiveEventLoop, i: usize, at: (f32, f32)) {
        self.finish_text(true);
        let o = &mut self.overlays[i];
        let scale = o.window.scale_factor() as f32;
        match TextInput::open(el, o.window.id(), o.origin(), at, o.text_size(), scale) {
            Ok(input) => {
                o.set_pending_text(Some((at, String::new())));
                self.text_input = Some(input);
            }
            Err(e) => eprintln!("[text] failed to open text input: {e}"),
        }
    }

    /// Tutup kotak input teks; `commit` menaruh teksnya sebagai objek (bila tidak kosong).
    fn finish_text(&mut self, commit: bool) {
        let Some(input) = self.text_input.take() else { return };
        let Some(o) = self.overlays.iter_mut().find(|o| o.window.id() == input.overlay) else { return };
        if commit && !input.text.trim().is_empty() {
            self.stroke_seq += 1;
            o.commit_text(input.at, &input.text, self.stroke_seq);
        } else {
            o.set_pending_text(None);
        }
    }

    fn text_event(&mut self, event: &WindowEvent) {
        let Some(input) = &mut self.text_input else { return };
        match input.handle(event) {
            InputResult::Continue => {}
            InputResult::Changed => {
                let (overlay, at, text) = (input.overlay, input.at, input.text.clone());
                if let Some(o) = self.overlays.iter_mut().find(|o| o.window.id() == overlay) {
                    o.set_pending_text(Some((at, text)));
                }
            }
            InputResult::Commit => self.finish_text(true),
            InputResult::Cancel => self.finish_text(false),
        }
    }

    // ---- Panel shortcut ----

    fn open_help(&mut self, el: &ActiveEventLoop) {
        self.close_menu();
        if self.overlays.is_empty() {
            // Idle: overlay khusus panel; dihapus lagi saat panel ditutup.
            self.create_overlays(el, Mode::Idle);
            if self.overlays.is_empty() {
                return;
            }
        }
        self.help_open = true;
        for o in &mut self.overlays {
            o.set_menu_capture(true);
        }
        // Tampil di monitor tempat kursor berada, atau monitor pertama.
        let i = self.overlays.iter().position(|o| o.cursor().is_some()).unwrap_or(0);
        self.overlays[i].show_help(self.shortcut_sections.clone());
    }

    fn close_help(&mut self) {
        if !std::mem::take(&mut self.help_open) {
            return;
        }
        for o in &mut self.overlays {
            o.hide_help();
            o.set_menu_capture(false);
        }
        if self.mode == Mode::Idle {
            self.overlays.clear();
        }
    }

    fn menu_placed(&self) -> bool {
        self.overlays.iter().any(Overlay::has_menu)
    }

    fn menu_action(&mut self, el: &ActiveEventLoop, hit: Hit) {
        let drawing = self.mode == Mode::Draw;
        match hit {
            Hit::Outside => self.close_menu(),
            Hit::Center => self.set_mode(el, Mode::Idle),
            Hit::Help => self.open_help(el),
            Hit::Item(Item::Laser) => self.toggle(el, Mode::Laser),
            Hit::Item(Item::Spotlight) => self.toggle(el, Mode::Spotlight),
            Hit::Item(Item::Cursor) => self.toggle(el, Mode::Cursor),
            Hit::Item(Item::Draw) | Hit::Item(Item::Shape) => {
                if hit == Hit::Item(Item::Draw) {
                    self.ink.select_freehand(drawing);
                } else {
                    self.ink.select_shape(drawing);
                }
                self.sync_ink();
                self.close_menu();
                self.set_mode(el, Mode::Draw);
            }
            Hit::Item(Item::Color) => {
                // Menu tetap terbuka agar perubahan warna langsung terlihat.
                self.ink.next_color(self.styles.ink.palette.len());
                self.sync_ink();
            }
            Hit::ShapeOption(tool) => {
                self.ink.set_tool(tool);
                self.sync_ink();
                self.close_menu();
                self.set_mode(el, Mode::Draw);
            }
            Hit::Swatch(i) => {
                self.ink.color = i;
                self.sync_ink();
                self.close_menu();
            }
            Hit::Item(Item::Erase) => {
                self.overlays.iter_mut().for_each(Overlay::clear);
                self.close_menu();
            }
        }
        self.sync_tray();
    }

    /// Windows: overlay click-through tidak menerima event mouse, jadi kursor dan tombol
    /// kiri di-poll.
    #[cfg(windows)]
    fn poll_mouse(&mut self, now: Instant) {
        // Mode Draw dan menu menangkap mouse sendiri lewat event window; posisi kursor tetap
        // di-poll agar hover menu terasa responsif.
        let menu_open = self.menu_opened.is_some() || self.help_open;
        if self.mode == Mode::Draw && !menu_open {
            return;
        }
        let down = crate::overlay::left_button_down();
        let pressed = down && !self.button_was_down && !menu_open;
        self.button_was_down = down;
        let Some((x, y)) = crate::overlay::cursor_pos() else { return };
        for o in &mut self.overlays {
            match o.to_local(x, y) {
                Some((lx, ly)) => {
                    o.pointer_moved(lx, ly, now);
                    if pressed {
                        o.clicked(now);
                    }
                }
                None if o.has_cursor() => o.pointer_left(),
                None => {}
            }
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, _el: &ActiveEventLoop) {}

    fn user_event(&mut self, el: &ActiveEventLoop, ev: AppEvent) {
        let reply = self.handle(el, ev.cmd);
        if let Some(tx) = ev.reply {
            let _ = tx.try_send(reply);
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.text_input.as_ref().is_some_and(|t| t.id() == id) {
            self.text_event(&event);
            return;
        }
        let menu_open = self.menu_opened.is_some();
        let place_menu = menu_open && !self.menu_placed();
        let Some(i) = self.overlays.iter().position(|o| o.window.id() == id) else {
            return;
        };
        let o = &mut self.overlays[i];
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let p = (position.x as f32, position.y as f32);
                o.pointer_moved(p.0, p.1, Instant::now());
                if place_menu {
                    o.open_menu(p);
                }
            }
            WindowEvent::CursorLeft { .. } => o.pointer_left(),
            WindowEvent::MouseInput { state, .. } if self.help_open => {
                if state == ElementState::Pressed {
                    if o.help_link_hit() {
                        if let Err(e) = crate::platform::open_url(crate::platform::SUPPORT_URL) {
                            eprintln!("[help] {e}");
                        }
                    }
                    self.close_help();
                    self.sync_tray();
                }
            }
            WindowEvent::MouseInput { state, button, .. } if menu_open => {
                if state == ElementState::Pressed && button == winit::event::MouseButton::Left {
                    let hit = o.menu_hit();
                    self.menu_action(el, hit);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                if pressed && self.text_input.is_some() {
                    // Klik di luar kotak input: taruh teks yang sedang diketik dulu.
                    self.finish_text(true);
                }
                let o = &mut self.overlays[i];
                match o.button(button, pressed, Instant::now(), &mut self.stroke_seq) {
                    ButtonOutcome::Nothing => {}
                    ButtonOutcome::Ink(ink) => {
                        self.ink = ink;
                        self.sync_ink();
                    }
                    ButtonOutcome::StartText(at) => self.start_text(el, i, at),
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.overlays.is_empty() {
            el.set_control_flow(ControlFlow::Wait);
            return;
        }
        let now = Instant::now();
        #[cfg(windows)]
        self.poll_mouse(now);

        let mut wake: Option<Instant> = None;
        if let Some(opened) = self.menu_opened {
            if !self.menu_placed() {
                let deadline = opened + MENU_PLACE_TIMEOUT;
                if now >= deadline {
                    let o = &mut self.overlays[0];
                    let at = o.screen_center();
                    o.open_menu(at);
                } else {
                    wake = Some(deadline);
                }
            }
        }
        for o in &mut self.overlays {
            let Some(due) = o.next_frame_at(now) else { continue };
            if due <= now {
                if let Err(e) = o.render(now) {
                    eprintln!("[render] {e}");
                }
            }
            if let Some(next) = o.next_frame_at(now) {
                wake = Some(wake.map_or(next, |w| w.min(next)));
            }
        }
        // Windows perlu terus mem-poll mouse selama overlay aktif (~60 Hz).
        #[cfg(windows)]
        {
            let poll = now + Duration::from_millis(16);
            wake = Some(wake.map_or(poll, |w| w.min(poll)));
        }
        el.set_control_flow(wake.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }
}
