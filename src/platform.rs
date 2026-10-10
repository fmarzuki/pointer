//! Deteksi lingkungan desktop.

/// Sesi Wayland (overlay tetap lewat XWayland, tapi hotkey memakai portal).
#[cfg(target_os = "linux")]
pub fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub fn session_name() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        if is_wayland() { "Linux Wayland via XWayland" } else { "Linux X11" }
    }
    #[cfg(windows)]
    {
        "Windows"
    }
}

/// Path binary untuk dijalankan ulang (autostart, file .desktop). Di AppImage,
/// `current_exe` menunjuk ke mount sementara `/tmp/.mount_*`; pakai `$APPIMAGE`.
pub fn exe_path() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        return Ok(appimage.to_string_lossy().into_owned());
    }
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
const ICON_SVG: &str = include_str!("../packaging/linux/io.github.fmarzuki.Pointer.svg");

/// Pasang file `.desktop` dan ikon untuk user saat ini. GNOME menolak GlobalShortcuts dari
/// aplikasi non-sandbox tanpa file `.desktop` ("An app id is required"). Tidak perlu bila
/// dipasang dari paket `.deb` (file sudah ada di `/usr/share`). Mengembalikan path file.
#[cfg(target_os = "linux")]
pub fn install_desktop_file() -> Result<std::path::PathBuf, String> {
    let data = dirs::data_dir().ok_or("data directory not found")?;
    let write = |path: std::path::PathBuf, contents: &str| -> Result<std::path::PathBuf, String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, contents).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    };
    let id = crate::hotkey::APP_ID;
    write(data.join(format!("icons/hicolor/scalable/apps/{id}.svg")), ICON_SVG)?;
    write(data.join(format!("applications/{id}.desktop")), &desktop_entry(&exe_path()?))
}

/// Halaman dukungan proyek (tray, panel shortcut, `--help`, README).
pub const SUPPORT_URL: &str = "https://buymeacoffee.com/fmarzuki";

/// Buka URL di browser bawaan, tanpa menunggu.
pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let mut cmd = std::process::Command::new("xdg-open");
    #[cfg(windows)]
    let mut cmd = std::process::Command::new("explorer.exe");
    cmd.arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to open {url}: {e}"))
}

/// Buka file dengan aplikasi bawaan (Linux: `xdg-open`, Windows: Notepad), tanpa menunggu.
pub fn open_in_editor(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let mut cmd = std::process::Command::new("xdg-open");
    // .toml sering tidak punya asosiasi di Windows; Notepad selalu ada.
    #[cfg(windows)]
    let mut cmd = std::process::Command::new("notepad.exe");
    cmd.arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to open {}: {e}", path.display()))
}

#[cfg(target_os = "linux")]
pub fn desktop_entry(exe: &str) -> String {
    // Spesifikasi Desktop Entry: argumen Exec yang mengandung spasi harus dikutip.
    let exe = if exe.contains(' ') { format!("\"{exe}\"") } else { exe.to_string() };
    let icon = crate::hotkey::APP_ID;
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Pointer\n\
         Comment=Laser pointer overlay for presentations and screen sharing\n\
         Exec={exe} run\n\
         Icon={icon}\n\
         Terminal=false\n\
         Categories=Utility;\n"
    )
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn desktop_entry_quotes_paths_with_spaces() {
        assert!(desktop_entry("/opt/pointer").contains("Exec=/opt/pointer run\n"));
        assert!(desktop_entry("/home/a b/pointer").contains("Exec=\"/home/a b/pointer\" run\n"));
    }

    #[test]
    fn packaged_desktop_file_matches_generated_entry() {
        let packaged = include_str!("../packaging/linux/io.github.fmarzuki.Pointer.desktop");
        assert_eq!(packaged, desktop_entry("pointer"));
    }
}
