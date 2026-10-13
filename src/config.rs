//! Konfigurasi TOML: `~/.config/pointer/config.toml` (Linux) atau
//! `%APPDATA%\pointer\config.toml` (Windows). File boleh tidak ada; semua field punya default.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Sembunyikan kursor sistem saat laser aktif (Linux, mode capture).
    pub hide_cursor: bool,
    pub laser: LaserConfig,
    pub spotlight: SpotlightConfig,
    pub cursor: CursorConfig,
    pub click: ClickConfig,
    pub draw: DrawConfig,
    pub hotkeys: HotkeyConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LaserConfig {
    /// Ikuti warna aktif (dipilih lewat menu radial atau klik kanan di mode Draw).
    pub follow_color: bool,
    /// Warna `#RRGGBB`, dipakai bila `follow_color = false`.
    pub color: String,
    /// Jari-jari titik laser dalam piksel logis.
    pub radius: f32,
    /// Lama trail sebelum hilang, milidetik.
    pub trail_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpotlightConfig {
    /// Jari-jari lingkaran terang dalam piksel logis.
    pub radius: f32,
    /// Opasitas area gelap, 0.1–0.95.
    pub dim: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CursorConfig {
    /// Ikuti warna aktif, seperti laser.
    pub follow_color: bool,
    /// Warna `#RRGGBB`, dipakai bila `follow_color = false`.
    pub color: String,
    /// Sisi kotak di sekitar kursor dalam piksel logis.
    pub size: f32,
    /// Tebal garis dalam piksel logis.
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClickConfig {
    /// Tampilkan cincin saat mouse diklik (selama laser/spotlight aktif).
    pub enabled: bool,
    /// Warna `#RRGGBB`.
    pub color: String,
    /// Jari-jari akhir cincin dalam piksel logis.
    pub radius: f32,
    /// Lama animasi, milidetik.
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DrawConfig {
    /// Palet warna `#RRGGBB`; klik kanan berpindah ke warna berikutnya.
    pub colors: Vec<String>,
    /// Lebar pena dalam piksel logis.
    pub pen_width: f32,
    /// Lebar stabilo dalam piksel logis.
    pub highlighter_width: f32,
    /// Opasitas stabilo, 0.1–0.9.
    pub highlighter_alpha: f32,
    /// Jari-jari sudut alat kotak dalam piksel logis (0 = sudut tajam).
    pub rect_radius: f32,
    /// Ukuran huruf objek teks dalam piksel logis.
    pub text_size: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HotkeyConfig {
    /// Format: `Ctrl+Alt+L`. Modifier: Ctrl, Alt, Shift, Super.
    pub menu: String,
    pub laser: String,
    pub spotlight: String,
    pub cursor: String,
    pub draw: String,
    pub clear: String,
    pub undo: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hide_cursor: true,
            laser: LaserConfig::default(),
            spotlight: SpotlightConfig::default(),
            cursor: CursorConfig::default(),
            click: ClickConfig::default(),
            draw: DrawConfig::default(),
            hotkeys: HotkeyConfig::default(),
        }
    }
}

impl Default for LaserConfig {
    fn default() -> Self {
        Self {
            follow_color: true,
            color: "#ff2020".into(),
            radius: 6.0,
            trail_ms: 450,
        }
    }
}

impl Default for SpotlightConfig {
    fn default() -> Self {
        Self {
            radius: 140.0,
            dim: 0.6,
        }
    }
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            follow_color: false,
            color: "#2fd47a".into(),
            size: 80.0,
            width: 3.0,
        }
    }
}

impl Default for ClickConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            color: "#ffc400".into(),
            radius: 28.0,
            duration_ms: 500,
        }
    }
}

impl Default for DrawConfig {
    fn default() -> Self {
        Self {
            colors: ["#ff2020", "#ffd400", "#20c040", "#2080ff", "#ffffff"]
                .map(String::from)
                .into(),
            pen_width: 4.0,
            highlighter_width: 18.0,
            highlighter_alpha: 0.35,
            rect_radius: 12.0,
            text_size: 32.0,
        }
    }
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        // Ctrl+Alt+D tidak dipakai: bawaan GNOME untuk "show desktop".
        Self {
            menu: "Ctrl+Alt+Space".into(),
            laser: "Ctrl+Alt+L".into(),
            spotlight: "Ctrl+Alt+S".into(),
            cursor: "Ctrl+Alt+H".into(),
            draw: "Ctrl+Alt+P".into(),
            clear: "Ctrl+Alt+C".into(),
            undo: "Ctrl+Alt+Z".into(),
        }
    }
}

impl HotkeyConfig {
    /// Pasangan (nama field, hotkey) untuk validasi dan pesan error.
    pub fn entries(&self) -> [(&'static str, &str); 7] {
        [
            ("menu", &self.menu),
            ("laser", &self.laser),
            ("spotlight", &self.spotlight),
            ("cursor", &self.cursor),
            ("draw", &self.draw),
            ("clear", &self.clear),
            ("undo", &self.undo),
        ]
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("pointer").join("config.toml"))
    }

