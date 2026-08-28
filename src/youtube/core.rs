use crate::download::{download_text, post_json};
use crate::error::{MusicFreeError, Result};
use crate::youtube::utils::{WEB_USER_AGENT, is_valid_playlist_id, parse_playlist_id};
use crate::youtube::parse_id;
use crate::{Audio, AudioFormat, Platform, Playlist};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, ORIGIN, USER_AGENT};
use serde_youtube::types::{
    ContentPlaybackContext, InnertubeContext, InnertubeRequest, PlaybackContext, PlayerResponse,
    YtConfig, YtInitialData,
};
use serde_youtube::url::{
    is_valid_video_id, playlist_url as build_playlist_url, thumbnail_url as build_thumbnail_url,
    watch_url as build_watch_url,
};

/// Map a `serde_youtube` parsing error into a musicfree config error.
fn map_yt_error(e: serde_youtube::Error) -> MusicFreeError {
    MusicFreeError::ConfigParseError(e.to_string())
}

/// Extract `ytcfg` configuration from HTML (via the shared parser).
pub fn parse_ytcfg(html: &str) -> Result<YtConfig> {
    serde_youtube::parser::extract_ytcfg(html).map_err(map_yt_error)
}

/// Extract `ytInitialPlayerResponse` from HTML (via the shared parser).
///
/// Scans every embedded blob and returns the first that deserializes into a
/// [`PlayerResponse`], skipping `null`/incompatible blocks.
pub fn parse_player_response_from_html(html: &str) -> Result<PlayerResponse> {
    serde_youtube::parser::extract_player_response(html).map_err(map_yt_error)
}

/// Extract `ytInitialData` from HTML (via the shared parser).
pub fn parse_yt_initial_data(html: &str) -> Result<YtInitialData> {
    serde_youtube::parser::extract_yt_initial_data(html).map_err(map_yt_error)
}

/// Fetch player response from YouTube Android API
pub async fn parse_player(video_id: &str, ytcfg: &YtConfig) -> Result<PlayerResponse> {
    let api_url = format!(
        "https://www.youtube.com/youtubei/v1/player?key={}&prettyPrint=false",
        ytcfg.innertube_api_key
    );

    // Try the plain `ANDROID` client (yt-dlp uses it and marks it as not
    // requiring a PO token for the player). It returns stream URLs that the
    // googlevideo CDN accepts.
    let client = serde_json::json!({
          "clientName": "ANDROID",
          "clientVersion": "21.26.364",
          "androidSdkVersion": 30,
          "userAgent": "com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip",
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
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(
            "com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip",
        )?,
    );
    headers.insert("X-YouTube-Client-Name", HeaderValue::from_static("3"));
    headers.insert(
        "X-YouTube-Client-Version",
        HeaderValue::from_static("21.26.364"),
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
    // Prefer the player response embedded in the watch page HTML. Fall back to
    // the Android API when the embedded response is missing, or when it carries
    // no *usable* audio format (YouTube often omits direct stream URLs on watch
    // pages, so an otherwise-valid embedded response can still lack them).
    let player_response = match parse_player_response_from_html(&html) {
        Ok(pr) if pr.streaming_data.best_playable_format().is_some() => pr,
        _ => parse_player(video_id, &ytcfg).await?,
    };
    let title = &player_response.video_details.title;
    // Prefer a pure audio format (itag 140 m4a), but fall back to any
    // downloadable stream (e.g. a muxed video/audio format) when YouTube only
    // exposes direct URLs for muxed formats.
    let best_format = player_response
        .streaming_data
        .best_playable_format()
        .ok_or(MusicFreeError::AudioNotFound)?;
    let audios: Vec<Audio> = [best_format]
        .into_iter()
        .map(|i| {
            let mut audio = Audio::new(
                video_id.clone(),
                title.clone(),
                build_watch_url(video_id),
                Platform::Youtube,
            )
            .with_format(AudioFormat::from_youtube(&i.mime_type))
            .with_bitrate(i.bitrate)
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
    // Unify both watch-next and browse playlist layouts into one list.
    let videos = yt_data.playlist_video_infos();
    let playlist_title = yt_data
        .playlist_title()
        .unwrap_or_else(|| "YouTube Playlist".to_string());

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

/// Download audio with Android-first, Web+EJS fallback strategy (yt-dlp style).
///
/// Phase 1 (Android): Fetch Android Innertube API → `c=ANDROID` URL → download.
///   Android CDN is more lenient and usually works without a browser fingerprint.
///
/// Phase 2 (Web fallback): If Android fails, extract player_response from web HTML,
///   decrypt signatureCipher via EJS, and download with Web UA.
pub async fn download_audio(url: &str) -> Result<Vec<u8>> {
    let video_id = &parse_id(url)?;
    let page_url = build_watch_url(video_id);
    let mut web_headers = HeaderMap::new();
    web_headers.insert(USER_AGENT, HeaderValue::from_static(WEB_USER_AGENT));
    let html = download_text(&page_url, web_headers).await?;
    let ytcfg = parse_ytcfg(&html)?;

    match crate::youtube::android::android_download(video_id, &ytcfg, &html).await {
        Ok(data) => return Ok(data),
        #[allow(unused_variables)]
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("[debug] Android download failed: {e}, falling back to Web+EJS");
        }
    }
    crate::youtube::web::web_download(&html).await
}
