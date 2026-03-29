use anyhow::{bail, Result};
use futures::StreamExt;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::config::Config;
use super::{
    ChosenFile, DownloadEvent, DownloadProgressData, DownloadStatus, SearchResultData,
    SldlEvent, TrackStateData,
};

/// One unit of work handed to the runner.
#[derive(Debug, Clone)]
pub struct RunnerTask {
    /// Index into the App queue — used as the item_id in events.
    pub id: usize,
    /// The search term passed as the positional argument to sldl.
    /// e.g. "Daft Punk - Random Access Memory"
    pub search_term: String,
    /// Album mode: pass `-a` to sldl.
    pub album_mode: bool,
    /// Override output path for this task (falls back to config default).
    pub output_path: Option<PathBuf>,
}

/// Spawn all tasks with a concurrency cap, pushing events back to `tx`.
pub async fn run_all(
    tasks: Vec<RunnerTask>,
    config: Config,
    tx: mpsc::Sender<DownloadEvent>,
) {
    use futures::stream;

    let config = std::sync::Arc::new(config);
    let tx = tx.clone();

    stream::iter(tasks)
        .for_each_concurrent(config.defaults.max_concurrent_downloads, |task| {
            let config = config.clone();
            let tx = tx.clone();
            async move {
                if let Err(e) = run_task(task.id, &task, &config, &tx).await {
                    let _ = tx.send(DownloadEvent::StatusChanged {
                        item_id: task.id,
                        status: DownloadStatus::Failed { reason: e.to_string() },
                    }).await;
                }
            }
        })
        .await;
}

async fn run_task(
    id: usize,
    task: &RunnerTask,
    config: &Config,
    tx: &mpsc::Sender<DownloadEvent>,
) -> Result<()> {
    let mut cmd = build_command(task, config)?;

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn()?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let mut stdout_lines = BufReader::new(stdout).lines();
    let mut stderr_lines = BufReader::new(stderr).lines();

    // Track whether we've seen a search_result so we can detect "settled"
    let mut chosen_ext: Option<String> = None;
    let preferred_format = config.format.preferred.first().cloned().unwrap_or_default();

    loop {
        tokio::select! {
            line = stdout_lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if let Err(e) = handle_stdout_line(
                            id, &line, &preferred_format, &mut chosen_ext, tx
                        ).await {
                            tracing::warn!("sldl JSON parse error: {e}");
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        tracing::error!("sldl stdout read error: {e}");
                        break;
                    }
                }
            }
            line = stderr_lines.next_line() => {
                if let Ok(Some(line)) = line {
                    if !line.trim().is_empty() {
                        let _ = tx.send(DownloadEvent::Log {
                            item_id: id,
                            message: line.clone(),
                        }).await;
                        tracing::debug!("sldl stderr [{}]: {}", id, line);
                    }
                }
            }
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        bail!("sldl exited with code {code}");
    }

    Ok(())
}

async fn handle_stdout_line(
    id: usize,
    line: &str,
    preferred_format: &str,
    chosen_ext: &mut Option<String>,
    tx: &mpsc::Sender<DownloadEvent>,
) -> Result<()> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(());
    }

    let event: SldlEvent = serde_json::from_str(line)?;

    let status = match event.kind.as_str() {
        "search_start" => Some(DownloadStatus::Searching),

        "search_result" => {
            // Record what format sldl picked so we can detect settled later
            if let Ok(data) = serde_json::from_value::<SearchResultData>(event.data) {
                if let Some(file) = data.chosen_file {
                    *chosen_ext = file.extension.map(|e| e.to_lowercase());
                }
            }
            None
        }

        "download_start" => Some(DownloadStatus::Downloading { progress_pct: 0 }),

        "download_progress" => {
            if let Ok(data) = serde_json::from_value::<DownloadProgressData>(event.data) {
                let pct = data.percent.clamp(0.0, 100.0) as u8;
                Some(DownloadStatus::Downloading { progress_pct: pct })
            } else {
                None
            }
        }

        "track_state" => {
            if let Ok(data) = serde_json::from_value::<TrackStateData>(event.data) {
                Some(map_track_state(data, preferred_format, chosen_ext))
            } else {
                None
            }
        }

        _ => None,
    };

    if let Some(s) = status {
        let _ = tx.send(DownloadEvent::StatusChanged { item_id: id, status: s }).await;
    }

    Ok(())
}

