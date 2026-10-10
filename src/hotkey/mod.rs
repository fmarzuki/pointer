//! Hotkey global.
//!
//! - Windows & sesi X11: `global-hotkey` (RegisterHotKey / XGrabKey).
//! - GNOME Wayland: portal `org.freedesktop.portal.GlobalShortcuts`. Bila portal gagal,
//!   user tetap bisa memasang `pointer laser` sebagai custom shortcut di Settings GNOME.

mod native;
#[cfg(target_os = "linux")]
mod portal;
#[cfg(target_os = "linux")]
pub use portal::APP_ID;

use std::sync::Arc;

use crate::command::Command;
use crate::config::HotkeyConfig;

pub type Handler = Arc<dyn Fn(Command) + Send + Sync + 'static>;

/// Satu hotkey global dan perintah yang dipicunya.
pub struct Binding {
    /// Id stabil untuk portal (GNOME mengingat persetujuan per id).
    pub id: &'static str,
    /// Teks yang ditampilkan GNOME di dialog persetujuan portal.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub description: &'static str,
    /// Format config, mis. `Ctrl+Alt+L`.
    pub keys: String,
    pub cmd: Command,
}

pub fn bindings(cfg: &HotkeyConfig) -> Vec<Binding> {
    vec![
        Binding {
            id: "menu",
            description: "Open/close the radial menu",
            keys: cfg.menu.clone(),
            cmd: Command::Menu,
        },
        Binding {
            id: "laser",
            description: "Toggle the laser pointer",
            keys: cfg.laser.clone(),
            cmd: Command::ToggleLaser,
        },
        Binding {
            id: "spotlight",
            description: "Toggle the spotlight",
            keys: cfg.spotlight.clone(),
            cmd: Command::ToggleSpotlight,
        },
        Binding {
            id: "draw",
            description: "Toggle annotation mode",
            keys: cfg.draw.clone(),
            cmd: Command::ToggleDraw,
        },
        Binding {
            id: "clear",
            description: "Clear all annotations",
            keys: cfg.clear.clone(),
            cmd: Command::Clear,
        },
        Binding {
            id: "undo",
            description: "Undo the last annotation",
            keys: cfg.undo.clone(),
            cmd: Command::Undo,
        },
    ]
}

/// Harus disimpan selama daemon berjalan; hotkey native dilepas saat di-drop.
pub enum Hotkeys {
    Native(#[allow(dead_code)] native::NativeHotkeys),
    #[cfg(target_os = "linux")]
    Portal,
    Disabled,
}

pub fn start(cfg: &HotkeyConfig, handler: Handler) -> Hotkeys {
    #[cfg(target_os = "linux")]
    if crate::platform::is_wayland() {
        portal::spawn(cfg.clone(), handler);
        return Hotkeys::Portal;
    }
    match native::start(cfg, handler) {
        Ok(h) => Hotkeys::Native(h),
        Err(e) => {
            eprintln!("[hotkey] failed to register hotkeys: {e}");
            eprintln!("[hotkey] use the `pointer laser` command instead");
            Hotkeys::Disabled
        }
    }
}

/// `Ctrl+Alt+L` -> `CTRL+ALT+l` (format spesifikasi XDG "shortcuts" untuk portal).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn to_portal_trigger(s: &str) -> Result<String, String> {
    let mut parts = Vec::new();
    let tokens: Vec<&str> = s.split('+').map(str::trim).collect();
    let (key, mods) = tokens
        .split_last()
        .filter(|(k, _)| !k.is_empty())
        .ok_or_else(|| format!("empty hotkey: {s:?}"))?;
    for m in mods {
        let m = match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => "CTRL",
            "alt" => "ALT",
            "shift" => "SHIFT",
            "super" | "logo" | "meta" | "cmd" => "LOGO",
            other => return Err(format!("unknown modifier: {other:?}")),
        };
        parts.push(m.to_string());
    }
    // Nama keysym XKB: huruf tunggal dan `space` huruf kecil, selain itu apa adanya
    // (F9, Return, ...).
    if key.chars().count() == 1 || key.eq_ignore_ascii_case("space") {
        parts.push(key.to_ascii_lowercase());
    } else {
        parts.push(key.to_string());
    }
    Ok(parts.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_trigger() {
        assert_eq!(to_portal_trigger("Ctrl+Alt+L").unwrap(), "CTRL+ALT+l");
        assert_eq!(to_portal_trigger("super + shift + F9").unwrap(), "LOGO+SHIFT+F9");
        assert_eq!(to_portal_trigger("Ctrl+Alt+Space").unwrap(), "CTRL+ALT+space");
        assert!(to_portal_trigger("Hyper+L").is_err());
        assert!(to_portal_trigger("").is_err());
    }

    #[test]
    fn default_hotkeys_parse_for_both_backends() {
        for b in bindings(&HotkeyConfig::default()) {
            assert!(b.keys.parse::<global_hotkey::hotkey::HotKey>().is_ok(), "{}", b.keys);
            assert!(to_portal_trigger(&b.keys).is_ok(), "{}", b.keys);
        }
    }
}
