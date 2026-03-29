pub mod runner;

use serde::{Deserialize, Serialize};

/// Aggregated download state for one queue entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    Queued,
    Searching,
    Downloading { progress_pct: u8 },
    Done { format: String },
    /// Got a lower-priority format than preferred
    Settled { wanted: String, received: String },
    Failed { reason: String },
}

impl DownloadStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Searching => "searching",
            Self::Downloading { .. } => "downloading",
            Self::Done { .. } => "done",
            Self::Settled { .. } => "settled",
            Self::Failed { .. } => "failed",
        }
    }
}

/// Events the runner pushes back to the TUI over an mpsc channel.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    StatusChanged { item_id: usize, status: DownloadStatus },
    /// A human-readable log line (errors, warnings from sldl stderr)
    Log { item_id: usize, message: String },
}

// ---------------------------------------------------------------------------
// sldl NDJSON progress event shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SldlEvent {
    #[serde(rename = "type")]
    pub kind: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressData {
    pub percent: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackStateData {
    pub state: String,
    pub failure_reason: Option<String>,
    pub extension: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultData {
    pub chosen_file: Option<ChosenFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChosenFile {
    pub extension: Option<String>,
    pub bit_rate: Option<u32>,
    pub sample_rate: Option<u32>,
}
