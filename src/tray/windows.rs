//! Tray Windows via `tray-icon`. Harus dibuat di thread event loop (thread utama); pesan
//! menu diproses oleh message pump winit.
//!
//! BELUM DIUJI di Windows nyata; hanya diverifikasi dengan `cargo check`.

use std::collections::HashMap;

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use super::{Action, MODES, SharedMode, perform, tooltip};
use crate::app::Mode;
use crate::hotkey::Handler;

const ICON_SIZE: u32 = 32;

pub struct Tray {
    icon: TrayIcon,
    idle_icon: Icon,
    active_icon: Icon,
    modes: Vec<(Mode, CheckMenuItem)>,
    undo: MenuItem,
    clear: MenuItem,
    autostart: CheckMenuItem,
}

fn make_icon(active: bool) -> Result<Icon, String> {
    Icon::from_rgba(super::icon::rgba(ICON_SIZE, active), ICON_SIZE, ICON_SIZE).map_err(|e| e.to_string())
}

impl Tray {
    pub fn start(handler: Handler, shared: SharedMode) -> Result<Self, String> {
        let menu = Menu::new();
        let mut actions: HashMap<MenuId, Action> = HashMap::new();

        let modes: Vec<(Mode, CheckMenuItem)> = MODES
            .iter()
            .map(|&(mode, label)| (mode, CheckMenuItem::new(label, true, mode == Mode::Idle, None)))
            .collect();
        for (mode, item) in &modes {
            actions.insert(item.id().clone(), Action::SetMode(*mode));
            menu.append(item).map_err(|e| e.to_string())?;
        }
        let undo = MenuItem::new("Undo last annotation", false, None);
        let clear = MenuItem::new("Clear all annotations", false, None);
        let autostart = CheckMenuItem::new("Start at login", true, crate::autostart::is_enabled(), None);
        let shortcuts = MenuItem::new("Shortcuts", true, None);
        let open_config = MenuItem::new("Open config", true, None);
        let support = MenuItem::new("Support Pointer (Buy Me a Coffee)", true, None);
        let quit = MenuItem::new("Quit", true, None);
        actions.insert(undo.id().clone(), Action::Undo);
        actions.insert(clear.id().clone(), Action::Clear);
        actions.insert(autostart.id().clone(), Action::ToggleAutostart);
        actions.insert(shortcuts.id().clone(), Action::Shortcuts);
        actions.insert(open_config.id().clone(), Action::OpenConfig);
        actions.insert(support.id().clone(), Action::Support);
        actions.insert(quit.id().clone(), Action::Quit);
        menu.append_items(&[
            &PredefinedMenuItem::separator(),
            &undo,
            &clear,
            &PredefinedMenuItem::separator(),
            &autostart,
            &shortcuts,
            &open_config,
            &PredefinedMenuItem::separator(),
            &support,
            &quit,
        ])
        .map_err(|e| e.to_string())?;

        MenuEvent::set_event_handler(Some(move |ev: MenuEvent| {
            if let Some(&action) = actions.get(&ev.id) {
                perform(action, shared.get(), &handler);
            }
        }));

        let idle_icon = make_icon(false)?;
        let active_icon = make_icon(true)?;
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(tooltip(Mode::Idle))
            .with_icon(idle_icon.clone())
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            icon,
            idle_icon,
            active_icon,
            modes,
            undo,
            clear,
            autostart,
        })
    }

    /// Selalu menerapkan ulang semua status: muda membalik centang item secara otomatis
    /// saat diklik, jadi status menu harus disinkronkan dengan mode sebenarnya.
    pub fn set_mode(&self, mode: Mode) {
        for (m, item) in &self.modes {
            item.set_checked(*m == mode);
        }
        self.undo.set_enabled(mode == Mode::Draw);
        self.clear.set_enabled(mode == Mode::Draw);
        self.autostart.set_checked(crate::autostart::is_enabled());
        let icon = if mode == Mode::Idle { &self.idle_icon } else { &self.active_icon };
        let _ = self.icon.set_icon(Some(icon.clone()));
        let _ = self.icon.set_tooltip(Some(tooltip(mode)));
    }
}
