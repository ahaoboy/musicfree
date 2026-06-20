//! Web HTML player response + EJS decryption download strategy.
//!
//! Used as fallback when Android API download fails.
//! Extracts the player response embedded in the YouTube watch page HTML,
//! decrypts signatureCipher/n via EJS, and downloads with a Web browser UA.

use crate::download::{download_binary_chunked, download_text};
use crate::error::{MusicFreeError, Result};
use crate::youtube::core::{
    extract_audio_formats_web, get_player_url, parse_player_response_from_html,
    select_best_audio_format,
};
use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, CONNECTION, HeaderMap, HeaderName, HeaderValue, ORIGIN, REFERER,
    USER_AGENT,
};

use super::utils::WEB_USER_AGENT;

#[cfg(feature = "ytdlp-ejs")]
use crate::youtube::ejs::{solve_cipher, solve_n};

/// Download audio via Web HTML player response + EJS decryption.
///
/// 1. Extract `ytInitialPlayerResponse` from the watch page HTML
/// 2. Select best audio format
/// 3. Decrypt signatureCipher/n via EJS
/// 4. Download with Web UA + Accept-Encoding: identity
pub async fn web_download(html: &str) -> Result<Vec<u8>> {
    let player_response = parse_player_response_from_html(html)?;

    #[cfg(debug_assertions)]
    {
        let title = &player_response.video_details.title;
        eprintln!("[debug] Web HTML: title={title:?}");
    }

    let formats = extract_audio_formats_web(&player_response)?;
    let format = select_best_audio_format(&formats)?;

    #[cfg(debug_assertions)]
    eprintln!(
        "[debug] Web selected format: itag={}, mime={}",
        format.itag, format.mime_type
    );

    let download_url = resolve_web_url(format, html).await?;

    #[cfg(debug_assertions)]
    eprintln!("[debug] Web download URL: {download_url}");

    let mut dl_headers = HeaderMap::new();
    dl_headers.insert(USER_AGENT, HeaderValue::from_static(WEB_USER_AGENT));
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

/// Resolve the download URL for a Web format.
/// Web formats often use `signatureCipher` which needs both s and n decryption,
/// or have a plain `url` with just `&n=` that needs decryption.
async fn resolve_web_url(format: &crate::youtube::types::Format, html: &str) -> Result<String> {
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
        let player_url = get_player_url(html)
            .await
            .ok_or(MusicFreeError::PlayerJsNotFound)?;
        let player_js_content = download_text(&player_url, HeaderMap::new()).await?;

        #[cfg(debug_assertions)]
        {
            let js_preview: String = player_js_content.chars().take(200).collect();
            eprintln!("[debug] Web Player JS URL: {player_url}");
            eprintln!("[debug] Web Player JS preview: {js_preview}...");
        }

        if let Some(raw_url) = &format.url {
            #[cfg(debug_assertions)]
            eprintln!("[debug] Web raw URL: {raw_url}");

            if raw_url.contains("&n=") {
                let decrypted = solve_n(raw_url, player_js_content.clone())?;
                #[cfg(debug_assertions)]
                eprintln!("[debug] Web n-decrypted URL: {decrypted}");
                return Ok(decrypted);
            }
            return Ok(raw_url.clone());
        }

        if let Some(cipher) = &format.signature_cipher {
            #[cfg(debug_assertions)]
            eprintln!("[debug] Web cipher: {cipher}");

            let decrypted = solve_cipher(cipher, player_js_content)?;
            #[cfg(debug_assertions)]
            eprintln!("[debug] Web decrypted from cipher: {decrypted}");
            return Ok(decrypted);
        }

        Err(MusicFreeError::AudioNotFound)
    }
}
