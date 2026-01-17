use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, ORIGIN, USER_AGENT};
use serde::Serialize;
use serde_json::Value;

use crate::download::post_json;
use crate::error::{MusicFreeError, Result};
use crate::utils::get_md5;
use crate::{Audio, Platform};

use super::common::{
    ANDROID_USER_AGENT, AudioFormat, INNERTUBE_CLIENT_NAME, INNERTUBE_CLIENT_VERSION,
    download_audio_data, extract_ytcfg_from_html, fetch_video_page, get_video_title,
};

#[derive(Serialize)]
struct InnertubeRequest {
    #[serde(rename = "videoId")]
    video_id: String,
    context: InnertubeContext,
    #[serde(rename = "playbackContext")]
    playback_context: PlaybackContext,
    #[serde(rename = "contentCheckOk")]
    content_check_ok: bool,
    #[serde(rename = "racyCheckOk")]
    racy_check_ok: bool,
}

#[derive(Serialize)]
struct InnertubeContext {
    client: ClientInfo,
}

#[derive(Serialize)]
struct ClientInfo {
    #[serde(rename = "clientName")]
    client_name: String,
    #[serde(rename = "clientVersion")]
    client_version: String,
    #[serde(rename = "userAgent")]
    user_agent: String,
    #[serde(rename = "osName")]
    os_name: String,
    #[serde(rename = "osVersion")]
    os_version: String,
    hl: String,
    #[serde(rename = "timeZone")]
    time_zone: String,
    #[serde(rename = "utcOffsetMinutes")]
    utc_offset_minutes: i32,
}

#[derive(Serialize)]
struct PlaybackContext {
    #[serde(rename = "contentPlaybackContext")]
    content_playback_context: ContentPlaybackContext,
}

#[derive(Serialize)]
struct ContentPlaybackContext {
    #[serde(rename = "html5Preference")]
    html5_preference: String,
}

/// Fetch player response from YouTube Android API
async fn fetch_player_response_android(
    video_id: &str,
    api_key: &str,
    visitor_data: Option<&str>,
) -> Result<Value> {
    let api_url = format!(
        "https://www.youtube.com/youtubei/v1/player?key={}&prettyPrint=false",
        api_key
    );

    let request_body = InnertubeRequest {
        video_id: video_id.to_string(),
        context: InnertubeContext {
            client: ClientInfo {
                client_name: INNERTUBE_CLIENT_NAME.to_string(),
                client_version: INNERTUBE_CLIENT_VERSION.to_string(),
                user_agent: ANDROID_USER_AGENT.to_string(),
                os_name: "Android".to_string(),
                os_version: "11".to_string(),
                hl: "en".to_string(),
                time_zone: "UTC".to_string(),
                utc_offset_minutes: 0,
            },
        },
        playback_context: PlaybackContext {
            content_playback_context: ContentPlaybackContext {
                html5_preference: "HTML5_PREF_WANTS".to_string(),
            },
        },
        content_check_ok: true,
        racy_check_ok: true,
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static(ANDROID_USER_AGENT));
    headers.insert("X-YouTube-Client-Name", HeaderValue::from_static("3"));
    headers.insert(
        "X-YouTube-Client-Version",
        HeaderValue::from_static(INNERTUBE_CLIENT_VERSION),
    );
    headers.insert(ORIGIN, HeaderValue::from_static("https://www.youtube.com"));

    if let Some(vd) = visitor_data
        && let Ok(val) = HeaderValue::from_str(vd)
    {
        headers.insert("X-Goog-Visitor-Id", val);
    }

    let player_response: Value = post_json(&api_url, &request_body, headers).await?;

    // Check playability status
    if let Some(status) = player_response.get("playabilityStatus") {
        let status_str = status["status"].as_str().unwrap_or("");
        if status_str != "OK" {
            let reason = status["reason"].as_str().unwrap_or("Unknown error");
            return Err(MusicFreeError::YoutubeError(format!(
                "Video unavailable: {}",
                reason
            )));
        }
    }

    Ok(player_response)
}

/// Extract audio formats from Android API response (no decryption needed)
fn extract_audio_formats_android(player_response: &Value) -> Result<Vec<AudioFormat>> {
    let streaming_data = player_response
        .get("streamingData")
        .ok_or(MusicFreeError::AudioNotFound)?;

    let mut formats = Vec::new();

    if let Some(adaptive_formats) = streaming_data
        .get("adaptiveFormats")
        .and_then(|f| f.as_array())
    {
        for format in adaptive_formats {
            let mime_type = format["mimeType"].as_str().unwrap_or("");

            if !mime_type.starts_with("audio/") {
                continue;
            }

            // Android API provides direct URLs
            let url = match format.get("url").and_then(|u| u.as_str()) {
                Some(u) => u.to_string(),
                None => continue,
            };

            let itag = format["itag"].as_i64().unwrap_or(0);
            let bitrate = format["bitrate"].as_i64();
            let content_length = format["contentLength"].as_str().map(|s| s.to_string());
            let audio_quality = format["audioQuality"].as_str().map(|s| s.to_string());

            formats.push(AudioFormat {
                itag,
                mime_type: mime_type.to_string(),
                bitrate,
                content_length,
                audio_quality,
                url,
            });
        }
    }

    if formats.is_empty() {
        return Err(MusicFreeError::AudioNotFound);
    }

    formats.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));
    Ok(formats)
}

/// Download audio using Android client
pub async fn download_audio_android(video_id: &str) -> Result<Audio> {
    // First fetch page to get API key
    let html = fetch_video_page(video_id).await?;
    let ytcfg = extract_ytcfg_from_html(&html)?;

    let player_response =
        fetch_player_response_android(video_id, &ytcfg.api_key, ytcfg.visitor_data.as_deref())
            .await?;

    let title = get_video_title(&player_response);
    let formats = extract_audio_formats_android(&player_response)?;

    let format = formats
        .iter()
        .find(|f| f.itag == 140)
        .or_else(|| formats.first())
        .ok_or(MusicFreeError::AudioNotFound)?;

    let data = download_audio_data(&format.url).await?;
    let audio = Audio::new(
        get_md5(&format.url),
        title,
        format.url.to_string(),
        Platform::Youtube,
    )
    .with_binary(data);

    Ok(audio)
}

/// Get available audio formats without downloading
pub async fn get_audio_formats_android(video_id: &str) -> Result<(String, Vec<AudioFormat>)> {
    let html = fetch_video_page(video_id).await?;
    let ytcfg = extract_ytcfg_from_html(&html)?;

    let player_response =
        fetch_player_response_android(video_id, &ytcfg.api_key, ytcfg.visitor_data.as_deref())
            .await?;

    let title = get_video_title(&player_response);
    let formats = extract_audio_formats_android(&player_response)?;

    Ok((title, formats))
}
