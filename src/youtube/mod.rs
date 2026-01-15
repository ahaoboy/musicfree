mod android;
mod common;
use crate::core::{Audio, Extractor, Platform};
use crate::error::Result;
use async_trait::async_trait;
pub use common::{AudioFormat, extract_video_id, is_youtube_url};
#[cfg(feature = "ytdlp-ejs")]
mod web;

/// Download audio from YouTube
pub async fn download_audio(url: &str) -> Result<Audio> {
    let video_id = extract_video_id(url)?;

    #[cfg(feature = "ytdlp-ejs")]
    {
        match web::download_audio_ejs(&video_id).await {
            Ok(info) => return Ok(info),
            Err(e) => {
                eprintln!("Web(EJS) client failed: {e}, falling back to Android client...");
            }
        }
    }

    android::download_audio_android(&video_id).await
}

/// Get available audio formats without downloading
pub async fn get_audio_formats(url: &str) -> Result<(String, Vec<AudioFormat>)> {
    let video_id = extract_video_id(url)?;
    android::get_audio_formats_android(&video_id).await
}

/// YouTube extractor implementing the Extractor trait
#[derive(Debug, Clone)]
pub struct YoutubeExtractor;

#[async_trait]
impl Extractor for YoutubeExtractor {
    fn matches(&self, url: &str) -> bool {
        is_youtube_url(url)
    }

    async fn extract(&self, url: &str) -> Result<Vec<Audio>> {
        let audio = download_audio(url).await?;
        Ok(vec![audio])
    }

    fn platform(&self) -> Platform {
        Platform::Youtube
    }
}
