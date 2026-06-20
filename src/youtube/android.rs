//! Android Innertube API download strategy.
//!
//! Uses the Android YouTube client (`c=ANDROID`) which provides more lenient
//! CDN URLs that don't require a full browser TLS fingerprint.

use crate::download::{download_binary_chunked, download_text};
use crate::error::{MusicFreeError, Result};
use crate::youtube::core::{
    extract_audio_formats_web, get_player_url, parse_player, select_best_audio_format,
};
use crate::youtube::types::{Format, YtConfig};
use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, CONNECTION, HeaderMap, HeaderName, HeaderValue, ORIGIN, REFERER,
    USER_AGENT,
};

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

    let formats = extract_audio_formats_web(&player_response)?;
    let format = select_best_audio_format(&formats)?;

    #[cfg(debug_assertions)]
    eprintln!(
        "[debug] Android selected format: itag={}, mime={}",
        format.itag, format.mime_type
    );

    let download_url = resolve_android_url(format, html).await?;

    #[cfg(debug_assertions)]
    eprintln!("[debug] Android download URL: {download_url}");

    let mut dl_headers = HeaderMap::new();
    dl_headers.insert(USER_AGENT, HeaderValue::from_static(ANDROID_VR_USER_AGENT));
    dl_headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    dl_headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    dl_headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    dl_headers.insert(
        REFERER,
        HeaderValue::from_static("https://www.youtube.com/"),
    );
    dl_headers.insert(ORIGIN, HeaderValue::from_static("https://www.youtube.com"));
    dl_headers.insert(
        HeaderName::from_static("sec-fetch-mode"),
        HeaderValue::from_static("no-cors"),
    );
    dl_headers.insert(
        HeaderName::from_static("sec-fetch-site"),
        HeaderValue::from_static("cross-site"),
    );

    download_binary_chunked(&download_url, dl_headers).await
}

/// Resolve the download URL for an Android format.
/// Android URLs usually have a plain `url` field with just `&n=` that needs
/// decryption. They rarely use `signatureCipher`.
async fn resolve_android_url(format: &Format, html: &str) -> Result<String> {
    #[cfg(not(feature = "ytdlp-ejs"))]
    {
        format
            .url
            .clone()
            .or_else(|| format.signature_cipher.clone())
            .ok_or(MusicFreeError::AudioNotFound)
    }

    #[cfg(feature = "ytdlp-ejs")]
    {
        if let Some(raw_url) = &format.url {
            if raw_url.contains("&n=") {
                let player_url = get_player_url(html)
                    .await
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
            let player_url = get_player_url(html)
                .await
                .ok_or(MusicFreeError::PlayerJsNotFound)?;
            let player_js_content = download_text(&player_url, HeaderMap::new()).await?;
            return crate::youtube::ejs::solve_cipher(cipher, player_js_content);
        }

        Err(MusicFreeError::AudioNotFound)
    }
}
