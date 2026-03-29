#![allow(dead_code)]
#![allow(unused_imports)]

mod config;
mod resolver;
mod slsk;
mod sources;
mod tui;

use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Write logs to file so they don't corrupt the TUI
    let log_file = std::fs::File::create("soulpull.log")?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    let config_path = config::find_config_path();
    let cfg = if config_path.exists() {
        config::Config::load(&config_path)?
    } else {
        tracing::info!("No config file found, will write to {:?} on save", config_path);
        config::Config::default()
    };

    // Optional input as first positional argument:
    // - a CSV file path  → parsed into tracks
    // - anything else    → passed straight to sldl as-is (URL, search string, etc.)
    let args: Vec<String> = std::env::args().collect();
    let input: Option<String> = args.get(1).cloned();

    let (tracks, raw_input) = match input {
        Some(ref s) if PathBuf::from(s).extension().map_or(false, |e| e == "csv") => {
            let path = PathBuf::from(s);
            (sources::csv::parse_csv(&path)?, None)
        }
        Some(s) => (Vec::new(), Some(s)),
        None => (Vec::new(), None),
    };

    tui::run(cfg, config_path, tracks, raw_input).await
}
