use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;

use crate::download::download_text;
use crate::error::{MusicFreeError, Result};
use crate::utils::get_md5;
use crate::{Audio, Platform};

use super::common::{
    AudioFormat, WEB_USER_AGENT, download_audio_data, extract_ytcfg_from_html, fetch_video_page,
    get_video_title,
};

use ytdlp_ejs::{JsChallengeOutput, RuntimeType};

/// Download player JS file
async fn download_player_js(player_url: &str) -> Result<String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(WEB_USER_AGENT));

    download_text(player_url, headers).await
}

/// Extract player response from HTML
fn extract_player_response_from_html(html: &str) -> Result<Value> {
    let patterns = [
        r#"var\s+ytInitialPlayerResponse\s*=\s*(\{.+?\})\s*;"#,
        r#"ytInitialPlayerResponse\s*=\s*(\{.+?\})\s*;"#,
    ];

    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(html)
            && let Some(json_str) = caps.get(1)
        {
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

    Err(MusicFreeError::ParseError(
        "Cannot extract player response from HTML".to_string(),
    ))
}

fn decrypt(player: &str, challenges: Vec<String>) -> Option<JsChallengeOutput> {
    ytdlp_ejs::run(player.to_string(), RuntimeType::QuickJS, challenges).ok()
}

/// Process format URL with signature and n parameter decryption
fn process_format_url(format: &Value, player: String) -> Option<String> {
    let mut url = format
        .get("url")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string());

    // Handle signatureCipher
    if url.is_none()
        && let Some(cipher) = format.get("signatureCipher").and_then(|c| c.as_str())
    {
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
                    ytdlp_ejs::JsChallengeResponse::Result { data } => {
                        url = Some(format!(
                            "{}&{}={}",
                            base_url,
                            sp,
                            urlencoding::encode(data.get(&s).unwrap())
                        ));
                    }
                    ytdlp_ejs::JsChallengeResponse::Error { error: _ } => todo!(),
                },
                JsChallengeOutput::Error { error: _ } => todo!(),
            }
        }
    }

    let mut url = url?;

    // Process n parameter
    if let Ok(parsed_url) = reqwest::Url::parse(&url)
        && let Some(n_value) = parsed_url
            .query_pairs()
            .find(|(k, _)| k == "n")
            .map(|(_, v)| v.to_string())
        && let Some(decrypted_n) = decrypt(&player, vec![format!("n:{n_value}")])
    {
        match decrypted_n {
            JsChallengeOutput::Result {
                preprocessed_player: _,
                responses,
            } => {
                match &responses[0] {
                    ytdlp_ejs::JsChallengeResponse::Result { data } => {
                        // Replace n parameter in URL
                        let new_url = url.replace(
                            &format!("n={}", n_value),
                            &format!("n={}", data.get(&n_value).unwrap()),
                        );
                        url = new_url;
                    }
                    ytdlp_ejs::JsChallengeResponse::Error { error: _ } => todo!(),
                }
            }
            JsChallengeOutput::Error { error: _ } => todo!(),
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
pub async fn download_audio_ejs(video_id: &str) -> Result<Audio> {
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

    // Step 5: Extract audio formats
    let formats = extract_audio_formats_web(&player_response, player_js_content.unwrap())?;

    // Step 6: Get title
    let title = get_video_title(&player_response);

    // Step 7: Select best audio format (prefer itag 140)
    let format = formats
        .iter()
        .find(|f| f.itag == 140)
        .or_else(|| formats.first())
        .ok_or(MusicFreeError::AudioNotFound)?;

    // Step 8: Download audio
    let _data = download_audio_data(&format.url).await?;
    let audio = Audio::new(
        get_md5(&format.url),
        title,
        format.url.to_string(),
        Platform::Youtube,
    );

    Ok(audio)
}
