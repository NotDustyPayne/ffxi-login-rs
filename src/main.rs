mod config;
mod hosts;
mod launcher;
mod logging;
mod login_bin;
mod proxy;
mod win32;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "login-rs", version, about = "Automated FFXI multi-character login")]
struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "login-config.json")]
    config: PathBuf,

    /// Specific characters to log in (by name). If omitted, logs in all.
    #[arg(long, num_args = 1..)]
    characters: Vec<String>,
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    let config = match config::Config::load(&args.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config from {:?}: {}", args.config, e);
            std::process::exit(1);
        }
    };

    let file_logger = match logging::FileLogger::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to initialize logging: {}", e);
            std::process::exit(1);
        }
    };

    let characters = config.filter_characters(&args.characters);

    if characters.is_empty() {
        eprintln!("No matching characters found for: {:?}", args.characters);
        std::process::exit(1);
    }

    println!("login-rs v{}", env!("CARGO_PKG_VERSION"));
    println!("Launching {} character(s):", characters.len());
    for ch in &characters {
        println!("  - {} (slot {})", ch.name, ch.slot);
    }
    println!("Logs: {}", file_logger.log_dir().display());

    ctrlc::set_handler(move || {
        eprintln!("\nInterrupted! Cleaning up...");
        win32::block_input(false);
        hosts::cleanup_stale();
        std::process::exit(1);
    })
    .expect("Failed to set Ctrl+C handler");

    launcher::run(&config, &characters, &file_logger);
}
