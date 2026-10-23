pub mod api;
mod download;
mod ejs;
mod formats;
mod mobile;
mod playlist;
pub mod url;
mod web;

use crate::{
    AudioFormat, MediaFormat, Playlist, Result, Site, Track, http, http::HeaderMapExt,
    sites::Extractor,
};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

pub struct Youtube;

/// Download strategy for YouTube.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    /// Try the mobile API first, then the web API.
    #[default]
    Auto,
    /// Web player API plus fallback clients and EJS challenge solving.
    Web,
    /// Android/mobile player API.
    Mobile,
}

#[async_trait]
impl Extractor for Youtube {
    fn matches(&self, url: &str) -> bool {
        self::url::is_url(url)
    }

    async fn extract(&self, url: &str) -> Result<Playlist> {
        extract_playlist(url).await
    }

    async fn formats(&self, url: &str) -> Result<Vec<MediaFormat>> {
        formats::list(url).await
    }

    async fn download_format(&self, url: &str, format_id: &str) -> Result<Vec<u8>> {
        download_format(url, Mode::Auto, format_id).await
    }

    async fn download(&self, url: &str) -> Result<Vec<u8>> {
        download(url, Mode::Auto).await
    }
}

/// Download a YouTube video with an explicit strategy.
pub async fn download(input: &str, mode: Mode) -> Result<Vec<u8>> {
    let video_id = url::video_id(input)?;
    let html = fetch_page(&video_id).await?;
    let config = api::config(&html)?;
    let sts = api::sts(&html);

    match mode {
        Mode::Mobile => mobile::download(&video_id, &config, &html, sts).await,
        Mode::Web => web::download(&video_id, &html, &config, sts).await,
        Mode::Auto => match mobile::download(&video_id, &config, &html, sts).await {
            Ok(data) => Ok(data),
            Err(_) => web::download(&video_id, &html, &config, sts).await,
        },
    }
}

/// Download a specific YouTube format with an explicit strategy.
pub async fn download_format(input: &str, mode: Mode, format_id: &str) -> Result<Vec<u8>> {
    let (format, user_agent, html) = formats::find_format(input, format_id, mode).await?;
    let media_url = download::resolve_url(&format, &html, None).await?;
    let headers = http::media_headers(user_agent, "https://www.youtube.com/")?
        .with_origin("https://www.youtube.com")
        .with_sec_fetch();

    http::download(&media_url, headers).await
}

async fn extract_playlist(input: &str) -> Result<Playlist> {
    let (fetch_url, playlist_id) = fetch_target(input);
    let html = http::get_text(&fetch_url, page_headers()).await?;
    match playlist_id {
        Some(playlist_id) => parse_playlist(input, &playlist_id, &html),
        None => parse_video(input, &html).await,
    }
}

async fn parse_video(input: &str, html: &str) -> Result<Playlist> {
    let video_id = url::video_id(input)?;
    let config = api::config(html)?;
    let sts = api::sts(html);
    let player = match api::player_from_html(html) {
        Ok(player) if player.streaming_data.best_playable_format().is_some() => player,
        _ => api::android_player(&video_id, &config, sts).await?,
    };
    let format = player
        .streaming_data
        .best_playable_format()
        .ok_or(crate::Error::AudioNotFound)?;
    let title = player.video_details.title.clone();
    let cover = url::thumbnail_url(&video_id);

    let mut track = Track::new(
        video_id.clone(),
        title.clone(),
        url::watch_url(&video_id),
        Site::Youtube,
    )
    .with_format(AudioFormat::from_mime(&format.mime_type))
    .with_bitrate(format.bitrate)
    .with_cover(cover.clone());
    if let Some(duration) = format
        .approx_duration_ms
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
    {
        track = track.with_duration(duration / 1000);
    }

    Ok(Playlist {
        id: None,
        url: Some(url::watch_url(&video_id)),
        title: Some(title),
        tracks: vec![track],
        cover: Some(cover),
        site: Site::Youtube,
        default: Some(0),
    })
}

fn parse_playlist(input: &str, playlist_id: &str, html: &str) -> Result<Playlist> {
    let playlist = playlist::parse(html)?;
    let requested = url::video_id(input).ok();
    let mut tracks = Vec::new();
    let mut default = None;

    for video in playlist.videos {
        if requested.as_deref() == Some(video.id.as_str()) {
            default = Some(tracks.len());
        }
        let mut track = Track::new(
            video.id.clone(),
            video.title,
            url::watch_url(&video.id),
            Site::Youtube,
        )
        .with_format(AudioFormat::Mp4)
        .with_cover(url::thumbnail_url(&video.id));
        if let Some(duration) = video.duration {
            track = track.with_duration(duration);
        }
        tracks.push(track);
    }

    let cover = tracks.first().and_then(|track| track.cover.clone());

    Ok(Playlist {
        id: Some(playlist_id.to_owned()),
        url: Some(url::playlist_url(playlist_id)),
        title: Some(playlist.title.unwrap_or_else(|| "YouTube Playlist".into())),
        tracks,
        cover,
        site: Site::Youtube,
        default,
    })
}

pub(super) async fn fetch_page(video_id: &str) -> Result<String> {
    http::get_text(&url::watch_url(video_id), page_headers()).await
}

fn fetch_target(input: &str) -> (String, Option<String>) {
    if serde_youtube::url::is_valid_video_id(input) {
        return (url::watch_url(input), None);
    }
    if let Some(playlist_id) = url::playlist_id(input) {
        return (url::playlist_url(&playlist_id), Some(playlist_id));
    }
    (input.to_owned(), None)
}

fn page_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(url::WEB_UA));
    headers
}
