//! Hotkey via crate `global-hotkey` (Windows: RegisterHotKey, X11: XGrabKey).

use std::collections::HashMap;

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use super::Handler;
use crate::config::HotkeyConfig;

pub struct NativeHotkeys {
    _manager: GlobalHotKeyManager,
}

/// Di Windows harus dipanggil dari thread event loop (thread utama).
pub fn start(cfg: &HotkeyConfig, handler: Handler) -> Result<NativeHotkeys, String> {
    let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
    let mut commands = HashMap::new();
    for b in super::bindings(cfg) {
        let hotkey: HotKey = b
            .keys
            .parse()
            .map_err(|e| format!("hotkeys.{} {:?}: {e}", b.id, b.keys))?;
        manager
            .register(hotkey)
            .map_err(|e| format!("hotkeys.{} {:?}: {e}", b.id, b.keys))?;
        commands.insert(hotkey.id(), b.cmd);
        eprintln!("[hotkey] registered: {} -> {}", b.keys, b.id);
    }
    GlobalHotKeyEvent::set_event_handler(Some(move |ev: GlobalHotKeyEvent| {
        if ev.state() == HotKeyState::Pressed {
            if let Some(&cmd) = commands.get(&ev.id()) {
                handler(cmd);
            }
        }
    }));
    Ok(NativeHotkeys { _manager: manager })
}
