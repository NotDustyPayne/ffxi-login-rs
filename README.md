# login-rs

Automated multi-character FFXI login tool. Launches multiple Windower instances and automates PlayOnline login using saved credentials, getting all your characters in-game with a single command.

## Quickstart

1. Download `login-rs-vX.X.X.zip` from the [latest release](https://github.com/NotDustyPayne/ffxi-login-rs/releases/latest)
2. Extract the `login-rs` folder into your Windower directory (e.g., `C:\Windower4\login-rs\`)
3. Open `config.json` and replace the placeholder characters with your own:
   ```json
   {
     "characters": [
       { "name": "MyMain", "slot": 1 },
       { "name": "MyMule", "slot": 2 }
     ]
   }
   ```
   Each `slot` is the character's position in PlayOnline's account list (the order they appear when you open PlayOnline).
4. Right-click `login-rs.exe` and **Run as Administrator**

That's it — all your characters will launch and log in automatically.

## Prerequisites

- Windows with Windower 4 installed
- All PlayOnline accounts saved with credentials (Windower Dev 4.6.3.6+ supports up to 20 stored accounts)
- Run as Administrator (required for input blocking and hosts file modification)

## Configuration

The only required field is `characters`. Everything else has sensible defaults:

| Setting | Default | Description |
|---------|---------|-------------|
| `windower_path` | `C:\Windower4\Windower.exe` | Path to Windower executable |
| `playonline_dir` | `C:\Program Files (x86)\PlayOnline\SquareEnix\PlayOnlineViewer\usr\all` | PlayOnline data directory (contains `login_w.bin`) |
| `stagger_delay_seconds` | `10` | Delay between each character's login in Phase 2 |
| `launch_delay_seconds` | `2` | Delay between launching Windower instances in Phase 1 |
| `region` | `us` | POL region (`us`, `jp`, or `eu`) |

## Usage

```cmd
:: Log in all characters
login-rs.exe

:: Log in specific characters by name
login-rs.exe --characters MyMain MyMule

:: Use a config file in a different location
login-rs.exe --config C:\path\to\config.json

:: Enable debug logging
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

## Development (cross-compile from macOS)

```bash
rustup target add x86_64-pc-windows-gnu
brew install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

Output: `target/x86_64-pc-windows-gnu/release/login-rs.exe`
