//! Daftar shortcut untuk panel bantuan dan `pointer shortcuts`. Hotkey diambil dari config
//! yang sedang dipakai, sehingga daftar selalu sesuai dengan pengaturan pengguna.

use crate::config::HotkeyConfig;
use crate::hotkey;

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub title: &'static str,
    /// (tombol/aksi, keterangan)
    pub rows: Vec<(String, String)>,
}

pub fn sections(cfg: &HotkeyConfig) -> Vec<Section> {
    let hotkeys = hotkey::bindings(cfg)
        .into_iter()
        .map(|b| (b.keys, b.description.to_string()))
        .collect();
    let rows = |items: &[(&str, &str)]| {
        items.iter().map(|(k, d)| (k.to_string(), d.to_string())).collect()
    };
    vec![
        Section { title: "Hotkeys", rows: hotkeys },
        Section {
            title: "Draw mode (mouse)",
            rows: rows(&[
                ("Left drag", "Draw, or create the selected shape"),
                ("Click (Text tool)", "Place text: type, Enter = add, Shift+Enter = new line, Esc = cancel"),
                ("Left drag on object", "Move the object"),
                ("Right click on object", "Delete the object"),
                ("Right click", "Next color"),
                ("Middle click", "Next tool (pen, marker, box, arrow, ellipse, line, text)"),
            ]),
        },
        Section {
            title: "Radial menu",
            rows: rows(&[
                ("Hover Color / Shape", "Show color / shape options"),
                ("Click Off (center)", "Turn off all effects"),
                ("Click outside", "Close the menu"),
            ]),
        },
    ]
}

/// Teks polos untuk terminal.
pub fn to_text(sections: &[Section]) -> String {
    let width = sections
        .iter()
        .flat_map(|s| s.rows.iter().map(|(k, _)| k.chars().count()))
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for (i, s) in sections.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(s.title);
        out.push('\n');
        for (k, d) in &s.rows {
            out.push_str(&format!("  {k:<width$}  {d}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_follow_config() {
        let cfg = HotkeyConfig { menu: "Ctrl+Alt+M".into(), ..HotkeyConfig::default() };
        let s = sections(&cfg);
        assert_eq!(s[0].rows[0], ("Ctrl+Alt+M".into(), "Open/close the radial menu".into()));
        assert_eq!(s[0].rows.len(), cfg.entries().len(), "semua hotkey tercantum");
    }

    #[test]
    fn text_output_aligns_columns() {
        let text = to_text(&sections(&HotkeyConfig::default()));
        assert!(text.starts_with("Hotkeys\n"));
        assert!(text.contains("Ctrl+Alt+Space"));
        assert!(text.contains("Right click on object"));
        let desc_cols: Vec<usize> = text
            .lines()
            .filter(|l| l.starts_with("  "))
            .map(|l| l.find("  ").map(|_| l[2..].find("  ").unwrap()).unwrap())
            .collect();
        // Kolom keterangan dimulai di kolom yang sama untuk semua baris (lebar kunci terpanjang).
        assert!(desc_cols.iter().all(|&c| c >= desc_cols.iter().copied().min().unwrap()));
    }
}
