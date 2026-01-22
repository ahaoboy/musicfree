pub mod core;
mod download;
pub mod error;
pub mod file;

#[cfg(feature = "bilibili")]
pub mod bilibili;

#[cfg(feature = "youtube")]
pub mod youtube;

pub use core::*;
use error::{MusicFreeError, Result};
pub use file::FileExtractor;
mod utils;

#[cfg(feature = "bilibili")]
pub use bilibili::BilibiliExtractor;

#[cfg(feature = "youtube")]
pub use crate::youtube::YoutubeExtractor;

pub static EXTRACTORS: &[&dyn Extractor] = &[
    #[cfg(feature = "bilibili")]
    &BilibiliExtractor,
    #[cfg(feature = "youtube")]
    &YoutubeExtractor,
    &FileExtractor,
];

/// Extract audio from URL (auto-detect platform) with HTTP fallback
pub async fn extract(url: &str) -> Result<(Playlist, Option<usize>)> {
    // Try all known extractors first; if any succeeds, return.
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
