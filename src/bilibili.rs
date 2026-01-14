use reqwest::header::{HeaderMap, REFERER, USER_AGENT};
use serde_json::Value;

use crate::error::{MusicFreeError, Result};

const USER_AGENT_VALUE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

/// Audio metadata from Bilibili
pub struct AudioInfo {
    pub title: String,
    pub data: Vec<u8>,
}

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
pub async fn download_audio(url: &str) -> Result<AudioInfo> {
    let bvid = extract_bvid(url)?;
    let client = reqwest::Client::new();

    // Get video info
    let api_url = format!("https://api.bilibili.com/x/web-interface/view?bvid={}", bvid);
    let resp: Value = client.get(&api_url).send().await?.json().await?;

    let data = resp.get("data").ok_or(MusicFreeError::VideoNotFound)?;

    let cid = data["cid"]
        .as_i64()
        .ok_or_else(|| MusicFreeError::ParseError("Cannot get CID".to_string()))?;

    let title = data["title"]
        .as_str()
        .unwrap_or("audio")
        .to_string();

    // Get play URL (fnval=16 for DASH format)
    let play_url = format!(
        "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&fnval=16",
        bvid, cid
    );
    let play_resp: Value = client.get(&play_url).send().await?.json().await?;

    // Extract audio URL
    let audio_url = play_resp["data"]["dash"]["audio"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|a| a["base_url"].as_str())
        .ok_or(MusicFreeError::AudioNotFound)?;

    // Download audio with proper headers
    let mut headers = HeaderMap::new();
    headers.insert(REFERER, "https://www.bilibili.com".parse()?);
    headers.insert(USER_AGENT, USER_AGENT_VALUE.parse()?);

    let response = client.get(audio_url).headers(headers).send().await?;
    let data = response.bytes().await?.to_vec();

    Ok(AudioInfo { title, data })
}
