use crate::error::{MusicFreeError, Result};

pub const WEB_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
pub const ANDROID_USER_AGENT: &str =
    "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip";
pub const ANDROID_VR_USER_AGENT: &str = "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip";

/// Parse video ID from a YouTube URL or direct video ID string.
pub fn parse_id(url: &str) -> Result<String> {
    if serde_youtube::url::is_valid_video_id(url) {
        return Ok(url.to_string());
    }
    if !serde_youtube::url::is_youtube_url(url) {
        return Err(MusicFreeError::InvalidUrl(format!(
            "Not a valid YouTube URL: {}",
            url
        )));
    }
    serde_youtube::url::parse_id(url).ok_or_else(|| {
        MusicFreeError::InvalidUrl(format!("Cannot extract video ID from: {}", url))
    })
}

/// Validate if a string is a valid YouTube playlist ID.
/// Playlist IDs typically start with PL, UU, LL, or OL and are 2-34 characters.
pub fn is_valid_playlist_id(id: &str) -> bool {
    if id.len() < 2 || id.len() > 34 {
        return false;
    }
    let has_valid_prefix = id.starts_with("PL")
        || id.starts_with("UU")
        || id.starts_with("LL")
        // || id.starts_with("RD") // skip RD (Radio/MIX)
        || id.starts_with("OL")
        || id.starts_with("FL");
    let valid_chars = id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    has_valid_prefix && valid_chars
}

/// Extract playlist ID from a YouTube URL or validate a direct playlist ID.
pub fn parse_playlist_id(url: &str) -> Option<String> {
    if is_valid_playlist_id(url) {
        return Some(url.to_string());
    }
    serde_youtube::url::parse_playlist_id(url).filter(|id| is_valid_playlist_id(id))
}
