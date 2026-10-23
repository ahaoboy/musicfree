use crate::{Error, Result};

pub const WEB_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";
pub const ANDROID_UA: &str = "com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip";
pub const MWEB_UA: &str = "Mozilla/5.0 (iPad; CPU OS 16_7_10 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1,gzip(gfe)";
pub const VISIONOS_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";

pub const ANDROID_VERSION: &str = "21.26.364";
pub const MWEB_VERSION: &str = "2.20260708.05.00";
pub const VISIONOS_VERSION: &str = "1.02";

pub fn is_url(input: &str) -> bool {
    serde_youtube::url::is_valid_video_id(input)
        || serde_youtube::url::is_youtube_url(input)
        || playlist_id(input).is_some()
}

pub fn video_id(input: &str) -> Result<String> {
    if serde_youtube::url::is_valid_video_id(input) {
        return Ok(input.to_owned());
    }
    if !serde_youtube::url::is_youtube_url(input) {
        return Err(Error::InvalidUrl(format!("not a YouTube URL: {input}")));
    }
    serde_youtube::url::parse_id(input)
        .ok_or_else(|| Error::InvalidUrl(format!("cannot extract YouTube video ID from {input}")))
}

pub fn playlist_id(input: &str) -> Option<String> {
    if valid_playlist_id(input) {
        return Some(input.to_owned());
    }
    serde_youtube::url::parse_playlist_id(input).filter(|id| valid_playlist_id(id))
}

pub fn watch_url(video_id: &str) -> String {
    serde_youtube::url::watch_url(video_id)
}

pub fn playlist_url(playlist_id: &str) -> String {
    serde_youtube::url::playlist_url(playlist_id)
}

pub fn thumbnail_url(video_id: &str) -> String {
    serde_youtube::url::thumbnail_url(video_id)
}

fn valid_playlist_id(id: &str) -> bool {
    (2..=34).contains(&id.len())
        && ["PL", "UU", "LL", "OL", "FL"]
            .iter()
            .any(|prefix| id.starts_with(prefix))
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_video_ids() {
        assert_eq!(
            video_id("https://www.youtube.com/watch?v=BnnbP7pCIvQ").unwrap(),
            "BnnbP7pCIvQ"
        );
        assert_eq!(video_id("BnnbP7pCIvQ").unwrap(), "BnnbP7pCIvQ");
    }

    #[test]
    fn accepts_known_playlist_prefixes() {
        assert_eq!(playlist_id("PL123").as_deref(), Some("PL123"));
        assert!(playlist_id("RD123").is_none());
    }
}
