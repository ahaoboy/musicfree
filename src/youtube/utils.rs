use crate::error::{MusicFreeError, Result};

pub const WEB_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
pub const ANDROID_USER_AGENT: &str =
    "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip";

/// Parse video ID from YouTube URL or direct video ID string
pub fn parse_id(url: &str) -> Result<String> {
    // Direct video ID (11 characters)
    if is_valid_video_id(url) {
        return Ok(url.to_string());
    }

    // Validate it's a YouTube URL first
    if !is_youtube_url(url) {
        return Err(MusicFreeError::InvalidUrl(format!(
            "Not a valid YouTube URL: {}",
            url
        )));
    }

    // youtube.com/watch?v=VIDEO_ID
    if let Some(pos) = url.find("v=") {
        let id: String = url[pos + 2..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take(11)
            .collect();
        if is_valid_video_id(&id) {
            return Ok(id);
        }
    }

    // youtu.be/VIDEO_ID
    if url.contains("youtu.be/")
        && let Some(pos) = url.find("youtu.be/")
    {
        let id: String = url[pos + 9..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take(11)
            .collect();
        if is_valid_video_id(&id) {
            return Ok(id);
        }
    }

    Err(MusicFreeError::InvalidUrl(format!(
        "Cannot extract video ID from: {}",
        url
    )))
}

/// Validate if a string is a valid YouTube video ID (11 characters, alphanumeric + - and _)
pub fn is_valid_video_id(id: &str) -> bool {
    id.len() == 11 && id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Validate if a string is a valid YouTube playlist ID
/// Playlist IDs typically start with PL, UU, LL, RD, or OL and are 16-34 characters long
pub fn is_valid_playlist_id(id: &str) -> bool {
    if id.len() < 2 || id.len() > 34 {
        return false;
    }

    // Check if starts with common playlist prefixes
    let has_valid_prefix = id.starts_with("PL")
        || id.starts_with("UU")
        || id.starts_with("LL")
        || id.starts_with("RD")
        || id.starts_with("OL")
        || id.starts_with("FL");

    // All characters should be alphanumeric, -, or _
    let valid_chars = id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_');

    has_valid_prefix && valid_chars
}

/// Check if URL is a YouTube link using strict domain validation
pub fn is_youtube_url(url: &str) -> bool {
    // If it's a valid video or playlist ID, consider it valid
    if is_valid_video_id(url) || is_valid_playlist_id(url) {
        return true;
    }

    // Parse URL to check domain strictly
    if let Ok(parsed) = url::Url::parse(url)
        && let Some(domain) = parsed.domain() {
            // Strict domain matching - must be exactly youtube.com or youtu.be (or subdomains)
            return domain == "youtube.com"
                || domain.ends_with(".youtube.com")
                || domain == "youtu.be"
                || domain.ends_with(".youtu.be");
        }

    // Fallback for URLs without scheme
    let normalized = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    if let Ok(parsed) = url::Url::parse(&normalized)
        && let Some(domain) = parsed.domain() {
            return domain == "youtube.com"
                || domain.ends_with(".youtube.com")
                || domain == "youtu.be"
                || domain.ends_with(".youtu.be");
        }

    false
}

/// Check if URL contains a playlist parameter or is a valid playlist ID
pub fn is_playlist_url(url: &str) -> bool {
    // Direct playlist ID
    if is_valid_playlist_id(url) {
        return true;
    }

    // URL with list parameter
    if url.contains("list=") && is_youtube_url(url) {
        // Extract and validate the playlist ID
        if let Some(playlist_id) = parse_playlist_id(url) {
            return is_valid_playlist_id(&playlist_id);
        }
    }

    false
}

/// Extract playlist ID from YouTube URL or validate direct playlist ID
pub fn parse_playlist_id(url: &str) -> Option<String> {
    // Direct playlist ID
    if is_valid_playlist_id(url) {
        return Some(url.to_string());
    }

    // Extract from URL parameter
    if let Some(pos) = url.find("list=") {
        let id: String = url[pos + 5..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if is_valid_playlist_id(&id) {
            return Some(id);
        }
    }
    None
}

/// Construct YouTube watch URL from video ID
pub fn build_watch_url(video_id: &str) -> String {
    format!("https://www.youtube.com/watch?v={}", video_id)
}

/// Construct YouTube playlist URL from playlist ID
pub fn build_playlist_url(playlist_id: &str) -> String {
    format!("https://www.youtube.com/playlist?list={}", playlist_id)
}

/// Construct YouTube watch URL with playlist parameter
pub fn build_watch_url_with_playlist(video_id: &str, playlist_id: &str) -> String {
    format!("https://www.youtube.com/watch?v={}&list={}", video_id, playlist_id)
}

/// Construct YouTube thumbnail URL from video ID
pub fn build_thumbnail_url(video_id: &str) -> String {
    format!("https://i.ytimg.com/vi/{}/hq720.jpg", video_id)
}
