mod files;
mod output;
mod selection;

use crate::{AudioFormat, Playlist, extract, http};
use anyhow::{Result, bail};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "musicfree",
    version,
    about = "Download audio from the enabled sources",
    long_about = "Extract and download audio tracks from the sources enabled at build time."
)]
struct Args {
    /// URL or site ID to extract audio from
    url: String,

    /// Download directory
    #[arg(short = 'd', long = "dir", value_name = "DIR")]
    dir: Option<PathBuf>,

    /// Output filename for a single track
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output: Option<PathBuf>,

    /// List formats without downloading
    #[arg(short = 'F', long = "list-formats")]
    list_formats: bool,

    /// Show information without downloading
    #[arg(short = 'i', long = "info", visible_alias = "no-download")]
    info: bool,

    /// Only keep tracks in this format
    #[arg(short = 'f', long = "format", value_name = "FORMAT", value_parser = parse_format)]
    format: Option<AudioFormat>,

    /// Download cover images too
    #[arg(short = 'c', long = "cover")]
    cover: bool,

    /// Directory for cover images (defaults to the audio directory)
    #[arg(long = "cover-dir", value_name = "DIR")]
    cover_dir: Option<PathBuf>,

    /// Select playlist items, e.g. "1,3-5"
    #[arg(short = 'I', long = "items", visible_alias = "playlist-items")]
    items: Option<String>,

    /// Netscape-format cookies.txt file
    #[arg(long = "cookies", value_name = "FILE")]
    cookies: Option<PathBuf>,

    /// YouTube: try mobile first, then web (default)
    #[cfg(feature = "youtube")]
    #[arg(long, group = "youtube-mode")]
    auto: bool,

    /// YouTube: use the web player API and EJS challenge solving
    #[cfg(feature = "youtube")]
    #[arg(long, group = "youtube-mode")]
    web: bool,

    /// YouTube: use the mobile (Android) player API
    #[cfg(feature = "youtube")]
    #[arg(long, group = "youtube-mode")]
    mobile: bool,
}

/// YouTube strategy selected by the CLI, independent of the optional site.
#[derive(Debug, Clone, Copy, Default)]
enum YoutubeMode {
    #[default]
    Auto,
    #[cfg(feature = "youtube")]
    Web,
    #[cfg(feature = "youtube")]
    Mobile,
}

#[cfg(feature = "youtube")]
impl From<YoutubeMode> for crate::sites::youtube::Mode {
    fn from(mode: YoutubeMode) -> Self {
        match mode {
            YoutubeMode::Auto => Self::Auto,
            YoutubeMode::Web => Self::Web,
            YoutubeMode::Mobile => Self::Mobile,
        }
    }
}

impl Args {
    #[cfg(feature = "youtube")]
    fn youtube_mode(&self) -> YoutubeMode {
        match (self.auto, self.web, self.mobile) {
            (_, true, _) => YoutubeMode::Web,
            (_, _, true) => YoutubeMode::Mobile,
            _ => YoutubeMode::Auto,
        }
    }

    #[cfg(not(feature = "youtube"))]
    fn youtube_mode(&self) -> YoutubeMode {
        YoutubeMode::Auto
    }

    fn output_name(&self, playlist: &Playlist) -> Option<&std::path::Path> {
        if playlist.tracks.len() == 1 {
            self.output.as_deref()
        } else {
            None
        }
    }
}

pub async fn run() -> Result<()> {
    let args = Args::parse();

    if let Some(path) = &args.cookies {
        http::load_cookies_from_file(path)?;
        println!("Loaded cookies from: {}", path.display());
    }

    println!("Extracting audio from: {}", args.url);
    let mut playlist = extract(&args.url).await?;
    if playlist.tracks.is_empty() {
        bail!("no audio files found");
    }

    if let Some(items) = &args.items {
        selection::apply(&mut playlist, items)?;
    }

    if let Some(format) = args.format {
        let default = playlist
            .default
            .and_then(|index| playlist.tracks.get(index))
            .map(|track| track.id.clone());
        playlist
            .tracks
            .retain(|track| track.format.is_none_or(|value| value == format));
        playlist.default =
            default.and_then(|id| playlist.tracks.iter().position(|track| track.id == id));
        if playlist.tracks.is_empty() {
            bail!("no audio files found in format {format}");
        }
    }

    if args.list_formats {
        output::formats(&playlist);
        return Ok(());
    }

    output::playlist(&playlist);
    if args.info {
        println!("Information only mode - skipping download.");
        return Ok(());
    }

    if args.output.is_some() && playlist.tracks.len() > 1 {
        eprintln!("warning: -o/--output is ignored for multi-track playlists");
    }

    let mode = args.youtube_mode();
    let output_name = args.output_name(&playlist);
    let cover_dir = args.cover_dir.as_deref().or(args.dir.as_deref());

    for track in &playlist.tracks {
        files::download(track, args.dir.as_deref(), output_name, mode).await?;
        if args.cover
            && let Err(error) = files::download_cover(track, cover_dir, output_name).await
        {
            eprintln!("warning: cover for '{}': {error:#}", track.title);
        }
    }

    println!();
    println!("Download completed successfully!");
    Ok(())
}

fn parse_format(value: &str) -> std::result::Result<AudioFormat, String> {
    AudioFormat::from_name(value).ok_or_else(|| {
        let supported = AudioFormat::ALL
            .iter()
            .map(|format| format.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!("unsupported format '{value}' (expected one of: {supported})")
    })
}
