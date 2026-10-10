//! Tray Linux via StatusNotifierItem (`ksni`, API blocking: service berjalan di thread sendiri).

use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{CheckmarkItem, RadioGroup, RadioItem, StandardItem};
use ksni::{Icon, MenuItem, ToolTip};

use super::{Action, MODES, SharedMode, perform, tooltip};
use crate::app::Mode;
use crate::hotkey::Handler;

const ICON_SIZES: [u32; 3] = [22, 32, 48];

struct SniTray {
    mode: Mode,
    autostart: bool,
    handler: Handler,
    shared: SharedMode,
    idle_icons: Vec<Icon>,
    active_icons: Vec<Icon>,
}

impl SniTray {
    fn act(&self, action: Action) {
        perform(action, self.shared.get(), &self.handler);
    }
}

fn icons(active: bool) -> Vec<Icon> {
    ICON_SIZES
        .iter()
        .map(|&size| Icon {
            width: size as i32,
            height: size as i32,
            // RGBA -> ARGB (urutan byte jaringan).
            data: super::icon::rgba(size, active)
                .chunks_exact(4)
                .flat_map(|p| [p[3], p[0], p[1], p[2]])
                .collect(),
        })
        .collect()
}

impl ksni::Tray for SniTray {
    fn id(&self) -> String {
        "pointer".into()
    }

    fn title(&self) -> String {
        "Pointer".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        if self.mode == Mode::Idle {
            self.idle_icons.clone()
        } else {
            self.active_icons.clone()
        }
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: tooltip(self.mode),
            ..Default::default()
        }
    }

    /// Klik kiri (bila panel mendukung): nyalakan/matikan laser.
    fn activate(&mut self, _x: i32, _y: i32) {
        let target = if self.mode == Mode::Laser { Mode::Idle } else { Mode::Laser };
        self.act(Action::SetMode(target));
    }

    fn menu_about_to_show(&mut self) {
        self.autostart = crate::autostart::is_enabled();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let drawing = self.mode == Mode::Draw;
        vec![
            RadioGroup {
                selected: MODES.iter().position(|m| m.0 == self.mode).unwrap_or(0),
                select: Box::new(|this: &mut Self, i| this.act(Action::SetMode(MODES[i].0))),
                options: MODES
                    .iter()
                    .map(|(_, label)| RadioItem {
                        label: (*label).into(),
                        ..Default::default()
                    })
                    .collect(),
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Undo last annotation".into(),
                enabled: drawing,
                activate: Box::new(|this: &mut Self| this.act(Action::Undo)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Clear all annotations".into(),
                enabled: drawing,
                activate: Box::new(|this: &mut Self| this.act(Action::Clear)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            CheckmarkItem {
                label: "Start at login".into(),
                checked: self.autostart,
                activate: Box::new(|this: &mut Self| {
                    this.act(Action::ToggleAutostart);
                    this.autostart = crate::autostart::is_enabled();
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Shortcuts".into(),
                activate: Box::new(|this: &mut Self| this.act(Action::Shortcuts)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Open config".into(),
                activate: Box::new(|this: &mut Self| this.act(Action::OpenConfig)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Support Pointer (Buy Me a Coffee)".into(),
                activate: Box::new(|this: &mut Self| this.act(Action::Support)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|this: &mut Self| this.act(Action::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub struct Tray {
    handle: Handle<SniTray>,
}

impl Tray {
    pub fn start(handler: Handler, shared: SharedMode) -> Result<Self, String> {
        let tray = SniTray {
            mode: Mode::Idle,
            autostart: crate::autostart::is_enabled(),
            handler,
            shared,
            idle_icons: icons(false),
            active_icons: icons(true),
        };
        let handle = tray.spawn().map_err(|e| e.to_string())?;
        Ok(Self { handle })
    }

    pub fn set_mode(&self, mode: Mode) {
        self.handle.update(|t: &mut SniTray| t.mode = mode);
    }
}
