use async_trait::async_trait;
use reqwest::header::{HeaderMap, REFERER,  };
use serde_json::Value;
use crate::core::{Audio, Extractor, Platform};
use crate::download::{download_binary, download_json};
use crate::error::{MusicFreeError, Result};

/// Extract BV ID from Bilibili URL
pub fn extract_bvid(url: &str) -> Result<String> {
    // Direct BV ID
    if url.starts_with("BV") && url.len() >= 12 {
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
    url.contains("bilibili.com") || url.starts_with("BV")
}

/// Download audio from Bilibili video
pub async fn download_audio(url: &str) -> Result<Audio> {
    let bvid = extract_bvid(url)?;

    // Get video info
    let api_url = format!(
        "https://api.bilibili.com/x/web-interface/view?bvid={}",
        bvid
    );
    let resp: Value = download_json(&api_url, HeaderMap::new()).await?;

    let data = resp.get("data").ok_or(MusicFreeError::VideoNotFound)?;

    let cid = data["cid"]
        .as_i64()
        .ok_or_else(|| MusicFreeError::ParseError("Cannot get CID".to_string()))?;

    let title = data["title"].as_str().unwrap_or("audio").to_string();

    // Get play URL (fnval=16 for DASH format)
    let play_url = format!(
        "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&fnval=16",
        bvid, cid
    );
    let play_resp: Value = download_json(&play_url, HeaderMap::new()).await?;

    // Extract audio URL
    let audio_url = play_resp["data"]["dash"]["audio"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|a| a["base_url"].as_str())
        .ok_or(MusicFreeError::AudioNotFound)?;

    // Download audio with proper headers
    let mut headers = HeaderMap::new();
    headers.insert(REFERER, "https://www.bilibili.com".parse()?);

    let data = download_binary(audio_url, headers).await?;

    let audio = Audio::new(title, url.to_string(), Platform::Bilibili).with_binary(data);

    Ok(audio)
}

/// Bilibili extractor implementing the Extractor trait
#[derive(Debug, Clone)]
pub struct BilibiliExtractor;

#[async_trait]
impl Extractor for BilibiliExtractor {
    fn matches(&self, url: &str) -> bool {
        is_bilibili_url(url)
    }

    async fn extract(&self, url: &str) -> Result<Vec<Audio>> {
        let audio = download_audio(url).await?;
        Ok(vec![audio])
    }

    fn platform(&self) -> Platform {
        Platform::Bilibili
    }
}
