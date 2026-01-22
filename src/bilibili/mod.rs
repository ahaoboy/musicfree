use crate::Playlist;
use crate::bilibili::core::{download_audio, extract_audio};
use crate::core::{Extractor, Platform};
use crate::error::Result;
use async_trait::async_trait;

pub mod core;
pub mod types;
pub mod utils;

// Re-export commonly used types and functions
pub use core::get_audio_info;
pub use types::{
    Audio, AudioInfo, Dash, Durl, Episode, EpisodeArc, EpisodePage, Owner, PlayData,
    PlayUrlResponse, Section, UgcSession, ViewData, ViewResponse,
};
pub use utils::{is_bilibili_short_url, is_bilibili_url, parse_id, resolve_short_link};

/// Bilibili extractor implementing the Extractor trait
#[derive(Debug, Clone)]
pub struct BilibiliExtractor;

#[async_trait]
impl Extractor for BilibiliExtractor {
    fn matches(&self, url: &str) -> bool {
        is_bilibili_url(url)
    }

    async fn extract(&self, url: &str) -> Result<(Playlist, Option<usize>)> {
        extract_audio(url).await
    }

    fn platform(&self) -> Platform {
        Platform::Bilibili
    }

    async fn download(&self, url: &str) -> Result<Vec<u8>> {
        download_audio(url).await
    }
}
