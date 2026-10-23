use crate::{MediaFormat, Playlist, Track};

pub fn playlist(playlist: &Playlist) {
    if playlist.tracks.len() > 1 {
        let mut has_header = false;
        if let Some(title) = &playlist.title {
            println!("Playlist: {title}");
            has_header = true;
        }
        if let Some(id) = &playlist.id {
            println!("Playlist ID: {id}");
            has_header = true;
        }
        if let Some(url) = &playlist.url {
            println!("Playlist URL: {url}");
            has_header = true;
        }
        if let Some(cover) = &playlist.cover {
            println!("Playlist Cover: {cover}");
            has_header = true;
        }
        if let Some(index) = playlist.default
            && let Some(track) = playlist.tracks.get(index)
        {
            println!("Default Audio: #{} ({})", index + 1, track.title);
            has_header = true;
        }
        if has_header {
            println!();
        }
    }

    println!("Found {} audio file(s):", playlist.tracks.len());
    println!();
    for (index, track) in playlist.tracks.iter().enumerate() {
        print_track(index, track, playlist.default == Some(index));
    }
}

pub fn media_formats(formats: &[MediaFormat]) {
    if formats.is_empty() {
        println!("No downloadable formats found.");
        return;
    }
    for (index, format) in formats.iter().enumerate() {
        print_media_format(index, format);
    }
}

pub fn selected_format(format: &MediaFormat) {
    println!("Selected format:");
    print_media_format(0, format);
}

fn print_media_format(index: usize, format: &MediaFormat) {
    let quality = format
        .quality
        .as_deref()
        .map(|quality| format!(" {quality}"))
        .unwrap_or_default();
    println!(
        "[{}] {} {}{}",
        index + 1,
        format.kind,
        format.format,
        quality
    );
    println!("    ID: {}", format.id);
    println!("    MIME: {}", format.mime_type);
    if let Some(codec) = &format.codec {
        println!("    Codec: {codec}");
    }
    if let Some(bitrate) = format.bitrate {
        println!("    Bitrate: {} kbps", bitrate / 1000);
    }
    if let Some(resolution) = format.resolution() {
        println!("    Resolution: {resolution}");
    }
    if let Some(size) = format.size_or_estimate() {
        let estimated = if format.size.is_some() {
            ""
        } else {
            " (estimated)"
        };
        println!("    Size: {}{estimated}", format_size(size));
    }
    if let Some(duration) = format.duration {
        println!("    Duration: {}", format_duration(duration));
    }
    println!();
}

fn print_track(index: usize, track: &Track, default: bool) {
    let marker = if default { " ◀" } else { "" };
    println!("[{}] {}{}", index + 1, track.title, marker);
    println!("    Site: {}", track.site);
    println!("    Format: {}", track.format_name());
    if let Some(bitrate) = track.bitrate {
        println!("    Bitrate: {} kbps", bitrate / 1000);
    }
    println!("    URL: {}", track.url);
    if let Some(seconds) = track.duration {
        println!("    Duration: {}", format_duration(seconds));
    }
    if let Some(cover) = &track.cover {
        println!("    Cover: {cover}");
    }
    println!();
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{size:.2} {}", UNITS[unit])
    }
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}
