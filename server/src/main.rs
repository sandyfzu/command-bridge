use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{self, Command};

const SOCKET_NAME: &str = "msl-code.sock";
const REMOTE: &str = "ssh-remote+lima-msl";

/// Returns a stable socket path under `~/.local/run/`.
///
/// Unlike `$TMPDIR` (which changes across macOS reboots), this path is
/// deterministic and survives restarts — critical for the SSH `RemoteForward`
/// config to remain valid.
fn socket_path() -> PathBuf {
    let home = env::var_os("HOME").expect("$HOME is not set");
    PathBuf::from(home).join(".local/run").join(SOCKET_NAME)
}

fn main() {
    let socket = socket_path();

    // Ensure the parent directory exists
    if let Some(parent) = socket.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("msl-server: failed to create {}: {e}", parent.display());
            process::exit(1);
        });
    }

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

                    let folder_uri = format!("vscode-remote://{REMOTE}{path}");
                    let status = Command::new("code")
                        .arg("--folder-uri")
                        .arg(&folder_uri)
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
