use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

use super::Track;

/// Raw CSV row — flexible enough to handle optional fields.
#[derive(Debug, Deserialize)]
struct CsvRow {
    artist: String,
    title: String,
    #[serde(default)]
    album: String,
    /// Length in seconds (named "length" in the CSV)
    #[serde(rename = "length", default)]
    length_secs: Option<u32>,
}

pub fn parse_csv(path: &Path) -> Result<Vec<Track>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("Failed to open CSV: {}", path.display()))?;

    let mut tracks = Vec::new();

    for (i, result) in reader.deserialize::<CsvRow>().enumerate() {
        let row = result.with_context(|| format!("CSV parse error on row {}", i + 2))?;

        if row.artist.is_empty() || row.title.is_empty() {
            tracing::warn!("Skipping CSV row {} — missing artist or title", i + 2);
            continue;
        }

        tracks.push(Track {
            artist: row.artist,
            title: row.title,
            album: if row.album.is_empty() { None } else { Some(row.album) },
            length_secs: row.length_secs,
        });
    }

    tracing::info!("Parsed {} tracks from {}", tracks.len(), path.display());
    Ok(tracks)
}
