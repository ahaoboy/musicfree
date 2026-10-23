pub mod api;
pub mod url;

use crate::{
    AudioFormat, Error, MediaFormat, MediaKind, Playlist, Result, Site, Track, http,
    http::HeaderMapExt, sites::Extractor,
};
use api::{AudioStream, Page, PlayData, VideoStream, ViewData};
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use std::path::Path;

pub struct Bilibili;

#[async_trait]
impl Extractor for Bilibili {
    fn matches(&self, url: &str) -> bool {
        url::is_url(url)
    }

    async fn extract(&self, input: &str) -> Result<Playlist> {
        api::warm_up().await?;
        let bvid = url::video_id(input).await?;
        let view = api::view(&bvid).await?;
        let tracks = tracks(&view);
        if tracks.is_empty() {
            return Err(Error::AudioNotFound);
        }

        let page = url::page(input);
        let default = if view.ugc_season.is_some() {
            let target = url::video_url(&bvid, (page > 1).then_some(page));
            tracks
                .iter()
                .position(|track| track.url == target)
                .or_else(|| tracks.iter().position(|track| track.url.contains(&bvid)))
        } else {
            Some(page.saturating_sub(1).min(tracks.len() - 1))
        };
        let title = view
            .ugc_season
            .as_ref()
            .map(|season| season.title.clone())
            .unwrap_or_else(|| view.title.clone());
        let cover = view
            .ugc_season
            .as_ref()
            .map(|season| season.cover.clone())
            .unwrap_or_else(|| view.pic.clone());
        let id = view
            .ugc_season
            .as_ref()
            .map(|season| season.id.to_string())
            .or_else(|| view.season_id.map(|id| id.to_string()))
            .unwrap_or_else(|| bvid.clone());

        Ok(Playlist {
            id: Some(id),
            url: Some(input.to_owned()),
            title: Some(title),
            tracks,
            cover: (!cover.is_empty()).then_some(cover),
            site: Site::Bilibili,
            default,
        })
    }

    async fn formats(&self, input: &str) -> Result<Vec<MediaFormat>> {
        let media = load(input).await?;
        let mut formats = formats_of(&media);
        if formats.is_empty() {
            return Err(Error::AudioNotFound);
        }
        crate::sort_formats(&mut formats);
        Ok(formats)
    }

    async fn download_format(&self, input: &str, format_id: &str) -> Result<Vec<u8>> {
        let media = load(input).await?;
        let url = media_url(&media.play, format_id)
            .ok_or_else(|| Error::Unsupported(format!("Bilibili format not found: {format_id}")))?;
        http::download(&url, media_headers(&media.referer())?).await
    }

    async fn download(&self, input: &str) -> Result<Vec<u8>> {
        let media = load(input).await?;
        let audio_url = media
            .play
            .dash
            .as_ref()
            .and_then(|dash| {
                dash.audio
                    .iter()
                    .max_by_key(|audio| audio.bandwidth)
                    .map(|audio| audio.base_url.clone())
            })
            .or_else(|| {
                media
                    .play
                    .durl
                    .as_ref()
                    .and_then(|durls| durls.first())
                    .map(|durl| durl.url.clone())
            })
            .ok_or(Error::AudioNotFound)?;

        http::download(&audio_url, media_headers(&media.referer())?).await
    }

    async fn download_cover(&self, url: &str) -> Result<Vec<u8>> {
        http::get_bytes(url, media_headers("https://www.bilibili.com/")?).await
    }
}

struct MediaData {
    bvid: String,
    page: usize,
    duration: u64,
    play: PlayData,
}

impl MediaData {
    fn referer(&self) -> String {
        url::video_url(&self.bvid, (self.page > 1).then_some(self.page))
    }
}

async fn load(input: &str) -> Result<MediaData> {
    api::warm_up().await?;
    let bvid = url::video_id(input).await?;
    let page = url::page(input);
    let view = api::view(&bvid).await?;
    let page_data = view
        .pages
        .get(page.saturating_sub(1))
        .ok_or(Error::AudioNotFound)?;
    let play = api::play(&bvid, page_data.cid).await?;
    Ok(MediaData {
        bvid,
        page,
        duration: page_data.duration,
        play,
    })
}

fn formats_of(media: &MediaData) -> Vec<MediaFormat> {
    let mut formats = Vec::new();

    if let Some(dash) = &media.play.dash {
        for (index, audio) in dash.audio.iter().enumerate() {
            formats.push(audio_format(media, audio, index));
        }
        for (index, video) in dash.video.iter().enumerate() {
            formats.push(video_format(media, video, index));
        }
    }

    if let Some(durls) = &media.play.durl {
        for (index, durl) in durls.iter().enumerate() {
            formats.push(MediaFormat {
                id: format!("d:{index}"),
                kind: MediaKind::Muxed,
                format: format_from_url(&durl.url),
                mime_type: "video/mp4".into(),
                codec: None,
                bitrate: None,
                size: durl.size,
                width: None,
                height: None,
                quality: Some("muxed".into()),
                duration: Some(durl.length.map_or(media.duration, |ms| ms / 1000)),
            });
        }
    }

    formats
}

