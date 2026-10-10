//! Jalankan daemon otomatis saat login.
//!
//! Linux: `~/.config/autostart/<APP_ID>.desktop` (spesifikasi XDG Autostart).
//! Windows: nilai `Pointer` di `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

#[cfg(target_os = "linux")]
mod imp {
    use std::path::PathBuf;

    fn path() -> Result<PathBuf, String> {
        let dir = dirs::config_dir().ok_or("config directory not found")?;
        Ok(dir
            .join("autostart")
            .join(format!("{}.desktop", crate::hotkey::APP_ID)))
    }

    pub fn is_enabled() -> bool {
        path().is_ok_and(|p| p.exists())
    }

    pub fn set(enabled: bool) -> Result<(), String> {
        let path = path()?;
        if !enabled {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    Err(format!("{}: {e}", path.display()))
                }
                _ => Ok(()),
            };
        }
        let entry = crate::platform::desktop_entry(&crate::platform::exe_path()?)
            + "X-GNOME-Autostart-enabled=true\n";
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, entry).map_err(|e| format!("{}: {e}", path.display()))
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW,
        RegSetKeyValueW,
    };

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE: &str = "Pointer";

    fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
        s.as_ref().encode_wide().chain([0]).collect()
    }

    pub fn is_enabled() -> bool {
        let (key, name) = (wide(RUN_KEY), wide(VALUE));
        // SAFETY: string null-terminated; buffer data null hanya menanyakan keberadaan nilai.
        let rc = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        rc == ERROR_SUCCESS
    }

    pub fn set(enabled: bool) -> Result<(), String> {
        let (key, name) = (wide(RUN_KEY), wide(VALUE));
        if !enabled {
            // SAFETY: string null-terminated.
            let rc = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) };
            return match rc {
                ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
                rc => Err(format!("failed to remove autostart entry (code {rc})")),
            };
        }
        let data = wide(format!("\"{}\" run", crate::platform::exe_path()?));
        // SAFETY: data REG_SZ null-terminated, panjang dalam byte termasuk terminator.
        let rc = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            )
        };
        if rc == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("failed to write autostart entry (code {rc})"))
        }
    }
}

pub use imp::{is_enabled, set};
