# soulpull

```
░██████╗░█████╗░██╗░░░██╗██╗░░░░░██████╗░██╗░░░██╗██╗░░░░░██╗░░░░░
██╔════╝██╔══██╗██║░░░██║██║░░░░░██╔══██╗██║░░░██║██║░░░░░██║░░░░░
╚█████╗░██║░░██║██║░░░██║██║░░░░░██████╔╝██║░░░██║██║░░░░░██║░░░░░
░╚═══██╗██║░░██║██║░░░██║██║░░░░░██╔═══╝░██║░░░██║██║░░░░░██║░░░░░
██████╔╝╚█████╔╝╚██████╔╝███████╗██║░░░░░╚██████╔╝███████╗███████╗
╚═════╝░░╚════╝░░╚═════╝░╚══════╝╚═╝░░░░░░╚═════╝░╚══════╝╚══════╝
```

> **A terminal-based music acquisition tool for the Soulseek peer-to-peer network.**
> Give it a CSV. Get your music. No GUI required.

---

## what it does

soulpull takes a list of music you want, figures out the best available files on Soulseek, and downloads them — all inside a clean, real-time terminal UI. configure your format preferences once, point it at a CSV, press `r`, and walk away.

```
artist,title,album,length
Daft Punk,Get Lucky,Random Access Memory,248
Massive Attack,Teardrop,Mezzanine,330
Portishead,Glory Box,Dummy,342
```

soulpull handles the rest.

---

## features

