# soulpull

```
░██████╗░█████╗░██╗░░░██╗██╗░░░░░██████╗░██╗░░░██╗██╗░░░░░██╗░░░░░
██╔════╝██╔══██╗██║░░░██║██║░░░░░██╔══██╗██║░░░██║██║░░░░░██║░░░░░
╚█████╗░██║░░██║██║░░░██║██║░░░░░██████╔╝██║░░░██║██║░░░░░██║░░░░░
░╚═══██╗██║░░██║██║░░░██║██║░░░░░██╔═══╝░██║░░░██║██║░░░░░██║░░░░░
██████╔╝╚█████╔╝╚██████╔╝███████╗██║░░░░░╚██████╔╝███████╗███████╗
╚═════╝░░╚════╝░░╚═════╝░╚══════╝╚═╝░░░░░░╚═════╝░╚══════╝╚══════╝
```

> a terminal-based music acquisition tool for the soulseek peer-to-peer network.
> give it a list. get your music. no gui required.

---

## what it does

soulpull takes a list of music you want and downloads it from soulseek — all inside a clean, real-time terminal UI. point it at a CSV, a spotify playlist, a youtube playlist, a search string, whatever. configure your format preferences once, press `r`, and walk away.

```
artist,title,album,length
Daft Punk,Get Lucky,Random Access Memory,248
Massive Attack,Teardrop,Mezzanine,330
Portishead,Glory Box,Dummy,342
```

---

## features

- **real-time TUI** — live download queue with per-track status, progress %, and format received
- **flexible input** — CSV, spotify, youtube, musicbrainz URLs, search strings, direct soulseek links
- **format preferences** — prefer flac hi-res, fall back to mp3 320, settle gracefully and report it
- **aggregation modes** — download a single song, the full album, or an entire discography
- **concurrent downloads** — configurable parallelism, all streaming progress back to the UI
- **inline config editor** — edit and save your config without leaving the terminal
- **first-run setup screen** — enter credentials on first launch, no config file editing required
- **portable** — config lives next to the binary, no appdata/hidden folders
- **summary view** — post-run breakdown: done / settled (got lower quality) / failed with reasons
- **single static binary** — no runtime, no installer

