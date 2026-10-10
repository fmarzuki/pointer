// Windows: tanpa jendela konsol saat dijalankan dari autostart/Explorer. Output CLI tetap
// muncul bila dijalankan dari terminal (lihat `attach_parent_console`).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod autostart;
mod command;
mod config;
mod hotkey;
mod ipc;
mod overlay;
mod platform;
mod render;
mod shortcuts;
mod textinput;
mod tray;

use std::process::ExitCode;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use winit::event_loop::EventLoop;

use crate::app::{App, AppEvent};
use crate::command::Command;
use crate::config::Config;

#[derive(Parser)]
#[command(
    name = "pointer",
    version,
    about = "Laser pointer overlay for presentations and screen sharing",
    after_help = "Support Pointer: https://buymeacoffee.com/fmarzuki"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the overlay daemon (default when no subcommand is given)
    Run,
    /// Toggle the laser on the running daemon
    Laser,
    /// Toggle the spotlight on the running daemon
    Spotlight,
    /// Toggle annotation mode (pen/highlighter/shapes)
    Draw,
    /// Open/close the radial menu
    Menu,
    /// Print the available shortcuts (--show: display them on screen)
    Shortcuts {
        /// Show the shortcuts panel on screen (daemon must be running)
        #[arg(long)]
        show: bool,
    },
    /// Clear all annotations
    Clear,
    /// Undo the last annotation
    Undo,
    /// Turn off all effects (back to idle)
    Off,
    /// Show the daemon's current mode
    Status,
    /// Stop the daemon
    Quit,
    /// Write the default config (if missing) and print its path
    Config,
    /// Configure the daemon to start automatically at login
    Autostart {
        #[arg(value_enum)]
        state: AutostartState,
    },
    /// Install the .desktop file and icon (Linux) so GNOME portal hotkeys work
    #[cfg(target_os = "linux")]
    Install,
}

#[derive(Clone, Copy, ValueEnum)]
enum AutostartState {
    On,
    Off,
    Status,
}

fn main() -> ExitCode {
    #[cfg(windows)]
    attach_parent_console();
    let cli = Cli::parse();
    match cli.cmd.unwrap_or(Cmd::Run) {
        Cmd::Run => run_daemon(),
        Cmd::Laser => client(Command::ToggleLaser),
        Cmd::Spotlight => client(Command::ToggleSpotlight),
        Cmd::Draw => client(Command::ToggleDraw),
        Cmd::Menu => client(Command::Menu),
        Cmd::Shortcuts { show: true } => client(Command::Shortcuts),
        Cmd::Shortcuts { show: false } => print_shortcuts(),
        Cmd::Clear => client(Command::Clear),
        Cmd::Undo => client(Command::Undo),
        Cmd::Off => client(Command::Off),
        Cmd::Status => client(Command::Status),
        Cmd::Quit => client(Command::Quit),
        Cmd::Config => write_default_config(),
        Cmd::Autostart { state } => set_autostart(state),
        #[cfg(target_os = "linux")]
        Cmd::Install => match platform::install_desktop_file() {
            Ok(path) => {
                println!("{}", path.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("failed to install .desktop file: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

fn client(cmd: Command) -> ExitCode {
    match ipc::send(cmd) {
        Ok(reply) => {
            println!("{reply}");
            if reply.starts_with("ok") { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        }
        Err(e) => {
            eprintln!("pointer is not running ({e}). Start it first: pointer run");
            ExitCode::FAILURE
        }
    }
}

/// Binary Windows memakai subsystem GUI; tempelkan ke konsol pemanggil (cmd/PowerShell)
/// agar `println!` dari perintah CLI tetap terlihat.
#[cfg(windows)]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    // SAFETY: tanpa pointer; gagal (mis. dijalankan dari Explorer) tidak masalah.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn print_shortcuts() -> ExitCode {
    // Pakai hotkey dari config bila valid; selain itu default.
    let cfg = Config::load().unwrap_or_default();
    print!("{}", shortcuts::to_text(&shortcuts::sections(&cfg.hotkeys)));
    ExitCode::SUCCESS
}

fn set_autostart(state: AutostartState) -> ExitCode {
    let result = match state {
        AutostartState::On => autostart::set(true),
        AutostartState::Off => autostart::set(false),
        AutostartState::Status => Ok(()),
    };
    match result {
        Ok(()) => {
            println!("autostart {}", if autostart::is_enabled() { "on" } else { "off" });
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn write_default_config() -> ExitCode {
    match Config::ensure_file() {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run_daemon() -> ExitCode {
    if let Ok(reply) = ipc::send(Command::Status) {
        eprintln!("pointer is already running ({reply})");
        return ExitCode::FAILURE;
    }
    let cfg = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("invalid config: {e}");
            return ExitCode::from(2);
        }
    };

    let mut builder = EventLoop::<AppEvent>::with_user_event();
    #[cfg(target_os = "linux")]
    {
        // Selalu X11 (XWayland di sesi Wayland): override-redirect hanya ada di X11.
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_x11();
    }
    let event_loop = match builder.build() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("failed to create event loop: {e}");
            return ExitCode::FAILURE;
        }
    };

    let proxy = Mutex::new(event_loop.create_proxy());
    let send = move |cmd: Command, reply: Option<mpsc::SyncSender<String>>| {
        proxy
            .lock()
            .map(|p| p.send_event(AppEvent { cmd, reply }).is_ok())
            .unwrap_or(false)
    };
    let send = Arc::new(send);

    let ipc_send = send.clone();
    if let Err(e) = ipc::serve(move |cmd| {
        let (tx, rx) = mpsc::sync_channel(1);
        if !ipc_send(cmd, Some(tx)) {
            return "err daemon is shutting down".into();
        }
        rx.recv_timeout(Duration::from_secs(2))
            .unwrap_or_else(|_| "err timeout".into())
    }) {
        eprintln!("[ipc] failed to open socket: {e}");
    }

    let hotkey_send = send.clone();
    let _hotkeys = hotkey::start(
        &cfg.hotkeys,
        Arc::new(move |cmd| {
            hotkey_send(cmd, None);
        }),
    );

    let tray_send = send.clone();
    let tray = tray::start(Arc::new(move |cmd| {
        tray_send(cmd, None);
    }));

    let keys: Vec<String> = cfg
        .hotkeys
        .entries()
        .iter()
        .map(|(name, key)| format!("{name} {key}"))
        .collect();
    eprintln!(
        "[pointer] ready ({}). Hotkeys: {}. CLI: pointer --help",
        platform::session_name(),
        keys.join(", ")
    );
    let mut app = App::new(cfg, tray);
    let result = event_loop.run_app(&mut app);
    ipc::cleanup();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("event loop stopped with error: {e}");
            ExitCode::FAILURE
        }
    }
}
