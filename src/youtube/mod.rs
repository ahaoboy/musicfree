use crate::Playlist;
use crate::core::{Extractor, Platform};
use crate::error::Result;
use crate::youtube::core::download_audio;
use async_trait::async_trait;

pub mod core;
pub mod types;
pub mod utils;

// Re-export commonly used types and functions
pub use core::{
    extract_audio, extract_audio_formats_web, parse_player, parse_player_response_from_html,
    parse_ytcfg, resolve_url,
};
pub use types::{
    ContentPlaybackContext, Format, InnertubeContext, InnertubeRequest, PlaybackContext,
    PlayerResponse, YtConfig,
};
pub use utils::{is_youtube_url, parse_id};

/// YouTube extractor implementing the Extractor trait
#[derive(Debug, Clone)]
pub struct YoutubeExtractor;

#[async_trait]
impl Extractor for YoutubeExtractor {
    fn matches(&self, url: &str) -> bool {
        is_youtube_url(url)
    }

    async fn extract(&self, url: &str) -> Result<Playlist> {
        extract_audio(url).await
    }

    async fn download(&self, url: &str) -> Result<Vec<u8>> {
        download_audio(url).await
    }

    fn platform(&self) -> Platform {
        Platform::Youtube
    }
}
