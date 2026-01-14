use serde::{Deserialize, Serialize};

pub mod bilibili;
pub mod error;
pub mod youtube;

use error::{MusicFreeError, Result};

/// Unified audio structure for download results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
    pub title: String,
    pub data: Vec<u8>,
    pub source: Site,
}

/// Supported sites
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub async fn download_audio(url: &str) -> Result<Audio> {
    let site = detect_site(url)?;
    match site {
        Site::Bilibili => {
            let info = bilibili::download_audio(url).await?;
            Ok(Audio {
                title: info.title,
                data: info.data,
                source: Site::Bilibili,
            })
        }
        Site::YouTube => {
            let info = youtube::download_audio(url).await?;
            Ok(Audio {
                title: info.title,
                data: info.data,
                source: Site::YouTube,
            })
        }
    }
}
