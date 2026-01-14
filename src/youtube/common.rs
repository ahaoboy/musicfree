use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::download::{download_binary_with_headers, download_text_with_headers};
use crate::error::{MusicFreeError, Result};

pub const WEB_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
pub const ANDROID_USER_AGENT: &str =
    "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip";
pub const INNERTUBE_CLIENT_NAME: &str = "ANDROID";
pub const INNERTUBE_CLIENT_VERSION: &str = "20.10.38";

/// Audio metadata from YouTube
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInfo {
    pub title: String,
    pub data: Vec<u8>,
}

/// Audio format information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFormat {
    pub itag: i64,
    pub mime_type: String,
    pub bitrate: Option<i64>,
    pub content_length: Option<String>,
    pub audio_quality: Option<String>,
    pub url: String,
}

/// YouTube configuration extracted from page
#[derive(Debug, Clone)]
pub struct YtConfig {
    pub api_key: String,
    pub visitor_data: Option<String>,
    pub player_url: Option<String>,
}

/// Extract video ID from YouTube URL
pub fn extract_video_id(url: &str) -> Result<String> {
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
        && let Some(pos) = url.find("youtu.be/") {
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

/// Fetch video page HTML
pub async fn fetch_video_page(video_id: &str) -> Result<String> {
    let url = format!("https://www.youtube.com/watch?v={}", video_id);

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(WEB_USER_AGENT));
    headers.insert(
        "Cookie",
        HeaderValue::from_static("CONSENT=YES+cb; SOCS=CAI"),
    );

    download_text_with_headers(&url, headers).await
}

/// Extract ytcfg configuration from HTML
pub fn extract_ytcfg_from_html(html: &str) -> Result<YtConfig> {
    // Extract INNERTUBE_API_KEY
    let api_key_re = Regex::new(r#""INNERTUBE_API_KEY"\s*:\s*"([^"]+)""#).unwrap();
    let api_key = api_key_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| MusicFreeError::ParseError("Cannot find INNERTUBE_API_KEY".to_string()))?;

    // Extract VISITOR_DATA (optional)
    let visitor_re = Regex::new(r#""VISITOR_DATA"\s*:\s*"([^"]+)""#).unwrap();
    let visitor_data = visitor_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    // Extract PLAYER_JS_URL
    let player_re = Regex::new(r#""PLAYER_JS_URL"\s*:\s*"([^"]+)""#).unwrap();
    let player_url = player_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| format!("https://www.youtube.com{}", m.as_str().replace("\\/", "/")));

    // Fallback: extract from jsUrl
    let player_url = player_url.or_else(|| {
        let js_url_re = Regex::new(r#""jsUrl"\s*:\s*"([^"]+)""#).unwrap();
        js_url_re
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| format!("https://www.youtube.com{}", m.as_str().replace("\\/", "/")))
    });

    Ok(YtConfig {
        api_key,
        visitor_data,
        player_url,
    })
}

/// Get video title from player response
pub fn get_video_title(player_response: &Value) -> String {
    player_response["videoDetails"]["title"]
        .as_str()
        .unwrap_or("audio")
        .to_string()
}

/// Download audio data from URL
pub async fn download_audio_data(url: &str) -> Result<Vec<u8>> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(ANDROID_USER_AGENT));
    headers.insert("Range", HeaderValue::from_static("bytes=0-"));

    download_binary_with_headers(url, headers).await
}
