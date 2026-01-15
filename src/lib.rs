pub mod bilibili;
pub mod core;
mod download;
pub mod error;
pub mod file;
pub mod youtube; // legacy direct file link extractor

pub use bilibili::BilibiliExtractor;
pub use core::{Audio, Extractor, Platform, PlayList};
pub use file::FileExtractor;
use error::{MusicFreeError, Result};

use crate::youtube::YoutubeExtractor;

static EXTRACTORS: &[&dyn Extractor] = &[
    &BilibiliExtractor,
    &YoutubeExtractor,
    &FileExtractor,
];

/// Extract audio from URL (auto-detect platform) with HTTP fallback
pub async fn extract(url: &str) -> Result<Vec<Audio>> {
    // Try all known extractors first; if any succeeds, return.
    for i in EXTRACTORS {
        if i.matches(url)
            && let Ok(res) = i.extract(url).await
        {
            return Ok(res);
        }
    }
    Err(MusicFreeError::PlatformNotSupported(format!(
        "No extractor found for: {}",
        url
    )))
}
