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
    SldlEvent, TrackListData, TrackListEntry, TrackStateData,
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A single sldl invocation (one per CSV track, or one for a raw input).
#[derive(Debug, Clone)]
pub struct RunnerTask {
    /// Index into the App queue — used as the item_id in events.
    pub id: usize,
    /// The search term or file path passed as the positional argument to sldl.
    pub input: String,
    /// Album mode: pass `-a` to sldl.
    pub album_mode: bool,
    /// Override output path for this task (falls back to config default).
    pub output_path: Option<PathBuf>,
}

/// Run all tasks with a concurrency cap, pushing events to `tx`.
pub async fn run_all(tasks: Vec<RunnerTask>, config: Config, tx: mpsc::Sender<DownloadEvent>) {
    use futures::stream;
    let config = std::sync::Arc::new(config);
    stream::iter(tasks)
        .for_each_concurrent(config.defaults.max_concurrent_downloads, |task| {
            let config = config.clone();
            let tx = tx.clone();
            async move {
                if let Err(e) = run_task(&task, &config, &tx).await {
                    let _ = tx.send(DownloadEvent::StatusChanged {
                        item_id: task.id,
                        status: DownloadStatus::Failed { reason: e.to_string() },
                    }).await;
                }
            }
        })
        .await;
}

/// Run a single sldl process for a raw input (URL, search string, CSV path).
/// sldl handles input type detection itself; we just read the NDJSON back.
/// Queue entries are populated from `track_list` events as they arrive.
pub async fn run_raw(input: &str, config: &Config, tx: &mpsc::Sender<DownloadEvent>) -> Result<()> {
    let task = RunnerTask {
        id: 0,
        input: input.to_string(),
        album_mode: matches!(config.defaults.aggregation, crate::config::Aggregation::Album | crate::config::Aggregation::Artist),
        output_path: None,
    };
    run_task(&task, config, tx).await
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

async fn run_task(task: &RunnerTask, config: &Config, tx: &mpsc::Sender<DownloadEvent>) -> Result<()> {
    let mut cmd = build_command(task, config)?;
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);

    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let mut stdout_lines = BufReader::new(stdout).lines();
    let mut stderr_lines = BufReader::new(stderr).lines();

    let preferred_format = config.format.preferred.first().cloned().unwrap_or_default();
    // Per-track state: maps track index → chosen extension
    let mut chosen_exts: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    // For single-task mode the track id is fixed; for track_list mode we map by index
    let base_id = task.id;

    loop {
        tokio::select! {
            line = stdout_lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if let Err(e) = handle_line(
                            base_id, &line, &preferred_format,
                            &mut chosen_exts, tx
                        ).await {
                            tracing::warn!("sldl JSON parse: {e}");
                        }
                    }
                    Ok(None) => break,
                    Err(e) => { tracing::error!("sldl stdout: {e}"); break; }
                }
            }
            line = stderr_lines.next_line() => {
                if let Ok(Some(line)) = line {
                    let line = line.trim().to_string();
                    if !line.is_empty() {
                        let _ = tx.send(DownloadEvent::Log { item_id: base_id, message: line.clone() }).await;
                        tracing::debug!("sldl stderr [{}]: {}", base_id, line);
                    }
                }
            }
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        // sldl exits non-zero if any track failed, but others may have succeeded — not fatal
        tracing::warn!("sldl exited with code {code} for task {}", task.id);
    }

    Ok(())
}

async fn handle_line(
    base_id: usize,
    line: &str,
    preferred_format: &str,
    chosen_exts: &mut std::collections::HashMap<usize, String>,
    tx: &mpsc::Sender<DownloadEvent>,
) -> Result<()> {
    let line = line.trim();
    if line.is_empty() { return Ok(()); }

    let event: SldlEvent = serde_json::from_str(line)?;

    match event.kind.as_str() {
        // sldl tells us all the tracks it's about to process — populate the queue
        "track_list" => {
            if let Ok(data) = serde_json::from_value::<TrackListData>(event.data) {
                for entry in data.tracks {
                    let item_id = base_id + entry.index;
                    let label = format_track_label(&entry);
                    let _ = tx.send(DownloadEvent::TrackDiscovered { item_id, label }).await;
                }
            }
        }

        "search_start" => {
            let item_id = extract_index(&event.kind, base_id);
            let _ = tx.send(DownloadEvent::StatusChanged {
                item_id,
                status: DownloadStatus::Searching,
            }).await;
        }

        "search_result" => {
            if let Ok(data) = serde_json::from_value::<SearchResultData>(event.data) {
                if let Some(file) = data.chosen_file {
                    let item_id = extract_index(&event.kind, base_id);
                    chosen_exts.insert(item_id, file.extension.unwrap_or_default().to_lowercase());
                }
            }
        }

        "download_start" => {
            let item_id = extract_index(&event.kind, base_id);
            let _ = tx.send(DownloadEvent::StatusChanged {
                item_id,
                status: DownloadStatus::Downloading { progress_pct: 0 },
            }).await;
        }

        "download_progress" => {
            if let Ok(data) = serde_json::from_value::<DownloadProgressData>(event.data) {
                let item_id = extract_index(&event.kind, base_id);
                let pct = data.percent.clamp(0.0, 100.0) as u8;
                let _ = tx.send(DownloadEvent::StatusChanged {
                    item_id,
                    status: DownloadStatus::Downloading { progress_pct: pct },
                }).await;
            }
        }

        "track_state" => {
            if let Ok(data) = serde_json::from_value::<TrackStateData>(event.data) {
                let item_id = extract_index(&event.kind, base_id);
                let chosen = chosen_exts.get(&item_id).cloned();
                let status = map_track_state(data, preferred_format, &chosen);
                let _ = tx.send(DownloadEvent::StatusChanged { item_id, status }).await;
            }
        }

        _ => {}
    }

    Ok(())
}

