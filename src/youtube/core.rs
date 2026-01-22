use crate::download::{download_binary, download_text, post_json};
use crate::error::{MusicFreeError, Result};
use crate::youtube::types::{
    ContentPlaybackContext, Format, InnertubeContext, InnertubeRequest, PlaybackContext,
    PlayerResponse, PlaylistContent, Title, YtConfig, YtInitialData,
};
use crate::youtube::utils::{ANDROID_USER_AGENT, WEB_USER_AGENT};
use crate::{Audio, AudioFormat, Platform, Playlist};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, ORIGIN, USER_AGENT};

#[cfg(feature = "ytdlp-ejs")]
use crate::youtube::ejs::{solve_cipher, solve_n};

#[cfg(feature = "ytdlp-ejs")]
async fn get_player_url(html: &str) -> Option<String> {
    let marker = r#"name="player/base""#;
    let marker_pos = html.find(marker)?;
    let before_marker = &html[..marker_pos];
    let src_key = r#"src=""#;
    let src_pos = before_marker.rfind(src_key)?;
    let rest = &before_marker[src_pos + src_key.len()..];
    let end_pos = rest.find('"')?;
    let url = format!("https://www.youtube.com{}", &rest[..end_pos]);
    Some(url)
}

/// Extract audio formats from player response (web client)
pub fn extract_audio_formats_web(player_response: &PlayerResponse) -> Result<Vec<&Format>> {
    let formats = &player_response.streaming_data.formats;
    let adaptive_formats = &player_response.streaming_data.adaptive_formats;
    let formats = formats
        .iter()
        .chain(adaptive_formats)
        .filter(|i| i.url.is_some() || i.signature_cipher.is_some());
    Ok(formats.collect())
}

/// Fetch player response from YouTube Android API
pub async fn parse_player(video_id: &str, ytcfg: &YtConfig) -> Result<PlayerResponse> {
    let api_url = format!(
        "https://www.youtube.com/youtubei/v1/player?key={}&prettyPrint=false",
        ytcfg.innertube_api_key
    );

    let client = serde_json::json!({
          "clientName": "ANDROID",
          "clientVersion": "20.10.38",
          // "androidSdkVersion": 30,
          "userAgent": ANDROID_USER_AGENT,
          "osName": "Android",
          "osVersion": "11",
    });

    let request_body = InnertubeRequest {
        video_id: video_id.to_string(),
        context: InnertubeContext { client },
        playback_context: PlaybackContext {
            content_playback_context: ContentPlaybackContext {
                // html5_preference: "HTML5_PREF_WANTS".to_string(),
                pcm2: "yes".to_string(),
            },
        },
        content_check_ok: true,
        racy_check_ok: true,
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_str(ANDROID_USER_AGENT)?);
    headers.insert("X-YouTube-Client-Name", HeaderValue::from_static("3"));
    headers.insert(
        "X-YouTube-Client-Version",
        HeaderValue::from_str(&ytcfg.innertube_client_version)?,
    );
    headers.insert(ORIGIN, HeaderValue::from_static("https://www.youtube.com"));

    if let Some(vd) = &ytcfg.visitor_data
        && let Ok(val) = HeaderValue::from_str(vd)
    {
        headers.insert("X-Goog-Visitor-Id", val);
    }

    let player_response: PlayerResponse = post_json(&api_url, &request_body, headers).await?;
    Ok(player_response)
}

/// Extract playlist information from YouTube URL
pub async fn extract_audio(url: &str) -> Result<(Playlist, Option<usize>)> {
    let html = crate::download::download_text(url, HeaderMap::new()).await?;

    // Check if this is a playlist URL
    if crate::youtube::utils::is_playlist_url(url) {
        return extract_playlist_audio(url, &html).await;
    }

    // Single video processing
    let video_id = &crate::youtube::utils::parse_id(url)?;
    let ytcfg = parse_ytcfg(&html)?;
    let player_response = if let Ok(pr) = parse_player_response_from_html(&html) {
        pr
    } else {
        parse_player(video_id, &ytcfg).await?
    };
    let title = &player_response.video_details.title;
    let audios = extract_audio_formats_web(&player_response)?
        .into_iter()
        .map(|i| {
            Audio::new(
                video_id.clone(),
                title.clone(),
                format!("https://www.youtube.com/watch?v={video_id}"),
                Platform::Youtube,
            )
            .with_format(AudioFormat::from_youtube(&i.mime_type))
            .with_cover(format!("https://i.ytimg.com/vi/{video_id}/hq720.jpg"))
        })
        .collect();

    let playlist = Playlist {
        id: None,
        title: Some(title.clone()),
        audios,
        cover: Some(format!("https://i.ytimg.com/vi/{video_id}/hq720.jpg")),
        platform: Platform::Youtube,
    };

    // For single video, position is 0 if playlist is not empty
    let position = if playlist.audios.is_empty() {
        None
    } else {
        Some(0)
    };

    Ok((playlist, position))
}

