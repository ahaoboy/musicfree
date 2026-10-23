use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Site {
    #[cfg(feature = "bilibili")]
    #[serde(alias = "bilibili")]
    Bilibili,
    #[cfg(feature = "youtube")]
    #[serde(alias = "youtube")]
    Youtube,
    #[cfg(feature = "file")]
    #[serde(alias = "file")]
    File,
}

impl fmt::Display for Site {
    #[cfg_attr(
        not(any(feature = "bilibili", feature = "youtube", feature = "file")),
        allow(unused_variables)
    )]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(any(feature = "bilibili", feature = "youtube", feature = "file"))]
        {
            f.write_str(match *self {
                #[cfg(feature = "bilibili")]
                Self::Bilibili => "bilibili",
                #[cfg(feature = "youtube")]
                Self::Youtube => "youtube",
                #[cfg(feature = "file")]
                Self::File => "file",
            })
        }
        #[cfg(not(any(feature = "bilibili", feature = "youtube", feature = "file")))]
        match *self {}
    }
}

/// Audio/container formats produced by the extractors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioFormat {
    #[serde(rename = "Mp3", alias = "mp3")]
    Mp3,
    #[serde(rename = "M4A", alias = "M4a", alias = "m4a")]
    M4a,
    #[serde(rename = "Flac", alias = "flac")]
    Flac,
    #[serde(rename = "Wav", alias = "wav")]
    Wav,
    #[serde(rename = "AAC", alias = "Aac", alias = "aac")]
    Aac,
    #[serde(rename = "Ogg", alias = "ogg")]
    Ogg,
    #[serde(rename = "Mp4", alias = "mp4")]
    Mp4,
    #[serde(rename = "Webm", alias = "webm")]
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
    #[serde(rename = "download_url", alias = "url")]
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u64>,
    #[serde(rename = "platform", alias = "site")]
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

    /// File extension without the leading dot.
    pub fn extension(&self) -> &'static str {
        self.format.unwrap_or(AudioFormat::Mp3).extension()
    }

    pub fn format_name(&self) -> &'static str {
        self.format.map_or("unknown", AudioFormat::as_str)
    }

    /// Human-readable file name using the sanitized title.
    pub fn file_name(&self) -> String {
        format!("{}.{}", safe_title(&self.title), self.extension())
    }

    /// Stable cache file name: `{id}_{url hash}.{extension}`.
    pub fn cache_file_name(&self) -> String {
        format!("{}_{}.{}", self.id, hash(&self.url), self.extension())
    }

    /// Export file name: `{sanitized title}-{short url hash}.{extension}`.
    pub fn save_file_name(&self) -> String {
        let hash = hash(&self.url);
        let prefix = &hash[..6.min(hash.len())];
        format!(
            "{}-{}.{}",
            safe_title(&self.title),
            prefix,
            self.extension()
        )
    }

    /// Cover file name if this track has a cover URL.
    pub fn cover_file_name(&self) -> Option<String> {
        self.cover.as_deref().map(cover_file_name)
    }
}

/// Stable file name for a remote cover URL.
pub fn cover_file_name(cover_url: &str) -> String {
    let hash = hash(cover_url);
    let name = cover_url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("cover.jpg");
    format!("{hash}_{}", sanitize_filename::sanitize(name))
}

/// Sanitize a title for use as a file or directory name.
pub fn safe_title(title: &str) -> String {
    let title = sanitize_filename::sanitize(title);
    if title.is_empty() {
        "audio".into()
    } else {
        title
    }
}

fn hash(value: &str) -> String {
    format!("{:x}", md5::compute(value))
}

/// A video, playlist, or single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        default,
        rename = "download_url",
        alias = "url",
        skip_serializing_if = "Option::is_none"
    )]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(
        default,
        rename = "audios",
        alias = "tracks",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub tracks: Vec<Track>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(rename = "platform", alias = "site")]
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

    pub fn default_track(&self) -> Option<&Track> {
        self.default.and_then(|index| self.tracks.get(index))
    }

    /// Title sanitized for use as a directory or file name.
    pub fn safe_title(&self) -> String {
        safe_title(self.title.as_deref().unwrap_or("Unknown"))
    }
}
