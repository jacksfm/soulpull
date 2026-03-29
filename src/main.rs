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
        config::Config::default()
    };

    // Optional: any positional args are pre-loaded into the queue on launch
    let args: Vec<String> = std::env::args().skip(1).collect();

    tui::run(cfg, config_path, args).await
}
