# Pointer

Aplikasi overlay lintas platform (Linux dan Windows) untuk menampilkan efek **laser pointer** saat presentasi atau screen sharing. Efek digambar di overlay transparan di atas layar, jadi ikut terlihat oleh audiens saat layar dibagikan lewat Zoom/Meet/Teams/OBS.

## Demo

[![Demo Pointer: laser pointer overlay untuk presentasi dan screen sharing](https://img.youtube.com/vi/HNEtq7MrwPQ/maxresdefault.jpg)](https://www.youtube.com/watch?v=HNEtq7MrwPQ)

▶️ Tonton di YouTube: https://www.youtube.com/watch?v=HNEtq7MrwPQ

> **Status:** versi 0.4.0 sudah dirilis di [GitHub Releases](https://github.com/fmarzuki/pointer/releases) (`.deb`, AppImage, `.exe`, `.zip`, `.msi`), dibangun dan diuji otomatis oleh GitHub Actions di Linux dan Windows. Fitur diuji langsung di GNOME Wayland; build Windows lolos test di CI tetapi **belum diuji langsung di Windows**.

## Fitur

| Fitur | Status |
|---|---|
| **Laser pointer**: titik bercahaya dengan trail yang memudar | ✅ Fase 1 |
| **Spotlight**: layar digelapkan kecuali lingkaran lembut di sekitar kursor | ✅ Fase 2 |
| **Highlight klik**: cincin membesar dan memudar saat mouse diklik (selama laser/spotlight aktif) | ✅ Fase 2 |
| **Anotasi**: coretan pena/stabilo dan kotak sudut membulat, dengan palet warna, undo, dan clear | ✅ Fase 3 |

### Menu radial

Tekan **`Ctrl+Alt+Space`** (atau `pointer menu`) untuk memunculkan menu radial di posisi kursor. Menu berupa cincin 6 sektor dengan tombol **Off** di tengah. Semua teks aplikasi (menu, tray, CLI, pesan error) berbahasa Inggris.

| Sektor | Klik | Keterangan kecil |
|---|---|---|
| **Laser** (atas) | Nyalakan/matikan laser | `ON` saat aktif |
| **Draw** (kanan atas) | Masuk mode anotasi; bila sudah menggambar, tukar pena ↔ stabilo | `PEN` / `MARKER` |
| **Color** (kanan bawah) | Warna berikutnya (menu tetap terbuka); arahkan kursor untuk memunculkan kipas warna, klik salah satu untuk memilih. Warna ini juga dipakai laser (lihat `laser.follow_color`) | Nama warna (`RED`, `YELLOW`, ...) |
| **Shape** (bawah) | Arahkan kursor untuk memunculkan flyout bentuk (Box, Arrow, Ellipse, Line, Text), klik salah satu. Klik sektornya langsung: bentuk terakhir, atau bentuk berikutnya bila sudah memakai bentuk | `BOX` / `ARROW` / `ELLIPSE` / `LINE` / `TEXT` |
| **Erase** (kiri bawah) | Hapus semua coretan | `ALL` |
| **Spotlight** (kiri atas) | Nyalakan/matikan spotlight | `ON` saat aktif |
| **Off** (tengah) | Matikan semua efek | |

Klik di luar menu atau tekan `Ctrl+Alt+Space` lagi untuk menutup tanpa memilih. Bila posisi kursor belum diketahui (overlay baru muncul dan mouse belum bergerak), menu tampil di tengah layar. Label memakai font Noto Sans Bold yang ditanam di binary (SIL OFL 1.1, lihat `assets/fonts/OFL.txt`).

### Daftar shortcut

Daftar semua hotkey (sesuai config) dan aksi mouse bisa dilihat lewat:

- tombol **?** kecil di kanan atas menu radial
- menu tray **Shortcuts**
- `pointer shortcuts` (cetak di terminal) atau `pointer shortcuts --show` (tampilkan panel di layar)

Panel tampil di tengah monitor tempat kursor berada; klik di mana saja untuk menutupnya. Klik baris *Support it* di bagian bawah panel membuka halaman dukungan di browser.

### Hotkey default

| Hotkey | Fungsi |
|---|---|
| `Ctrl+Alt+Space` | Buka/tutup menu radial |
| `Ctrl+Alt+L` | Laser on/off |
| `Ctrl+Alt+S` | Spotlight on/off |
| `Ctrl+Alt+P` | Anotasi (pena) on/off |
| `Ctrl+Alt+C` | Hapus semua coretan |
| `Ctrl+Alt+Z` | Batalkan coretan terakhir |

Menekan hotkey mode yang sedang aktif mematikannya; menekan hotkey mode lain langsung berpindah mode. `Ctrl+Alt+D` tidak dipakai karena bawaan GNOME untuk *show desktop*.

### Mouse di mode anotasi

| Tombol | Fungsi |
|---|---|
| Kiri (tahan + geser) | Mencoret, atau membentuk kotak/panah/elips/garis saat alat bentuk aktif |
| Kiri di atas garis objek (tahan + geser) | **Geser objek** yang sudah dibuat |
| Kanan | Warna berikutnya di palet |
| Kanan di atas garis objek | **Hapus objek** itu saja |
| Tengah | Ganti alat: pena → stabilo → kotak → panah → elips → garis → teks → pena |

Alat dan warna yang dipilih diingat selama daemon berjalan (juga saat mode berganti) dan sama di semua monitor.

**Teks:** pilih *Text* di flyout Shape (atau klik tengah sampai alat teks aktif; kursor berubah menjadi ikon teks), lalu klik di tempat teks akan ditaruh. Kotak input kecil muncul di bawah titik itu dan langsung menerima ketikan; teks tampil langsung di layar dengan warna aktif. **Enter** menaruh teks, **Shift+Enter** membuat baris baru, **Esc** membatalkan, dan klik di tempat lain juga menaruh teks yang sedang diketik. Overlay sendiri tidak pernah menerima keyboard, jadi kotak input ini window terpisah yang mengambil fokus selama mengetik lalu menutup diri; fokus kembali ke aplikasi sebelumnya (mis. slideshow). Objek teks bisa digeser, dihapus dengan klik kanan, dan di-undo seperti objek lain. Ukuran huruf diatur dengan `draw.text_size`.

**Menggeser objek:** arahkan kursor ke garis objek (coretan, kotak, elips, panah, atau garis). Kursor berubah menjadi ikon *move* dan objek diberi kotak seleksi putus-putus hijau. Tahan klik kiri lalu tarik untuk memindahkannya. Hanya garisnya yang bisa dipegang, bukan bagian dalam kotak/elips, jadi Anda tetap bisa menggambar di dalam bentuk. Klik kanan saat objek disorot menghapus objek itu saja (tanpa menghapus coretan lain). Undo (`Ctrl+Alt+Z`) membatalkan geseran atau penghapusan dan mengembalikan objek ke posisi dan urutan tumpukan semula.

Lingkaran kecil di kursor menunjukkan warna dan ukuran alat yang aktif. Coretan terhapus saat keluar dari mode anotasi. Tombol yang dipakai slideshow (`F5`, panah, PageUp/PageDown, `Esc`) sengaja tidak dipakai. Overlay juga tidak pernah menerima keyboard, jadi mode dimatikan dengan hotkey yang sama.

## Lisensi

Pointer dirilis di bawah **GNU General Public License v3.0**, lihat [`LICENSE`](LICENSE). Font Noto Sans Bold yang ditanam di binary memakai SIL Open Font License 1.1 (`assets/fonts/OFL.txt`).

## Dukung proyek ini

Kalau Pointer bermanfaat, Anda bisa mendukung pengembangannya lewat **[Buy Me a Coffee](https://buymeacoffee.com/fmarzuki)**. Link yang sama ada di menu tray, di panel Shortcuts, dan di `pointer --help`.

## Instalasi dari paket

Unduh dari halaman GitHub Release (setelah workflow rilis dijalankan):

| Paket | Platform | Cara pasang |
|---|---|---|
| `pointer_<versi>-1_amd64.deb` | Ubuntu 24.04+ / Debian 13+ | `sudo apt install ./pointer_<versi>-1_amd64.deb` |
| `Pointer-<versi>-x86_64.AppImage` | Linux x86_64 lain | `chmod +x Pointer-*.AppImage && ./Pointer-*.AppImage install && ./Pointer-*.AppImage` |
| `pointer-<versi>-x64.msi` | Windows 10/11 | Jalankan installer; shortcut "Pointer" ada di Start menu |
| `pointer-<versi>-windows-x64.zip` | Windows 10/11 (portable) | Ekstrak, jalankan `pointer.exe` |

- Paket `.deb` sudah memasang file `.desktop` dan ikon ke `/usr/share`, jadi `pointer install` **tidak** perlu dijalankan.
- AppImage butuh FUSE 2 (`sudo apt install libfuse2t64` di Ubuntu 24.04+). Library X11 diambil dari sistem, tidak dibundel. `install` dan `autostart on` dari AppImage memakai path file AppImage itu sendiri (`$APPIMAGE`), jadi jangan pindahkan file setelahnya.
- Paket Linux dibangun di Ubuntu 24.04 (glibc 2.39). Paket yang dibangun di mesin dengan glibc lebih baru (mis. Ubuntu 26.04) tidak bisa dipasang di 24.04.

## Membuat rilis

```bash
# 1. Samakan versi di Cargo.toml, mis. version = "0.4.0"
# 2. Push tag yang sama ke GitHub
git tag v0.4.0
git push origin v0.4.0
```

Workflow `.github/workflows/release.yml` lalu menjalankan test di Linux dan Windows, membangun `.deb` (`cargo-deb`), AppImage (`appimagetool` 1.9.1, checksum diverifikasi), `.zip`, dan `.msi` (WiX 5.0.2), kemudian membuat GitHub Release berisi semua paket. Workflow `ci.yml` menjalankan test + build release di Linux dan Windows pada setiap push ke `main` dan pull request.

Membangun `.deb` dan `.exe` sekaligus lewat Docker (tanpa sudo; Debian 12 + mingw-w64):

```bash
scripts/build-packages.sh   # -> dist/pointer_<versi>-1_amd64.deb, dist/pointer-<versi>-windows-x64.exe
```

`.deb` hasil skrip ini butuh glibc ≥ 2.35 (Ubuntu 22.04+ / Debian 12+). `.exe` adalah binary portable tunggal yang hanya bergantung pada DLL sistem Windows (target `x86_64-pc-windows-gnu`; CI memakai target MSVC).

Membangun paket secara lokal tanpa Docker (Linux, glibc mengikuti host):

```bash
cargo install cargo-deb --locked
cargo deb                                    # -> target/debian/pointer_<versi>-1_amd64.deb
APPIMAGETOOL=/path/appimagetool-x86_64.AppImage scripts/build-appimage.sh   # -> dist/
```

## Build dari source (Linux)

### Prasyarat

- Rust toolchain via [rustup](https://rustup.rs). Kalau `~/.cargo/bin` belum ada di PATH, jalankan `source ~/.cargo/env` dulu.
- `build-essential`
- Library runtime X11: `libx11-6`, `libxcb1`, `libxi6`, `libxcursor1`, `libxrandr2`, `libxkbcommon-x11-0`

```bash
sudo apt install build-essential libxkbcommon-x11-0
```

`pkg-config` dan paket `-dev` X11 **tidak** diperlukan, karena `softbuffer` memakai fitur `x11-dlopen` (libX11 dimuat saat runtime).

### Build dan jalankan

```bash
cargo build --release
./target/release/pointer install   # sekali saja, khusus GNOME Wayland (lihat di bawah)
./target/release/pointer run       # jalankan daemon
```

Di GNOME Wayland, hotkey memakai portal `GlobalShortcuts`. GNOME hanya mengizinkannya bila app id `io.github.fmarzuki.Pointer` punya file `.desktop`. `pointer install` menulis file itu ke `~/.local/share/applications/`. Saat daemon pertama kali jalan (dan setiap kali ada shortcut baru), GNOME menampilkan dialog persetujuan shortcut.

### Perintah

| Perintah | Fungsi |
|---|---|
| `pointer` / `pointer run` | Jalankan daemon (ditolak bila sudah ada yang berjalan) |
| `pointer laser` | Nyalakan/matikan laser |
| `pointer spotlight` | Nyalakan/matikan spotlight |
| `pointer menu` | Buka/tutup menu radial |
| `pointer shortcuts [--show]` | Cetak daftar shortcut; `--show` menampilkannya sebagai panel di layar |
| `pointer draw` | Nyalakan/matikan anotasi |
| `pointer clear` / `pointer undo` | Hapus semua coretan / batalkan coretan terakhir |
| `pointer off` | Matikan semua efek |
| `pointer status` | Tampilkan mode saat ini (`ok idle` / `ok laser` / `ok spotlight` / `ok draw`) |
| `pointer quit` | Hentikan daemon |
| `pointer config` | Tulis config default (bila belum ada) dan tampilkan lokasinya |
| `pointer install` | (Linux) Pasang file `.desktop` untuk hotkey portal GNOME |
| `pointer autostart on\|off\|status` | Jalankan daemon otomatis saat login |

### Ikon tray

Selama daemon berjalan, ikon titik laser muncul di panel (menyala saat ada mode aktif). Menunya:

- **Off / Laser / Spotlight / Annotate**: pilih mode (ditandai mode yang aktif)
- **Undo last annotation / Clear all annotations**: aktif hanya di mode anotasi
- **Start at login**: sama dengan `pointer autostart on/off`
- **Shortcuts**: tampilkan panel daftar shortcut
- **Support Pointer (Buy Me a Coffee)**: buka halaman dukungan di browser
- **Open config**: membuat config default bila belum ada, lalu membukanya (`xdg-open` / Notepad)
- **Quit**: hentikan daemon

Di GNOME, tray membutuhkan extension AppIndicator (`ubuntu-appindicators`, aktif bawaan di Ubuntu). Tanpa extension itu daemon tetap jalan, hanya tanpa ikon.

Autostart menulis `~/.config/autostart/io.github.fmarzuki.Pointer.desktop` (Linux) atau nilai `Pointer` di `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (Windows). Path binary yang dipakai adalah lokasi binary saat autostart diaktifkan.

Perintah dikirim ke daemon lewat unix socket `$XDG_RUNTIME_DIR/pointer.sock` (Windows: named pipe `pointer-<USERNAME>`). Kalau hotkey portal tidak tersedia, pasang `pointer laser` sebagai custom shortcut di *Settings → Keyboard → Custom Shortcuts*.

### Config

Lokasi: `~/.config/pointer/config.toml` (Windows: `%APPDATA%\pointer\config.toml`). Semua field opsional. Config yang tidak valid membuat daemon berhenti dengan pesan error.

```toml
hide_cursor = true        # sembunyikan kursor sistem saat laser aktif (Linux)

[laser]
follow_color = true       # warna laser = warna aktif (menu Color / klik kanan)
color = "#ff2020"         # dipakai bila follow_color = false
radius = 6.0              # piksel logis, 1–100 (dikali skala monitor)
trail_ms = 450            # lama trail memudar, 50–5000

[spotlight]
radius = 140.0            # jari-jari lingkaran terang, 20–1000
dim = 0.6                 # opasitas area gelap, 0.1–0.95

[click]
enabled = true            # cincin saat klik (selama laser/spotlight aktif)
color = "#ffc400"
radius = 28.0             # jari-jari akhir cincin, 5–200
duration_ms = 500         # 100–3000

[draw]
colors = ["#ff2020", "#ffd400", "#20c040", "#2080ff", "#ffffff"]  # 1–9 warna
pen_width = 4.0           # 1–50
highlighter_width = 18.0  # 4–100
highlighter_alpha = 0.35  # 0.1–0.9
rect_radius = 12.0        # jari-jari sudut kotak, 0–100 (0 = sudut tajam)
text_size = 32.0          # ukuran huruf objek teks, 8–200

[hotkeys]                 # modifier: Ctrl, Alt, Shift, Super; semua harus berbeda
menu = "Ctrl+Alt+Space"
laser = "Ctrl+Alt+L"
spotlight = "Ctrl+Alt+S"
draw = "Ctrl+Alt+P"
clear = "Ctrl+Alt+C"
undo = "Ctrl+Alt+Z"
```

Set `POINTER_DEBUG=1` saat menjalankan daemon untuk mencetak statistik render (jumlah frame dan waktu per frame) setiap kali mode berganti.

## Tech stack

**Rust**, dengan crate utama:

| Kebutuhan | Crate |
|---|---|
| Window & event | `winit` 0.30 (backend X11 dipaksa di Linux, berjalan via XWayland) |
| Gambar 2D anti-alias | `tiny-skia` |
| Menampilkan buffer | `softbuffer` (Linux), `UpdateLayeredWindow` via `windows-sys` (Windows) |
| Hotkey global | `ashpd` (portal GlobalShortcuts, Wayland), `global-hotkey` (Windows/X11) |
| Config, IPC, CLI | `serde` + `toml`, `interprocess`, `clap` |
| Tray | `ksni` (Linux, StatusNotifierItem lewat D-Bus, tanpa GTK), `tray-icon` (Windows) |

Rust dipilih karena bagian yang sulit (overlay click-through, hotkey global, tray, akses API native) sudah tersedia sebagai crate matang. Tidak perlu cgo seperti di Go, dan tidak perlu webview seperti di Tauri/Electron.

## Cara kerja

- **Mode:** *Idle* tidak punya window sama sekali (CPU 0). *Laser*, *Spotlight*, dan *Draw* membuat satu overlay per monitor. Overlay dibuat ulang setiap kali diaktifkan, jadi perubahan monitor langsung terbawa.
- **Linux (X11/XWayland):** window override-redirect yang transparan. Window ini tetap di atas slideshow fullscreen dan tidak pernah menerima fokus keyboard. Overlay menangkap pointer untuk membaca posisinya, karena di Wayland posisi kursor global tidak tersedia untuk aplikasi biasa.
- **Windows:** layered window click-through (`WS_EX_LAYERED | TRANSPARENT | NOACTIVATE | TOOLWINDOW`). Posisi kursor dan tombol kiri di-poll ~60 Hz (`GetCursorPos`, `GetAsyncKeyState`) selama overlay aktif.
- **Anotasi:** coretan selesai digambar ke layer terpisah; layer itu hanya dirender ulang penuh saat undo/clear. Coretan yang sedang dibuat digambar utuh sebagai satu path yang dihaluskan dengan kurva kuadratik, jadi stabilo semi-transparan tidak menumpuk di sambungan. Alat kotak menyimpan titik awal drag dan sudut berlawanan yang mengikuti kursor; kotak dengan lebar atau tinggi di bawah 4 px dianggap klik tak sengaja dan dibuang. Di Windows, mode Draw mematikan click-through dan memakai latar alpha 1/255 (tidak terlihat) agar drag tertangkap.
- **Render:** hanya saat ada gerakan atau animasi (trail, cincin klik) sedang berjalan, maksimal ~60 fps. Hanya area yang berubah (*damage rect*) yang dibersihkan, dikonversi, dan dikirim ke compositor.

### Dukungan platform

| | Overlay | Posisi pointer | Hotkey global | Status |
|---|---|---|---|---|
| GNOME Wayland | XWayland override-redirect, menangkap pointer | Event pointer overlay | Portal GlobalShortcuts | ✅ Diuji |
| Linux X11 | Sama seperti di atas | Event pointer overlay | `XGrabKey` (`global-hotkey`) | ⚠️ Belum diuji |
| Windows 10/11 | Layered window click-through | Polling `GetCursorPos` | `RegisterHotKey` (`global-hotkey`) | ⚠️ Hanya `cargo check` |

### Batasan yang diketahui

- Di Windows, binary memakai subsystem GUI agar tidak ada jendela konsol saat autostart. Output perintah CLI tetap tampil di cmd/PowerShell (lewat `AttachConsole`), tetapi muncul setelah prompt berikutnya.

- Overlay hanya terlihat bila yang dibagikan adalah **seluruh layar/monitor**, bukan "share window" tunggal.
- Di Linux, selama laser/spotlight aktif, **klik tidak tembus** ke aplikasi di bawahnya (highlight klik hanya visual). Navigasi slide tetap bisa lewat keyboard atau remote clicker. Di Windows, klik tetap tembus.
- Di mode spotlight dan draw, kursor sistem tetap terlihat; `hide_cursor` hanya berlaku untuk laser.
- Selama mode anotasi aktif, klik tidak tembus di semua platform. Navigasi slide tetap lewat keyboard/clicker; coretan tetap ada setelah slide berganti sampai di-clear.

## Hasil pengujian

Diuji di Ubuntu 26.04, GNOME 50 (Wayland), satu monitor 2560x1440.

| Uji | Hasil |
|---|---|
| Overlay tampil di atas slideshow fullscreen (spike) | ✅ Lolos dengan override-redirect |
| PageDown/panah tetap memindah slide saat overlay aktif (spike) | ✅ Lolos |
| Efek terlihat saat share layar penuh (spike) | ✅ Lolos |
| Hotkey `Ctrl+Alt+L` via portal GlobalShortcuts | ✅ Lolos |
| Laser + trail di aplikasi utama | ✅ Lolos |
| CPU saat Idle / laser aktif tapi mouse diam | ✅ ~0 |
| Waktu render per frame saat mouse bergerak | ✅ rata-rata 1,6 ms (187 frame, uji nyata) |
| IPC CLI dan penolakan instance kedua | ✅ Lolos |
| Spotlight via IPC (Fase 2) | ✅ Jalan; rata-rata 4,1 ms/frame (135 frame, termasuk frame gelap penuh pertama) |
| Hotkey `Ctrl+Alt+S`, tampilan spotlight, cincin klik (Fase 2) | ✅ Lolos |
| Mode draw, undo, clear via IPC (Fase 3) | ✅ Jalan |
| Mencoret, ganti warna/alat, hotkey `Ctrl+Alt+P/C/Z` (Fase 3) | ⏳ Menunggu uji visual |
| Tray terdaftar di StatusNotifierWatcher, tooltip mengikuti mode (Fase 4) | ✅ Lolos (dicek via D-Bus) |
| `pointer autostart on/status/off` (Fase 4) | ✅ Lolos |
| Menu tray diklik langsung (Fase 4) | ⏳ Menunggu uji visual |
| Paket `.deb` dibangun dan diperiksa isinya (Fase 5) | ✅ Lolos (1,6 MB; binary 5 MB dengan LTO + strip) |
| Skrip AppImage dengan `appimagetool` tiruan (Fase 5) | ✅ Struktur AppDir dan `AppRun` benar |
| Workflow GitHub Actions, AppImage asli, `.zip`, `.msi` (Fase 5) | ⏳ Belum dijalankan (butuh repo GitHub) |
| Menu radial dibuka/ditutup via IPC dari Idle dan dari mode laser (GUI) | ✅ Jalan |
| Klik sektor, hover, kipas warna, panah/elips di layar nyata (GUI) | ⏳ Menunggu uji visual |
| Unit test (`cargo test`) | ✅ 44 lolos (termasuk hit-test menu dan batas gambar menu) |
| Windows (`cargo check --target x86_64-pc-windows-msvc`) | ✅ Lolos type-check |
| Windows nyata (PowerPoint + Teams/Zoom, DPI 125%/150%) | ⏳ Belum diuji |

Benchmark render terberat (mouse bergerak cepat tanpa henti, layar 2560x1440): `cargo test --release -- --ignored --nocapture render_cost`, hasilnya ~2,7 ms/frame.

## Struktur proyek

```
pointer/
├── .github/workflows/    # ci.yml (test + build), release.yml (paket + GitHub Release)
├── packaging/
│   ├── docker/           # image build Debian 12 + mingw-w64
│   ├── linux/            # .desktop + ikon SVG (dipakai .deb, AppImage, dan `pointer install`)
│   └── windows/          # pointer.wxs (installer MSI, WiX 5)
├── scripts/
│   ├── build-appimage.sh
│   └── build-packages.sh # .deb + .exe via Docker (packaging/docker/Dockerfile)
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs           # CLI (clap) + start daemon
│   ├── app.rs            # event loop winit, state machine Idle/Laser/Spotlight/Draw, frame pacing
│   ├── command.rs        # perintah daemon (laser/spotlight/draw/clear/undo/off/status/quit)
│   ├── autostart.rs      # XDG autostart (Linux), registry Run (Windows)
│   ├── config.rs         # config TOML + validasi
│   ├── ipc.rs            # unix socket / named pipe
│   ├── platform.rs       # deteksi Wayland, file .desktop
│   ├── hotkey/           # portal GlobalShortcuts (Wayland), global-hotkey (Windows/X11)
│   ├── overlay/          # overlay per monitor, damage rect; linux.rs, windows.rs
│   ├── render/           # tiny-skia; laser.rs, spotlight.rs, ripple.rs (cincin klik), ink.rs (anotasi),
│   │                     # menu.rs (menu radial), text.rs (teks dari kontur font)
│   └── tray/             # menu bersama + ikon; linux.rs (ksni), windows.rs (tray-icon)
├── assets/fonts/         # Noto Sans Bold + lisensi OFL (label menu radial)
└── spike/                # prototipe Fase 0 (tidak dipakai aplikasi utama)
```

## Roadmap

1. ~~**Fase 0**: toolchain + spike kelayakan di GNOME Wayland~~ ✅ (spike Windows belum dilakukan)
2. ~~**Fase 1**: overlay per monitor, mode laser, hotkey, config TOML~~ ✅ di Linux
3. ~~**Fase 2**: spotlight + highlight klik~~ ✅
4. ~~**Fase 3**: anotasi (pena/stabilo, warna, undo, clear)~~ ✅ (menunggu uji visual; auto-fade coretan belum dibuat)
5. ~~**Fase 4**: tray icon, autostart~~ ✅ (IPC CLI sudah dibuat di Fase 1; menu tray menunggu uji visual)
6. ~~**Fase 5**: packaging (AppImage, `.deb`, `.exe`/MSI) + CI GitHub Actions untuk Linux dan Windows~~ ✅ (`.deb` diuji lokal; sisanya menunggu run pertama di GitHub)