    /// Muat config dari disk; default bila file tidak ada.
    pub fn load() -> Result<Self, String> {
        let Some(path) = Self::path() else {
            return Ok(Self::default());
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let cfg: Config = toml::from_str(text).map_err(|e| e.to_string())?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), String> {
        parse_hex_color(&self.laser.color)?;
        if !(1.0..=100.0).contains(&self.laser.radius) {
            return Err("laser.radius must be between 1 and 100".into());
        }
        if !(50..=5000).contains(&self.laser.trail_ms) {
            return Err("laser.trail_ms must be between 50 and 5000".into());
        }
        if !(20.0..=1000.0).contains(&self.spotlight.radius) {
            return Err("spotlight.radius must be between 20 and 1000".into());
        }
        if !(0.1..=0.95).contains(&self.spotlight.dim) {
            return Err("spotlight.dim must be between 0.1 and 0.95".into());
        }
        parse_hex_color(&self.cursor.color)?;
        if !(16.0..=400.0).contains(&self.cursor.size) {
            return Err("cursor.size must be between 16 and 400".into());
        }
        if !(1.0..=20.0).contains(&self.cursor.width) {
            return Err("cursor.width must be between 1 and 20".into());
        }
        parse_hex_color(&self.click.color)?;
        if !(5.0..=200.0).contains(&self.click.radius) {
            return Err("click.radius must be between 5 and 200".into());
        }
        if !(100..=3000).contains(&self.click.duration_ms) {
            return Err("click.duration_ms must be between 100 and 3000".into());
        }
        if !(1..=9).contains(&self.draw.colors.len()) {
            return Err("draw.colors must contain 1–9 colors".into());
        }
        for c in &self.draw.colors {
            parse_hex_color(c).map_err(|e| format!("draw.colors: {e}"))?;
        }
        if !(1.0..=50.0).contains(&self.draw.pen_width) {
            return Err("draw.pen_width must be between 1 and 50".into());
        }
        if !(4.0..=100.0).contains(&self.draw.highlighter_width) {
            return Err("draw.highlighter_width must be between 4 and 100".into());
        }
        if !(0.1..=0.9).contains(&self.draw.highlighter_alpha) {
            return Err("draw.highlighter_alpha must be between 0.1 and 0.9".into());
        }
        if !(0.0..=100.0).contains(&self.draw.rect_radius) {
            return Err("draw.rect_radius must be between 0 and 100".into());
        }
        if !(8.0..=200.0).contains(&self.draw.text_size) {
            return Err("draw.text_size must be between 8 and 200".into());
        }
        let keys = self.hotkeys.entries();
        for (i, (a, ka)) in keys.iter().enumerate() {
            for (b, kb) in &keys[i + 1..] {
                if normalize_hotkey(ka) == normalize_hotkey(kb) {
                    return Err(format!("hotkeys.{a} and hotkeys.{b} must be different"));
                }
            }
        }
        Ok(())
    }

    /// Tulis config default bila file belum ada; kembalikan path-nya.
    pub fn ensure_file() -> Result<PathBuf, String> {
        let path = Self::path().ok_or("config directory not found")?;
        if !path.exists() {
            path.parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|_| std::fs::write(&path, Self::default().to_toml()))
                .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
        }
        Ok(path)
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("config selalu bisa diserialisasi")
    }
}

/// Bandingkan hotkey tanpa peduli huruf besar/kecil dan spasi.
fn normalize_hotkey(s: &str) -> String {
    s.split('+').map(|t| t.trim().to_ascii_lowercase()).collect::<Vec<_>>().join("+")
}

/// `#RRGGBB` -> (r, g, b).
pub fn parse_hex_color(s: &str) -> Result<(u8, u8, u8), String> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    let err = || format!("invalid color: {s:?} (use the #RRGGBB format)");
    if hex.len() != 6 || !hex.is_ascii() {
        return Err(err());
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| err());
    Ok((byte(0)?, byte(2)?, byte(4)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_gives_defaults() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
    }

    #[test]
    fn partial_override() {
        let cfg = Config::parse("[laser]\ncolor = \"#00ff00\"\n").unwrap();
        assert_eq!(cfg.laser.color, "#00ff00");
        assert_eq!(cfg.laser.trail_ms, LaserConfig::default().trail_ms);
    }

    #[test]
    fn default_roundtrips_through_toml() {
        let cfg = Config::default();
        assert_eq!(Config::parse(&cfg.to_toml()).unwrap(), cfg);
    }

    #[test]
    fn rejects_bad_values() {
        assert!(Config::parse("[laser]\ncolor = \"red\"\n").is_err());
        assert!(Config::parse("[laser]\nradius = 0.0\n").is_err());
        assert!(Config::parse("unknown = 1\n").is_err());
        assert!(Config::parse("[spotlight]\ndim = 1.0\n").is_err());
        assert!(Config::parse("[cursor]\nsize = 5.0\n").is_err());
        assert!(Config::parse("[hotkeys]\ncursor = \"Ctrl+Alt+L\"\n").is_err());
        assert!(Config::parse("[click]\ncolor = \"#12\"\n").is_err());
        assert!(Config::parse("[hotkeys]\nspotlight = \"ctrl + alt+l\"\n").is_err());
        assert!(Config::parse("[draw]\ncolors = []\n").is_err());
        assert!(Config::parse("[draw]\ncolors = [\"#ff0000\", \"blue\"]\n").is_err());
    }

    #[test]
    fn hex_color() {
        assert_eq!(parse_hex_color("#ff2020"), Ok((255, 32, 32)));
        assert_eq!(parse_hex_color("0a0B0c"), Ok((10, 11, 12)));
        assert!(parse_hex_color("#fff").is_err());
        assert!(parse_hex_color("#gg0000").is_err());
    }

    #[test]
    fn laser_follows_active_color_by_default() {
        assert!(Config::default().laser.follow_color);
        // Config lama tanpa `follow_color` tetap valid.
        let cfg = Config::parse("[laser]\ncolor = \"#00ff00\"\n").unwrap();
        assert!(cfg.laser.follow_color);
        let cfg = Config::parse("[laser]\nfollow_color = false\n").unwrap();
        assert!(!cfg.laser.follow_color);
    }
}
