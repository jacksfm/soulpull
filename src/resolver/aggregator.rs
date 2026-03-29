use anyhow::Result;

use crate::config::{Aggregation, Filters, ReleasePreference};
use crate::sources::Track;
use super::{Release, ResolvedTrack, WorkItem};
use super::musicbrainz::MusicBrainzClient;

pub struct Aggregator {
    mb: MusicBrainzClient,
    aggregation: Aggregation,
    release_pref: ReleasePreference,
    filters: Filters,
}

impl Aggregator {
    pub fn new(
        aggregation: Aggregation,
        release_pref: ReleasePreference,
        filters: Filters,
    ) -> Result<Self> {
        Ok(Self {
            mb: MusicBrainzClient::new()?,
            aggregation,
            release_pref,
            filters,
        })
    }

    /// Resolve a single input track into one or more WorkItems.
    pub async fn expand(&self, track: &Track) -> Result<Vec<WorkItem>> {
        match self.aggregation {
            Aggregation::Song => {
                let release = self.mb.resolve_track(track, self.release_pref, &self.filters).await?;
                // Find the specific track within the resolved release
                let resolved = find_track_in_release(&release, &track.title)
                    .unwrap_or_else(|| stub_resolved_track(track, &release));
                Ok(vec![WorkItem::Track(resolved)])
            }

            Aggregation::Album => {
                let release = self.mb.resolve_track(track, self.release_pref, &self.filters).await?;
                Ok(vec![WorkItem::Album(release)])
            }

            Aggregation::Artist => {
                // Resolve one release to get the artist MBID
                let release = self.mb.resolve_track(track, self.release_pref, &self.filters).await?;
                let artist_mbid = release.artist_mbid.clone();
                let artist = release.artist.clone();

                let releases = self.mb.artist_releases(&artist_mbid, &self.filters).await?;
                Ok(vec![WorkItem::Discography {
                    artist,
                    artist_mbid,
                    releases,
                }])
            }
        }
    }
}

fn find_track_in_release(release: &Release, title: &str) -> Option<ResolvedTrack> {
    let title_lower = title.to_lowercase();
    release.tracks.iter()
        .find(|t| t.title.to_lowercase() == title_lower)
        .cloned()
}

fn stub_resolved_track(track: &Track, release: &Release) -> ResolvedTrack {
    ResolvedTrack {
        mbid: None,
        artist: track.artist.clone(),
        title: track.title.clone(),
        album: release.title.clone(),
        album_mbid: release.mbid.clone(),
        track_number: None,
        disc_number: None,
        year: release.year,
        length_ms: track.length_secs.map(|s| s * 1000),
    }
}