> **powered by [sldl](https://github.com/fiso64/slsk-batchdl)** under the hood for the soulseek protocol layer. native protocol implementation is on the roadmap.

---

## installation

### option 1 — download a release (no rust required)

1. grab the latest binary for your platform from the [releases page](https://github.com/jacksfm/soulpull/releases)
2. grab [sldl](https://github.com/fiso64/slsk-batchdl/releases) and place `sldl.exe` (or `sldl`) **in the same folder** as `soulpull`, or anywhere on your PATH
3. run `soulpull` — a setup screen appears on first launch to enter your soulseek credentials
4. your config is saved as `soulpull.toml` right next to the binary

that's it.

### option 2 — build from source

you'll need [rust](https://rustup.rs) installed.

```bash
git clone https://github.com/jacksfm/soulpull
cd soulpull
cargo build --release
```

the binary ends up at `target/release/soulpull` (or `soulpull.exe` on windows).

to run directly without building first:

```bash
cargo run                          # open TUI with no input
cargo run -- example.csv           # load a CSV
cargo run -- "Artist - Album"      # search string
cargo run -- https://open.spotify.com/album/...
```

sldl still needs to be available — either drop `sldl.exe` in the repo folder or put it on your PATH. the TUI, setup screen, and config editor all work without sldl. you only need it when you actually press `r` to start downloading.

### cross-compile targets

```bash
# linux x86_64 (static musl)
cargo build --release --target x86_64-unknown-linux-musl

# macos apple silicon
cargo build --release --target aarch64-apple-darwin

# windows
cargo build --release --target x86_64-pc-windows-msvc
```

---

## configuration

on first run with no config, soulpull shows a setup screen. fill in your credentials, hit enter, and you're in. the config saves as `soulpull.toml` next to the binary.

**config lookup order:**
1. `soulpull.toml` next to the executable ← this is where it saves by default
2. `soulpull.toml` in the current working directory
3. platform config dir (`%APPDATA%\soulpull\`, `~/.config/soulpull/`, etc.)

press `c` in the queue view to open the inline editor. `ctrl+s` saves to disk.

### full config reference

```toml
[defaults]
aggregation = "album"            # song | album | artist
release_preference = "original"  # original | latest | ask
output_path = "~/Music/soulpull"
max_concurrent_downloads = 2     # how many downloads run at once

[format]
preferred = ["flac", "mp3"]      # try these formats, in order
fallback = []                    # also accepted, but not preferred

preferred_min_samplerate = 96000 # prefer hi-res flac (hz)
preferred_min_bitrate = 320      # prefer 320kbps if no flac (kbps)
min_bitrate = 128                # hard floor — reject anything below this

[filters]
# skip these release types when resolving via musicbrainz
exclude_release_types = ["live", "remix", "anniversary", "compilation"]

[soulseek]
username = "your_username"
password = "your_password"
sldl_path = "sldl"               # "sldl" if on PATH, otherwise full path to the binary
listen_port = 49998
search_timeout_ms = 6000
max_stale_time_ms = 30000
```

---

## usage

soulpull accepts anything sldl accepts — it passes your input straight through:

```bash
# csv file
soulpull my-list.csv

# search string — wraps in album mode automatically
soulpull "Daft Punk - Random Access Memory"

# spotify playlist or album
soulpull https://open.spotify.com/playlist/xxxxx
soulpull https://open.spotify.com/album/xxxxx

# youtube playlist
soulpull https://www.youtube.com/playlist?list=xxxxx

# musicbrainz release, release group, or collection
soulpull https://musicbrainz.org/release/xxxxx
soulpull https://musicbrainz.org/release-group/xxxxx
soulpull https://musicbrainz.org/collection/xxxxx

# direct soulseek link
soulpull slsk://username/path/to/folder/

# no input — opens TUI, configure from the setup/config screen
soulpull
```

### csv format

```csv
artist,title,album,length
Daft Punk,Get Lucky,Random Access Memory,248
Boards of Canada,Roygbiv,Music Has the Right to Children,173
The Knife,Silent Shout,,
```

`album` and `length` (seconds) are optional. rows with no title are treated as album downloads.

---

## keybinds

### queue view
| key | action |
|-----|--------|
| `j` / `↓` | move down |
| `k` / `↑` | move up |
| `r` / `enter` | start downloads |
| `c` | open config editor |
| `s` | open summary view |
| `q` / `esc` | quit |
| `ctrl+c` | force quit |

### config editor
| key | action |
|-----|--------|
| `ctrl+s` | save to disk |
| `esc` | back to queue (discards unsaved changes) |

### setup screen (first run)
| key | action |
|-----|--------|
| `tab` | next field |
| `enter` | save and continue |
| `space` | toggle password visibility |
| `esc` | skip for now |

---

## status indicators

```
·  queued       waiting to start
?  searching    looking for matches on soulseek
⇣  downloading  transfer in progress  [67%]
✓  done         downloaded at preferred quality  [FLAC]
~  settled      downloaded at lower quality than preferred  [MP3]
✗  failed       nothing found, or transfer died
```

---

## views

### queue view
the main screen. every track/album with its live status and progress. updates in real time as sldl streams events back.

```
┌─ soulpull — Queue [j/k navigate | r run | c config | s summary | q quit] ──┐
│▶ ✓  Daft Punk — Random Access Memory                              [FLAC]    │
│  ⇣  Massive Attack — Mezzanine                                    [ 67%]    │
│  ·  Portishead — Dummy                                                       │
│  ✗  Unknown Artist — Ghost Track              [no results found]            │
└─────────────────────────────────────────────────────────────────────────────┘
 ✓ 1 done  ~ 0 settled  ✗ 1 failed  · 1 queued  ⇣ 1 active
```

### summary view
press `s` anytime to see the post-run breakdown.

```
┌─ Results ───────────────────────────────────────────────────────────────────┐
│  ✓ done:       8                                                             │
│  ~ settled:    2                                                             │
│  ✗ failed:     1                                                             │
│    total:      11                                                            │
└─────────────────────────────────────────────────────────────────────────────┘
┌─ Per-Item Breakdown ────────────────────────────────────────────────────────┐
│ ✓  Daft Punk — Random Access Memory [FLAC]                                  │
│ ~  Massive Attack — Mezzanine [wanted: FLAC, got: MP3]                      │
│ ✗  Unknown Artist — Ghost Track — no results found                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

### config editor
press `c` to edit your `soulpull.toml` inline. `ctrl+s` validates and saves. shows a green confirmation or a red parse error if your toml is broken.

---

## architecture

```
src/
├── main.rs              entry point — cli args, log-to-file, hands off to tui
├── config.rs            toml config, portable path discovery, save/load
├── sources/
│   └── csv.rs           csv → vec<track>
├── resolver/
│   ├── musicbrainz.rs   musicbrainz api client (rate-limited, 1 req/s)
│   └── aggregator.rs    expand a track into song / album / discography
├── slsk/
│   ├── mod.rs           downloadstatus, downloadevent, sldl ndjson shapes
│   └── runner.rs        build sldl commands · spawn subprocess · parse ndjson → tui
└── tui/
    ├── mod.rs           terminal setup/teardown · 50ms event loop · dispatch downloads
    ├── app.rs           all app state — queue, config, setup fields, event channel
    ├── events.rs        keybind dispatch (press-only, no double-fire on windows)
    └── views/
        ├── setup.rs     first-run credential setup screen
        ├── queue.rs     live download queue
        ├── summary.rs   post-run breakdown
        └── config.rs    inline toml editor with ctrl+s save
```

---

## roadmap

- [ ] native soulseek protocol implementation (no sldl dependency)
- [ ] musicbrainz resolution wired to download dispatch
- [ ] artist discography mode
- [ ] skip-existing logic (don't re-download files already in output folder)
- [ ] `--dry-run` flag — print what would be downloaded without transferring
- [ ] scroll in config editor and queue view

---

## credits

- [sldl](https://github.com/fiso64/slsk-batchdl) by fiso64 — soulseek protocol and download engine
- [ratatui](https://ratatui.rs) — terminal UI framework
- [musicbrainz](https://musicbrainz.org) — open music metadata

---

## license

mit
