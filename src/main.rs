use clap::Parser;
use musicfree::extract;
use std::fs;
use std::path::Path;

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const GIT_HASH: &str = git_version::git_version!();
const VERSION: &str = const_str::concat!(CARGO_PKG_VERSION, " ", GIT_HASH);

#[derive(Parser)]
#[command(
    name = "musicfree",
    version = VERSION,
    about = "Extract audio from various platforms",
    long_about = "A tool to extract and download audio from platforms like Bilibili and YouTube.\n\
    Similar to ytdlp but focused on audio extraction.\n\n\
    Examples:\n\
      musicfree https://example.com/video                 # Download audio\n\
      musicfree -F https://example.com/video             # List available formats\n\
      musicfree -i https://example.com/video             # Show info only\n\
      musicfree -f mp3 https://example.com/video         # Download as MP3\n\
      musicfree -d ./music https://example.com/video     # Download to directory\n\
      musicfree -o song.mp3 https://example.com/video    # Custom filename\n\
      musicfree -c https://example.com/video            # Download audio + cover\n\
      musicfree -c --cover-dir ./covers https://example.com/video  # Custom cover dir"
)]
struct Args {
    /// URL to extract audio from
    #[arg(help = "URL to extract audio from (supports Bilibili, YouTube)")]
    url: String,

    /// Download to specified directory
    #[arg(short = 'd', long = "dir", help = "Download to specified directory")]
    output_dir: Option<String>,

    /// Output filename (only works when single audio found)
    #[arg(
        short = 'o',
        long = "output",
        help = "Output filename (only works when single audio found)"
    )]
    output_name: Option<String>,

    /// List all available audio formats without downloading
    #[arg(
        short = 'F',
        long = "list-formats",
        help = "List all available audio formats without downloading"
    )]
    list_formats: bool,

    /// Extract and show information only, no download
    #[arg(
        short = 'i',
        long = "info-only",
        help = "Extract and show information only, no download"
    )]
    info_only: bool,

    /// Skip download (same as --info-only)
    #[arg(long = "no-download", help = "Skip download (same as --info-only)")]
    no_download: bool,

    /// Audio format to download (mp3, m4a, flac, wav, aac, ogg)
    #[arg(
        short = 'f',
        long = "format",
        help = "Audio format to download (mp3, m4a, flac, wav, aac, ogg)"
    )]
    format: Option<String>,

    /// Download cover/artwork image along with audio
    #[arg(
        short = 'c',
        long = "download-cover",
        help = "Download cover/artwork image along with audio"
    )]
    download_cover: bool,

    /// Directory to save cover images (default: same as audio directory)
    #[arg(
        long = "cover-dir",
        help = "Directory to save cover images (default: same as audio directory)"
    )]
    cover_dir: Option<String>,

    /// Select specific items from playlist to download (e.g., "1,3,5" or "2-4" or "1,3-5,7")
    #[arg(
        short = 'I',
        long = "playlist-items",
        help = "Select specific items from playlist to download (e.g., \"1,3,5\" or \"2-4\" or \"1,3-5,7\")"
    )]
    playlist_items: Option<String>,
}

fn parse_format(format_str: &str) -> Option<musicfree::core::AudioFormat> {
    match format_str.to_lowercase().as_str() {
        "mp3" => Some(musicfree::core::AudioFormat::Mp3),
        "m4a" => Some(musicfree::core::AudioFormat::M4A),
        "flac" => Some(musicfree::core::AudioFormat::Flac),
        "wav" => Some(musicfree::core::AudioFormat::Wav),
        "aac" => Some(musicfree::core::AudioFormat::AAC),
        "ogg" => Some(musicfree::core::AudioFormat::Ogg),
        _ => {
            eprintln!(
                "Warning: Unsupported format '{}', using default",
                format_str
            );
            None
        }
    }
}

fn list_available_formats(audios: &[musicfree::core::Audio]) {
    println!("Available audio formats:");
    println!();

    if audios.is_empty() {
        println!("No audio files found.");
        return;
    }

    for (index, audio) in audios.iter().enumerate() {
        println!("[{}]: {}", index + 1, audio.title);
        println!("  Format: {}", format_display_format(&audio.format));
        println!("  Download URL: {}", audio.download_url);

        if let Some(duration) = audio.duration {
            println!("  Duration: {}", format_duration(duration));
        }

        if let Some(cover_url) = &audio.cover {
            println!("  Cover: {}", cover_url);
        }

        println!();
    }
}

