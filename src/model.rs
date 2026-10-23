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

/// What tracks a downloadable media format contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    /// Video with an audio track.
    Muxed,
    /// Video only.
    Video,
    /// Audio only.
    Audio,
}

impl fmt::Display for MediaKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Muxed => "video+audio",
            Self::Video => "video",
            Self::Audio => "audio",
        })
    }
}

/// One selectable media stream returned by `formats()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFormat {
    /// Site-specific format identifier, e.g. YouTube `itag` or a Bilibili
    /// quality id.
    pub id: String,
    pub kind: MediaKind,
    pub format: AudioFormat,
    pub mime_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u64>,
    /// Exact size when the site reports it, otherwise an estimate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

/// Pick the highest-quality format that contains video.
pub fn best_video(formats: &[MediaFormat]) -> Option<&MediaFormat> {
    formats
        .iter()
        .filter(|format| format.has_video())
        .max_by_key(|format| {
            (
                format.height.unwrap_or(0),
                format.bitrate.unwrap_or(0),
                format.kind == MediaKind::Muxed,
            )
        })
}

/// Sort formats for display: muxed, video, then audio, highest quality first.
pub fn sort_formats(formats: &mut [MediaFormat]) {
    formats.sort_by(|left, right| {
        kind_rank(left.kind)
            .cmp(&kind_rank(right.kind))
            .then_with(|| right.height.unwrap_or(0).cmp(&left.height.unwrap_or(0)))
            .then_with(|| right.bitrate.unwrap_or(0).cmp(&left.bitrate.unwrap_or(0)))
    });
}

fn kind_rank(kind: MediaKind) -> u8 {
    match kind {
        MediaKind::Muxed => 0,
        MediaKind::Video => 1,
        MediaKind::Audio => 2,
    }
}

impl MediaFormat {
    pub fn has_audio(&self) -> bool {
        matches!(self.kind, MediaKind::Audio | MediaKind::Muxed)
    }

    pub fn has_video(&self) -> bool {
        matches!(self.kind, MediaKind::Video | MediaKind::Muxed)
    }

    pub fn resolution(&self) -> Option<String> {
        Some(format!("{}x{}", self.width?, self.height?))
    }

    pub fn size_or_estimate(&self) -> Option<u64> {
        self.size.or_else(|| {
            let bitrate = self.bitrate?;
            let duration = self.duration?;
            Some(bitrate.saturating_mul(duration) / 8)
        })
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
    let path = cover_url.split(['?', '#']).next().unwrap_or(cover_url);
    let name = path
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_cover_file_name() {
        let name = cover_file_name("https://example.com/cover art.jpg");
        assert!(name.ends_with("_cover art.jpg"));
        assert_eq!(name.len(), 32 + 1 + "cover art.jpg".len());
        assert!(cover_file_name("https://example.com/cover.jpg?x=1").ends_with("_cover.jpg"));
    }

    #[test]
    fn picks_best_video_format() {
        let format = |id: &str, kind: MediaKind, height, bitrate| MediaFormat {
            id: id.into(),
            kind,
            format: AudioFormat::Mp4,
            mime_type: "video/mp4".into(),
            codec: None,
            bitrate: Some(bitrate),
            size: None,
            width: None,
            height: Some(height),
            quality: None,
            duration: Some(60),
        };
        let formats = vec![
            format("audio", MediaKind::Audio, 0, 999_999),
            format("720", MediaKind::Video, 720, 1_000_000),
            format("1080", MediaKind::Video, 1080, 2_000_000),
        ];
        assert_eq!(best_video(&formats).unwrap().id, "1080");
    }

    #[cfg(feature = "file")]
    #[test]
    fn builds_track_file_names() {
        let track = Track::new("id", "A / B", "https://example.com/a.mp3", Site::File)
            .with_format(AudioFormat::Mp4);

        assert_eq!(track.file_name(), "A  B.mp4");
        assert!(track.cache_file_name().starts_with("id_"));
        assert!(track.cache_file_name().ends_with(".mp4"));
        assert!(track.save_file_name().starts_with("A  B-"));
        assert!(track.save_file_name().ends_with(".mp4"));
    }

    #[cfg(feature = "file")]
    #[test]
    fn uses_frontend_serde_names() {
        let track = Track::new("id", "Song", "https://example.com/a.mp3", Site::File)
            .with_format(AudioFormat::Mp4);
        let playlist = Playlist {
            url: Some("https://example.com/playlist".into()),
            tracks: vec![track.clone()],
            site: Site::File,
            ..Playlist::new(Site::File)
        };

        let track_json = serde_json::to_value(&track).unwrap();
        assert_eq!(track_json["download_url"], "https://example.com/a.mp3");
        assert_eq!(track_json["platform"], "File");
        assert_eq!(track_json["format"], "Mp4");

        let playlist_json = serde_json::to_value(&playlist).unwrap();
        assert_eq!(
            playlist_json["download_url"],
            "https://example.com/playlist"
        );
        assert_eq!(playlist_json["platform"], "File");
        assert_eq!(playlist_json["audios"][0]["id"], "id");

        let parsed: Playlist = serde_json::from_value(playlist_json).unwrap();
        assert_eq!(parsed.tracks[0].url, "https://example.com/a.mp3");
    }
}