/// Extract playlist audio from YouTube playlist URL
async fn extract_playlist_audio(url: &str, html: &str) -> Result<(Playlist, Option<usize>)> {
    let yt_data = parse_yt_initial_data(html)?;
    let videos = extract_playlist_videos(&yt_data)?;

    let playlist_id = crate::youtube::utils::parse_playlist_id(url)
        .ok_or_else(|| MusicFreeError::InvalidUrl("Cannot extract playlist ID".to_string()))?;

    // Extract playlist title from ytInitialData
    let playlist_title = extract_playlist_title(&yt_data)?;

    // Parse the video ID from the URL to find its position in the playlist
    let requested_video_id = crate::youtube::utils::parse_id(url).ok();

    let mut audios = Vec::new();
    let mut position = None;

    // Process each video in the playlist
    for (index, (title, video_url, video_id)) in videos.into_iter().enumerate() {
        // Check if this is the requested video
        if let Some(ref req_id) = requested_video_id
            && &video_id == req_id {
                position = Some(index);
            }

        let audio = Audio::new(
            video_id.clone(),
            title,
            format!("https://www.youtube.com{}", video_url),
            Platform::Youtube,
        )
        .with_cover(format!("https://i.ytimg.com/vi/{video_id}/hq720.jpg"));
        audios.push(audio);
    }

    let playlist = Playlist {
        id: Some(playlist_id.clone()),
        title: Some(playlist_title),
        audios,
        cover: Some(format!("https://i.ytimg.com/vi/{playlist_id}/hq720.jpg")),
        platform: Platform::Youtube,
    };

    // If playlist is empty, position should be None
    let final_position = if playlist.audios.is_empty() {
        None
    } else {
        position
    };

    Ok((playlist, final_position))
}

/// Download audio using web client with EJS decryption
pub async fn download_audio(url: &str) -> Result<Vec<u8>> {
    let video_id = &crate::youtube::utils::parse_id(url)?;

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(ANDROID_USER_AGENT));
    // Step 1: Fetch video page
    let pagr_url = format!("https://www.youtube.com/watch?v={}", video_id);
    let html = download_text(&pagr_url, headers.clone()).await?;

    let ytcfg = parse_ytcfg(&html)?;

    // Step 2: Extract player response from HTML
    let player_response = if let Ok(pr) = parse_player(video_id, &ytcfg).await {
        pr
    } else {
        parse_player_response_from_html(&html)?
    };

    // Step 4: Extract audio formats
    let formats = extract_audio_formats_web(&player_response)?;

    // Step 5: Select best audio format (prefer itag 140)
    let format = formats
        .iter()
        .find(|f| f.itag == 140)
        .or_else(|| formats.iter().find(|f| f.mime_type.starts_with("audio/")))
        .or_else(|| formats.first())
        .ok_or(MusicFreeError::AudioNotFound)?;

    let download_url = {
        #[cfg(not(feature = "ytdlp-ejs"))]
        {
            if let Some(url) = &format.url {
                url.clone()
            } else if let Some(cipher) = &format.signature_cipher {
                cipher.clone()
            } else {
                return Err(MusicFreeError::AudioNotFound);
            }
        }

        #[cfg(feature = "ytdlp-ejs")]
        {
            let player_url = get_player_url(&html)
                .await
                .ok_or(MusicFreeError::PlayerJsNotFound)?;
            // Step 3: Download player JS if available
            let player_js_content = download_text(&player_url, HeaderMap::new()).await?;

            if let Some(url) = &format.url {
                if url.contains("&n=") && url.contains("&sig=") {
                    solve_n(url, player_js_content.clone())?
                } else {
                    url.clone()
                }
            } else if let Some(cipher) = &format.signature_cipher {
                solve_cipher(cipher, player_js_content)?
            } else {
                return Err(MusicFreeError::AudioNotFound);
            }
        }
    };

    headers.insert(USER_AGENT, WEB_USER_AGENT.parse().unwrap());
    // Step 6: Download audio
    download_binary(&download_url, headers).await
}