- **real-time TUI** — live download queue with per-track status, progress %, and format received
- **format priority chain** — prefer FLAC hi-res, fall back to MP3 320, settle gracefully and report it
- **aggregation modes** — download a single song, the full album, or an entire artist discography from one input line
- **MusicBrainz resolution** — canonicalize artist/album metadata, filter out live albums, remixes, compilations
- **sldl backend** — powered by [sldl](https://github.com/fiso64/slsk-batchdl) under the hood; no Soulseek daemon required
- **concurrent downloads** — configurable parallelism, all streaming progress back to the UI
- **summary view** — post-run breakdown of done / settled (got lower quality than wanted) / failed with reasons
- **inline config editor** — tweak your TOML without leaving the terminal
- **single static binary** — no runtime dependencies, cross-compiles for Linux, macOS, Windows

---

## installation

### download a binary (no Rust required)

Grab the latest release from the [releases page](https://github.com/jacksfm/soulpull/releases).

You also need [sldl](https://github.com/fiso64/slsk-batchdl/releases) — place `sldl.exe` next to `soulpull.exe`, or anywhere on your PATH.

That's it. Run `soulpull` and a setup screen will appear on first run to enter your Soulseek credentials. Your config is saved as `soulpull.toml` in the same folder as the binary.

### build from source

```bash
git clone https://github.com/jacksfm/soulpull
cd soulpull
cargo build --release
./target/release/soulpull
```

### cross-compile targets

```bash
# Linux x86_64
cargo build --release --target x86_64-unknown-linux-musl

# macOS Apple Silicon
cargo build --release --target aarch64-apple-darwin

# Windows
cargo build --release --target x86_64-pc-windows-gnu
```

---

## configuration

On first run with no config, soulpull shows a setup screen where you enter your credentials. The config is then saved as `soulpull.toml` **next to the binary** — portable, no AppData hunting.

Config lookup order:
1. `soulpull.toml` next to the executable ← preferred, portable
2. `soulpull.toml` in the current working directory
3. Platform config dir (`%APPDATA%\soulpull\`, `~/.config/soulpull/`, etc.)

Press `c` in the queue view to open the inline editor. **Ctrl+S** saves to disk.

### full config reference

```toml
[defaults]
aggregation = "album"            # song | album | artist
release_preference = "original"  # original | latest | ask
output_path = "~/Music/soulpull"
max_concurrent_downloads = 2

[format]
# sldl tries these in order — first match wins
preferred = ["flac", "mp3"]
fallback = []

# preferred quality (sldl works hardest to satisfy these)
preferred_min_samplerate = 96000   # Hz — prefer hi-res FLAC
preferred_min_bitrate = 320        # kbps — prefer 320kbps if no FLAC

# hard floor — files below these are rejected outright
min_bitrate = 128

[filters]
# skip these MusicBrainz release types when resolving albums
exclude_release_types = ["live", "remix", "anniversary", "compilation"]

[soulseek]
username = "your_username"
password = "your_password"
sldl_path = "sldl"               # or full path to sldl binary
listen_port = 49998
search_timeout_ms = 6000
max_stale_time_ms = 30000
```

---

## usage

soulpull accepts anything sldl accepts as input:

```bash
# CSV file
soulpull my-list.csv

# Search string (album mode)
soulpull "Daft Punk - Random Access Memory"

# Spotify playlist or album
soulpull https://open.spotify.com/playlist/xxxxx
soulpull https://open.spotify.com/album/xxxxx

# YouTube playlist
soulpull https://www.youtube.com/playlist?list=xxxxx

# MusicBrainz release, release group, or collection
soulpull https://musicbrainz.org/release/xxxxx
soulpull https://musicbrainz.org/release-group/xxxxx

# Direct Soulseek link
soulpull slsk://username/path/to/folder/

# No input — opens TUI, use config view to set up
soulpull
```

### CSV format

```csv
artist,title,album,length
Daft Punk,Get Lucky,Random Access Memory,248
Boards of Canada,Roygbiv,Music Has the Right to Children,173
```

`album` and `length` (seconds) are optional. Rows without a title are treated as album downloads.

---

## TUI keybinds

### queue view
| key | action |
|-----|--------|
| `j` / `↓` | move down |
| `k` / `↑` | move up |
| `r` / `Enter` | start downloads |
| `c` | open config editor |
| `s` | open summary view |
| `q` / `Esc` | quit |
| `Ctrl+C` | force quit |

### config editor
| key | action |
|-----|--------|
| `Ctrl+S` | save to disk |
| `Esc` | back to queue |

### setup screen (first run)
| key | action |
|-----|--------|
| `Tab` | next field |
| `Enter` | save and continue |
| `Space` | toggle password visibility |
| `Esc` | skip (can configure later) |

---

## status indicators

```
·  queued       — waiting to start
?  searching    — sldl is searching Soulseek
⇣  downloading  — transfer in progress  [xx%]
✓  done         — downloaded at preferred quality  [FLAC]
~  settled      — downloaded at lower quality than wanted  [MP3]
✗  failed       — could not find or download
```

---

## views

### queue view
the main screen. shows every track/album with its live status and progress. updates in real time as sldl streams progress events back.

```
┌─ soulpull — Queue [j/k navigate | r run | c config | s summary | q quit] ──┐
│ ✓  Daft Punk — Random Access Memory                              [FLAC]     │
│ ⇣  Massive Attack — Mezzanine                                    [ 67%]     │
│ ·  Portishead — Dummy                                                        │
│ ✗  Unknown Artist — Ghost Track              [no results found]             │
└─────────────────────────────────────────────────────────────────────────────┘
 ✓ 1 done  ~ 0 settled  ✗ 1 failed  · 1 queued  ⇣ 1 active
```

### summary view
post-run breakdown. press `s` to switch to it anytime.

```
┌─ Results ───────────────────────────────────────────────────────────────────┐
│  ✓ Done:       8                                                             │
│  ~ Settled:    2                                                             │
│  ✗ Failed:     1                                                             │
│    Total:      11                                                            │
└─────────────────────────────────────────────────────────────────────────────┘
┌─ Per-Item Breakdown ────────────────────────────────────────────────────────┐
│ ✓  Daft Punk — Random Access Memory [FLAC]                                  │
│ ~  Massive Attack — Mezzanine [wanted: FLAC, got: MP3]                      │
│ ✗  Unknown Artist — Ghost Track — no results found                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## architecture

```
soulpull/
├── src/
│   ├── main.rs              entry point, CLI args, log → file
│   ├── config.rs            TOML config loading and structs
│   ├── sources/
│   │   └── csv.rs           CSV → Vec<Track>
│   ├── resolver/
│   │   ├── musicbrainz.rs   MusicBrainz API client (rate-limited)
│   │   └── aggregator.rs    expands tracks into song / album / discography
│   ├── slsk/
│   │   ├── mod.rs           DownloadStatus, DownloadEvent, sldl JSON shapes
│   │   └── runner.rs        builds sldl commands, streams NDJSON → TUI events
│   └── tui/
│       ├── mod.rs           terminal setup, event loop
│       ├── app.rs           central app state
│       ├── events.rs        keybind dispatch
│       └── views/
│           ├── queue.rs     live download queue
│           ├── summary.rs   post-run breakdown
│           └── config.rs    inline TOML editor
```

soulpull delegates all Soulseek protocol work to [sldl](https://github.com/fiso64/slsk-batchdl). it builds the right `sldl` invocation from your config, spawns it as a subprocess, and parses its `--progress-json` NDJSON output stream to drive the TUI. no Soulseek daemon, no separate server process.

---

## roadmap

- [ ] MusicBrainz resolution wired to download dispatch (canonical album lookup before searching)
- [ ] Artist discography mode fully wired
- [ ] Spotify / YouTube playlist as input source
- [ ] Native Soulseek protocol implementation (replace sldl subprocess)
- [ ] Config editor persists changes to disk
- [ ] `--dry-run` mode (print what would be downloaded, no transfers)
- [ ] Skip-existing logic (don't re-download files already in output_path)

---

## credits

- [sldl](https://github.com/fiso64/slsk-batchdl) by fiso64 — the actual Soulseek download engine
- [ratatui](https://ratatui.rs) — terminal UI framework
- [MusicBrainz](https://musicbrainz.org) — open music metadata database

---

## license

MIT
