#[cfg(feature = "bilibili")]
pub mod bilibili;
#[cfg(feature = "file")]
pub mod file;
#[cfg(feature = "youtube")]
pub mod youtube;

use crate::{Error, Playlist, Result, Site};
use async_trait::async_trait;
use reqwest::header::HeaderMap;

#[cfg(feature = "bilibili")]
pub use bilibili::Bilibili;
#[cfg(feature = "file")]
pub use file::File;
#[cfg(feature = "youtube")]
pub use youtube::Youtube;

/// A source-specific extractor.
#[async_trait]
pub trait Extractor: Send + Sync {
    fn matches(&self, url: &str) -> bool;

    async fn extract(&self, url: &str) -> Result<Playlist>;

    async fn download(&self, url: &str) -> Result<Vec<u8>> {
        crate::http::download(url, HeaderMap::new()).await
    }

    async fn download_cover(&self, url: &str) -> Result<Vec<u8>> {
        crate::http::get_bytes(url, HeaderMap::new()).await
    }
}

static EXTRACTORS: &[&dyn Extractor] = &[
    #[cfg(feature = "bilibili")]
    &Bilibili,
    #[cfg(feature = "youtube")]
    &Youtube,
    #[cfg(feature = "file")]
    &File,
];

/// Find the extractor for a site.
pub fn extractor(site: Site) -> &'static dyn Extractor {
    match site {
        #[cfg(feature = "bilibili")]
        Site::Bilibili => &Bilibili,
        #[cfg(feature = "youtube")]
        Site::Youtube => &Youtube,
        #[cfg(feature = "file")]
        Site::File => &File,
    }
}

/// Extract a playlist or track from any enabled source.
pub async fn extract(url: &str) -> Result<Playlist> {
    for extractor in EXTRACTORS {
        if extractor.matches(url) {
            return extractor.extract(url).await;
        }
    }
    Err(Error::Unsupported(url.to_owned()))
}
