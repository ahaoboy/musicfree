use crate::download::{download_binary, download_text, post_json};
use crate::error::{MusicFreeError, Result};
use crate::youtube::parse_id;
use crate::youtube::types::{
    ContentPlaybackContext, Format, InnertubeContext, InnertubeRequest, PlaybackContext,
    PlayerResponse, PlaylistContent, Title, YtConfig, YtInitialData,
};
use crate::youtube::utils::{
    ANDROID_USER_AGENT, WEB_USER_AGENT, build_playlist_url, build_thumbnail_url, build_watch_url,
    is_valid_playlist_id, is_valid_video_id, parse_playlist_id,
};
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

fn get_fetch_url(url: &str) -> (String, bool) {
    if let Some(playlist_id) = parse_playlist_id(url) {
        return (build_playlist_url(&playlist_id), true);
    }
    if is_valid_video_id(url) {
        return (build_watch_url(url), false);
    }
    (url.to_string(), false)
}

/// Extract playlist information from YouTube URL or ID
pub async fn extract_audio(url: &str) -> Result<(Playlist, Option<usize>)> {
    // Construct full URL for fetching HTML
    let (fetch_url, is_playlist) = get_fetch_url(url);
    let html = crate::download::download_text(&fetch_url, HeaderMap::new()).await?;

    // Handle playlist
    if is_playlist {
        return extract_playlist_audio(url, &html).await;
    }

    // Single video processing
    let video_id = &parse_id(url)?;
    let ytcfg = parse_ytcfg(&html)?;
    let player_response = if let Ok(pr) = parse_player_response_from_html(&html) {
        pr
    } else {
        parse_player(video_id, &ytcfg).await?
    };
    let title = &player_response.video_details.title;
    let audios: Vec<Audio> = extract_audio_formats_web(&player_response)?
        .into_iter()
        .map(|i| {
            let mut audio = Audio::new(
                video_id.clone(),
                title.clone(),
                build_watch_url(video_id),
                Platform::Youtube,
            )
            .with_format(AudioFormat::from_youtube(&i.mime_type))
            .with_cover(build_thumbnail_url(video_id));
            if let Some(ms) = i
                .approx_duration_ms
                .clone()
                .and_then(|s| s.parse::<u64>().ok())
            {
                audio.duration = Some(ms / 1000);
            }
            audio
        })
        .collect();

    // For single video, use first audio's download_url
    let download_url = audios.first().map(|a| a.download_url.clone());

    let playlist = Playlist {
        id: None,
        download_url,
        title: Some(title.clone()),
        audios,
        cover: Some(build_thumbnail_url(video_id)),
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

/// Extract playlist audio from YouTube playlist URL or ID
async fn extract_playlist_audio(url: &str, html: &str) -> Result<(Playlist, Option<usize>)> {
    // Extract playlist ID from URL or use URL as playlist ID
    let playlist_id = if is_valid_playlist_id(url) {
        url.to_string()
    } else {
        parse_playlist_id(url)
            .ok_or_else(|| MusicFreeError::InvalidUrl("Cannot extract playlist ID".to_string()))?
    };

    // Try to extract video ID from the original URL (if it's a watch URL with playlist)
    let requested_video_id = parse_id(url).ok();

    let yt_data = parse_yt_initial_data(html)?;
    let videos = extract_playlist_videos(&yt_data)?;

    // Extract playlist title from ytInitialData
    let playlist_title = extract_playlist_title(&yt_data)?;

    let mut audios = Vec::new();
    let mut position = None;

    // Process each video in the playlist
    for (index, video) in videos.into_iter().enumerate() {
        // Check if this is the requested video
        if let Some(ref req_id) = requested_video_id
            && &video.video_id == req_id
        {
            position = Some(index);
        }

        let mut audio = Audio::new(
            video.video_id.clone(),
            video.title,
            format!("https://www.youtube.com{}", video.url),
            Platform::Youtube,
        )
        .with_cover(build_thumbnail_url(&video.video_id));

        if let Some(d) = video.duration {
            audio.duration = Some(d);
        }
        audios.push(audio);
    }

    let cover = audios.iter().find_map(|i| i.cover.clone());

    // Construct playlist download URL
    let playlist_download_url = Some(build_playlist_url(&playlist_id));

    let playlist = Playlist {
        id: Some(playlist_id.clone()),
        download_url: playlist_download_url,
        title: Some(playlist_title),
        audios,
        cover,
        platform: Platform::Youtube,
    };

    // Return position only if playlist is not empty and a video was found
    let final_position = if !playlist.audios.is_empty() && position.is_some() {
        position
    } else {
        None
    };

    Ok((playlist, final_position))
}

/// Download audio using web client with EJS decryption
pub async fn download_audio(url: &str) -> Result<Vec<u8>> {
    let video_id = &parse_id(url)?;

    let mut headers = HeaderMap::new();
    // FIXME: android_sdkless 403
    headers.insert(USER_AGENT, HeaderValue::from_static(WEB_USER_AGENT));
    // Step 1: Fetch video page
    let pagr_url = build_watch_url(video_id);
    let html = download_text(&pagr_url, headers.clone()).await?;

    // Step 2: Extract player response from HTML
    let (player_response, is_web) = if let Ok(pr) = parse_player_response_from_html(&html) {
        (pr, true)
    } else {
        let mut headers = HeaderMap::new();
        // FIXME: android_sdkless 403
        headers.insert(USER_AGENT, HeaderValue::from_static(ANDROID_USER_AGENT));
        let html = download_text(&pagr_url, headers).await?;
        let ytcfg = parse_ytcfg(&html)?;
        (parse_player(video_id, &ytcfg).await?, false)
    };

    // Step 4: Extract audio formats
    let formats = extract_audio_formats_web(&player_response)?;

    // Step 5: Select best audio format (prefer itag 140)
    let format = formats
        .iter()
        .find(|f| f.itag == if is_web { 18 } else { 140 })
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
    let ua = if is_web {
        WEB_USER_AGENT
    } else {
        ANDROID_USER_AGENT
    };
    headers.insert(USER_AGENT, HeaderValue::from_static(ua));
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

pub fn parse_player_response_from_html(html: &str) -> Result<PlayerResponse> {
    let pattern = "var ytInitialPlayerResponse = ";
    let mut current_pos = 0;

    let mut last_error =
        MusicFreeError::ConfigParseError("ytInitialPlayerResponse not found".to_string());

    while let Some(st_offset) = html[current_pos..].find(pattern) {
        let st_index = current_pos + st_offset + pattern.len();

        if let Some(ed_offset) = html[st_index..].find("};") {
            let ed_index = st_index + ed_offset + 1; // +1
            let json_str = &html[st_index..ed_index];
            // let _ = std::fs::write("yt.json", json_str);

            match serde_json::from_str::<serde_json::Value>(json_str) {
                Ok(value) if !value.is_null() => {
                    match serde_json::from_value::<PlayerResponse>(value) {
                        Ok(response) => {
                            return Ok(response);
                        }
                        Err(e) => {
                            last_error =
                                MusicFreeError::ConfigParseError(format!("Schema mismatch: {}", e));
                        }
                    }
                }
                Ok(_) => {
                    // skip null
                }
                Err(e) => {
                    last_error =
                        MusicFreeError::ConfigParseError(format!("Invalid JSON block: {}", e));
                }
            }
            current_pos = ed_index;
        } else {
            current_pos = st_index;
        }
    }

    Err(last_error)
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

/// Represents a video in a YouTube playlist
#[derive(Debug, Clone)]
pub struct PlaylistVideoInfo {
    pub title: String,
    pub url: String,
    pub video_id: String,
    pub duration: Option<u64>,
}

/// Extract playlist video information from ytInitialData
/// Supports both twoColumnWatchNextResults and twoColumnBrowseResultsRenderer formats
pub fn extract_playlist_videos(yt_data: &YtInitialData) -> Result<Vec<PlaylistVideoInfo>> {
    // Try twoColumnWatchNextResults format first (used in watch page with playlist)
    if let Some(watch_next) = &yt_data.contents.two_column_watch_next_results {
        let videos = &watch_next.playlist.playlist.contents;
        let mut result = Vec::new();

        for content in videos {
            if let PlaylistContent::Video(video_content) = content {
                let renderer = &video_content.playlist_panel_video_renderer;

                let title = match &renderer.title {
                    Title::SimpleText { simple_text } => simple_text.clone(),
                    Title::Runs { runs } => {
                        runs.iter().map(|r| r.text.as_str()).collect::<String>()
                    }
                };

                let url = renderer
                    .navigation_endpoint
                    .command_metadata
                    .web_command_metadata
                    .url
                    .clone();
                let video_id = renderer.navigation_endpoint.watch_endpoint.video_id.clone();

                result.push(PlaylistVideoInfo {
                    title,
                    url,
                    video_id,
                    duration: None,
                });
            }
        }

        return Ok(result);
    }

    // Try twoColumnBrowseResultsRenderer format (used in playlist page)
    if let Some(browse_results) = &yt_data.contents.two_column_browse_results_renderer {
        // Find the selected tab and extract videos directly
        let result = browse_results
            .tabs
            .iter()
            .find(|tab| tab.tab_renderer.selected)
            .map(|tab| {
                tab.tab_renderer
                    .content
                    .section_list_renderer
                    .contents
                    .iter()
                    .filter_map(|section| section.item_section_renderer.as_ref())
                    .flat_map(|renderer| &renderer.contents)
                    .flat_map(|item| &item.playlist_video_list_renderer.contents)
                    .map(|video_element| {
                        let renderer = &video_element.playlist_video_renderer;
                        let video_id = renderer.video_id.clone();
                        let title: String = renderer
                            .title
                            .runs
                            .iter()
                            .map(|r| r.text.as_str())
                            .collect();
                        let url = format!("/watch?v={}", video_id);
                        let duration = video_element
                            .playlist_video_renderer
                            .length_seconds
                            .parse()
                            .ok();

                        PlaylistVideoInfo {
                            title,
                            url,
                            video_id,
                            duration,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        return Ok(result);
    }

    Err(MusicFreeError::ConfigParseError(
        "No valid playlist format found in ytInitialData".to_string(),
    ))
}

/// Extract playlist title from ytInitialData
/// Supports both twoColumnWatchNextResults and twoColumnBrowseResultsRenderer formats
pub fn extract_playlist_title(yt_data: &YtInitialData) -> Result<String> {
    // Try extracting from top-level header first (for browse results)
    if let Some(header) = &yt_data.header {
        // Try pageHeaderRenderer
        if let Some(page_header) = &header.page_header_renderer {
            return Ok(page_header.page_title.clone());
        }
    }

    // Try twoColumnWatchNextResults format
    if let Some(watch_next) = &yt_data.contents.two_column_watch_next_results {
        let playlist = &watch_next.playlist.playlist;
        if let Some(title) = &playlist.title {
            return Ok(title.to_string());
        }
    }

    // Fallback to default title if not found
    Ok("YouTube Playlist".to_string())
}