fn format_duration(seconds: u64) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}", minutes, secs)
}

fn format_display_format(format: &Option<musicfree::core::AudioFormat>) -> String {
    format
        .as_ref()
        .map_or("Unknown".to_string(), |f| format!("{:?}", f))
}

/// Parse playlist items string into a set of indices
/// Supports formats like "1,3,5" or "2-4" or "1,3-5,7"
fn parse_playlist_items(items_str: &str) -> Result<Vec<usize>, String> {
    let mut indices = Vec::new();

    for part in items_str.split(',') {
        let part = part.trim();

        if part.contains('-') {
            // Handle range like "2-4"
            let range_parts: Vec<&str> = part.split('-').collect();
            if range_parts.len() != 2 {
                return Err(format!("Invalid range format: {}", part));
            }

            let start: usize = range_parts[0].trim().parse()
                .map_err(|_| format!("Invalid number in range: {}", range_parts[0]))?;
            let end: usize = range_parts[1].trim().parse()
                .map_err(|_| format!("Invalid number in range: {}", range_parts[1]))?;

            if start == 0 || end == 0 {
                return Err("Playlist indices must start from 1".to_string());
            }

            if start > end {
                return Err(format!("Invalid range: {} > {}", start, end));
            }

            for i in start..=end {
                indices.push(i);
            }
        } else {
            // Handle single number like "3"
            let num: usize = part.parse()
                .map_err(|_| format!("Invalid number: {}", part))?;

            if num == 0 {
                return Err("Playlist indices must start from 1".to_string());
            }

            indices.push(num);
        }
    }

    // Remove duplicates and sort
    indices.sort_unstable();
    indices.dedup();

    Ok(indices)
}

/// Filter audios based on playlist items selection
fn filter_playlist_items(audios: Vec<musicfree::core::Audio>, items_str: &str) -> Result<Vec<musicfree::core::Audio>, String> {
    let indices = parse_playlist_items(items_str)?;

    let total_count = audios.len();
    let mut filtered = Vec::new();

    for idx in indices {
        if idx > total_count {
            eprintln!("Warning: Index {} is out of range (playlist has {} items), skipping", idx, total_count);
            continue;
        }

        // Convert 1-based index to 0-based
        filtered.push(audios[idx - 1].clone());
    }

    if filtered.is_empty() {
        return Err("No valid items selected from playlist".to_string());
    }

    Ok(filtered)
}

fn display_audio_info(audios: &[musicfree::core::Audio]) {
    println!("Found {} audio file(s):", audios.len());
    println!();

    for (index, audio) in audios.iter().enumerate() {
        println!("[{}] {}", index + 1, audio.title);
        println!("    Platform: {:?}", audio.platform);
        println!("    Format: {}", format_display_format(&audio.format));
        println!("    URL: {}", audio.download_url);

        if let Some(duration) = audio.duration {
            println!("    Duration: {}", format_duration(duration));
        }

        if let Some(cover_url) = &audio.cover {
            println!("    Cover: {}", cover_url);
        }

        println!();
    }
}

fn get_filename(audio: &musicfree::core::Audio, output_name: &Option<String>) -> String {
    if let Some(name) = output_name {
        // If output name is provided, use it without changing extension
        let base_name = Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("audio");

        let extension = audio
            .format
            .as_ref()
            .unwrap_or(&musicfree::core::AudioFormat::Mp3)
            .extension();

        format!("{}{}", base_name, extension)
    } else {
        // Use sanitized title + extension
        sanitize_filename::sanitize(&audio.title)
            + audio
                .format
                .as_ref()
                .unwrap_or(&musicfree::core::AudioFormat::Mp3)
                .extension()
    }
}

async fn download_audio(
    audio: &musicfree::core::Audio,
    output_dir: &Option<String>,
    output_name: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let filename = get_filename(audio, output_name);
    let base_path = if let Some(dir) = output_dir {
        // Create directory if it doesn't exist
        fs::create_dir_all(dir)?;
        Path::new(dir).join(&filename)
    } else {
        Path::new(".").join(&filename)
    };

    // Check if file already exists
    if base_path.exists() {
        println!("⏭ File already exists, skipping: {}", base_path.display());
        return Ok(());
    }

    // Find appropriate extractor and download binary data
    match audio
        .platform
        .extractor()
        .download(&audio.download_url)
        .await
    {
        Ok(bin) => match fs::write(&base_path, bin) {
            Ok(_) => println!("✓ Saved to: {}", base_path.display()),
            Err(e) => {
                eprintln!("✗ Error saving file: {}", e);
                return Err(e.into());
            }
        },
        Err(e) => {
            eprintln!("✗ No binary data available for download: {:?}", e);
            return Err(e.into());
        }
    }
    Ok(())
}

