use crate::bilibili::core::{PlayUrlResponse, ViewResponse};
use crate::download::{download_text, get_response};
use crate::{
    Audio, Platform,
    download::{download_binary, download_json},
    error::{MusicFreeError, Result},
};
use abv::av2bv;
use reqwest::header::{HeaderMap, HeaderValue};
use url::Url;

/// Resolve short link by following redirects
async fn resolve_short_link(short_url: &str) -> Result<String> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Accept",
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
        ),
    );
    headers.insert(
        "Accept-Language",
        HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
    );
    headers.insert(
        "Accept-Encoding",
        HeaderValue::from_static("gzip, deflate, br"),
    );
    headers.insert("Connection", HeaderValue::from_static("keep-alive"));
    headers.insert("Upgrade-Insecure-Requests", HeaderValue::from_static("1"));
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://www.bilibili.com/"),
    );
    let response = get_response(short_url, headers).await?;
    let final_url = response.url().to_string();
    Ok(final_url)
}

/// Extract BV ID from Bilibili URL
pub async fn extract_bvid(url: &str) -> Result<String> {
    // Direct BV ID
    if url.starts_with("BV") && url.len() == 12 {
        return Ok(url[..12].to_string());
    }

    // Parse URL to extract path
    if let Ok(parsed_url) = Url::parse(url) {
        let path = parsed_url.path();

        // Handle short URLs (b23.tv)
        if parsed_url.domain() == Some("b23.tv") {
            let path_segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();

            for segment in path_segments {
                if segment.is_empty() {
                    continue;
                }

                // Check if it's a BV short link
                if segment.starts_with("BV") && segment.len() == 12 {
                    return Ok(segment.to_string());
                }

                // Check if it's an AV short link
                if segment.starts_with("av")
                    && let Ok(av_id) = segment[2..].parse::<u64>()
                    && let Ok(bvid) = av2bv(av_id)
                {
                    return Ok(bvid);
                }

                // Check if it's a general short code (7 alphanumeric chars)
                if segment.len() == 7 && segment.chars().all(|c| c.is_alphanumeric()) {
                    // Resolve the short link by making HTTP request
                    let resolved_url = resolve_short_link(url).await?;
                    // Use a non-recursive approach: try to extract from the resolved URL directly
                    let resolved_parsed = Url::parse(&resolved_url).map_err(|e| {
                        MusicFreeError::InvalidUrl(format!("Failed to parse resolved URL: {}", e))
                    })?;
                    let resolved_path = resolved_parsed.path();

                    if let Some(pos) = resolved_path.find("BV") {
                        let bvid: String = resolved_path[pos..].chars().take(12).collect();
                        if bvid.len() == 12 {
                            return Ok(bvid);
                        }
                    }

                    return Err(MusicFreeError::InvalidUrl(format!(
                        "Cannot extract BV ID from resolved URL: {}",
                        resolved_url
                    )));
                }
            }
        }

        // Handle regular bilibili.com URLs
        if let Some(pos) = path.find("BV") {
            let bvid: String = path[pos..].chars().take(12).collect();
            if bvid.len() == 12 {
                return Ok(bvid);
            }
        }
    }

    Err(MusicFreeError::InvalidUrl(format!(
        "Cannot extract BV ID from: {}",
        url
    )))
}

pub fn is_bilibili_url(url: &str) -> bool {
    // Direct BV ID
    if url.starts_with("BV") && url.len() == 12 {
        return true;
    }

    // Parse URL for proper domain validation
    if let Ok(parsed_url) = Url::parse(url) {
        match parsed_url.domain() {
            Some(domain) => {
                // Check for official bilibili domains
                domain == "bilibili.com"
                    || domain == "www.bilibili.com"
                    || domain == "b23.tv"
                    || domain == "m.bilibili.com"
            }
            None => false,
        }
    } else {
        false
    }
}

pub fn is_bilibili_short_url(url: &str) -> bool {
    if let Ok(parsed_url) = Url::parse(url)
        && let Some(domain) = parsed_url.domain()
        && domain == "b23.tv"
    {
        let path = parsed_url.path().trim_start_matches('/');

        // Check for valid short link patterns
        // General short code: 7 alphanumeric characters
        if path.len() == 7 && path.chars().all(|c| c.is_alphanumeric()) {
            return true;
        }

        // AV short link: av + numeric ID
        if path.starts_with("av") && path[2..].chars().all(|c| c.is_numeric()) {
            return true;
        }

        // BV short link: BV + 12 characters
        if path.starts_with("BV") && path.len() >= 12 {
            return true;
        }
    }
    false
}

pub enum Quality {
    Low,
    Standard,
    High,
    Super,
}

/// Download audio from Bilibili video
pub async fn download_audio(url: &str) -> Result<Vec<Audio>> {
    // Get cookies first
    download_text("https://www.bilibili.com", HeaderMap::new()).await?;
    download_text(
        "https://api.bilibili.com/x/frontend/finger/spi",
        HeaderMap::new(),
    )
    .await?;

    let bvid = extract_bvid(url).await?;

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
