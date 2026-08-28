//! Android Innertube API download strategy.
//!
//! Uses the Android YouTube client (`c=ANDROID`) which provides more lenient
//! CDN URLs that don't require a full browser TLS fingerprint.

use crate::download::{download_binary_chunked, download_text};
use crate::headers::{download_headers, with_origin, with_sec_fetch};
use crate::error::{MusicFreeError, Result};
use crate::youtube::core::parse_player;
use reqwest::header::HeaderMap;
use serde_youtube::types::{Format, YtConfig};

use super::utils::ANDROID_VR_USER_AGENT;

#[cfg(feature = "ytdlp-ejs")]
use crate::youtube::ejs::solve_n;

/// Download audio via Android Innertube API.
///
/// 1. Call `parse_player` to get Android player response
/// 2. Select best audio format (itag 140 preferred)
/// 3. n-decrypt the URL via EJS if needed
/// 4. Download with Android UA + Accept-Encoding: identity
pub async fn android_download(video_id: &str, ytcfg: &YtConfig, html: &str) -> Result<Vec<u8>> {
    let player_response = parse_player(video_id, ytcfg).await?;

    #[cfg(debug_assertions)]
    {
        let title = &player_response.video_details.title;
        let format_count = player_response.streaming_data.formats.len()
            + player_response.streaming_data.adaptive_formats.len();
        eprintln!("[debug] Android API: title={title:?}, formats={format_count}");
    }

    let format = player_response
        .streaming_data
        .best_audio_format()
        .or_else(|| player_response.streaming_data.first_downloadable())
        .ok_or(MusicFreeError::AudioNotFound)?;

    #[cfg(debug_assertions)]
    eprintln!(
        "[debug] Android selected format: itag={}, mime={}",
        format.itag, format.mime_type
    );

    let download_url = resolve_android_url(format, html).await?;

    #[cfg(debug_assertions)]
    eprintln!("[debug] Android download URL: {download_url}");

    let mut dl_headers = download_headers(ANDROID_VR_USER_AGENT, "https://www.youtube.com/");
    with_origin(&mut dl_headers, "https://www.youtube.com");
    with_sec_fetch(&mut dl_headers);

    download_binary_chunked(&download_url, dl_headers).await
}

/// Resolve the download URL for an Android format.
/// Android URLs usually have a plain `url` field with just `&n=` that needs
/// decryption. They rarely use `signatureCipher`.
async fn resolve_android_url(format: &Format, html: &str) -> Result<String> {
    #[cfg(not(feature = "ytdlp-ejs"))]
    {
        format.candidate_url().ok_or(MusicFreeError::AudioNotFound)
    }

    #[cfg(feature = "ytdlp-ejs")]
    {
        if let Some(raw_url) = &format.url {
            if raw_url.contains("&n=") {
                let player_url = serde_youtube::parser::player_js_url(html)
                    .ok_or(MusicFreeError::PlayerJsNotFound)?;
                let player_js_content = download_text(&player_url, HeaderMap::new()).await?;

                #[cfg(debug_assertions)]
                eprintln!("[debug] Android n-decrypt: {raw_url}");

                let decrypted = solve_n(raw_url, player_js_content)?;

                #[cfg(debug_assertions)]
                eprintln!("[debug] Android n-decrypted: {decrypted}");

                return Ok(decrypted);
            }
            return Ok(raw_url.clone());
        }

        if let Some(cipher) = &format.signature_cipher {
            let player_url = serde_youtube::parser::player_js_url(html)
                .ok_or(MusicFreeError::PlayerJsNotFound)?;
            let player_js_content = download_text(&player_url, HeaderMap::new()).await?;
            return crate::youtube::ejs::solve_cipher(cipher, player_js_content);
        }

        Err(MusicFreeError::AudioNotFound)
    }
}
