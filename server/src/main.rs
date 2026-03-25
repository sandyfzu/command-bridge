use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{self, Command};

const SOCKET_NAME: &str = "msl-code.sock";
const REMOTE: &str = "ssh-remote+lima-msl";

/// Commands the server is allowed to launch.
///
/// This allowlist prevents arbitrary command execution if a malicious client
/// sends a crafted message through the socket.
const ALLOWED_COMMANDS: &[&str] = &["code", "code-insiders"];

/// Returns a stable socket path under `~/.local/run/`.
///
/// Unlike `$TMPDIR` (which changes across macOS reboots), this path is
/// deterministic and survives restarts — critical for the SSH `RemoteForward`
/// config to remain valid.
fn socket_path() -> PathBuf {
    let home = env::var_os("HOME").expect("$HOME is not set");
    PathBuf::from(home).join(".local/run").join(SOCKET_NAME)
}

/// Parses a protocol line into (command, path).
///
/// Protocol format: `<command>\t<path>\n`
///   - `command` — one of [`ALLOWED_COMMANDS`]
///   - `path`    — absolute path on the remote VM
///
/// For backward compatibility, a bare path (no tab) defaults to `"code"`.
fn parse_line(line: &str) -> Option<(&str, &str)> {
    let (cmd, path) = match line.split_once('\t') {
        Some((cmd, path)) => (cmd, path),
        None => ("code", line.as_ref()),
    };

    let cmd = cmd.trim();
    let path = path.trim();

    if path.is_empty() {
        return None;
    }

    if !ALLOWED_COMMANDS.contains(&cmd) {
        eprintln!("msl-server: rejected unknown command: {cmd:?}");
        return None;
    }

    // Require an absolute path to prevent path-traversal tricks
    if !path.starts_with('/') {
        eprintln!("msl-server: rejected non-absolute path: {path:?}");
        return None;
    }

    Some((cmd, path))
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
                    let line = match line {
                        Ok(l) => l,
                        Err(e) => {
                            eprintln!("msl-server: read error: {e}");
                            continue;
                        }
                    };

                    let (cmd, path) = match parse_line(&line) {
                        Some(pair) => pair,
                        None => continue,
                    };

                    eprintln!("msl-server: [{cmd}] opening {path}");

                    let folder_uri = format!("vscode-remote://{REMOTE}{path}");
                    let status = Command::new(cmd)
                        .arg("--folder-uri")
                        .arg(&folder_uri)
                        .status();

                    match status {
                        Ok(s) if s.success() => {}
                        Ok(s) => eprintln!("msl-server: {cmd} exited with {s}"),
                        Err(e) => eprintln!("msl-server: failed to exec {cmd}: {e}"),
                    }
                }
            }
            Err(e) => eprintln!("msl-server: accept error: {e}"),
        }
    }
}
