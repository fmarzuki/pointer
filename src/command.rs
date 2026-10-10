//! Perintah yang bisa dikirim ke daemon, baik dari hotkey maupun dari CLI (IPC).

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Nyalakan/matikan mode laser.
    ToggleLaser,
    /// Nyalakan/matikan mode spotlight.
    ToggleSpotlight,
    /// Nyalakan/matikan mode anotasi (pena/stabilo).
    ToggleDraw,
    /// Buka/tutup menu radial.
    Menu,
    /// Tampilkan/tutup panel daftar shortcut.
    Shortcuts,
    /// Hapus semua coretan.
    Clear,
    /// Batalkan coretan terakhir.
    Undo,
    /// Kembali ke Idle (overlay disembunyikan).
    Off,
    /// Tanyakan mode saat ini.
    Status,
    /// Hentikan daemon.
    Quit,
}

impl Command {
    pub fn as_str(self) -> &'static str {
        match self {
            Command::ToggleLaser => "laser",
            Command::ToggleSpotlight => "spotlight",
            Command::ToggleDraw => "draw",
            Command::Menu => "menu",
            Command::Shortcuts => "shortcuts",
            Command::Clear => "clear",
            Command::Undo => "undo",
            Command::Off => "off",
            Command::Status => "status",
            Command::Quit => "quit",
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "laser" => Ok(Command::ToggleLaser),
            "spotlight" => Ok(Command::ToggleSpotlight),
            "draw" => Ok(Command::ToggleDraw),
            "menu" => Ok(Command::Menu),
            "shortcuts" => Ok(Command::Shortcuts),
            "clear" => Ok(Command::Clear),
            "undo" => Ok(Command::Undo),
            "off" => Ok(Command::Off),
            "status" => Ok(Command::Status),
            "quit" => Ok(Command::Quit),
            other => Err(format!("unknown command: {other:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for c in [
            Command::ToggleLaser,
            Command::ToggleSpotlight,
            Command::ToggleDraw,
            Command::Menu,
            Command::Shortcuts,
            Command::Clear,
            Command::Undo,
            Command::Off,
            Command::Status,
            Command::Quit,
        ] {
            assert_eq!(c.as_str().parse::<Command>(), Ok(c));
        }
    }

    #[test]
    fn trims_newline_and_rejects_unknown() {
        assert_eq!("laser\n".parse::<Command>(), Ok(Command::ToggleLaser));
        assert!("erase".parse::<Command>().is_err());
    }
}
