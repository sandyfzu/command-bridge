use std::env;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process;

const SOCKET_NAME: &str = "msl-code.sock";

/// Returns the socket path following Linux best practices:
/// 1. `$XDG_RUNTIME_DIR/msl-code.sock` — stable on systemd-based Linux (`/run/user/<uid>`)
/// 2. `/tmp/msl-code.sock` — universal fallback
fn socket_path() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(SOCKET_NAME)
}

fn main() {
    let socket = socket_path();

    let arg = env::args().nth(1).unwrap_or_else(|| ".".into());
    let path = env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(&arg);
    let abs = std::fs::canonicalize(&path).unwrap_or(path);

    let mut stream = UnixStream::connect(&socket).unwrap_or_else(|e| {
        eprintln!("msl: cannot connect to host socket ({}): {e}", socket.display());
        process::exit(1);
    });

    stream
        .write_all(format!("{}\n", abs.display()).as_bytes())
        .unwrap_or_else(|e| {
            eprintln!("msl: write failed: {e}");
            process::exit(1);
        });
}
