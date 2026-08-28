use crate::Playlist;
use crate::core::{Extractor, Platform};
use crate::error::Result;
use async_trait::async_trait;

pub mod android;
pub mod core;
pub mod utils;
pub mod web;

// Re-export the public API used by the `Extractor` implementation.
pub use core::*;
pub use utils::parse_id;

#[cfg(feature = "ytdlp-ejs")]
mod ejs;

/// YouTube extractor implementing the Extractor trait
#[derive(Debug, Clone)]
pub struct YoutubeExtractor;

#[async_trait]
impl Extractor for YoutubeExtractor {
    fn matches(&self, url: &str) -> bool {
        serde_youtube::url::is_youtube_url(url)
    }

    async fn extract(&self, url: &str) -> Result<(Playlist, Option<usize>)> {
        extract_audio(url).await
    }

    async fn download(&self, url: &str) -> Result<Vec<u8>> {
        download_audio(url).await
    }

    fn platform(&self) -> Platform {
        Platform::Youtube
    }
}
