use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::sources::Track;
use crate::slsk::{DownloadEvent, DownloadStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Setup,
    Queue,
    Config,
    Summary,
}

/// Whether the queue view is in normal mode or accepting text input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    /// User is typing in the add-item bar
    Adding,
}

/// One entry in the download queue.
#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: usize,
    /// Display label shown in the queue list
    pub label: String,
    /// The raw string passed to sldl (search term, URL, or CSV path)
    pub input: String,
    pub album_mode: bool,
    pub status: DownloadStatus,
}

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
    pub queue: Vec<QueueEntry>,
    pub selected_index: usize,
    pub active_view: ActiveView,
    pub input_mode: InputMode,
    /// Text currently being typed in the add bar
    pub input_buffer: String,
    pub should_quit: bool,
    pub config_editor_content: String,
    pub config_save_message: Option<String>,

    pub event_rx: mpsc::Receiver<DownloadEvent>,
    pub event_tx: mpsc::Sender<DownloadEvent>,

    pub status_message: Option<String>,
    pub is_running: bool,
    pub log_lines: Vec<String>,

    // counter for assigning unique IDs to queue entries
    next_id: usize,

    // setup view state
    pub setup_field: SetupField,
    pub setup_username: String,
    pub setup_password: String,
    pub setup_sldl_path: String,
    pub setup_output_path: String,
    pub setup_show_password: bool,
}

impl App {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        let (event_tx, event_rx) = mpsc::channel(512);
        let needs_setup = config.soulseek.username.is_empty() || config.soulseek.password.is_empty();
        let active_view = if needs_setup { ActiveView::Setup } else { ActiveView::Queue };
        let config_editor_content = toml::to_string_pretty(&config).unwrap_or_default();

        let setup_username = config.soulseek.username.clone();
        let setup_password = config.soulseek.password.clone();
        let setup_sldl_path = config.soulseek.sldl_path.clone();
        let setup_output_path = config.defaults.output_path.clone();

        Self {
            config,
            config_path,
            queue: Vec::new(),
            selected_index: 0,
            active_view,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            should_quit: false,
            config_editor_content,
            config_save_message: None,
            event_rx,
            event_tx,
            status_message: None,
            is_running: false,
            log_lines: Vec::new(),
            next_id: 0,
            setup_field: SetupField::Username,
            setup_username,
            setup_password,
            setup_sldl_path,
            setup_output_path,
            setup_show_password: false,
        }
    }

    /// Add a raw input string to the queue.
    /// If it's a CSV path, expand it into individual track entries.
    /// Otherwise add it as a single entry (search string, URL, etc.)
    pub fn add_input(&mut self, raw: String) {
        let raw = raw.trim().to_string();
        if raw.is_empty() {
            return;
        }

        let path = PathBuf::from(&raw);
        if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("csv")) && path.exists() {
            match crate::sources::csv::parse_csv(&path) {
                Ok(tracks) => {
                    for track in tracks {
                        self.add_track_entry(&track);
                    }
                    self.status_message = Some(format!("loaded csv: {}", path.display()));
                }
                Err(e) => {
                    self.status_message = Some(format!("csv error: {e}"));
                }
            }
        } else {
            // Search string, URL, or soulseek link — add as-is
            let album_mode = matches!(
                self.config.defaults.aggregation,
                crate::config::Aggregation::Album | crate::config::Aggregation::Artist
            );
            let id = self.next_id;
            self.next_id += 1;
            self.queue.push(QueueEntry {
                id,
                label: raw.clone(),
                input: raw,
                album_mode,
                status: DownloadStatus::Queued,
            });
        }
    }

    fn add_track_entry(&mut self, track: &Track) {
        use crate::config::Aggregation;
        let album_mode = matches!(
            self.config.defaults.aggregation,
            Aggregation::Album | Aggregation::Artist
        );
        let input = match self.config.defaults.aggregation {
            Aggregation::Song => format!("{} - {}", track.artist, track.title),
            Aggregation::Album | Aggregation::Artist => {
                let album = track.album.as_deref().unwrap_or(&track.title);
                format!("{} - {}", track.artist, album)
            }
        };
        let label = format!(
            "{} — {}{}",
            track.artist,
            track.title,
            track.album.as_deref().map(|a| format!(" ({})", a)).unwrap_or_default()
        );
        let id = self.next_id;
        self.next_id += 1;
        self.queue.push(QueueEntry {
            id,
            label,
            input,
            album_mode,
            status: DownloadStatus::Queued,
        });
    }

    /// Delete the currently selected queue entry (only if not running).
    pub fn delete_selected(&mut self) {
        if self.is_running || self.queue.is_empty() {
            return;
        }
        self.queue.remove(self.selected_index);
        if self.selected_index > 0 && self.selected_index >= self.queue.len() {
            self.selected_index = self.queue.len().saturating_sub(1);
        }
    }

    pub fn commit_setup(&mut self) -> anyhow::Result<()> {
        self.config.soulseek.username = self.setup_username.clone();
        self.config.soulseek.password = self.setup_password.clone();
        self.config.soulseek.sldl_path = self.setup_sldl_path.clone();
        self.config.defaults.output_path = self.setup_output_path.clone();
        self.config.save(&self.config_path)?;
        self.config_editor_content = toml::to_string_pretty(&self.config).unwrap_or_default();
        Ok(())
    }

    pub fn save_config_editor(&mut self) {
        match toml::from_str::<Config>(&self.config_editor_content) {
            Ok(parsed) => {
                self.config = parsed;
                match self.config.save(&self.config_path) {
                    Ok(()) => {
                        self.config_save_message =
                            Some(format!("saved to {}", self.config_path.display()));
                    }
                    Err(e) => {
                        self.config_save_message = Some(format!("save failed: {e}"));
                    }
                }
            }
            Err(e) => {
                self.config_save_message = Some(format!("toml error: {e}"));
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
                    if !self.queue.iter().any(|e| e.id == item_id) {
                        self.queue.push(QueueEntry {
                            id: item_id,
                            label: label.clone(),
                            input: label,
                            album_mode: false,
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

#[derive(Debug, Default)]
pub struct SummaryCounts {
    pub done: usize,
    pub settled: usize,
    pub failed: usize,
    pub queued: usize,
    pub in_progress: usize,
}
