use crate::Playlist;
use crate::core::{Audio, AudioFormat, Extractor, Platform};
use crate::error::Result;
use crate::utils::get_md5;
pub use strum::IntoEnumIterator;

/// Direct file extractor: treat http/https URLs as file downloads
#[derive(Debug, Clone)]
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

    async fn extract(&self, url: &str) -> Result<(Playlist, Option<usize>)> {
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
        .with_format(fmt);

        let playlist = Playlist {
            id: None,
            title: None,
            audios: vec![audio],
            cover: None,
            platform: Platform::File,
        };

        // For file extractor, the position is always 0 if playlist is not empty
        let position = if playlist.audios.is_empty() {
            None
        } else {
            Some(0)
        };

        Ok((playlist, position))
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
