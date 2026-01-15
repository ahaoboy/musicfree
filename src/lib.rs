pub mod bilibili;
mod download;
pub mod error;
pub mod youtube;
use error::{MusicFreeError, Result};
pub mod core;

pub use bilibili::BilibiliExtractor;
pub use core::{Audio, Extractor, Platform, PlayList};
pub use youtube::YoutubeExtractor;

static EXTRACTORS: &[&dyn Extractor] = &[&BilibiliExtractor, &YoutubeExtractor];

/// Extract audio from URL (auto-detect platform)
pub async fn extract(url: &str) -> Result<Vec<Audio>> {
    for i in EXTRACTORS {
        if i.matches(url) {
            return i.extract(url).await;
        }
    }

    Err(MusicFreeError::PlatformNotSupported(format!(
        "No extractor found for: {}",
        url
    )))
}