fn audio_format(media: &MediaData, audio: &AudioStream, index: usize) -> MediaFormat {
    MediaFormat {
        id: audio_key(audio, index),
        kind: MediaKind::Audio,
        format: AudioFormat::from_mime(&audio.mime_type),
        mime_type: audio.mime_type.clone(),
        codec: (!audio.codecs.is_empty()).then(|| audio.codecs.clone()),
        bitrate: (audio.bandwidth > 0).then_some(audio.bandwidth),
        size: None,
        width: None,
        height: None,
        quality: None,
        duration: Some(media.duration),
    }
}

fn video_format(media: &MediaData, video: &VideoStream, index: usize) -> MediaFormat {
    MediaFormat {
        id: video_key(video, index),
        kind: MediaKind::Video,
        format: AudioFormat::from_mime(&video.mime_type),
        mime_type: video.mime_type.clone(),
        codec: (!video.codecs.is_empty()).then(|| video.codecs.clone()),
        bitrate: (video.bandwidth > 0).then_some(video.bandwidth),
        size: None,
        width: (video.width > 0).then_some(video.width),
        height: (video.height > 0).then_some(video.height),
        quality: (video.height > 0).then(|| format!("{}p", video.height)),
        duration: Some(media.duration),
    }
}

fn audio_key(audio: &AudioStream, index: usize) -> String {
    audio
        .id
        .map_or_else(|| format!("a:{index}"), |id| format!("a:{id}"))
}

fn video_key(video: &VideoStream, index: usize) -> String {
    video.id.map_or_else(
        || format!("v:{index}:{}", video.codecid),
        |id| format!("v:{id}:{}", video.codecid),
    )
}

fn media_url(play: &PlayData, format_id: &str) -> Option<String> {
    if let Some(dash) = &play.dash {
        if let Some(rest) = format_id.strip_prefix("a:") {
            for (index, audio) in dash.audio.iter().enumerate() {
                if audio_key(audio, index).strip_prefix("a:") == Some(rest) {
                    return Some(audio.base_url.clone());
                }
            }
        }
        if let Some(rest) = format_id.strip_prefix("v:") {
            for (index, video) in dash.video.iter().enumerate() {
                if video_key(video, index).strip_prefix("v:") == Some(rest) {
                    return Some(video.base_url.clone());
                }
            }
        }
    }

    if let Some(rest) = format_id.strip_prefix("d:")
        && let Ok(index) = rest.parse::<usize>()
    {
        return play.durl.as_ref()?.get(index).map(|durl| durl.url.clone());
    }

    None
}

fn format_from_url(url: &str) -> AudioFormat {
    ::url::Url::parse(url)
        .ok()
        .and_then(|url| {
            Path::new(url.path())
                .extension()
                .and_then(|extension| extension.to_str())
                .and_then(AudioFormat::from_extension)
        })
        .unwrap_or(AudioFormat::Mp4)
}

fn tracks(view: &ViewData) -> Vec<Track> {
    if let Some(season) = &view.ugc_season {
        return season
            .sections
            .iter()
            .flat_map(|section| section.episodes.iter())
            .flat_map(|episode| {
                let multi_page = episode.pages.len() > 1;
                episode.pages.iter().enumerate().map(move |(index, page)| {
                    track(
                        &episode.bvid,
                        page,
                        &episode.arc.pic,
                        multi_page.then_some(index + 1),
                    )
                })
            })
            .collect();
    }

    if view.pages.len() <= 1 {
        let page = view.pages.first();
        let mut track = Track::new(
            page.map_or_else(|| view.bvid.clone(), |page| page.cid.to_string()),
            view.title.clone(),
            url::video_url(&view.bvid, None),
            Site::Bilibili,
        )
        .with_format(AudioFormat::M4a)
        .with_cover(view.pic.clone());
        if let Some(page) = page {
            track = track.with_duration(page.duration);
        }
        return vec![track];
    }

    view.pages
        .iter()
        .enumerate()
        .map(|(index, page)| track(&view.bvid, page, &view.pic, Some(index + 1)))
        .collect()
}

fn track(bvid: &str, page: &Page, cover: &str, page_number: Option<usize>) -> Track {
    Track::new(
        page.cid.to_string(),
        page.part.clone(),
        url::video_url(bvid, page_number),
        Site::Bilibili,
    )
    .with_format(AudioFormat::M4a)
    .with_duration(page.duration)
    .with_cover(cover)
}

fn media_headers(referer: &str) -> Result<HeaderMap> {
    Ok(http::media_headers(http::DEFAULT_UA, referer)?
        .with_accept_encoding("gzip, deflate, br")
        .with_accept_language("zh-CN,zh;q=0.9,en;q=0.8")
        .with_origin("https://www.bilibili.com"))
}
