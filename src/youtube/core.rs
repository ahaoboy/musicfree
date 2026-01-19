use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, ORIGIN, USER_AGENT};
use std::collections::HashMap;
use url::Url;

use crate::download::{download_binary, download_text, post_json};
use crate::error::{MusicFreeError, Result};

use crate::youtube::types::{
    ContentPlaybackContext, Format, InnertubeContext, InnertubeRequest, PlaybackContext,
    PlayerResponse, PlaylistContent, Title, YtConfig, YtInitialData,
};
use crate::youtube::utils::{ANDROID_USER_AGENT, WEB_USER_AGENT};
use crate::{Audio, AudioFormat, Platform, Playlist};

use ytdlp_ejs::{
    JsChallengeInput, JsChallengeOutput, JsChallengeRequest, JsChallengeResponse, JsChallengeType,
    RuntimeType,
};

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
    let formats = player_response
        .streaming_data
        .formats
        .iter()
        .filter(|i| i.url.is_some() || i.signature_cipher.is_some());
    Ok(formats.collect())
}

fn solve_n(url_str: &str, player: String) -> Result<String> {
    let mut url_obj = Url::parse(url_str)
        .map_err(|e| MusicFreeError::CipherParseError(format!("Failed to parse URL: {}", e)))?;

    // Extract n parameter from URL query params
    let n = url_obj
        .query_pairs()
        .find(|(k, _)| k == "n")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| {
            MusicFreeError::CipherParseError("Parameter 'n' not found in URL".to_string())
        })?;

    // Execute JS challenge for n parameter only
    let input = JsChallengeInput::Player {
        player,
        requests: vec![JsChallengeRequest {
            challenge_type: JsChallengeType::N,
            challenges: vec![n.clone()],
        }],
        output_preprocessed: false,
    };

    let output = ytdlp_ejs::process_input(input, RuntimeType::QuickJS);

    match output {
        JsChallengeOutput::Result { responses, .. } => {
            // Check if result type is 'result' and responses[0] exists
            if let Some(first_response) = responses.first()
                && let JsChallengeResponse::Result { data } = first_response
            {
                // Get the transformed n value and set it in URL
                if let Some(new_n) = data.get(&n) {
                    // Remove existing n parameter and add new one
                    let mut pairs: Vec<(String, String)> = url_obj
                        .query_pairs()
                        .filter(|(k, _)| k != "n")
                        .map(|(k, v)| (k.into_owned(), v.into_owned()))
                        .collect();
                    // Rebuild URL with updated parameters
                    pairs.push(("n".to_owned(), new_n.to_string()));
                    url_obj.query_pairs_mut().clear().extend_pairs(pairs);
                    return Ok(url_obj.to_string());
                }
            }
            Err(MusicFreeError::JsDecryptionFailed(
                "Failed to get valid response for n parameter".to_string(),
            ))
        }
        JsChallengeOutput::Error { error } => Err(MusicFreeError::JsDecryptionFailed(format!(
            "JS execution failed: {}",
            error
        ))),
    }
}

fn solve_cipher(cipher_str: &str, player: String) -> Result<String> {
    // A. Parse signatureCipher (query string)
    let cipher_params: HashMap<String, String> = url::form_urlencoded::parse(cipher_str.as_bytes())
        .into_owned()
        .collect();

    let url_str = cipher_params
        .get("url")
        .ok_or_else(|| MusicFreeError::CipherParseError("Missing url in cipher".to_string()))?;
    let sp = cipher_params.get("sp").map(|s| s.as_str()).unwrap_or("sig");
    let s = cipher_params
        .get("s")
        .ok_or_else(|| MusicFreeError::CipherParseError("Missing s in cipher".to_string()))?;

    // B. Extract 'n' parameter from URL
    let mut url_obj = Url::parse(url_str)
        .map_err(|e| MusicFreeError::CipherParseError(format!("Failed to parse URL: {}", e)))?;
    let n = url_obj
        .query_pairs()
        .find(|(k, _)| k == "n")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| {
            MusicFreeError::CipherParseError("Parameter 'n' not found in URL".to_string())
        })?;

    // C. Construct JS execution input
    let input = JsChallengeInput::Player {
        player,
        requests: vec![
            JsChallengeRequest {
                challenge_type: JsChallengeType::N,
                challenges: vec![n.clone()],
            },
            JsChallengeRequest {
                challenge_type: JsChallengeType::Sig,
                challenges: vec![s.to_string()],
            },
        ],
        output_preprocessed: false,
    };

    // Execute JS
    let output = ytdlp_ejs::process_input(input, RuntimeType::QuickJS);

    match output {
        JsChallengeOutput::Result { responses, .. } => {
            let mut transformed_n: Option<String> = None;
            let mut deciphered_sig: Option<String> = None;

            // Iterate through responses to find n and s data
            for response in responses {
                if let JsChallengeResponse::Result { data } = response {
                    if let Some(val) = data.get(&n) {
                        transformed_n = Some(val.clone());
                    }
                    if let Some(val) = data.get(s) {
                        deciphered_sig = Some(val.clone());
                    }
                }
            }

            if let (Some(new_n), Some(new_sig)) = (transformed_n, deciphered_sig) {
                // D. Construct final URL
                // 1. Collect old parameters and replace 'n'
                let mut pairs: Vec<(String, String)> = url_obj
                    .query_pairs()
                    .filter(|(k, _)| k != "n" && k != sp)
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect();
                pairs.push(("n".to_owned(), new_n.to_string()));
                pairs.push((sp.to_owned(), new_sig.to_string()));
                url_obj.query_pairs_mut().clear().extend_pairs(pairs);
                let final_url = url_obj.to_string();
                Ok(final_url)
            } else {
                Err(MusicFreeError::JsDecryptionFailed(
                    "Failed to decrypt cipher parameters".to_string(),
                ))
            }
        }
        JsChallengeOutput::Error { error } => Err(MusicFreeError::JsDecryptionFailed(format!(
            "JS execution failed: {}",
            error
        ))),
    }
}

