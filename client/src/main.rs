use std::env;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process;

const SOCKET_NAME: &str = "msl-code.sock";

/// Commands the client is allowed to request.
const ALLOWED_COMMANDS: &[&str] = &["code", "code-insiders"];

/// Returns the socket path following Linux best practices:
/// 1. `$XDG_RUNTIME_DIR/msl-code.sock` — stable on systemd-based Linux (`/run/user/<uid>`)
/// 2. `/tmp/msl-code.sock` — universal fallback
fn socket_path() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(SOCKET_NAME)
}

/// Derives the editor command from `argv[0]`.
///
/// The binary is intended to be deployed as `~/bin/code` **and/or**
/// `~/bin/code-insiders` (via copy or symlink). The file-stem of the
/// invoked name selects which editor the server should launch.
///
/// Falls back to `"code"` when the name is unrecognised.
fn detect_command() -> &'static str {
    let argv0 = env::args().next().unwrap_or_default();
    let stem = Path::new(&argv0)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("code");

    // Return the matching &'static str from the allowlist so the borrow
    // lives long enough (avoids allocating a String).
    ALLOWED_COMMANDS
        .iter()
        .find(|&&allowed| allowed == stem)
        .copied()
        .unwrap_or("code")
}

fn main() {
    let socket = socket_path();
    let cmd = detect_command();

    let arg = env::args().nth(1).unwrap_or_else(|| ".".into());
    let path = env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(&arg);
    let abs = std::fs::canonicalize(&path).unwrap_or(path);

    let mut stream = UnixStream::connect(&socket).unwrap_or_else(|e| {
        eprintln!("msl: cannot connect to host socket ({}): {e}", socket.display());
        process::exit(1);
    });

    // Protocol: <command>\t<path>\n
    stream
        .write_all(format!("{cmd}\t{}\n", abs.display()).as_bytes())
        .unwrap_or_else(|e| {
            eprintln!("msl: write failed: {e}");
            process::exit(1);
        });
}
