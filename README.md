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

# List available tracks/formats
musicfree -F "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Choose an output directory / filename
musicfree -d ./music -o song "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Download audio and cover
musicfree -d ./music -c --cover-dir ./covers "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Select playlist items (1-based, supports ranges)
musicfree -d ./music -I 1,3-5 "https://www.youtube.com/playlist?list=..."

# Use a Netscape-format cookies.txt file
musicfree --cookies cookies.txt "https://www.bilibili.com/video/BV..."
```

### YouTube strategies

```bash
musicfree --auto   "https://www.youtube.com/watch?v=BnnbP7pCIvQ" # mobile, then web (default)
musicfree --mobile "https://www.youtube.com/watch?v=BnnbP7pCIvQ" # Android player API
musicfree --web    "https://www.youtube.com/watch?v=BnnbP7pCIvQ" # web API + EJS challenges
```

`--web` falls back through the clients that currently expose direct URLs when the
desktop WEB client only returns SABR metadata.

## Library layout

- `src/model.rs` — `Site`, `AudioFormat`, `Track`, `Playlist`
- `src/http.rs` — HTTP client, cookies, ranged downloads
- `src/sites/` — one module per source
  - `bilibili/` — API, URL parsing, extraction
  - `youtube/` — InnerTube clients, EJS solver, mobile/web download paths
  - `file.rs` — direct HTTP(S) audio URLs
- `src/cli/` — argument parsing, output, selection, file writing

Adding a source means adding a module under `src/sites/`, implementing
`Extractor`, and registering it in `src/sites/mod.rs`.

## Acknowledgments

- [Rust](https://github.com/rust-lang/rust)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [MusicFree](https://github.com/maotoumao/MusicFree)
