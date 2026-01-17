use crate::download::download_binary;
use crate::error::Result;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
pub use strum::IntoEnumIterator;
use strum_macros::EnumIter;

/// Supported platforms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Platform {
    Bilibili,
    Youtube,
    File,
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
        }
    }
}
/// Audio resource representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
    pub id: String,
    pub title: String,
    pub download_url: String,
    pub local_url: Option<String>,
    #[serde(skip_serializing)]
    pub binary: Option<Vec<u8>>,
    pub author: Vec<String>,
    pub cover: Option<String>,
    pub tags: Vec<String>,
    pub duration: Option<u32>,
    pub format: Option<AudioFormat>,
    pub platform: Platform,
    pub date: u32,
}

impl Audio {
    /// Create a new audio instance
    pub fn new(id: String, title: String, download_url: String, platform: Platform) -> Self {
        Self {
            id,
            title,
            download_url,
            local_url: None,
            binary: None,
            author: Vec::new(),
            cover: None,
            tags: Vec::new(),
            duration: None,
            format: None,
            platform,
            date: chrono::Utc::now().timestamp() as u32,
        }
    }

    /// Set format
    pub fn with_format(mut self, format: AudioFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set author
    pub fn with_author(mut self, author: Vec<String>) -> Self {
        self.author = author;
        self
    }

    /// Set cover URL
    pub fn with_cover(mut self, cover: String) -> Self {
        self.cover = Some(cover);
        self
    }

    /// Set tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set duration in seconds
    pub fn with_duration(mut self, duration: u32) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set binary data
    pub fn with_binary(mut self, binary: Vec<u8>) -> Self {
        self.binary = Some(binary);
        self
    }

    /// Set local URL
    pub fn with_local_url(mut self, local_url: String) -> Self {
        self.local_url = Some(local_url);
        self
    }
}

/// Playlist representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayList {
    pub title: String,
    pub audios: Vec<Audio>,
    pub date: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Quality {
    Low,
    Standard,
    High,
    #[default]
    Super,
}

impl PlayList {
    /// Create a new playlist
    pub fn new(title: String) -> Self {
        Self {
            title,
            audios: Vec::new(),
            date: chrono::Utc::now().timestamp() as u32,
        }
    }

    /// Add audio to playlist
    pub fn add_audio(mut self, audio: Audio) -> Self {
        self.audios.push(audio);
        self
    }
}

/// Trait for extracting audio from different platforms
#[async_trait::async_trait]
pub trait Extractor: Send + Sync {
    /// Check if the URL is supported by this extractor
    fn matches(&self, url: &str) -> bool;

    /// Extract audio resources from URL
    /// Returns a Vec<Audio> since a URL might contain multiple audio resources
    async fn extract(&self, url: &str) -> Result<Vec<Audio>>;

    /// Download audio binary data and populate the binary field
    /// Default implementation uses the download_url to fetch binary data
    async fn download(&self, audio: &mut Audio) -> Result<()> {
        let binary = download_binary(&audio.download_url, HeaderMap::new()).await?;
        audio.binary = Some(binary);
        Ok(())
    }

    /// Get platform identifier
    fn platform(&self) -> Platform;
}