pub fn resolve_url(format: &Format, player: String) -> Result<String> {
    if let Some(url) = &format.url {
        return solve_n(url, player);
    }

    if let Some(c) = &format.signature_cipher {
        return solve_cipher(c, player);
    }

    Err(MusicFreeError::AudioNotFound)
}

/// Fetch player response from YouTube Android API
pub async fn parse_player(video_id: &str, ytcfg: &YtConfig) -> Result<PlayerResponse> {
    let api_url = format!(
        "https://www.youtube.com/youtubei/v1/player?key={}&prettyPrint=false",
        ytcfg.innertube_api_key
    );

    let request_body = InnertubeRequest {
        video_id: video_id.to_string(),
        context: InnertubeContext {
            client: ytcfg.innertube_context.client.clone(),
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
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(
            ytcfg
                .innertube_context
                .client
                .get("userAgent")
                .and_then(|i| i.as_str())
                .unwrap_or(ANDROID_USER_AGENT),
        )?,
    );
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
pub async fn extract_audio(url: &str) -> Result<Playlist> {
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
        })
        .collect();

    let playlist = Playlist {
        title: title.clone(),
        audios,
        cover: Some(format!("https://i.ytimg.com/vi/{video_id}/hq720.jpg")),
        platform: Platform::Youtube,
    };

    Ok(playlist)
}

/// Extract playlist audio from YouTube playlist URL
async fn extract_playlist_audio(url: &str, html: &str) -> Result<Playlist> {
    let yt_data = parse_yt_initial_data(html)?;
    let videos = extract_playlist_videos(&yt_data)?;

    let playlist_id = crate::youtube::utils::parse_playlist_id(url)
        .ok_or_else(|| MusicFreeError::InvalidUrl("Cannot extract playlist ID".to_string()))?;

    // Extract playlist title from ytInitialData
    let playlist_title = extract_playlist_title(&yt_data)?;

    let mut audios = Vec::new();

    // Process each video in the playlist
    for (title, video_url, video_id) in videos {
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
        title: playlist_title,
        audios,
        cover: Some(format!("https://i.ytimg.com/vi/{playlist_id}/hq720.jpg")),
        platform: Platform::Youtube,
    };

    Ok(playlist)
}

/// Download audio using web client with EJS decryption
pub async fn download_audio(url: &str) -> Result<Vec<u8>> {
    let video_id = &crate::youtube::utils::parse_id(url)?;

    // Step 1: Fetch video page
    let pagr_url = format!("https://www.youtube.com/watch?v={}", video_id);
    let html = download_text(&pagr_url, HeaderMap::new()).await?;

    let ytcfg = parse_ytcfg(&html)?;

    // Step 2: Extract player response from HTML
    let player_response = if let Ok(pr) = parse_player_response_from_html(&html) {
        pr
    } else {
        parse_player(video_id, &ytcfg).await?
    };

    let player_url = get_player_url(&html)
        .await
        .ok_or(MusicFreeError::PlayerJsNotFound)?;
    // Step 3: Download player JS if available
    let player_js_content = download_text(&player_url, HeaderMap::new()).await?;

    // Step 4: Extract audio formats
    let formats = extract_audio_formats_web(&player_response)?;

    // Step 5: Select best audio format (prefer itag 140)
    let format = formats
        .iter()
        .find(|f| f.itag == 140)
        .or_else(|| formats.first())
        .ok_or(MusicFreeError::AudioNotFound)?;

    let download_url = resolve_url(format, player_js_content)?;

    let mut headers = HeaderMap::new();
    let ua = &ytcfg
        .innertube_context
        .client
        .get("userAgent")
        .and_then(|i| i.as_str())
        .unwrap_or(WEB_USER_AGENT)
        .to_string();
    if let Ok(val) = HeaderValue::from_str(ua) {
        headers.insert(USER_AGENT, val);
    }
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://www.youtube.com/"),
    );
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
