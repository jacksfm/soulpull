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

### prerequisites

- [Rust](https://rustup.rs/) (stable)
- [sldl](https://github.com/fiso64/slsk-batchdl/releases) — must be on your PATH or configured via `sldl_path`
- A Soulseek account

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

soulpull looks for its config at:

| Platform | Path |
|----------|------|
| Windows  | `%APPDATA%\soulpull\config.toml` |
| macOS    | `~/Library/Application Support/soulpull/config.toml` |
| Linux    | `~/.config/soulpull/config.toml` |

Copy the example config to get started:

```bash
# windows
mkdir %APPDATA%\soulpull
copy config.example.toml %APPDATA%\soulpull\config.toml

# unix
mkdir -p ~/.config/soulpull
cp config.example.toml ~/.config/soulpull/config.toml
```

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

```bash
# open the TUI with a CSV input file
soulpull my-list.csv

# open the TUI with no input (add entries manually later)
soulpull
```

### CSV format

```csv
artist,title,album,length
Daft Punk,Get Lucky,Random Access Memory,248
Boards of Canada,Roygbiv,Music Has the Right to Children,173
```

`album` and `length` (seconds) are optional but improve MusicBrainz resolution accuracy.

---

## TUI keybinds

| key | action |
|-----|--------|
| `j` / `↓` | move down |
| `k` / `↑` | move up |
| `r` / `Enter` | start downloads |
| `c` | open config view |
| `s` | open summary view |
| `q` / `Esc` | quit |
| `Ctrl+C` | force quit |

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
