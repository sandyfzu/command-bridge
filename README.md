# Command Bridge

A lightweight Rust tool that lets you run `code .` inside a [Lima](https://lima-vm.io/) VM and have VS Code open on your macOS host — just like WSL on Windows.

## How it works

```
Lima VM (Linux)                         macOS
─────────────────                       ──────────────────
code ~/projects/app                     msl-server
  → resolve to absolute path              ← listens on Unix socket
  → write path to socket ──────────────→  → receives path
         (SSH RemoteForward)              → runs: code --remote ssh-remote+lima-msl<path> <path>
                                          → VS Code opens remote window into VM
```

Two tiny binaries communicate over a Unix socket forwarded through SSH:

| Binary | Runs on | Purpose |
|---|---|---|
| **client** (`code`) | Linux VM | Resolves the path and sends it to the socket |
| **server** (`msl-server`) | macOS | Listens on the socket and launches VS Code with `--remote` |

Zero dependencies beyond `std`. Static binaries. ~40 lines each.

## Prerequisites

- [Lima](https://lima-vm.io/) VM named `msl` (any Lima VM works — adjust the `REMOTE` constant)
- [VS Code](https://code.visualstudio.com/) with `code` CLI on macOS PATH
- [Rust](https://rustup.rs/) toolchain (for building)
- SSH config with the Lima VM accessible as `lima-msl`

## Build

```sh
# Server (macOS native)
cargo build --release -p command-bridge-server

# Client (cross-compile static binary for aarch64 Linux)
rustup target add aarch64-unknown-linux-musl
cargo build --release -p command-bridge-client --target aarch64-unknown-linux-musl

# OR simply Build from inside the VM
cargo build --release -p command-bridge-client
```

## Install

### Server (macOS)

```sh
cp target/release/command-bridge-server /usr/local/bin/msl-server
```

### Client (Linux VM)

If cross-compiled from macOS:

```sh
# Create ~/bin if needed
ssh lima-msl 'mkdir -p ~/bin'

# Deploy and rename to "code"
scp target/aarch64-unknown-linux-musl/release/command-bridge-client lima-msl:~/bin/code
```

If built from inside the VM:

```sh
mkdir -p ~/bin
cp target/release/command-bridge-client ~/bin/code
```

Make sure `~/bin` is in `PATH` inside the VM. Add to `~/.bashrc`:

```sh
export PATH="$HOME/bin:$PATH"
```

## Setup

### 1. SSH socket forwarding

The client in the VM needs to reach the server's socket on macOS. SSH `RemoteForward` handles this.

Both paths are stable and deterministic:

| Side | Socket path |
|---|---|
| **macOS** (server) | `~/.local/run/msl-code.sock` |
| **Linux VM** (client) | `$XDG_RUNTIME_DIR/msl-code.sock` (typically `/run/user/<uid>/`) |

Find your VM's `$XDG_RUNTIME_DIR`:

```sh
ssh lima-msl 'echo $XDG_RUNTIME_DIR'
# e.g. /run/user/501
```

Add to `~/.ssh/config` (before or after the Lima `Include`):

```ssh-config
Host lima-msl
  RemoteForward /run/user/<uid>/msl-code.sock /Users/<your-user>/.local/run/msl-code.sock
```

> Replace `<uid>` with your VM user's UID and `<your-user>` with your macOS username.

### 2. Start on login (LaunchAgent)

A plist is included to run the server automatically on macOS login:

```sh
cp dev.msl.command-bridge-server.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/dev.msl.command-bridge-server.plist
```

Verify:

```sh
launchctl list | grep msl
```

Manage:

```sh
# Stop
launchctl unload ~/Library/LaunchAgents/dev.msl.command-bridge-server.plist

# Restart
launchctl unload ~/Library/LaunchAgents/dev.msl.command-bridge-server.plist
launchctl load ~/Library/LaunchAgents/dev.msl.command-bridge-server.plist

# Logs
tail -f /tmp/msl-server.stderr.log
```

## Usage

From inside the VM:

```sh
code .                      # open current directory
code ~/projects/myapp       # open specific path
```

VS Code opens on macOS with a remote SSH window connected to the VM at that path.

## Socket paths

Both paths are stable across reboots — no dynamic `$TMPDIR` that can change.

| Platform | Socket path | Notes |
|---|---|---|
| **macOS** (server) | `~/.local/run/msl-code.sock` | Fixed, deterministic, user-private |
| **Linux** (client) | `$XDG_RUNTIME_DIR/msl-code.sock` | Stable on systemd (`/run/user/<uid>`), fallback `/tmp` |

## Project structure

```
command-bridge/
├── Cargo.toml                              # workspace root
├── server/                                 # macOS — listens on socket, launches VS Code
│   ├── Cargo.toml
│   └── src/main.rs
├── client/                                 # Linux VM — resolves path, sends to socket
│   ├── Cargo.toml
│   └── src/main.rs
├── dev.msl.command-bridge-server.plist     # macOS LaunchAgent
└── README.md
```

## Future ideas

- Auto-detect VM name from SSH connection metadata
- Support additional commands (`open`, `pbcopy`, `xdg-open` → macOS equivalents)
- Bidirectional protocol with JSON messages
- Clipboard forwarding
- Browser URL forwarding

## License

MIT
