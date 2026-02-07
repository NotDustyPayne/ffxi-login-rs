# login-rs

Automated multi-character FFXI login tool. Launches multiple Windower instances and automates PlayOnline login using saved credentials, getting all your characters in-game with a single command.

## Prerequisites

- Windows with Windower 4 installed
- All PlayOnline accounts saved with credentials (Windower Dev 4.6.3.6+ supports up to 20 stored accounts)
- Run as Administrator (required for input blocking and hosts file modification)

## Setup

1. Download `login-rs.exe` and `login-config.json` from the [latest release](https://github.com/NotDustyPayne/ffxi-login-rs/releases/latest)
2. Place both files in your Windower directory (e.g., `C:\Windower4\`)
3. Edit `login-config.json` — replace the placeholder character names and slots with your own
4. Each character's `slot` number corresponds to their position in PlayOnline's account list

A minimal config only needs your characters:

```json
{
  "characters": [
    { "name": "CharOne", "slot": 1 },
    { "name": "CharTwo", "slot": 2 }
  ]
}
```

All other settings have sensible defaults:

| Setting | Default | Description |
|---------|---------|-------------|
| `windower_path` | `C:\Windower4\Windower.exe` | Path to Windower executable |
| `playonline_dir` | `C:\Program Files (x86)\PlayOnline\SquareEnix\PlayOnlineViewer\usr\all` | PlayOnline data directory (contains `login_w.bin`) |
| `stagger_delay_seconds` | `10` | Delay between each character's login in Phase 2 |
| `launch_delay_seconds` | `2` | Delay between launching Windower instances in Phase 1 |
| `region` | `us` | POL region (`us`, `jp`, or `eu`) |

## Usage

```bash
# Log in all characters
login-rs.exe

# Log in specific characters by name
login-rs.exe --characters CharOne CharTwo

# Custom config file path
login-rs.exe --config C:\path\to\login-config.json

# Enable debug logging
set RUST_LOG=debug
login-rs.exe
```

## How It Works

1. **Phase 1** — Launches all Windower instances rapidly (~2 seconds apart)
2. **Phase 2** — Sequentially automates each PlayOnline login:
   - Navigates to the correct account slot
   - Presses through login/confirmation screens
   - Uses a local HTTP proxy to skip the PlayOnline news screen and go directly into FFXI
3. All characters end up loading in parallel

## Error Handling

- Failed logins are retried once automatically
- Errors are logged to a temp directory (printed at startup)
- On failure, the tool skips the character and continues with the rest
- Ctrl+C safely cleans up (unblocks input, removes hosts file entries)

## Building on Windows

1. Install the [Rust toolchain](https://rustup.rs/)
2. Clone the repo and build:

```cmd
git clone https://github.com/NotDustyPayne/ffxi-login-rs.git
cd ffxi-login-rs
cargo build --release
```

3. The executable will be at `target\release\login-rs.exe`
4. Copy `login-rs.exe` and `login-config.json` to your Windower directory

## Development (cross-compile from macOS)

```bash
rustup target add x86_64-pc-windows-gnu
brew install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

Output: `target/x86_64-pc-windows-gnu/release/login-rs.exe`
