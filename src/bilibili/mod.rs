use crate::bilibili::api::{download, extract, is_bilibili_url};
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
        extract(url).await
    }

    fn platform(&self) -> Platform {
        Platform::Bilibili
    }

    async fn download(&self, audio: &mut Audio) -> Result<()> {
        download(audio).await
    }
}
