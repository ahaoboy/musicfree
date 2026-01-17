use crate::bilibili::api::{download_audio, is_bilibili_url};
use crate::core::{Audio, Extractor, Platform};
use crate::error::Result;
use async_trait::async_trait;
pub mod api;
pub mod core;

/// Bilibili extractor implementing the Extractor trait
#[derive(Debug, Clone)]
pub struct BilibiliExtractor;

#[async_trait]
impl Extractor for BilibiliExtractor {
    fn matches(&self, url: &str) -> bool {
        is_bilibili_url(url)
    }

    async fn extract(&self, url: &str) -> Result<Vec<Audio>> {
        let audio = download_audio(url).await?;
        Ok(audio)
    }

    fn platform(&self) -> Platform {
        Platform::Bilibili
    }
}
