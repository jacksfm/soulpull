use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::sources::Track;
use crate::slsk::{DownloadEvent, DownloadStatus};

/// Which TUI panel is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    /// First-run setup: no credentials configured yet
    Setup,
    Queue,
    Config,
    Summary,
}

/// One entry in the download queue (track or album).
#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: usize,
    pub label: String,
    pub status: DownloadStatus,
}

/// Fields being edited in the setup view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupField {
    Username,
    Password,
    SldlPath,
    OutputPath,
}

pub struct App {
    pub config: Config,
    pub config_path: PathBuf,
    pub tracks: Vec<Track>,
    /// Raw input passed straight to sldl (URL, search string, etc.)
    pub raw_input: Option<String>,
    pub queue: Vec<QueueEntry>,
    pub selected_index: usize,
    pub active_view: ActiveView,
    pub should_quit: bool,
    pub config_editor_content: String,
    pub config_save_message: Option<String>,

    pub event_rx: mpsc::Receiver<DownloadEvent>,
    pub event_tx: mpsc::Sender<DownloadEvent>,

    pub status_message: Option<String>,
    pub is_running: bool,
    pub log_lines: Vec<String>,

    // Setup view state
    pub setup_field: SetupField,
    pub setup_username: String,
    pub setup_password: String,
    pub setup_sldl_path: String,
    pub setup_output_path: String,
    pub setup_show_password: bool,
}

impl App {
    pub fn new(config: Config, config_path: PathBuf, tracks: Vec<Track>, raw_input: Option<String>) -> Self {
        let (event_tx, event_rx) = mpsc::channel(512);

        let needs_setup = config.soulseek.username.is_empty() || config.soulseek.password.is_empty();
        let active_view = if needs_setup { ActiveView::Setup } else { ActiveView::Queue };

        let queue = build_queue(&tracks, raw_input.as_deref());
        let config_editor_content = toml::to_string_pretty(&config).unwrap_or_default();

        // Pre-fill setup fields from existing config
        let setup_username = config.soulseek.username.clone();
        let setup_password = config.soulseek.password.clone();
        let setup_sldl_path = config.soulseek.sldl_path.clone();
        let setup_output_path = config.defaults.output_path.clone();

        Self {
            config,
            config_path,
            tracks,
            raw_input,
            queue,
            selected_index: 0,
            active_view,
            should_quit: false,
            config_editor_content,
            config_save_message: None,
            event_rx,
            event_tx,
            status_message: None,
            is_running: false,
            log_lines: Vec::new(),
            setup_field: SetupField::Username,
            setup_username,
            setup_password,
            setup_sldl_path,
            setup_output_path,
            setup_show_password: false,
        }
    }

    /// Apply setup fields into config and save to disk.
    pub fn commit_setup(&mut self) -> anyhow::Result<()> {
        self.config.soulseek.username = self.setup_username.clone();
        self.config.soulseek.password = self.setup_password.clone();
        self.config.soulseek.sldl_path = self.setup_sldl_path.clone();
        self.config.defaults.output_path = self.setup_output_path.clone();
        self.config.save(&self.config_path)?;
        self.config_editor_content = toml::to_string_pretty(&self.config).unwrap_or_default();
        Ok(())
    }

    /// Save the current config editor content to disk.
    pub fn save_config_editor(&mut self) {
        match toml::from_str::<Config>(&self.config_editor_content) {
            Ok(parsed) => {
                self.config = parsed;
                match self.config.save(&self.config_path) {
                    Ok(()) => {
                        self.config_save_message = Some(format!(
                            "Saved to {}",
                            self.config_path.display()
                        ));
                    }
                    Err(e) => {
                        self.config_save_message = Some(format!("Save failed: {e}"));
                    }
                }
            }
            Err(e) => {
                self.config_save_message = Some(format!("TOML parse error: {e}"));
            }
        }
    }

    pub fn drain_download_events(&mut self) {
        while let Ok(ev) = self.event_rx.try_recv() {
            match ev {
                DownloadEvent::StatusChanged { item_id, status } => {
                    if let Some(entry) = self.queue.iter_mut().find(|e| e.id == item_id) {
                        entry.status = status;
                    }
                }
                DownloadEvent::Log { item_id, message } => {
                    self.log_lines.push(format!("[{}] {}", item_id, message));
                    if self.log_lines.len() > 200 {
                        self.log_lines.remove(0);
                    }
                }
                DownloadEvent::TrackDiscovered { item_id, label } => {
                    // sldl told us about a track we didn't know about (e.g. from a URL input)
                    if !self.queue.iter().any(|e| e.id == item_id) {
                        self.queue.push(QueueEntry {
                            id: item_id,
                            label,
                            status: DownloadStatus::Queued,
                        });
                    }
                }
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.queue.is_empty() && self.selected_index < self.queue.len() - 1 {
            self.selected_index += 1;
        }
    }

    pub fn summary_counts(&self) -> SummaryCounts {
        let mut counts = SummaryCounts::default();
        for entry in &self.queue {
            match &entry.status {
                DownloadStatus::Done { .. } => counts.done += 1,
                DownloadStatus::Settled { .. } => counts.settled += 1,
                DownloadStatus::Failed { .. } => counts.failed += 1,
                DownloadStatus::Queued => counts.queued += 1,
                DownloadStatus::Searching | DownloadStatus::Downloading { .. } => {
                    counts.in_progress += 1
                }
            }
        }
        counts
    }
}

fn build_queue(tracks: &[Track], raw_input: Option<&str>) -> Vec<QueueEntry> {
    if !tracks.is_empty() {
        tracks
            .iter()
            .enumerate()
            .map(|(i, t)| QueueEntry {
                id: i,
                label: format!(
                    "{} — {}{}",
                    t.artist,
                    t.title,
                    t.album.as_deref().map(|a| format!(" ({})", a)).unwrap_or_default()
                ),
                status: DownloadStatus::Queued,
            })
            .collect()
    } else if let Some(input) = raw_input {
        // Single entry for raw input — queue will be expanded by track_list events
        vec![QueueEntry {
            id: 0,
            label: input.to_string(),
            status: DownloadStatus::Queued,
        }]
    } else {
        Vec::new()
    }
}

#[derive(Debug, Default)]
pub struct SummaryCounts {
    pub done: usize,
    pub settled: usize,
    pub failed: usize,
    pub queued: usize,
    pub in_progress: usize,
}