fn get_cover_filename(audio: &musicfree::core::Audio, output_name: &Option<String>) -> String {
    if let Some(name) = output_name {
        // If output name is provided, use it without changing extension
        let base_name = Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("cover");
        format!("{}.jpg", base_name)
    } else {
        // Use audio.id as prefix to prevent filename conflicts
        let sanitized_title = sanitize_filename::sanitize(&audio.title);
        format!("{sanitized_title}_{}.jpg", audio.id)
    }
}

async fn download_cover(
    audio: &musicfree::core::Audio,
    cover_dir: &Option<String>,
    output_name: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(cover_url) = &audio.cover {
        let cover_filename = get_cover_filename(audio, output_name);

        let base_path = if let Some(dir) = cover_dir {
            // Create directory if it doesn't exist
            fs::create_dir_all(dir)?;
            Path::new(dir).join(&cover_filename)
        } else {
            Path::new(".").join(&cover_filename)
        };

        // Check if cover file already exists
        if base_path.exists() {
            println!(
                "⏭ Cover file already exists, skipping: {}",
                base_path.display()
            );
            return Ok(());
        }

        println!("Downloading cover from: {}", cover_url);

        // Download cover binary data
        match audio.platform.extractor().download_cover(cover_url).await {
            Ok(cover_data) => match fs::write(&base_path, cover_data) {
                Ok(_) => println!("✓ Cover saved to: {}", base_path.display()),
                Err(e) => {
                    eprintln!("✗ Error saving cover: {}", e);
                    return Err(e.into());
                }
            },
            Err(e) => {
                eprintln!("✗ Failed to download cover: {:?}", e);
                return Err(e.into());
            }
        }
    } else {
        println!("✗ No cover available for this audio");
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("Extracting audio from: {}", args.url);

    // Phase 1: Extract and display audio information
    let (playlist, position) = match extract(&args.url).await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut audios = playlist.audios;

    if audios.is_empty() {
        println!("No audio files found.");
        return;
    }

    // Display position information if available
    if let Some(pos) = position {
        println!("Found requested video at position {} in playlist", pos + 1);
        println!();
    }

    // Handle playlist items selection if specified
    if let Some(ref items_str) = args.playlist_items {
        match filter_playlist_items(audios, items_str) {
            Ok(filtered) => {
                println!("Selected {} item(s) from playlist", filtered.len());
                println!();
                audios = filtered;
            }
            Err(e) => {
                eprintln!("Error parsing playlist items: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Handle format selection if specified
    if let Some(ref format_str) = args.format
        && let Some(target_format) = parse_format(format_str)
    {
        // Filter audios by requested format (if they have format info)
        audios.retain(|audio| audio.format.as_ref().is_none_or(|f| f == &target_format));

        if audios.is_empty() {
            println!("No audio files found with format: {:?}", target_format);
            return;
        }
    }

    // Handle --list-formats option
    if args.list_formats {
        list_available_formats(&audios);
        return;
    }

    // Display audio information
    display_audio_info(&audios);

    // Check if we should skip downloading
    let skip_download = args.info_only || args.no_download;
    if skip_download {
        println!("Information only mode - skipping download.");
        return;
    }

    // Validate -o option usage
    if args.output_name.is_some() && audios.len() > 1 {
        eprintln!("Warning: -o option is only valid when a single audio file is found.");
        eprintln!("Found {} audio files, ignoring -o option.", audios.len());
    }

    // Phase 2: Download audio files
    println!("Downloading audio files...");
    println!();

    let audios_len = audios.len();
    for (index, audio) in audios.into_iter().enumerate() {
        println!("Downloading [{}]: {}", index + 1, audio.title);

        if let Err(e) = download_audio(&audio, &args.output_dir, &args.output_name).await {
            eprintln!("Failed to download audio [{}]: {}", index + 1, e);
            std::process::exit(1);
        }

        // Download cover if requested and available
        if args.download_cover
            && let Err(e) = download_cover(&audio, &args.cover_dir, &args.output_name).await
        {
            eprintln!("Failed to download cover for [{}]: {}", index + 1, e);
            // Don't exit on cover download failure, just continue
        }

        if index < audios_len - 1 {
            println!();
        }
    }

    println!();
    println!("Download completed successfully!");
}
