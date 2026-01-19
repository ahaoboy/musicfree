use crate::error::{MusicFreeError, Result};

pub const WEB_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
pub const ANDROID_USER_AGENT: &str =
    "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip";

/// Parse video ID from YouTube URL
pub fn parse_id(url: &str) -> Result<String> {
    // Direct video ID (11 characters)
    if url.len() == 11
        && url
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Ok(url.to_string());
    }

    // youtube.com/watch?v=VIDEO_ID
    if let Some(pos) = url.find("v=") {
        let id: String = url[pos + 2..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take(11)
            .collect();
        if id.len() == 11 {
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
        if id.len() == 11 {
            return Ok(id);
        }
    }

    Err(MusicFreeError::InvalidUrl(format!(
        "Cannot extract video ID from: {}",
        url
    )))
}

/// Check if URL is a YouTube link
pub fn is_youtube_url(url: &str) -> bool {
    url.contains("youtube.com") || url.contains("youtu.be")
}

/// Check if URL contains a playlist parameter
pub fn is_playlist_url(url: &str) -> bool {
    url.contains("list=") && is_youtube_url(url)
}

/// Extract playlist ID from YouTube URL
pub fn parse_playlist_id(url: &str) -> Option<String> {
    if let Some(pos) = url.find("list=") {
        let id: String = url[pos + 5..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take_while(|c| !"&?".contains(*c))
            .collect();
        if !id.is_empty() {
            return Some(id);
        }
    }
    None
}
