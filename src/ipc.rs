//! IPC lokal antara CLI (`pointer laser`, dst.) dan daemon.
//!
//! Linux: unix socket di `$XDG_RUNTIME_DIR/pointer.sock` (direktori 0700 milik user).
//! Windows: named pipe `\\.\pipe\pointer-<USERNAME>`.
//! Protokol: satu baris perintah, dibalas satu baris (`ok ...` / `err ...`).

use std::io::{self, BufRead, BufReader, Write};

use interprocess::local_socket::{prelude::*, ListenerOptions, Name, Stream};

use crate::command::Command;

fn socket_name() -> io::Result<Name<'static>> {
    #[cfg(unix)]
    {
        use interprocess::local_socket::GenericFilePath;
        socket_path().into_os_string().to_fs_name::<GenericFilePath>()
    }
    #[cfg(windows)]
    {
        use interprocess::local_socket::GenericNamespaced;
        let user = std::env::var("USERNAME").unwrap_or_default();
        format!("pointer-{user}").to_ns_name::<GenericNamespaced>()
    }
}

#[cfg(unix)]
fn socket_path() -> std::path::PathBuf {
    let user = std::env::var("USER").unwrap_or_default();
    match dirs::runtime_dir() {
        Some(d) => d.join("pointer.sock"),
        None => std::env::temp_dir().join(format!("pointer-{user}.sock")),
    }
}

/// Kirim perintah ke daemon dan kembalikan balasannya.
pub fn send(cmd: Command) -> io::Result<String> {
    let mut conn = BufReader::new(Stream::connect(socket_name()?)?);
    conn.get_mut().write_all(format!("{cmd}\n").as_bytes())?;
    let mut reply = String::new();
    conn.read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}

/// Jalankan server IPC di thread terpisah. `handle` dipanggil untuk tiap perintah
/// dan mengembalikan teks balasan.
pub fn serve<F>(handle: F) -> io::Result<()>
where
    F: Fn(Command) -> String + Send + 'static,
{
    let listener = match ListenerOptions::new().name(socket_name()?).create_sync() {
        Ok(l) => l,
        // Socket sisa proses yang crash: hapus lalu coba lagi. Pemanggil sudah
        // memastikan tidak ada daemon lain yang menjawab.
        #[cfg(unix)]
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            std::fs::remove_file(socket_path())?;
            ListenerOptions::new().name(socket_name()?).create_sync()?
        }
        Err(e) => return Err(e),
    };

    std::thread::Builder::new()
        .name("ipc".into())
        .spawn(move || {
            for conn in listener.incoming() {
                let conn = match conn {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[ipc] connection failed: {e}");
                        continue;
                    }
                };
                let mut conn = BufReader::new(conn);
                let mut line = String::new();
                if conn.read_line(&mut line).is_err() {
                    continue;
                }
                let reply = match line.parse::<Command>() {
                    Ok(cmd) => handle(cmd),
                    Err(e) => format!("err {e}"),
                };
                let _ = conn.get_mut().write_all(format!("{reply}\n").as_bytes());
            }
        })?;
    Ok(())
}

/// Hapus file socket saat daemon berhenti (Linux).
pub fn cleanup() {
    #[cfg(unix)]
    let _ = std::fs::remove_file(socket_path());
}
