use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Site {
    Bilibili,
    Youtube,
    File,
}

impl fmt::Display for Site {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Bilibili => "bilibili",
            Self::Youtube => "youtube",
            Self::File => "file",
        })
    }
}

/// Audio/container formats produced by the extractors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    M4a,
    Flac,
    Wav,
    Aac,
    Ogg,
    Mp4,
    Webm,
}

impl AudioFormat {
    pub const ALL: [Self; 8] = [
        Self::Mp3,
        Self::M4a,
        Self::Flac,
        Self::Wav,
        Self::Aac,
        Self::Ogg,
        Self::Mp4,
        Self::Webm,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Flac => "flac",
            Self::Wav => "wav",
            Self::Aac => "aac",
            Self::Ogg => "ogg",
            Self::Mp4 => "mp4",
            Self::Webm => "webm",
        }
    }

    /// File extension without the leading dot.
    pub const fn extension(self) -> &'static str {
        self.as_str()
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim().trim_start_matches('.').to_ascii_lowercase();
        Self::ALL.into_iter().find(|format| format.as_str() == name)
    }

    pub fn from_extension(extension: &str) -> Option<Self> {
        Self::from_name(extension)
    }

    pub fn from_mime(mime: &str) -> Self {
        let mime = mime.split(';').next().unwrap_or(mime).trim();
        match mime {
            "audio/webm" | "video/webm" => Self::Webm,
            "audio/ogg" => Self::Ogg,
            "audio/mpeg" => Self::Mp3,
            "audio/aac" => Self::Aac,
            "audio/flac" => Self::Flac,
            "audio/wav" | "audio/x-wav" => Self::Wav,
            "audio/mp4" | "audio/x-m4a" => Self::M4a,
            _ => Self::Mp4,
        }
    }
}

impl fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single downloadable audio resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    /// Page URL, not the resolved media URL.
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u64>,
    pub site: Site,
}

impl Track {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        url: impl Into<String>,
        site: Site,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            url: url.into(),
            cover: None,
            duration: None,
            format: None,
            bitrate: None,
            site,
        }
    }

    pub fn with_cover(mut self, cover: impl Into<String>) -> Self {
        let cover = cover.into();
        self.cover = (!cover.is_empty()).then_some(cover);
        self
    }

    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration = (duration > 0).then_some(duration);
        self
    }

    pub fn with_format(mut self, format: AudioFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn with_bitrate(mut self, bitrate: u64) -> Self {
        self.bitrate = (bitrate > 0).then_some(bitrate);
        self
    }

    pub fn extension(&self) -> &'static str {
        self.format.unwrap_or(AudioFormat::Mp3).extension()
    }

    pub fn format_name(&self) -> &'static str {
        self.format.map_or("unknown", AudioFormat::as_str)
    }
}

/// A video, playlist, or single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<Track>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    pub site: Site,
    /// Zero-based index of the track requested by the original URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<usize>,
}

impl Playlist {
    pub fn new(site: Site) -> Self {
        Self {
            id: None,
            url: None,
            title: None,
            tracks: Vec::new(),
            cover: None,
            site,
            default: None,
        }
    }
}
