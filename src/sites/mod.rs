pub mod bilibili;
pub mod file;
pub mod youtube;

use crate::{Error, Playlist, Result, Site};
use async_trait::async_trait;
use reqwest::header::HeaderMap;

pub use bilibili::Bilibili;
pub use file::File;
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

static EXTRACTORS: &[&dyn Extractor] = &[&Bilibili, &Youtube, &File];

/// Find the extractor for a site.
pub fn extractor(site: Site) -> &'static dyn Extractor {
    match site {
        Site::Bilibili => &Bilibili,
        Site::Youtube => &Youtube,
        Site::File => &File,
    }
}

/// Extract a playlist or track from any supported URL.
pub async fn extract(url: &str) -> Result<Playlist> {
    for extractor in EXTRACTORS {
        if extractor.matches(url) {
            return extractor.extract(url).await;
        }
    }
    Err(Error::Unsupported(url.to_owned()))
}
