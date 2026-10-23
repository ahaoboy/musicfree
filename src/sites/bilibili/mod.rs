pub mod api;
pub mod url;

use crate::{
    AudioFormat, Error, Playlist, Result, Site, Track, http, http::HeaderMapExt, sites::Extractor,
};
use api::{Page, ViewData};
use async_trait::async_trait;
use reqwest::header::HeaderMap;

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

    async fn download(&self, input: &str) -> Result<Vec<u8>> {
        api::warm_up().await?;
        let bvid = url::video_id(input).await?;
        let page_number = url::page(input);
        let view = api::view(&bvid).await?;
        let page = view
            .pages
            .get(page_number.saturating_sub(1))
            .ok_or(Error::AudioNotFound)?;
        let play = api::play(&bvid, page.cid).await?;

        let audio_url = play
            .dash
            .and_then(|dash| {
                dash.audio
                    .into_iter()
                    .max_by_key(|audio| audio.bandwidth)
                    .map(|audio| audio.base_url)
            })
            .or_else(|| {
                play.durl
                    .and_then(|durls| durls.into_iter().next())
                    .map(|durl| durl.url)
            })
            .ok_or(Error::AudioNotFound)?;

        let referer = url::video_url(&bvid, (page_number > 1).then_some(page_number));
        http::download(&audio_url, media_headers(&referer)?).await
    }

    async fn download_cover(&self, url: &str) -> Result<Vec<u8>> {
        http::get_bytes(url, media_headers("https://www.bilibili.com/")?).await
    }
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