/// sldl embeds the track index in the data payload, not the event type.
/// For single-task mode this is always base_id; for track_list mode we'd need
/// to parse it from the data. For now default to base_id — the track_list
/// path populates queue entries and track_state carries artist/title we can match on.
fn extract_index(_kind: &str, base_id: usize) -> usize {
    base_id
}

fn format_track_label(entry: &TrackListEntry) -> String {
    match (entry.artist.as_deref(), entry.title.as_deref(), entry.album.as_deref()) {
        (Some(a), Some(t), Some(al)) => format!("{a} — {t} ({al})"),
        (Some(a), Some(t), None) => format!("{a} — {t}"),
        (Some(a), None, Some(al)) => format!("{a} — {al}"),
        (None, Some(t), _) => t.to_string(),
        _ => format!("track {}", entry.index),
    }
}

fn map_track_state(data: TrackStateData, preferred_format: &str, chosen_ext: &Option<String>) -> DownloadStatus {
    let state_lower = data.state.to_lowercase();

    if state_lower.contains("downloaded") || state_lower == "succeeded" {
        let received = chosen_ext.clone().unwrap_or_else(|| "unknown".into());
        if !received.is_empty()
            && !preferred_format.is_empty()
            && !received.eq_ignore_ascii_case(preferred_format)
        {
            DownloadStatus::Settled { wanted: preferred_format.to_string(), received }
        } else {
            DownloadStatus::Done { format: received }
        }
    } else if state_lower.contains("failed")
        || state_lower.contains("errored")
        || state_lower.contains("notfound")
    {
        DownloadStatus::Failed {
            reason: data.failure_reason.unwrap_or(data.state),
        }
    } else {
        DownloadStatus::Failed { reason: data.state }
    }
}

// ---------------------------------------------------------------------------
// Command builder
// ---------------------------------------------------------------------------

pub fn build_command(task: &RunnerTask, config: &Config) -> Result<Command> {
    if config.soulseek.username.is_empty() || config.soulseek.password.is_empty() {
        bail!("Soulseek username/password not set — press 'c' to open config");
    }

    let output_path = task.output_path.clone()
        .unwrap_or_else(|| config.resolved_output_path());

    let mut cmd = Command::new(&config.soulseek.sldl_path);

    cmd.arg(&task.input);
    cmd.arg("--user").arg(&config.soulseek.username);
    cmd.arg("--pass").arg(&config.soulseek.password);
    cmd.arg("-p").arg(&output_path);

    if task.album_mode {
        cmd.arg("-a");
    }

    // Network
    cmd.arg("--listen-port").arg(config.soulseek.listen_port.to_string());
    cmd.arg("--search-timeout").arg(config.soulseek.search_timeout_ms.to_string());
    cmd.arg("--max-stale-time").arg(config.soulseek.max_stale_time_ms.to_string());

    // Format preferences
    if !config.format.preferred.is_empty() {
        cmd.arg("--pref-format").arg(config.format.preferred.join(","));
    }
    let all_formats = config.format.all_formats();
    if !all_formats.is_empty() {
        cmd.arg("--format").arg(all_formats.join(","));
    }
    if let Some(sr) = config.format.preferred_min_samplerate {
        cmd.arg("--pref-min-samplerate").arg(sr.to_string());
    }
    if let Some(br) = config.format.preferred_min_bitrate {
        cmd.arg("--pref-min-bitrate").arg(br.to_string());
    }
    if let Some(br) = config.format.min_bitrate {
        cmd.arg("--min-bitrate").arg(br.to_string());
    }
    if let Some(sr) = config.format.min_samplerate {
        cmd.arg("--min-samplerate").arg(sr.to_string());
    }

    // Machine-readable output
    cmd.arg("--progress-json");
    cmd.arg("--no-progress");

    tracing::debug!("sldl: {:?}", cmd);
    Ok(cmd)
}

/// Convenience: `"Artist - Album"` search term for a track/album.
pub fn search_term(artist: &str, title_or_album: &str) -> String {
    format!("{artist} - {title_or_album}")
}
