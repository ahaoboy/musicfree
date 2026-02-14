use crate::Playlist;
use crate::bilibili::types::{AudioInfo, PlayUrlResponse, ViewResponse};
use crate::core::{Platform, Quality};
use crate::download::{download_binary, download_json, download_text};
use crate::error::{MusicFreeError, Result};
use crate::{Audio, AudioFormat};
use reqwest::header::{HeaderMap, HeaderValue};

/// Extract audio info from view response
pub fn get_audio_info(view: &ViewResponse) -> Vec<AudioInfo> {
    let mut v = vec![];
    if let Some(ugc) = &view.data.ugc_season {
        for i in &ugc.sections {
            for e in &i.episodes {
                for p in &e.pages {
                    v.push(AudioInfo {
                        bvid: e.bvid.clone(),
                        title: p.part.clone(),
                        cid: p.cid,
                        duration: p.duration,
                        cover: e.arc.pic.clone(),
                    })
                }
            }
        }
    } else if let &[page] = &view.data.pages.as_slice() {
        v.push(AudioInfo {
            bvid: view.data.bvid.clone(),
            title: view.data.title.clone(),
            cid: page.cid,
            duration: page.duration,
            cover: view.data.pic.clone(),
        })
    } else {
        for i in &view.data.pages {
            v.push(AudioInfo {
                bvid: view.data.bvid.clone(),
                title: i.part.clone(),
                cid: i.cid,
                duration: i.duration,
                cover: view.data.pic.clone(),
            })
        }
    };
    v
}

/// Extract playlist information from Bilibili URL
pub async fn extract_audio(url: &str) -> Result<(Playlist, Option<usize>)> {
    // Get cookies first
    download_text("https://www.bilibili.com", HeaderMap::new()).await?;
    download_text(
        "https://api.bilibili.com/x/frontend/finger/spi",
        HeaderMap::new(),
    )
    .await?;

    let bvid = crate::bilibili::utils::parse_id(url).await?;
    let api_url = format!("https://api.bilibili.com/x/web-interface/view?bvid={bvid}");
    let view: ViewResponse = download_json(&api_url, HeaderMap::new()).await?;
    let v = get_audio_info(&view);
    let has_p = v.len() > 1;

    // Parse the page number from the input URL
    let requested_page = url::Url::parse(url).ok().and_then(|u| {
        u.query_pairs()
            .find(|(k, _v)| k == "p")
            .and_then(|(_, v)| v.parse::<usize>().ok())
    });

    let mut audios = vec![];
    let mut position = None;

    for (index, info) in v.into_iter().enumerate() {
        let audio_url = if has_p {
            format!("https://www.bilibili.com/video/{}?p={}", bvid, index + 1)
        } else {
            format!("https://www.bilibili.com/video/{}", bvid)
        };

        // Check if this is the requested video
        if Some(index + 1) == requested_page || info.bvid == bvid && position.is_none() {
            position = Some(index);
        }

        let id = info.cid.to_string();
        let audio = Audio::new(id, info.title, audio_url, Platform::Bilibili)
            .with_format(AudioFormat::M4A)
            .with_duration(info.duration)
            .with_cover(info.cover);

        audios.push(audio);
    }

    let (title, cover) = if let Some(ugc) = &view.data.ugc_season {
        (ugc.title.clone(), ugc.cover.clone())
    } else {
        (view.data.title, view.data.pic)
    };

    // For Bilibili, use first audio's download_url (or construct from bvid)
    let download_url = audios
        .first()
        .map(|a| a.download_url.clone())
        .or_else(|| Some(format!("https://www.bilibili.com/video/{}", view.data.bvid)));

    // Set playlist ID with fallback order: ugc_season.id -> season_id -> bvid
    let playlist_id = view
        .data
        .ugc_season
        .as_ref()
        .map(|i| i.id.to_string())
        .or_else(|| view.data.season_id.map(|id| id.to_string()))
        .or_else(|| Some(view.data.bvid.clone()));

    let playlist = Playlist {
        id: playlist_id,
        download_url,
        title: Some(title),
        audios,
        cover: Some(cover),
        platform: Platform::Bilibili,
    };

    // If playlist is empty, position should be None
    let final_position = if playlist.audios.is_empty() {
        None
    } else {
        position
    };

    Ok((playlist, final_position))
}

/// Download audio from Bilibili video
pub async fn download_audio(url: &str) -> Result<Vec<u8>> {
    let bvid = crate::bilibili::utils::parse_id(url).await?;
    let p = url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.query_pairs()
                .find(|(k, _v)| k == "p")
                .and_then(|(_, v)| v.parse::<usize>().ok())
        })
        .unwrap_or(1);

    let quality = Quality::Super;
    let url = format!("https://api.bilibili.com/x/web-interface/view?bvid={bvid}");
    let view: ViewResponse = download_json(&url, HeaderMap::new()).await?;

    let infos: Vec<_> = get_audio_info(&view).into_iter().collect();

    let Some(info) = infos.get(p - 1) else {
        return Err(MusicFreeError::DownloadFailed(format!(
            "Not found cid of page({p}) from bvid({bvid})"
        )));
    };
    let cid = &info.cid;
    let bvid = &info.bvid;
    let fnval = 16; //dash
    let play_url =
        format!("https://api.bilibili.com/x/player/playurl?bvid={bvid}&cid={cid}&fnval={fnval}");

    let resp: PlayUrlResponse = download_json(&play_url, HeaderMap::new()).await?;
    let play_data = resp.data;
    let Some(media_url) = (if let Some(dash) = play_data.dash {
        let mut audios = dash.audio;
        audios.sort_by_key(|a| a.bandwidth);

        let idx = match quality {
            Quality::Low => 0,
            Quality::Standard => 1,
            Quality::High => 2,
            Quality::Super => 3,
        };

        audios
            .get(idx)
            .or_else(|| audios.last())
            .map(|i| i.base_url.clone())
            .clone()
    } else {
        play_data
            .durl
            .and_then(|d| d.first().map(|i| i.url.clone()))
    }) else {
        return Err(MusicFreeError::DownloadFailed(
            "not found media_url".to_string(),
        ));
    };
    let audio_url = format!("https://www.bilibili.com/video/{}", bvid);
    let mut headers = HeaderMap::new();
    headers.insert("accept", HeaderValue::from_static("*/*"));
    headers.insert(
        "accept-encoding",
        HeaderValue::from_static("gzip, deflate, br"),
    );
    headers.insert("connection", HeaderValue::from_static("keep-alive"));
    headers.insert("referer", HeaderValue::from_str(&audio_url)?);
    headers.insert("range", HeaderValue::from_static("bytes=0-"));
    let bin = download_binary(&media_url, headers).await?;
    Ok(bin)
}
