use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level configuration loaded from the TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,

    #[serde(default)]
    pub format: FormatConfig,

    #[serde(default)]
    pub filters: Filters,

    #[serde(default)]
    pub soulseek: SoulseekConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    /// How to expand a single track: "song" | "album" | "artist"
    pub aggregation: Aggregation,

    /// Which release to prefer: "original" | "latest" | "ask"
    pub release_preference: ReleasePreference,

    /// Where to write downloaded files
    pub output_path: String,

    /// Max concurrent sldl invocations
    #[serde(default = "default_concurrency")]
    pub max_concurrent_downloads: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Aggregation {
    Song,
    #[default]
    Album,
    Artist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReleasePreference {
    #[default]
    Original,
    Latest,
    Ask,
}

fn default_concurrency() -> usize {
    2
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            aggregation: Aggregation::default(),
            release_preference: ReleasePreference::default(),
            output_path: "~/Music/soulpull".to_string(),
            max_concurrent_downloads: default_concurrency(),
        }
    }
}

/// Format preferences — map directly onto sldl CLI flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatConfig {
    /// Preferred formats, in priority order. Maps to `--pref-format`.
    /// e.g. ["flac", "mp3"]
    #[serde(default = "default_pref_formats")]
    pub preferred: Vec<String>,

    /// Also-acceptable formats beyond preferred. Combined with `preferred`
    /// to form the `--format` necessary-condition list.
    /// Leave empty to accept any format that passes quality gates.
    #[serde(default)]
    pub fallback: Vec<String>,

    /// Preferred minimum sample rate in Hz. Maps to `--pref-min-samplerate`.
    pub preferred_min_samplerate: Option<u32>,

    /// Preferred minimum bitrate in kbps. Maps to `--pref-min-bitrate`.
    pub preferred_min_bitrate: Option<u32>,

    /// Hard minimum bitrate (necessary condition). Maps to `--min-bitrate`.
    pub min_bitrate: Option<u32>,

    /// Hard minimum sample rate (necessary condition). Maps to `--min-samplerate`.
    pub min_samplerate: Option<u32>,
}

fn default_pref_formats() -> Vec<String> {
    vec!["flac".into(), "mp3".into()]
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            preferred: default_pref_formats(),
            fallback: vec![],
            preferred_min_samplerate: Some(96000),
            preferred_min_bitrate: None,
            min_bitrate: Some(128),
            min_samplerate: None,
        }
    }
}

impl FormatConfig {
    /// All accepted formats (preferred + fallback), deduplicated.
    pub fn all_formats(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        self.preferred
            .iter()
            .chain(self.fallback.iter())
            .filter_map(|f| {
                let s = f.as_str();
                if seen.insert(s) { Some(s) } else { None }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Filters {
    /// MusicBrainz release types to skip (e.g. "live", "remix", "compilation").
    #[serde(default)]
    pub exclude_release_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulseekConfig {
    pub username: String,
    pub password: String,

    /// Path to the sldl binary. Defaults to "sldl" (assumes it's on PATH).
    #[serde(default = "default_sldl_path")]
    pub sldl_path: String,

    /// Incoming connection port passed to sldl via --listen-port.
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,

    /// Search timeout in ms passed to sldl via --search-timeout.
    #[serde(default = "default_search_timeout")]
    pub search_timeout_ms: u32,

    /// Max stale download time in ms. Maps to --max-stale-time.
    #[serde(default = "default_stale_time")]
    pub max_stale_time_ms: u32,
}

fn default_sldl_path() -> String {
    "sldl".to_string()
}

fn default_listen_port() -> u16 {
    49998
}

fn default_search_timeout() -> u32 {
    6000
}

fn default_stale_time() -> u32 {
    30000
}

impl Default for SoulseekConfig {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            sldl_path: default_sldl_path(),
            listen_port: default_listen_port(),
            search_timeout_ms: default_search_timeout(),
            max_stale_time_ms: default_stale_time(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        toml::from_str(&raw)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(path, raw)
            .with_context(|| format!("Failed to write config file: {}", path.display()))
    }

    /// Expand `~/` in output_path to the user's home directory.
    pub fn resolved_output_path(&self) -> PathBuf {
        expand_tilde(&self.defaults.output_path)
    }
}

pub fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(p)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            defaults: Defaults::default(),
            format: FormatConfig::default(),
            filters: Filters::default(),
            soulseek: SoulseekConfig::default(),
        }
    }
}

/// Platform-appropriate config file location.
pub fn default_config_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("soulpull").join("config.toml")
    } else {
        PathBuf::from("soulpull.toml")
    }
}
