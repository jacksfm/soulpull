use tokio::sync::mpsc;

use crate::config::Config;
use crate::sources::Track;
use crate::slsk::{DownloadEvent, DownloadStatus};

/// Which TUI panel is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
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

/// The central application state shared across the TUI and async tasks.
pub struct App {
    pub config: Config,
    pub tracks: Vec<Track>,
    pub queue: Vec<QueueEntry>,
    pub selected_index: usize,
    pub active_view: ActiveView,
    pub should_quit: bool,
    pub config_editor_content: String,

    /// Channel for background sldl tasks to push status updates
    pub event_rx: mpsc::Receiver<DownloadEvent>,
    pub event_tx: mpsc::Sender<DownloadEvent>,

    pub status_message: Option<String>,
    pub is_running: bool,
    pub log_lines: Vec<String>,
}

impl App {
    pub fn new(config: Config, tracks: Vec<Track>) -> Self {
        let (event_tx, event_rx) = mpsc::channel(512);

        let queue = tracks
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
            .collect();

        let config_editor_content = toml::to_string_pretty(&config).unwrap_or_default();

        Self {
            config,
            tracks,
            queue,
            selected_index: 0,
            active_view: ActiveView::Queue,
            should_quit: false,
            config_editor_content,
            event_rx,
            event_tx,
            status_message: None,
            is_running: false,
            log_lines: Vec::new(),
        }
    }

    /// Drain all pending events from background sldl tasks.
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

#[derive(Debug, Default)]
pub struct SummaryCounts {
    pub done: usize,
    pub settled: usize,
    pub failed: usize,
    pub queued: usize,
    pub in_progress: usize,
}
