use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use tokio::time::{sleep, Duration};

use crate::config::{Filters, ReleasePreference};
use crate::sources::Track;
use super::{Release, ResolvedTrack};

const MB_BASE: &str = "https://musicbrainz.org/ws/2";
const USER_AGENT: &str = concat!(
    "soulpull/", env!("CARGO_PKG_VERSION"),
    " ( https://github.com/soulpull/soulpull )"
);

// MusicBrainz rate-limit: max 1 req/second
const RATE_LIMIT_MS: u64 = 1100;

pub struct MusicBrainzClient {
    client: Client,
}

impl MusicBrainzClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self { client })
    }

    /// Throttled GET helper — ensures we stay within MB rate limits.
    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        sleep(Duration::from_millis(RATE_LIMIT_MS)).await;
        tracing::debug!("MB GET {}", url);
        let resp = self.client.get(url).send().await
            .with_context(|| format!("HTTP request failed: {url}"))?;

        if !resp.status().is_success() {
            bail!("MusicBrainz returned {}: {}", resp.status(), url);
        }
        resp.json::<T>().await
            .with_context(|| format!("Failed to parse MB JSON from {url}"))
    }

    /// Resolve a Track to a canonical Release (and its track list).
    pub async fn resolve_track(
        &self,
        track: &Track,
        release_pref: ReleasePreference,
        filters: &Filters,
    ) -> Result<Release> {
        // Step 1: recording search
        let recording = self.search_recording(track).await?;
        tracing::debug!("Resolved recording MBID: {}", recording.id);

        // Step 2: get all releases for this recording
        let releases = self.releases_for_recording(&recording.id, filters).await?;
        if releases.is_empty() {
            bail!("No releases found for '{}' by '{}'", track.title, track.artist);
        }

        // Step 3: pick release according to preference
        let chosen = pick_release(releases, release_pref);
        Ok(chosen)
    }

    /// Full artist discography
    pub async fn artist_releases(
        &self,
        artist_mbid: &str,
        filters: &Filters,
    ) -> Result<Vec<Release>> {
        let url = format!(
            "{MB_BASE}/release?artist={artist_mbid}&inc=recordings&fmt=json&limit=100"
        );
        let resp: MbReleaseListResponse = self.get_json(&url).await?;
        let releases = resp.releases.into_iter()
            .filter(|r| !is_excluded_type(r.release_group.as_ref().and_then(|rg| rg.primary_type.as_deref()), filters))
            .map(mb_release_to_release)
            .collect();
        Ok(releases)
    }

    async fn search_recording(&self, track: &Track) -> Result<MbRecording> {
        let query = build_recording_query(track);
        let url = format!("{MB_BASE}/recording?query={query}&limit=5&fmt=json");
        let resp: MbRecordingSearchResponse = self.get_json(&url).await?;

        resp.recordings
            .into_iter()
            .next()
            .with_context(|| format!("No recording found for '{}' by '{}'", track.title, track.artist))
    }

    async fn releases_for_recording(
        &self,
        recording_mbid: &str,
        filters: &Filters,
    ) -> Result<Vec<Release>> {
        let url = format!(
            "{MB_BASE}/release?recording={recording_mbid}&inc=recordings+release-groups&fmt=json"
        );
        let resp: MbReleaseListResponse = self.get_json(&url).await?;
        let releases: Vec<Release> = resp.releases.into_iter()
            .filter(|r| {
                let rtype = r.release_group.as_ref().and_then(|rg| rg.primary_type.as_deref());
                !is_excluded_type(rtype, filters)
            })
            .map(mb_release_to_release)
            .collect();
        Ok(releases)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_recording_query(track: &Track) -> String {
    let title = urlencoding::encode(&track.title);
    let artist = urlencoding::encode(&track.artist);
    format!("recording:{title}%20AND%20artist:{artist}")
}

fn is_excluded_type(release_type: Option<&str>, filters: &Filters) -> bool {
    let Some(t) = release_type else { return false };
    filters.exclude_release_types.iter().any(|ex| ex.eq_ignore_ascii_case(t))
}

fn pick_release(mut releases: Vec<Release>, pref: ReleasePreference) -> Release {
    if releases.is_empty() {
        unreachable!("called with empty list");
    }
    releases.sort_by_key(|r| r.year.unwrap_or(u16::MAX));
    match pref {
        ReleasePreference::Original | ReleasePreference::Ask => releases.remove(0),
        ReleasePreference::Latest => releases.pop().unwrap(),
    }
}

fn mb_release_to_release(r: MbRelease) -> Release {
    let artist = r.artist_credit.as_ref()
        .and_then(|ac| ac.first())
        .map(|a| a.name.clone())
        .unwrap_or_default();
    let artist_mbid = r.artist_credit.as_ref()
        .and_then(|ac| ac.first())
        .and_then(|a| a.artist.as_ref())
        .map(|a| a.id.clone())
        .unwrap_or_default();
    let year = r.date.as_deref()
        .and_then(|d| d.split('-').next())
        .and_then(|y| y.parse().ok());
    let release_type = r.release_group.as_ref()
        .and_then(|rg| rg.primary_type.clone());

    // Pre-clone fields needed inside the flat_map closure so we can still
    // move them into the final Release struct below.
    let album_title = r.title.clone();
    let album_mbid_str = r.id.clone();
    let artist_clone = artist.clone();

    let tracks: Vec<ResolvedTrack> = r.media.unwrap_or_default()
        .into_iter()
        .enumerate()
        .flat_map(|(disc_i, media)| {
            let album_title = album_title.clone();
            let album_mbid_str = album_mbid_str.clone();
            let artist_clone = artist_clone.clone();
            media.tracks.unwrap_or_default()
                .into_iter()
                .map(move |t| ResolvedTrack {
                    mbid: Some(t.id.clone()),
                    artist: artist_clone.clone(),
                    title: t.title,
                    album: album_title.clone(),
                    album_mbid: album_mbid_str.clone(),
                    track_number: t.position,
                    disc_number: Some((disc_i + 1) as u32),
                    year,
                    length_ms: t.length,
                })
                .collect::<Vec<_>>()
        })
        .collect();

    Release {
        mbid: r.id,
        title: r.title,
        artist,
        artist_mbid,
        year,
        release_type,
        tracks,
    }
}

// ---------------------------------------------------------------------------
// MusicBrainz JSON shapes (minimal subset we need)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct MbRecordingSearchResponse {
    recordings: Vec<MbRecording>,
}

#[derive(Deserialize)]
struct MbRecording {
    id: String,
}

#[derive(Deserialize)]
struct MbReleaseListResponse {
    releases: Vec<MbRelease>,
}

#[derive(Deserialize, Clone)]
struct MbRelease {
    id: String,
    title: String,
    date: Option<String>,
    #[serde(rename = "artist-credit")]
    artist_credit: Option<Vec<MbArtistCredit>>,
    #[serde(rename = "release-group")]
    release_group: Option<MbReleaseGroup>,
    media: Option<Vec<MbMedia>>,
}

#[derive(Deserialize, Clone)]
struct MbArtistCredit {
    name: String,
    artist: Option<MbArtistRef>,
}

#[derive(Deserialize, Clone)]
struct MbArtistRef {
    id: String,
}

#[derive(Deserialize, Clone)]
struct MbReleaseGroup {
    #[serde(rename = "primary-type")]
    primary_type: Option<String>,
}

#[derive(Deserialize, Clone)]
struct MbMedia {
    tracks: Option<Vec<MbTrack>>,
}

#[derive(Deserialize, Clone)]
struct MbTrack {
    id: String,
    title: String,
    position: Option<u32>,
    length: Option<u32>,
}
