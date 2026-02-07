# login-rs

Automated multi-character FFXI login tool. Launches multiple Windower instances and automates PlayOnline login using saved credentials, getting all your characters in-game with a single command.

## Prerequisites

- Windows with Windower 4 installed
- All PlayOnline accounts saved with credentials (Windower Dev 4.6.3.6+ supports up to 20 stored accounts)
- Run as Administrator (required for input blocking and hosts file modification)

## Setup

1. Copy `login-rs.exe` and `config.example.json` into your Windower directory (e.g., `C:\Windower4\`)
2. Rename `config.example.json` to `config.json`
3. Edit `config.json` with your character names and slot numbers
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
login-rs.exe --config C:\path\to\config.json

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

## Building

Requires Rust toolchain. Cross-compile from macOS:

```bash
rustup target add x86_64-pc-windows-gnu
brew install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

Output: `target/x86_64-pc-windows-gnu/release/login-rs.exe`
