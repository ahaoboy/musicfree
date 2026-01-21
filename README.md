# MusicFree

<div align="center">
  <img src="https://github.com/ahaoboy/musicfree-tauri/blob/main/public/icon.png?raw=true" alt="MusicFree Icon" width="256" height="256">
</div>

<br>

> [!WARNING]
> This project is currently in the development phase. Breaking changes may occur at any time.

## Desktop UI Version

This repository contains the core logic and command-line interface. For the desktop application with a graphical user interface, please visit:
[https://github.com/ahaoboy/musicfree-tauri](https://github.com/ahaoboy/musicfree-tauri)

## CLI Usage

The `musicfree` command-line tool allows you to extract and download audio from various platforms (like Bilibili and YouTube).

### Basic Usage

```bash
# Download audio from a URL
musicfree "https://www.youtube.com/watch?v=BnnbP7pCIvQ"
```

### Options

```bash
# Show information only (no download)
musicfree -i "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Download to a specific directory
musicfree -d ./music "https://www.youtube.com/watch?v=BnnbP7pCIvQ"

# Specify custom output filename (works for single file)
musicfree -o song.mp3 "https://www.youtube.com/watch?v=BnnbP7pCIvQ"
```

## Acknowledgments

Special thanks to the following open source projects:

- [Rust](https://github.com/rust-lang/rust)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [MusicFree](https://github.com/maotoumao/MusicFree)
