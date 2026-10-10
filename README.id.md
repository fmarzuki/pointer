# Pointer

[English](README.md) | **Bahasa Indonesia**

Overlay laser pointer untuk presentasi dan screen sharing di Linux dan Windows. Efek digambar di overlay transparan di atas semua aplikasi, jadi audiens bisa melihatnya saat Anda membagikan layar penuh di Zoom, Meet, Teams, atau OBS.

Saya membuatnya karena sulit menemukan tool laser pointer yang bagus untuk presentasi di Linux.

## Demo

[![Demo Pointer](https://img.youtube.com/vi/HNEtq7MrwPQ/maxresdefault.jpg)](https://www.youtube.com/watch?v=HNEtq7MrwPQ)

## Fitur

- **Laser pointer** dengan trail yang memudar, memakai warna aktif
- **Spotlight**: layar diredupkan kecuali di sekitar kursor
- **Highlight klik**: muncul cincin di tempat Anda mengklik
- **Anotasi**: pena, stabilo, kotak, panah, elips, garis, dan teks
- **Edit objek**: geser, hapus, dan undo per objek anotasi
- **Menu radial** dengan satu hotkey, ditambah ikon tray dan panel shortcut
- Ringan dan cepat: satu file binary ~5 MB yang ditulis dengan Rust, CPU ~0% saat idle

## Instalasi

Unduh versi terbaru di [Releases](https://github.com/fmarzuki/pointer/releases).

| Platform | File | Cara pasang |
|---|---|---|
| Ubuntu 24.04+ / Debian 13+ | `pointer_<versi>-1_amd64.deb` | `sudo apt install ./pointer_*.deb` |
| Linux lain | `Pointer-<versi>-x86_64.AppImage` | `chmod +x Pointer-*.AppImage`, lalu `./Pointer-*.AppImage install` sekali dan jalankan |
| Windows 10/11 | `pointer-<versi>-x64.msi` | Jalankan installer |
| Windows (portable) | `pointer-<versi>-windows-x64.exe` | Langsung jalankan |

Setelah itu jalankan dengan `pointer` (atau dari menu aplikasi). Di GNOME, setujui dialog shortcut yang muncul saat pertama kali dijalankan.

> Linux sudah diuji di GNOME Wayland (Ubuntu). Build Windows lolos CI tetapi masih **beta** karena belum diuji di mesin Windows sungguhan. Windows mungkin menampilkan peringatan SmartScreen karena binary belum di-sign; pilih *More info → Run anyway*.

## Cara pakai

Tekan **`Ctrl+Alt+Space`** untuk membuka menu radial di posisi kursor:

| Sektor | Fungsi |
|---|---|
| **Laser** | Nyalakan/matikan laser pointer |
| **Spotlight** | Nyalakan/matikan spotlight |
| **Draw** | Menggambar bebas; klik lagi untuk ganti pena ↔ stabilo |
| **Shape** | Arahkan kursor untuk memilih Box, Arrow, Ellipse, Line, atau Text |
| **Color** | Arahkan kursor untuk memilih warna (juga dipakai laser) |
| **Erase** | Hapus semua anotasi |
| **Off** (tengah) | Matikan semua efek |
| **?** | Tampilkan semua shortcut |

### Hotkey

| Hotkey | Fungsi |
|---|---|
| `Ctrl+Alt+Space` | Menu radial |
| `Ctrl+Alt+L` | Laser |
| `Ctrl+Alt+S` | Spotlight |
| `Ctrl+Alt+P` | Menggambar |
| `Ctrl+Alt+C` | Hapus semua |
| `Ctrl+Alt+Z` | Undo |

### Mouse di mode menggambar

| Mouse | Fungsi |
|---|---|
| Tahan klik kiri + geser | Menggambar, atau membuat bentuk yang dipilih |
| Tahan klik kiri di atas objek + geser | Menggeser objek |
| Klik kanan di atas objek | Menghapus objek |
| Klik kanan | Warna berikutnya |
| Klik tengah | Alat berikutnya |
| Klik dengan alat Text | Ketik teks; `Enter` menaruh, `Shift+Enter` baris baru, `Esc` batal |

Tombol slideshow (panah, PageUp/PageDown, clicker) tetap berfungsi selama Pointer aktif.

### Command line

```bash
pointer                      # jalankan Pointer
pointer laser | spotlight | draw | menu | off
pointer clear | undo
pointer shortcuts [--show]   # daftar shortcut (atau tampilkan di layar)
pointer autostart on|off     # jalankan saat login
pointer config               # buat file config dan tampilkan lokasinya
pointer quit
```

## Konfigurasi

File config ada di `~/.config/pointer/config.toml` (Windows: `%APPDATA%\pointer\config.toml`). Jalankan `pointer config` untuk membuatnya. Semua pengaturan bersifat opsional:

```toml
[laser]
radius = 6.0
trail_ms = 450
follow_color = true        # false = selalu pakai laser.color

[spotlight]
radius = 140.0
dim = 0.6

[draw]
colors = ["#ff2020", "#ffd400", "#20c040", "#2080ff", "#ffffff"]
pen_width = 4.0
text_size = 32.0

[hotkeys]
menu = "Ctrl+Alt+Space"
laser = "Ctrl+Alt+L"
```

Jalankan ulang Pointer (`pointer quit`, lalu `pointer`) setelah mengubah config.

## Catatan

- Bagikan **layar penuh atau satu monitor**. Kalau Anda hanya membagikan satu jendela, overlay tidak ikut terlihat, karena screen sharing hanya merekam jendela itu.
- Di GNOME, ikon tray membutuhkan extension AppIndicator (sudah aktif bawaan di Ubuntu).
- Di Linux, klik tidak tembus ke aplikasi di bawah overlay selama efek aktif. Gunakan keyboard atau clicker untuk berpindah slide.

## Build dari source

Butuh [Rust](https://rustup.rs). Di Linux juga perlu `build-essential` dan `libxkbcommon-x11-0`.

```bash
cargo build --release
./target/release/pointer install   # khusus Linux, sekali saja: mendaftarkan aplikasi untuk hotkey GNOME
./target/release/pointer
```

Untuk merilis versi baru, ubah `version` di `Cargo.toml` lalu push tag yang sama (misalnya `git tag v0.4.0 && git push origin v0.4.0`). GitHub Actions akan membangun semua paket dan membuat halaman release.

## Lisensi

[GPL-3.0](LICENSE). Font Noto Sans yang dibundel memakai [SIL Open Font License](assets/fonts/OFL.txt).

## Dukungan

Kalau Pointer bermanfaat untuk Anda, dukung pengembangannya di **[Buy Me a Coffee](https://buymeacoffee.com/fmarzuki)** ☕
