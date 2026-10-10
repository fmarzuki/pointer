//! Overlay Windows: layered window click-through yang tidak pernah aktif, digambar dengan
//! `UpdateLayeredWindow` (per-pixel alpha). softbuffer tidak dipakai di sini karena present
//! GDI-nya tidak membawa alpha.
//!
//! BELUM DIUJI di Windows nyata; hanya diverifikasi dengan `cargo check --target
//! x86_64-pc-windows-msvc`.

use std::ffi::c_void;
use std::ptr;
use std::rc::Rc;

use tiny_skia::Pixmap;
use windows_sys::Win32::Foundation::{HWND, POINT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC,
    HGDIOBJ, SelectObject,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, GetWindowLongPtrW, SetWindowLongPtrW, ULW_ALPHA,
    UpdateLayeredWindow, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::platform::windows::WindowAttributesExtWindows;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowAttributes};

use super::{Damage, copy_damage};

pub fn window_attributes(attrs: WindowAttributes) -> WindowAttributes {
    // Jangan `with_transparent`: itu memakai DWM blur-behind yang bentrok dengan layered window.
    attrs.with_active(false).with_skip_taskbar(true)
}

/// Posisi kursor dalam koordinat layar fisik (winit membuat proses PerMonitorV2 DPI aware).
pub fn cursor_pos() -> Option<(i32, i32)> {
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: pointer ke POINT lokal yang valid.
    (unsafe { GetCursorPos(&mut p) } != 0).then_some((p.x, p.y))
}

/// Tombol kiri mouse sedang ditekan (fisik), untuk highlight klik saat click-through.
pub fn left_button_down() -> bool {
    // SAFETY: tanpa pointer; bit tertinggi = tombol sedang ditekan.
    (unsafe { GetAsyncKeyState(VK_LBUTTON as i32) } as u16 & 0x8000) != 0
}

/// Latar layer coretan. Layered window meneruskan klik pada piksel alpha 0, jadi mode Draw
/// memakai alpha 1/255 (tidak terlihat) agar drag mouse tertangkap di seluruh layar.
pub fn draw_backdrop() -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(0, 0, 0, 1)
}

pub struct Presenter {
    hwnd: HWND,
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u32,
    origin: POINT,
    size: SIZE,
}

impl Presenter {
    pub fn new(
        window: &Rc<Window>,
        origin: PhysicalPosition<i32>,
        size: PhysicalSize<u32>,
        click_through: bool,
    ) -> Result<Self, String> {
        let hwnd = match window.window_handle().map_err(|e| e.to_string())?.as_raw() {
            RawWindowHandle::Win32(h) => h.hwnd.get() as HWND,
            other => return Err(format!("bukan window Win32: {other:?}")),
        };
        // SAFETY: hwnd valid selama `window` hidup; Presenter disimpan bersama window-nya.
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            // WS_EX_NOACTIVATE tetap dipasang di mode Draw: klik tidak memindah fokus keyboard
            // dari slideshow.
            let mut ex = ex | WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW;
            if click_through {
                ex |= WS_EX_TRANSPARENT;
            }
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);

            let dc = CreateCompatibleDC(ptr::null_mut());
            if dc.is_null() {
                return Err("CreateCompatibleDC failed".into());
            }
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size.width as i32,
                biHeight: -(size.height as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let mut bits: *mut c_void = ptr::null_mut();
            let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
            if bitmap.is_null() || bits.is_null() {
                DeleteDC(dc);
                return Err("CreateDIBSection failed".into());
            }
            let old = SelectObject(dc, bitmap);
            Ok(Self {
                hwnd,
                dc,
                bitmap,
                old,
                bits: bits.cast(),
                origin: POINT { x: origin.x, y: origin.y },
                size: SIZE { cx: size.width as i32, cy: size.height as i32 },
            })
        }
    }

    /// Nyalakan/matikan WS_EX_TRANSPARENT (klik tembus ke aplikasi di bawah).
    pub fn set_click_through(&mut self, on: bool) {
        // SAFETY: hwnd valid selama Presenter (dan window-nya) hidup.
        unsafe {
            let ex = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) as u32;
            let ex = if on { ex | WS_EX_TRANSPARENT } else { ex & !WS_EX_TRANSPARENT };
            SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, ex as isize);
        }
    }

    pub fn present(&mut self, pm: &Pixmap, damage: Damage) -> Result<(), String> {
        let len = (self.size.cx * self.size.cy) as usize;
        // SAFETY: DIB section berukuran cx*cy piksel 32-bit, dimiliki Presenter.
        let dst = unsafe { std::slice::from_raw_parts_mut(self.bits, len) };
        // DIB 32-bit little-endian: byte B,G,R,A = u32 0xAARRGGBB, premultiplied. Isi DIB
        // bertahan antar frame, jadi cukup area yang berubah.
        copy_damage(dst, pm, damage);
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let src = POINT { x: 0, y: 0 };
        // SAFETY: semua handle valid; pointer ke struct lokal.
        let ok = unsafe {
            UpdateLayeredWindow(
                self.hwnd,
                ptr::null_mut(),
                &self.origin,
                &self.size,
                self.dc,
                &src,
                0,
                &blend,
                ULW_ALPHA,
            )
        };
        if ok == 0 {
            return Err("UpdateLayeredWindow failed".into());
        }
        Ok(())
    }
}

impl Drop for Presenter {
    fn drop(&mut self) {
        // SAFETY: handle dibuat di `new` dan hanya dilepas sekali di sini.
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}
