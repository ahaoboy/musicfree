use ejs::{JsChallengeOutput, RuntimeType};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, ORIGIN, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{MusicFreeError, Result};

const WEB_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const ANDROID_USER_AGENT: &str = "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip";
const INNERTUBE_CLIENT_NAME: &str = "ANDROID";
const INNERTUBE_CLIENT_VERSION: &str = "20.10.38";

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
struct YtConfig {
    api_key: String,
    visitor_data: Option<String>,
    player_url: Option<String>,
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
    if url.contains("youtu.be/") {
        if let Some(pos) = url.find("youtu.be/") {
            let id: String = url[pos + 9..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .take(11)
                .collect();
            if id.len() == 11 {
                return Ok(id);
            }
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

// ==================== Web Client (with EJS decryption) ====================

/// Fetch video page HTML
async fn fetch_video_page(video_id: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let url = format!("https://www.youtube.com/watch?v={}", video_id);

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(WEB_USER_AGENT));
    headers.insert(
        "Cookie",
        HeaderValue::from_static("CONSENT=YES+cb; SOCS=CAI"),
    );

    let response = client.get(&url).headers(headers).send().await?;

    if !response.status().is_success() {
        return Err(MusicFreeError::YoutubeError(format!(
            "Failed to fetch page: HTTP {}",
            response.status()
        )));
    }

    Ok(response.text().await?)
}

/// Extract ytcfg configuration from HTML
fn extract_ytcfg_from_html(html: &str) -> Result<YtConfig> {
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

/// Extract player response from HTML
fn extract_player_response_from_html(html: &str) -> Result<Value> {
    let patterns = [
        r#"var\s+ytInitialPlayerResponse\s*=\s*(\{.+?\})\s*;"#,
        r#"ytInitialPlayerResponse\s*=\s*(\{.+?\})\s*;"#,
    ];

    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(html) {
            if let Some(json_str) = caps.get(1) {
                // Find correct JSON end position by counting braces
                let s = json_str.as_str();
                let mut brace_count = 0;
                let mut end_index = 0;

                for (i, c) in s.chars().enumerate() {
                    match c {
                        '{' => brace_count += 1,
                        '}' => brace_count -= 1,
                        _ => {}
                    }
                    if brace_count == 0 {
                        end_index = i + 1;
                        break;
                    }
                }

                if end_index > 0 {
                    let json_str = &s[..end_index];
                    if let Ok(value) = serde_json::from_str(json_str) {
                        return Ok(value);
                    }
                }
            }
        }
    }

    Err(MusicFreeError::ParseError(
        "Cannot extract player response from HTML".to_string(),
    ))
}

/// Download player JS file
async fn download_player_js(player_url: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(WEB_USER_AGENT));

    let response = client.get(player_url).headers(headers).send().await?;

    if !response.status().is_success() {
        return Err(MusicFreeError::YoutubeError(format!(
            "Failed to download player JS: HTTP {}",
            response.status()
        )));
    }

    Ok(response.text().await?)
}

fn decrypt(player: &str, challenges: Vec<String>) -> Option<JsChallengeOutput> {
    ejs::run(player.to_string(), RuntimeType::QuickJS, challenges).ok()
}

/// Process format URL with signature and n parameter decryption
fn process_format_url(format: &Value, player: String) -> Option<String> {
    let mut url = format
        .get("url")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string());

    // Handle signatureCipher
    if url.is_none() {
        if let Some(cipher) = format.get("signatureCipher").and_then(|c| c.as_str()) {
            let params: std::collections::HashMap<_, _> = cipher
                .split('&')
                .filter_map(|p| {
                    let mut parts = p.splitn(2, '=');
                    Some((parts.next()?, parts.next()?))
                })
                .collect();

            let base_url = params
                .get("url")
                .map(|u| urlencoding::decode(u).unwrap_or_default().to_string())?;
            let s = params
                .get("s")
                .map(|s| urlencoding::decode(s).unwrap_or_default().to_string())?;
            let sp = params.get("sp").unwrap_or(&"signature");

            if let Some(decrypted_n) = decrypt(&player, vec![format!("sig:{s}")]) {
                match decrypted_n {
                    JsChallengeOutput::Result {
                        preprocessed_player: _,
                        responses,
                    } => match &responses[0] {
                        ejs::JsChallengeResponse::Result { data } => {
                            url = Some(format!(
                                "{}&{}={}",
                                base_url,
                                sp,
                                urlencoding::encode(data.get(&s).unwrap())
                            ));
                        }
                        ejs::JsChallengeResponse::Error { error: _ } => todo!(),
                    },
                    JsChallengeOutput::Error { error: _ } => todo!(),
                }
            }

            // if let Some(decrypted_sig) = decrypt(&player, &s) {
            //     url = Some(format!(
            //         "{}&{}={}",
            //         base_url,
            //         sp,
            //         urlencoding::encode(&decrypted_sig)
            //     ));
            // }
        }
    }

    let mut url = url?;

    // Process n parameter
    if let Ok(parsed_url) = reqwest::Url::parse(&url) {
        if let Some(n_value) = parsed_url
            .query_pairs()
            .find(|(k, _)| k == "n")
            .map(|(_, v)| v.to_string())
        {
            if let Some(decrypted_n) = decrypt(&player, vec![format!("n:{n_value}")]) {
                match decrypted_n {
                    JsChallengeOutput::Result {
                        preprocessed_player: _,
                        responses,
                    } => {
                        match &responses[0] {
                            ejs::JsChallengeResponse::Result { data } => {
                                // Replace n parameter in URL
                                let new_url = url.replace(
                                    &format!("n={}", n_value),
                                    &format!("n={}", data.get(&n_value).unwrap()),
                                );
                                url = new_url;
                            }
                            ejs::JsChallengeResponse::Error { error: _ } => todo!(),
                        }
                    }
                    JsChallengeOutput::Error { error: _ } => todo!(),
                }
            }
        }
    }

    Some(url)
}

/// Extract audio formats from player response (web client)
fn extract_audio_formats_web(player_response: &Value, player: String) -> Result<Vec<AudioFormat>> {
    let streaming_data = player_response
        .get("streamingData")
        .ok_or(MusicFreeError::AudioNotFound)?;

    let mut formats = Vec::new();

    // Get adaptive formats
    if let Some(adaptive_formats) = streaming_data
        .get("adaptiveFormats")
        .and_then(|f| f.as_array())
    {
        for format in adaptive_formats {
            let mime_type = format["mimeType"].as_str().unwrap_or("");

            // Only audio formats
            if !mime_type.starts_with("audio/") {
                continue;
            }

            let url = match process_format_url(format, player.clone()) {
                Some(u) => u,
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

    // Sort by bitrate (highest first)
    formats.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));

    Ok(formats)
}

/// Download audio using web client with EJS decryption
async fn download_audio_web(video_id: &str) -> Result<AudioInfo> {
    // Step 1: Fetch video page
    let html = fetch_video_page(video_id).await?;

    // Step 2: Extract ytcfg
    let ytcfg = extract_ytcfg_from_html(&html)?;

    // Step 3: Extract player response from HTML
    let player_response = extract_player_response_from_html(&html)?;

    // Step 4: Download player JS if available
    let player_js_content = if let Some(ref player_url) = ytcfg.player_url {
        Some(download_player_js(player_url).await?)
    } else {
        None
    };

    // Step 5: Save player JS to temp file for EJS
    // let player_js_path = if let Some(ref content) = player_js_content {
    //     let temp_path = std::env::temp_dir().join("youtube_player.js");
    //     std::fs::write(&temp_path, content)?;
    //     Some(temp_path)
    // } else {
    //     None
    // };

    // let js_path_str = player_js_path
    //     .as_ref()
    //     .map(|p| p.to_string_lossy().to_string());

    // Step 6: Extract audio formats
    let formats = extract_audio_formats_web(&player_response, player_js_content.unwrap())?;

    // Step 7: Get title
    let title = get_video_title(&player_response);

    // Step 8: Select best audio format (prefer itag 140)
    let format = formats
        .iter()
        .find(|f| f.itag == 140)
        .or_else(|| formats.first())
        .ok_or(MusicFreeError::AudioNotFound)?;

    // Step 9: Download audio
    let data = download_audio_data(&format.url).await?;

    Ok(AudioInfo { title, data })
}

// ==================== Android Client (fallback) ====================

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
    let client = reqwest::Client::new();

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

    if let Some(vd) = visitor_data {
        if let Ok(val) = HeaderValue::from_str(vd) {
            headers.insert("X-Goog-Visitor-Id", val);
        }
    }

    let response = client
        .post(&api_url)
        .headers(headers)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(MusicFreeError::YoutubeError(format!(
            "API request failed: HTTP {}",
            response.status()
        )));
    }

