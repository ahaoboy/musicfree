# musicfree

A lightweight command-line tool to download audio from YouTube and Bilibili.

## Overview
musicfree is a Rust-based CLI that fetches audio from supported video sites and saves it as an `.m4a` file. The output filename is derived from the video's title and sanitized to be filesystem-friendly.

## Features
- Detects site from URL and downloads audio accordingly (YouTube or Bilibili).
- Saves audio as a single `.m4a` file named after the video title.
- Self-contained, async I/O with robust error handling.
- Optional YouTube extractor powered by EJS (feature flag `ejs`).
