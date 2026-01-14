pub mod bilibili;
pub mod error;
pub mod youtube;

use error::{MusicFreeError, Result};

/// Supported sites
pub enum Site {
    Bilibili,
    YouTube,
}

/// Detect site from URL
pub fn detect_site(url: &str) -> Result<Site> {
    if bilibili::is_bilibili_url(url) {
        return Ok(Site::Bilibili);
    }
    if youtube::is_youtube_url(url) {
        return Ok(Site::YouTube);
    }
    Err(MusicFreeError::UnsupportedSite(url.to_string()))
}

/// Sanitize filename by removing invalid characters
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// Download audio from URL (auto-detect site)
pub async fn download_audio(url: &str) -> Result<(String, Vec<u8>)> {
    match detect_site(url)? {
        Site::Bilibili => {
            let info = bilibili::download_audio(url).await?;
            Ok((info.title, info.data))
        }
        Site::YouTube => {
            let info = youtube::download_audio(url).await?;
            Ok((info.title, info.data))
        }
    }
}