    let player_response: Value = response.json().await?;

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

/// Download audio using Android client (fallback)
async fn download_audio_android(video_id: &str) -> Result<AudioInfo> {
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

    Ok(AudioInfo { title, data })
}

// ==================== Common Functions ====================

/// Get video title from player response
fn get_video_title(player_response: &Value) -> String {
    player_response["videoDetails"]["title"]
        .as_str()
        .unwrap_or("audio")
        .to_string()
}

/// Download audio data from URL
async fn download_audio_data(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(ANDROID_USER_AGENT));
    headers.insert("Range", HeaderValue::from_static("bytes=0-"));

    let response = client.get(url).headers(headers).send().await?;

    if !response.status().is_success() && response.status().as_u16() != 206 {
        return Err(MusicFreeError::YoutubeError(format!(
            "Download failed: HTTP {}",
            response.status()
        )));
    }

    let data = response.bytes().await?.to_vec();
    Ok(data)
}

// ==================== Public API ====================

/// Download audio from YouTube (tries web client first, then Android fallback)
pub async fn download_audio(url: &str) -> Result<AudioInfo> {
    download_audio_with_ejs(url).await
}

/// Download audio with custom EJS path
pub async fn download_audio_with_ejs(url: &str) -> Result<AudioInfo> {
    let video_id = extract_video_id(url)?;

    // Try web client with EJS decryption first
    match download_audio_web(&video_id).await {
        Ok(info) => return Ok(info),
        Err(e) => {
            eprintln!("Web client failed: {}, trying Android fallback...", e);
        }
    }

    // Fallback to Android client
    download_audio_android(&video_id).await
}

/// Get available audio formats without downloading
pub async fn get_audio_formats(url: &str) -> Result<(String, Vec<AudioFormat>)> {
    let video_id = extract_video_id(url)?;

    // Try Android client for format listing (simpler)
    let html = fetch_video_page(&video_id).await?;
    let ytcfg = extract_ytcfg_from_html(&html)?;

    let player_response =
        fetch_player_response_android(&video_id, &ytcfg.api_key, ytcfg.visitor_data.as_deref())
            .await?;

    let title = get_video_title(&player_response);
    let formats = extract_audio_formats_android(&player_response)?;

    Ok((title, formats))
}
