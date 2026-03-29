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

    let config_path = config::default_config_path();
    let cfg = if config_path.exists() {
        config::Config::load(&config_path)?
    } else {
        tracing::info!("No config file at {:?}, using defaults", config_path);
        config::Config::default()
    };

    // Optional CSV path as first positional argument
    let args: Vec<String> = std::env::args().collect();
    let csv_path: Option<PathBuf> = args.get(1).map(PathBuf::from);

    let tracks = if let Some(path) = csv_path {
        sources::csv::parse_csv(&path)?
    } else {
        Vec::new()
    };

    tui::run(cfg, tracks).await
}
