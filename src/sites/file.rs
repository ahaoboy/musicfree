use crate::{
    AudioFormat, Error, MediaFormat, MediaKind, Playlist, Result, Site, Track, http,
    sites::Extractor,
};
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use std::path::Path;
use url::Url;

/// Treat direct HTTP(S) audio URLs as a single track.
pub struct File;

#[async_trait]
impl Extractor for File {
    fn matches(&self, url: &str) -> bool {
        parse_url(url).is_some_and(|url| format_of(&url).is_some())
    }

    async fn extract(&self, url: &str) -> Result<Playlist> {
        let parsed = Url::parse(url).map_err(|error| Error::InvalidUrl(error.to_string()))?;
        let format = format_of(&parsed).ok_or_else(|| Error::Unsupported(url.to_owned()))?;
        let title = file_stem(&parsed).unwrap_or_else(|| url.to_owned());
        let track = Track::new(format!("{:x}", md5::compute(url)), title, url, Site::File)
            .with_format(format);

        Ok(Playlist {
            url: Some(url.to_owned()),
            tracks: vec![track],
            default: Some(0),
            ..Playlist::new(Site::File)
        })
    }

    async fn formats(&self, url: &str) -> Result<Vec<MediaFormat>> {
        let parsed = Url::parse(url).map_err(|error| Error::InvalidUrl(error.to_string()))?;
        let format = format_of(&parsed).ok_or_else(|| Error::Unsupported(url.to_owned()))?;
        Ok(vec![MediaFormat {
            id: "file".into(),
            kind: match format {
                AudioFormat::Mp4 | AudioFormat::Webm => MediaKind::Muxed,
                _ => MediaKind::Audio,
            },
            format,
            mime_type: format.as_str().into(),
            codec: None,
            bitrate: None,
            size: None,
            width: None,
            height: None,
            quality: None,
            duration: None,
        }])
    }

    async fn download(&self, url: &str) -> Result<Vec<u8>> {
        http::download(url, HeaderMap::new()).await
    }
}

fn parse_url(url: &str) -> Option<Url> {
    let url = Url::parse(url).ok()?;
    matches!(url.scheme(), "http" | "https").then_some(url)
}

fn format_of(url: &Url) -> Option<AudioFormat> {
    let extension = Path::new(url.path()).extension()?.to_str()?;
    AudioFormat::from_extension(extension)
}

fn file_stem(url: &Url) -> Option<String> {
    let name = url.path().rsplit('/').next()?;
    let name = urlencoding::decode(name).ok()?;
    Path::new(name.as_ref())
        .file_stem()?
        .to_str()
        .map(str::to_owned)
}
