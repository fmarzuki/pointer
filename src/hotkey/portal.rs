//! Hotkey via portal GlobalShortcuts (GNOME/KDE di Wayland).
//!
//! Pertama kali dijalankan, GNOME menampilkan dialog untuk menyetujui shortcut.

use ashpd::AppID;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;

use super::Handler;
use crate::config::HotkeyConfig;

pub const APP_ID: &str = "io.github.fmarzuki.Pointer";

pub fn spawn(cfg: HotkeyConfig, handler: Handler) {
    let spawned = std::thread::Builder::new()
        .name("portal-hotkeys".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => return eprintln!("[hotkey] failed to create runtime: {e}"),
            };
            if let Err(e) = rt.block_on(run(&cfg, handler)) {
                eprintln!("[hotkey] GlobalShortcuts portal failed: {e}");
                eprintln!("[hotkey] run `pointer install` once so GNOME recognizes the app id,");
                eprintln!(
                    "[hotkey] or add a shortcut manually: Settings > Keyboard > Custom Shortcuts, \
                     command `pointer laser`"
                );
            }
        });
    if let Err(e) = spawned {
        eprintln!("[hotkey] failed to create portal thread: {e}");
    }
}

async fn run(cfg: &HotkeyConfig, handler: Handler) -> Result<(), String> {
    let bindings = super::bindings(cfg);
    let triggers = bindings
        .iter()
        .map(|b| super::to_portal_trigger(&b.keys).map_err(|e| format!("hotkeys.{}: {e}", b.id)))
        .collect::<Result<Vec<_>, _>>()?;

    // Aplikasi non-sandbox perlu mendaftarkan app id sebelum memakai portal lain.
    let app_id = AppID::try_from(APP_ID).map_err(|e| e.to_string())?;
    if let Err(e) = ashpd::register_host_app(app_id).await {
        eprintln!("[hotkey] register_host_app failed (continuing without app id): {e}");
    }

    let portal = GlobalShortcuts::new().await.map_err(|e| e.to_string())?;
    let session = portal
        .create_session(Default::default())
        .await
        .map_err(|e| e.to_string())?;
    let mut activated = portal
        .receive_activated()
        .await
        .map_err(|e| e.to_string())?;

    let shortcuts: Vec<NewShortcut> = bindings
        .iter()
        .zip(&triggers)
        .map(|(b, t)| NewShortcut::new(b.id, b.description).preferred_trigger(t.as_str()))
        .collect();
    let bound = portal
        .bind_shortcuts(&session, &shortcuts, None, Default::default())
        .await
        .and_then(|req| req.response())
        .map_err(|e| e.to_string())?;
    for s in bound.shortcuts() {
        eprintln!(
            "[hotkey] portal: {} -> {:?}",
            s.id(),
            s.trigger_description()
        );
    }

    while let Some(ev) = activated.next().await {
        if let Some(b) = bindings.iter().find(|b| b.id == ev.shortcut_id()) {
            handler(b.cmd);
        }
    }
    // `session` harus tetap hidup selama loop di atas; drop menutup sesi portal.
    drop(session);
    Ok(())
}
