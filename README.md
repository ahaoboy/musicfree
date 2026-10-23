# MusicFree

<div align="center">
  <img src="https://github.com/ahaoboy/musicfree-tauri/blob/main/public/icon.png?raw=true" alt="MusicFree Icon" width="256">
</div>

<br>

> [!WARNING]
> This project is in active development. APIs and CLI flags may change.

This repository contains the core extraction library and CLI. The desktop UI lives at
[musicfree-tauri](https://github.com/ahaoboy/musicfree-tauri).

## Usage

```bash
# Download a single track
musicfree "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Show metadata only
musicfree -i "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# List every downloadable format, including size, bitrate and resolution
musicfree -F "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Download a specific format shown by -F
musicfree -f 137 "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Download the best video format (may be video-only)
musicfree --video "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Choose an output directory / filename
musicfree -d ./music -o song "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Download audio and cover
musicfree -d ./music -c --cover-dir ./covers "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Select playlist items (1-based, supports ranges)
musicfree -d ./music -I 1,3-5 "https://www.youtube.com/playlist?list=..."

# Use a Netscape-format cookies.txt file
musicfree --cookies cookies.txt "https://www.bilibili.com/video/BV..."
```

`--video` selects the highest-resolution video stream. Adaptive video streams are
often video-only; use `-F` to find a muxed `video+audio` format and pass its ID with
`-f/--format` when audio is required.

### YouTube strategies

```bash
musicfree --auto   "https://www.youtube.com/watch?v=BnnbP7pCIvQ" # mobile, then web (default)
musicfree --mobile "https://www.youtube.com/watch?v=BnnbP7pCIvQ" # Android player API
musicfree --web    "https://www.youtube.com/watch?v=BnnbP7pCIvQ" # web API + EJS challenges
```

`--web` falls back through the clients that currently expose direct URLs when the
desktop WEB client only returns SABR metadata.

## Cargo features

Sites are independent features. The default includes the CLI and every site, while
library users can compile only the sources they need:

```bash
# Only YouTube + CLI
cargo build --no-default-features --features cli,youtube

# Only Bilibili + YouTube (library, no CLI)
cargo build --no-default-features --features bilibili,youtube

# Only direct file URLs
cargo build --no-default-features --features cli,file
```

- `sites` = `bilibili` + `youtube` + `file`
- `bilibili`, `youtube`, `file` — individual site support
- `cli` — the `musicfree` binary and its CLI dependencies
- `bindgen` — enables bindgen for the QuickJS runtime used by YouTube

Site-specific dependencies (`abv`, `serde-youtube`, `ytdlp-ejs`, `md5`, etc.) are
optional and enabled only by the corresponding site feature.

## Library layout

- `src/model.rs` — `Site`, `AudioFormat`, `Track`, `Playlist`
- `src/http.rs` — HTTP client, cookies, ranged downloads
- `src/sites/` — one module per source
  - `bilibili/` — API, URL parsing, extraction
  - `youtube/` — InnerTube clients, EJS solver, mobile/web download paths
  - `file.rs` — direct HTTP(S) audio URLs
- `src/cli/` — argument parsing, output, selection, file writing

Adding a source means adding a Cargo feature, a module under `src/sites/`,
implementing `Extractor`, and registering the feature-gated module/variant in
`src/sites/mod.rs`, `src/model.rs`, and `src/lib.rs`.

## Acknowledgments

- [Rust](https://github.com/rust-lang/rust)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [MusicFree](https://github.com/maotoumao/MusicFree)
