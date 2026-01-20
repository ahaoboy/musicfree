use crate::FileExtractor;
use crate::download::download_binary;
use crate::error::Result;

#[cfg(feature = "youtube")]
use crate::youtube::YoutubeExtractor;

#[cfg(feature = "bilibili")]
use crate::BilibiliExtractor;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
pub use strum::IntoEnumIterator;
use strum_macros::EnumIter;

/// Supported platforms
#[derive(EnumIter, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum Platform {
    #[cfg(feature = "bilibili")]
    Bilibili,
    #[cfg(feature = "youtube")]
    Youtube,
    File,
}

impl Platform {
    pub fn extractor(&self) -> &'static dyn Extractor {
        match self {
            #[cfg(feature = "bilibili")]
            Platform::Bilibili => &BilibiliExtractor,
            #[cfg(feature = "youtube")]
            Platform::Youtube => &YoutubeExtractor,
            Platform::File => &FileExtractor,
        }
    }
}

// Audio format representation
#[derive(EnumIter, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AudioFormat {
    Mp3,
    M4A,
    Flac,
    Wav,
    AAC,
    Ogg,
    Mp4,
    Webm,
}

impl AudioFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => ".mp3",
            AudioFormat::M4A => ".m4a",
            AudioFormat::Flac => ".flac",
            AudioFormat::Wav => ".wav",
            AudioFormat::AAC => ".aac",
            AudioFormat::Ogg => ".ogg",
            AudioFormat::Mp4 => ".mp4",
            AudioFormat::Webm => ".webm",
        }
    }

    pub fn from_youtube(s: &str) -> Self {
        if s.starts_with("audio/webm") {
            return Self::Webm;
        };
        Self::Mp4
    }
}
/// Audio resource representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
    pub id: String,
    pub title: String,
    pub download_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,
    pub platform: Platform,
}

impl Audio {
    /// Create a new audio instance
    pub fn new(id: String, title: String, download_url: String, platform: Platform) -> Self {
        Self {
            id,
            title,
            download_url,
            cover: None,
            duration: None,
            format: None,
            platform,
        }
    }

    /// Set format
    pub fn with_format(mut self, format: AudioFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set cover URL
    pub fn with_cover(mut self, cover: String) -> Self {
        self.cover = Some(cover);
        self
    }

    /// Set duration in seconds
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration = Some(duration);
        self
    }
}

/// Playlist representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub title: String,
    pub audios: Vec<Audio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    pub platform: Platform,
}

impl Playlist {
    /// Create a new playlist
    pub fn new(title: String, platform: Platform) -> Self {
        Self {
            title,
            audios: Vec::new(),
            cover: None,
            platform,
        }
    }
}

#[derive(EnumIter, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
pub enum Quality {
    Low,
    Standard,
    High,
    #[default]
    Super,
}

/// Trait for extracting audio from different platforms
#[async_trait::async_trait]
pub trait Extractor: Send + Sync {
    /// Check if the URL is supported by this extractor
    fn matches(&self, url: &str) -> bool;

    /// Extract audio resources from URL
    /// Returns a Vec<Audio> since a URL might contain multiple audio resources
    async fn extract(&self, url: &str) -> Result<Playlist>;

    /// Download audio binary data and populate the binary field
    /// Default implementation uses the download_url to fetch binary data
    async fn download(&self, url: &str) -> Result<Vec<u8>> {
        let binary = download_binary(url, HeaderMap::new()).await?;
        Ok(binary)
    }

    async fn download_cover(&self, url: &str) -> Result<Vec<u8>> {
        let binary = download_binary(url, HeaderMap::new()).await?;
        Ok(binary)
    }

    /// Get platform identifier
    fn platform(&self) -> Platform;
}
