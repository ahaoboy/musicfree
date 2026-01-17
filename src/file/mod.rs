use crate::core::{Audio, AudioFormat, Extractor, Platform};
use crate::download::download_binary;
use crate::error::Result;
use crate::utils::get_md5;
use reqwest::header::HeaderMap;
pub use strum::IntoEnumIterator;

/// Direct file extractor: treat http/https URLs as file downloads
pub struct FileExtractor;

fn is_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn is_audio(url: &str) -> Option<AudioFormat> {
    AudioFormat::iter().find(|fmt| url.ends_with(fmt.extension()))
}

#[async_trait::async_trait]
impl Extractor for FileExtractor {
    fn matches(&self, url: &str) -> bool {
        // If URL is HTTP(S) and looks like an audio file
        is_http_url(url) && is_audio(url).is_some()
    }

    async fn extract(&self, url: &str) -> Result<Vec<Audio>> {
        let binary = download_binary(url, HeaderMap::new()).await?;
        let fmt = is_audio(url).unwrap_or(AudioFormat::Mp3);
        let id = get_md5(url);
        // Create a minimal Audio struct representing a downloadable file
        let audio = Audio::new(
            id,
            // Title can be derived from URL basename
            Self::basename(url).replace(fmt.extension(), ""),
            url.to_string(),
            Platform::File,
        )
        .with_format(fmt)
        .with_binary(binary);
        Ok(vec![audio])
    }

    fn platform(&self) -> Platform {
        Platform::File
    }
}

impl FileExtractor {
    fn basename(url: &str) -> String {
        // crude basename extraction
        let u = url.trim_end_matches('/');
        if let Some(pos) = u.rsplit('/').next().map(|s| s.to_string()) {
            return pos;
        }
        "direct_file".to_string()
    }
}
