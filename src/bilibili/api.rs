use crate::bilibili::core::{PlayUrlResponse, ViewResponse};
use crate::download::download_text;
use crate::{
    Audio, Platform,
    download::{download_binary, download_json},
    error::{MusicFreeError, Result},
};
use reqwest::header::{HeaderMap, HeaderValue};

/// Extract BV ID from Bilibili URL
pub fn extract_bvid(url: &str) -> Result<String> {
    // Direct BV ID
    if url.starts_with("BV") && url.len() == 12 {
        return Ok(url[..12].to_string());
    }

    // URL patterns: bilibili.com/video/BVxxxxx
    if let Some(pos) = url.find("BV") {
        let bvid: String = url[pos..].chars().take(12).collect();
        if bvid.len() == 12 {
            return Ok(bvid);
        }
    }

    Err(MusicFreeError::InvalidUrl(format!(
        "Cannot extract BV ID from: {}",
        url
    )))
}

/// Check if URL is a Bilibili link
pub fn is_bilibili_url(url: &str) -> bool {
    url.contains("bilibili.com") || (url.starts_with("BV") && url.len() == 12)
}

pub enum Quality {
    Low,
    Standard,
    High,
    Super,
}

/// Download audio from Bilibili video
pub async fn download_audio(url: &str) -> Result<Vec<Audio>> {
    let bvid = extract_bvid(url)?;
    download_text("https://www.bilibili.com", HeaderMap::new()).await?;
    download_text(
        "https://api.bilibili.com/x/frontend/finger/spi",
        HeaderMap::new(),
    )
    .await?;

    let quality = Quality::Super;

    let url = format!("https://api.bilibili.com/x/web-interface/view?bvid={bvid}");
    let view: ViewResponse = download_json(&url, HeaderMap::new()).await?;

    let mut v = vec![];
    if let Some(ugc) = &view.data.ugc_season {
        for i in &ugc.sections {
            for e in &i.episodes {
                for p in &e.pages {
                    v.push((e.bvid.clone(), p))
                }
            }
        }
    } else {
        for i in &view.data.pages {
            v.push((view.data.bvid.clone(), i))
        }
    };

    let mut audios = vec![];
    for (bvid, p) in v {
        let cid = p.cid;

        let fnval = 16; //dash
        let play_url = format!(
            "https://api.bilibili.com/x/player/playurl?bvid={bvid}&cid={cid}&fnval={fnval}"
        );

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
            continue;
        };
        let referer = format!("https://www.bilibili.com/video/{}", bvid);
        let mut headers = HeaderMap::new();
        headers.insert(
            "user-agent",
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            ),
        );
        headers.insert("accept", HeaderValue::from_static("*/*"));
        headers.insert(
            "accept-encoding",
            HeaderValue::from_static("gzip, deflate, br"),
        );
        headers.insert("connection", HeaderValue::from_static("keep-alive"));
        headers.insert("referer", HeaderValue::from_str(&referer).unwrap());
        headers.insert("range", HeaderValue::from_static("bytes=0-"));

        let id = p.cid.to_string();
        let bin = download_binary(&media_url, headers).await?;
        let audio = Audio::new(id, p.part.clone(), url.to_string(), Platform::Bilibili)
            .with_binary(bin)
            .with_format(crate::core::AudioFormat::M4A);

        audios.push(audio);
    }

    Ok(audios)
}
