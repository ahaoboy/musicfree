use crate::{Playlist, Track};

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

pub fn formats(playlist: &Playlist) {
    println!("Available audio formats:");
    println!();
    if playlist.tracks.is_empty() {
        println!("No audio files found.");
        return;
    }
    for (index, track) in playlist.tracks.iter().enumerate() {
        print_track(index, track, playlist.default == Some(index));
    }
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