fn map_track_state(
    data: TrackStateData,
    preferred_format: &str,
    chosen_ext: &Option<String>,
) -> DownloadStatus {
    let state_lower = data.state.to_lowercase();

    if state_lower.contains("downloaded") || state_lower == "succeeded" {
        let received = chosen_ext.clone().unwrap_or_else(|| "unknown".into());
        // "Settled" if we got a different format than the top preference
        if !received.is_empty()
            && !preferred_format.is_empty()
            && !received.eq_ignore_ascii_case(preferred_format)
        {
            DownloadStatus::Settled {
                wanted: preferred_format.to_string(),
                received,
            }
        } else {
            DownloadStatus::Done { format: received }
        }
    } else if state_lower.contains("failed")
        || state_lower.contains("errored")
        || state_lower.contains("notfound")
    {
        DownloadStatus::Failed {
            reason: data.failure_reason.unwrap_or_else(|| data.state.clone()),
        }
    } else {
        // Other states (e.g. "Skipped") — treat as failure so queue shows something
        DownloadStatus::Failed { reason: data.state }
    }
}

/// Build the sldl `Command` for one task.
fn build_command(task: &RunnerTask, config: &Config) -> Result<Command> {
    if config.soulseek.username.is_empty() || config.soulseek.password.is_empty() {
        bail!("Soulseek username/password not set in config");
    }

    let output_path = task
        .output_path
        .clone()
        .unwrap_or_else(|| config.resolved_output_path());

    let mut cmd = Command::new(&config.soulseek.sldl_path);

    // Positional search term
    cmd.arg(&task.search_term);

    // Authentication
    cmd.arg("--user").arg(&config.soulseek.username);
    cmd.arg("--pass").arg(&config.soulseek.password);

    // Output location
    cmd.arg("-p").arg(&output_path);

    // Album mode
    if task.album_mode {
        cmd.arg("-a");
    }

    // Network
    cmd.arg("--listen-port").arg(config.soulseek.listen_port.to_string());
    cmd.arg("--search-timeout").arg(config.soulseek.search_timeout_ms.to_string());
    cmd.arg("--max-stale-time").arg(config.soulseek.max_stale_time_ms.to_string());

    // Format — preferred
    if !config.format.preferred.is_empty() {
        cmd.arg("--pref-format").arg(config.format.preferred.join(","));
    }
    // Format — all accepted (necessary condition)
    let all_formats = config.format.all_formats();
    if !all_formats.is_empty() {
        cmd.arg("--format").arg(all_formats.join(","));
    }

    // Quality — preferred
    if let Some(sr) = config.format.preferred_min_samplerate {
        cmd.arg("--pref-min-samplerate").arg(sr.to_string());
    }
    if let Some(br) = config.format.preferred_min_bitrate {
        cmd.arg("--pref-min-bitrate").arg(br.to_string());
    }

    // Quality — necessary
    if let Some(br) = config.format.min_bitrate {
        cmd.arg("--min-bitrate").arg(br.to_string());
    }
    if let Some(sr) = config.format.min_samplerate {
        cmd.arg("--min-samplerate").arg(sr.to_string());
    }

    // Machine-readable JSON output; suppress human progress bars
    cmd.arg("--progress-json");
    cmd.arg("--no-progress");

    tracing::debug!("sldl command: {:?}", cmd);
    Ok(cmd)
}

/// Convenience: build a search term from artist + title or artist + album.
pub fn search_term(artist: &str, title_or_album: &str) -> String {
    format!("{artist} - {title_or_album}")
}
