pub mod aggregator;
pub mod musicbrainz;

use serde::{Deserialize, Serialize};

/// A resolved release from MusicBrainz.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub mbid: String,
    pub title: String,
    pub artist: String,
    pub artist_mbid: String,
    pub year: Option<u16>,
    pub release_type: Option<String>,
    pub tracks: Vec<ResolvedTrack>,
}

/// A fully resolved track with its canonical metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedTrack {
    pub mbid: Option<String>,
    pub artist: String,
    pub title: String,
    pub album: String,
    pub album_mbid: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u16>,
    pub length_ms: Option<u32>,
}

/// Expanded work item — what soulpull actually goes and fetches.
#[derive(Debug, Clone)]
pub enum WorkItem {
    /// A single track
    Track(ResolvedTrack),
    /// A full album
    Album(Release),
    /// All albums by an artist (artist name + mbid, list of releases)
    Discography { artist: String, artist_mbid: String, releases: Vec<Release> },
}

impl WorkItem {
    pub fn display_label(&self) -> String {
        match self {
            WorkItem::Track(t) => format!("{} — {} ({})", t.artist, t.title, t.album),
            WorkItem::Album(r) => format!("{} — {}", r.artist, r.title),
            WorkItem::Discography { artist, releases, .. } => {
                format!("{} ({} releases)", artist, releases.len())
            }
        }
    }
}
