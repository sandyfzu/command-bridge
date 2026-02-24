use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{self, Command};

const SOCKET_NAME: &str = "msl-code.sock";
const REMOTE: &str = "ssh-remote+lima-msl";

/// Returns the socket path following macOS best practices:
/// 1. `$TMPDIR/msl-code.sock` — per-user private dir (preferred on macOS)
/// 2. `$XDG_RUNTIME_DIR/msl-code.sock` — standard on Linux
/// 3. `/tmp/msl-code.sock` — universal fallback
fn socket_path() -> PathBuf {
    env::var_os("TMPDIR")
        .or_else(|| env::var_os("XDG_RUNTIME_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(SOCKET_NAME)
}

fn main() {
    let socket = socket_path();

    // Clean up stale socket
    let _ = fs::remove_file(&socket);

    let listener = UnixListener::bind(&socket).unwrap_or_else(|e| {
        eprintln!("msl-server: bind failed ({}): {e}", socket.display());
        process::exit(1);
    });

    eprintln!("msl-server: listening on {}", socket.display());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let reader = BufReader::new(stream);
                for line in reader.lines() {
                    let path = match line {
                        Ok(p) => p,
                        Err(_) => continue,
                    };

                    if path.trim().is_empty() {
                        continue;
                    }

                    eprintln!("msl-server: opening {path}");

                    let remote_arg = format!("{REMOTE}{path}");
                    let status = Command::new("code")
                        .arg("--remote")
                        .arg(&remote_arg)
                        .arg(&path)
                        .status();

                    match status {
                        Ok(s) if s.success() => {}
                        Ok(s) => eprintln!("msl-server: code exited with {s}"),
                        Err(e) => eprintln!("msl-server: failed to exec code: {e}"),
                    }
                }
            }
            Err(e) => eprintln!("msl-server: accept error: {e}"),
        }
    }
}
