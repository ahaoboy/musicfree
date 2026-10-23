#[cfg(feature = "bilibili")]
pub mod bilibili;
#[cfg(feature = "file")]
pub mod file;
#[cfg(feature = "youtube")]
pub mod youtube;

use crate::{Error, MediaFormat, Playlist, Result, Site, Track};
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use std::time::Duration;

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

    async fn formats(&self, url: &str) -> Result<Vec<MediaFormat>> {
        Err(Error::Unsupported(format!("format listing for {url}")))
    }

    async fn download_format(&self, url: &str, _format_id: &str) -> Result<Vec<u8>> {
        self.download(url).await
    }

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

/// List downloadable formats for a URL.
pub async fn formats(url: &str) -> Result<Vec<MediaFormat>> {
    for extractor in EXTRACTORS {
        if extractor.matches(url) {
            return extractor.formats(url).await;
        }
    }
    Err(Error::Unsupported(url.to_owned()))
}

/// Download a specific format through the matching site extractor.
pub async fn download_format(site: Site, url: &str, format_id: &str) -> Result<Vec<u8>> {
    extractor(site).download_format(url, format_id).await
}

/// Download a media URL through the matching site extractor.
pub async fn download(site: Site, url: &str) -> Result<Vec<u8>> {
    extractor(site).download(url).await
}

/// Download a cover URL through the matching site extractor.
pub async fn download_cover(site: Site, url: &str) -> Result<Vec<u8>> {
    extractor(site).download_cover(url).await
}

impl Track {
    /// List this track's downloadable formats.
    pub async fn formats(&self) -> Result<Vec<MediaFormat>> {
        extractor(self.site).formats(&self.url).await
    }

    /// Download this track's media using its site extractor.
    pub async fn download(&self) -> Result<Vec<u8>> {
        download(self.site, &self.url).await
    }

    /// Download a specific format of this track.
    pub async fn download_format(&self, format_id: &str) -> Result<Vec<u8>> {
        download_format(self.site, &self.url, format_id).await
    }

    /// Download this track's cover, if it has one.
    pub async fn download_cover(&self) -> Result<Option<Vec<u8>>> {
        let Some(url) = &self.cover else {
            return Ok(None);
        };
        download_cover(self.site, url).await.map(Some)
    }

    pub async fn download_with_timeout(&self, timeout: Duration) -> Result<Vec<u8>> {
        match tokio::time::timeout(timeout, self.download()).await {
            Ok(result) => result,
            Err(_) => Err(Error::Timeout(self.url.clone())),
        }
    }

    pub async fn download_cover_with_timeout(&self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        match tokio::time::timeout(timeout, self.download_cover()).await {
            Ok(result) => result,
            Err(_) => Err(Error::Timeout(
                self.cover.clone().unwrap_or_else(|| self.url.clone()),
            )),
        }
    }
}
