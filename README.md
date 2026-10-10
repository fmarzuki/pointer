# Pointer

**English** | [Bahasa Indonesia](README.id.md)

A laser pointer overlay for presentations and screen sharing on Linux and Windows. Effects are drawn on a transparent overlay above everything, so your audience sees them when you share your full screen in Zoom, Meet, Teams, or OBS.

I built it because I couldn't find a good laser pointer tool for presenting on Linux.

## Demo

[![Pointer demo](https://img.youtube.com/vi/HNEtq7MrwPQ/maxresdefault.jpg)](https://www.youtube.com/watch?v=HNEtq7MrwPQ)

## Features

- **Laser pointer** with a fading trail, in the active color
- **Spotlight**: dims the screen except around the cursor
- **Click highlight**: a ring appears where you click
- **Annotations**: pen, marker, box, arrow, ellipse, line, and text
- **Edit objects**: move, delete, and undo individual annotations
- **Radial menu** on one hotkey, plus a tray icon and a shortcuts panel
- Small and fast: a single ~5 MB binary written in Rust, ~0% CPU when idle

## Install

Download the latest version from [Releases](https://github.com/fmarzuki/pointer/releases).

| Platform | File | How to install |
|---|---|---|
| Ubuntu 24.04+ / Debian 13+ | `pointer_<version>-1_amd64.deb` | `sudo apt install ./pointer_*.deb` |
| Other Linux | `Pointer-<version>-x86_64.AppImage` | `chmod +x Pointer-*.AppImage`, then `./Pointer-*.AppImage install` once and run it |
| Windows 10/11 | `pointer-<version>-x64.msi` | Run the installer |
| Windows (portable) | `pointer-<version>-windows-x64.exe` | Just run it |

Then start it with `pointer` (or from your app menu). On GNOME, approve the shortcut dialog the first time it starts.

> Linux is tested on GNOME Wayland (Ubuntu). Windows builds pass CI but are still **beta**: not yet tested on a real Windows machine. Windows may show a SmartScreen warning because the binary isn't signed; choose *More info → Run anyway*.

## Usage

Press **`Ctrl+Alt+Space`** to open the radial menu at the cursor:

| Sector | Action |
|---|---|
| **Laser** | Toggle the laser pointer |
| **Spotlight** | Toggle the spotlight |
| **Draw** | Draw freehand; click again to switch pen ↔ marker |
| **Shape** | Hover to pick Box, Arrow, Ellipse, Line, or Text |
| **Color** | Hover to pick a color (also used by the laser) |
| **Erase** | Clear all annotations |
| **Off** (center) | Turn everything off |
| **?** | Show all shortcuts |

### Hotkeys

| Hotkey | Action |
|---|---|
| `Ctrl+Alt+Space` | Radial menu |
| `Ctrl+Alt+L` | Laser |
| `Ctrl+Alt+S` | Spotlight |
| `Ctrl+Alt+P` | Draw |
| `Ctrl+Alt+C` | Clear all |
| `Ctrl+Alt+Z` | Undo |

### Mouse in draw mode

| Mouse | Action |
|---|---|
| Left drag | Draw, or create the selected shape |
| Left drag on an object | Move it |
| Right click on an object | Delete it |
| Right click | Next color |
| Middle click | Next tool |
| Click with the Text tool | Type text; `Enter` adds it, `Shift+Enter` adds a new line, `Esc` cancels |

Your slideshow keys (arrows, PageUp/PageDown, clickers) keep working while Pointer is active.

### Command line

```bash
pointer                      # start Pointer
pointer laser | spotlight | draw | menu | off
pointer clear | undo
pointer shortcuts [--show]   # list shortcuts (or show them on screen)
pointer autostart on|off     # start at login
pointer config               # create and print the config file path
pointer quit
```

## Configuration

The config file is `~/.config/pointer/config.toml` (Windows: `%APPDATA%\pointer\config.toml`). Run `pointer config` to create it. Every setting is optional:

```toml
[laser]
radius = 6.0
trail_ms = 450
follow_color = true        # false = always use laser.color

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

Restart Pointer (`pointer quit`, then `pointer`) after editing.

## Notes

- Share your **full screen or a monitor**. When you share a single window, the overlay doesn't appear, because screen sharing only captures that one window.
- On GNOME, the tray icon needs the AppIndicator extension (enabled by default on Ubuntu).
- On Linux, clicks don't pass through to apps below while an effect is on. Use the keyboard or a clicker to change slides.

## Build from source

Requires [Rust](https://rustup.rs). On Linux you also need `build-essential` and `libxkbcommon-x11-0`.

```bash
cargo build --release
./target/release/pointer install   # Linux only, once: registers the app for GNOME hotkeys
./target/release/pointer
```

To publish a release, set `version` in `Cargo.toml` and push a matching tag (for example `git tag v0.4.0 && git push origin v0.4.0`). GitHub Actions builds every package and creates the release.

## License

[GPL-3.0](LICENSE). The bundled Noto Sans font uses the [SIL Open Font License](assets/fonts/OFL.txt).

## Support

If Pointer helps you, you can support it on **[Buy Me a Coffee](https://buymeacoffee.com/fmarzuki)** ☕
