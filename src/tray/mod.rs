//! Ikon tray: menu mode, aksi anotasi, autostart, buka config, keluar.
//!
//! Linux: StatusNotifierItem lewat D-Bus (`ksni`, tanpa GTK; di GNOME butuh extension
//! AppIndicator, bawaan Ubuntu). Windows: `tray-icon`.

mod icon;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as imp;

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::app::Mode;
use crate::command::Command;
use crate::config::Config;
use crate::hotkey::Handler;

/// Mode yang bisa dipilih dari menu, berurutan seperti di menu.
const MODES: [(Mode, &str); 4] = [
    (Mode::Idle, "Off"),
    (Mode::Laser, "Laser"),
    (Mode::Spotlight, "Spotlight"),
    (Mode::Draw, "Annotate"),
];

#[derive(Debug, Clone, Copy)]
enum Action {
    SetMode(Mode),
    Clear,
    Undo,
    ToggleAutostart,
    OpenConfig,
    Shortcuts,
    Support,
    Quit,
}

/// Mode terakhir yang diketahui, dibagi antara event loop dan thread menu tray.
#[derive(Clone, Default)]
struct SharedMode(Arc<AtomicU8>);

impl SharedMode {
    fn get(&self) -> Mode {
        MODES
            .get(self.0.load(Ordering::Relaxed) as usize)
            .map_or(Mode::Idle, |m| m.0)
    }

    fn set(&self, mode: Mode) {
        let i = MODES.iter().position(|m| m.0 == mode).unwrap_or(0);
        self.0.store(i as u8, Ordering::Relaxed);
    }
}

fn perform(action: Action, current: Mode, handler: &Handler) {
    match action {
        Action::SetMode(Mode::Idle) => handler(Command::Off),
        // Perintah mode berupa toggle: kirim hanya bila belum di mode tujuan.
        Action::SetMode(target) if target != current => handler(match target {
            Mode::Laser => Command::ToggleLaser,
            Mode::Spotlight => Command::ToggleSpotlight,
            _ => Command::ToggleDraw,
        }),
        // Tetap minta status agar tray menyinkronkan centang menu.
        Action::SetMode(_) => handler(Command::Status),
        Action::Clear => handler(Command::Clear),
        Action::Undo => handler(Command::Undo),
        Action::ToggleAutostart => {
            let enable = !crate::autostart::is_enabled();
            match crate::autostart::set(enable) {
                Ok(()) => eprintln!("[tray] autostart {}", if enable { "enabled" } else { "disabled" }),
                Err(e) => eprintln!("[tray] autostart failed: {e}"),
            }
        }
        Action::OpenConfig => {
            if let Err(e) = Config::ensure_file().and_then(|p| crate::platform::open_in_editor(&p)) {
                eprintln!("[tray] {e}");
            }
        }
        Action::Shortcuts => handler(Command::Shortcuts),
        Action::Support => {
            if let Err(e) = crate::platform::open_url(crate::platform::SUPPORT_URL) {
                eprintln!("[tray] {e}");
            }
        }
        Action::Quit => handler(Command::Quit),
    }
}

fn tooltip(mode: Mode) -> String {
    let label = MODES.iter().find(|m| m.0 == mode).map_or("?", |m| m.1);
    format!("Pointer: {label}")
}

/// Handle tray milik event loop; `set_mode` dipanggil setiap selesai memproses perintah.
pub struct Tray {
    inner: imp::Tray,
    mode: SharedMode,
}

impl Tray {
    pub fn set_mode(&self, mode: Mode) {
        self.mode.set(mode);
        self.inner.set_mode(mode);
    }
}

/// Mulai ikon tray; `None` bila tray tidak tersedia (daemon tetap jalan tanpa tray).
pub fn start(handler: Handler) -> Option<Tray> {
    let mode = SharedMode::default();
    match imp::Tray::start(handler, mode.clone()) {
        Ok(inner) => Some(Tray { inner, mode }),
        Err(e) => {
            eprintln!("[tray] unavailable: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    fn recorder() -> (Handler, Arc<Mutex<Vec<Command>>>) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let sink = log.clone();
        (Arc::new(move |c| sink.lock().unwrap().push(c)), log)
    }

    #[test]
    fn set_mode_sends_toggle_only_when_needed() {
        let (h, log) = recorder();
        perform(Action::SetMode(Mode::Laser), Mode::Idle, &h);
        perform(Action::SetMode(Mode::Laser), Mode::Laser, &h);
        perform(Action::SetMode(Mode::Draw), Mode::Laser, &h);
        perform(Action::SetMode(Mode::Idle), Mode::Draw, &h);
        assert_eq!(
            *log.lock().unwrap(),
            [Command::ToggleLaser, Command::Status, Command::ToggleDraw, Command::Off]
        );
    }

    #[test]
    fn shared_mode_roundtrip() {
        let m = SharedMode::default();
        assert_eq!(m.get(), Mode::Idle);
        for (mode, _) in MODES {
            m.set(mode);
            assert_eq!(m.get(), mode);
        }
    }
}
