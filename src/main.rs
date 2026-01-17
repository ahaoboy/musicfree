use clap::Parser;
use musicfree::{EXTRACTORS, extract};
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(
    name = "musicfree",
    about = "Extract audio from various platforms",
    long_about = "A tool to extract and download audio from platforms like Bilibili and YouTube.\n\
    Similar to ytdlp but focused on audio extraction.\n\n\
    Examples:\n\
      musicfree https://example.com/video                 # Download audio\n\
      musicfree -F https://example.com/video             # List available formats\n\
      musicfree -i https://example.com/video             # Show info only\n\
      musicfree -f mp3 https://example.com/video         # Download as MP3\n\
      musicfree -d ./music https://example.com/video     # Download to directory\n\
      musicfree -o song.mp3 https://example.com/video    # Custom filename"
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

        if !audio.author.is_empty() {
            println!("  Author: {}", audio.author.join(", "));
        }

        println!();
    }
}

fn format_duration(seconds: u32) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}", minutes, secs)
}

fn format_display_format(format: &Option<musicfree::core::AudioFormat>) -> String {
    format
        .as_ref()
        .map_or("Unknown".to_string(), |f| format!("{:?}", f))
}

fn display_audio_info(audios: &[musicfree::core::Audio]) {
    println!("Found {} audio file(s):", audios.len());
    println!();

    for (index, audio) in audios.iter().enumerate() {
        println!("[{}] {}", index + 1, audio.title);
        println!("    Platform: {:?}", audio.platform);
        println!("    Format: {}", format_display_format(&audio.format));

        if let Some(duration) = audio.duration {
            println!("    Duration: {}", format_duration(duration));
        }

        if !audio.author.is_empty() {
            println!("    Author: {}", audio.author.join(", "));
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
    mut audio: musicfree::core::Audio,
    output_dir: &Option<String>,
    output_name: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Find the appropriate extractor and download binary data
    for &extractor in EXTRACTORS {
        if extractor.matches(&audio.download_url) {
            extractor.download(&mut audio).await?;
            break;
        }
    }

    if let Some(bin) =  &audio.binary  {
        let filename = get_filename(&audio, output_name);
        let base_path = if let Some(dir) = output_dir {
            // Create directory if it doesn't exist
            fs::create_dir_all(dir)?;
            Path::new(dir).join(&filename)
        } else {
            Path::new(".").join(&filename)
        };

        match fs::write(&base_path, bin) {
            Ok(_) => println!("✓ Saved to: {}", base_path.display()),
            Err(e) => {
                eprintln!("✗ Error saving file: {}", e);
                return Err(e.into());
            }
        }
    } else {
        println!("✗ No binary data available for download");
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("Extracting audio from: {}", args.url);

    // Phase 1: Extract and display audio information
    let mut audios = match extract(&args.url).await {
        Ok(audios) => audios,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if audios.is_empty() {
        println!("No audio files found.");
        return;
    }

    // Handle format selection if specified
    if let Some(ref format_str) = args.format
        && let Some(target_format) = parse_format(format_str) {
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

        if let Err(e) = download_audio(audio, &args.output_dir, &args.output_name).await {
            eprintln!("Failed to download audio [{}]: {}", index + 1, e);
            std::process::exit(1);
        }

        if index < audios_len - 1 {
            println!();
        }
    }

    println!();
    println!("Download completed successfully!");
}
