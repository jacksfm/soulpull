pub mod csv;

use serde::{Deserialize, Serialize};

/// A single track as parsed from an input source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub artist: String,
    pub title: String,
    /// Album hint from input — may be empty
    pub album: Option<String>,
    /// Duration in seconds, if known
    pub length_secs: Option<u32>,
}