/// Extract ytcfg configuration from HTML
pub fn parse_ytcfg(html: &str) -> Result<YtConfig> {
    let st = "ytcfg.set({";
    let ed = ");";

    // Find start position
    let st_index = html
        .find(st)
        .ok_or_else(|| MusicFreeError::ConfigParseError("ytcfg.set not found".to_string()))?
        + st.len();

    // Find end position after start
    let remaining_html = &html[st_index..];
    let ed_offset = remaining_html
        .find(ed)
        .ok_or_else(|| MusicFreeError::ConfigParseError("ytcfg end not found".to_string()))?;
    let ed_index = st_index + ed_offset;

    // Extract and construct JSON string
    let json_content = &html[st_index..ed_index];
    let json = format!("{{{}", json_content);
    serde_json::from_str(&json)
        .map_err(|e| MusicFreeError::ConfigParseError(format!("Failed to parse ytcfg JSON: {}", e)))
}

/// Extract player response from HTML
pub fn parse_player_response_from_html(html: &str) -> Result<PlayerResponse> {
    let st = "var ytInitialPlayerResponse = ";
    let ed = "};";

    // Find start position
    let st_index = html.find(st).ok_or_else(|| {
        MusicFreeError::ConfigParseError("ytInitialPlayerResponse not found".to_string())
    })? + st.len();

    // Find end position after start
    let remaining_html = &html[st_index..];
    let ed_offset = remaining_html.find(ed).ok_or_else(|| {
        MusicFreeError::ConfigParseError("ytInitialPlayerResponse end not found".to_string())
    })? + 1; // +1 to include "}"
    let ed_index = st_index + ed_offset;

    // Extract JSON string
    let json = &html[st_index..ed_index];

    serde_json::from_str(json).map_err(|e| {
        MusicFreeError::ConfigParseError(format!("Failed to parse player response JSON: {}", e))
    })
}

/// Extract ytInitialData from HTML
pub fn parse_yt_initial_data(html: &str) -> Result<YtInitialData> {
    let st = "var ytInitialData = ";
    let ed = "};";

    // Find start position
    let st_index = html
        .find(st)
        .ok_or_else(|| MusicFreeError::ConfigParseError("ytInitialData not found".to_string()))?
        + st.len();

    // Find end position after start
    let remaining_html = &html[st_index..];
    let ed_offset = remaining_html.find(ed).ok_or_else(|| {
        MusicFreeError::ConfigParseError("ytInitialData end not found".to_string())
    })? + 1; // +1 to include "}"
    let ed_index = st_index + ed_offset;

    // Extract JSON string
    let json = &html[st_index..ed_index];

    serde_json::from_str(json).map_err(|e| {
        MusicFreeError::ConfigParseError(format!("Failed to parse ytInitialData JSON: {}", e))
    })
}

/// Extract playlist video information from ytInitialData
pub fn extract_playlist_videos(yt_data: &YtInitialData) -> Result<Vec<(String, String, String)>> {
    let videos = &yt_data
        .contents
        .two_column_watch_next_results
        .playlist
        .playlist
        .contents;

    let mut result = Vec::new();
    for content in videos {
        if let PlaylistContent::Video(video_content) = content {
            let renderer = &video_content.playlist_panel_video_renderer;

            let title = match &renderer.title {
                Title::SimpleText { simple_text } => simple_text.clone(),
                Title::Runs { runs } => runs.iter().map(|r| r.text.as_str()).collect::<String>(),
            };

            let url = renderer
                .navigation_endpoint
                .command_metadata
                .web_command_metadata
                .url
                .clone();
            let video_id = renderer.navigation_endpoint.watch_endpoint.video_id.clone();

            result.push((title, url, video_id));
        }
    }

    Ok(result)
}

/// Extract playlist title from ytInitialData
pub fn extract_playlist_title(yt_data: &YtInitialData) -> Result<String> {
    let playlist = &yt_data
        .contents
        .two_column_watch_next_results
        .playlist
        .playlist;

    if let Some(title) = &playlist.title {
        return Ok(title.to_string());
    }

    // Fallback to default title if not found
    Ok("YouTube Playlist".to_string())
}
