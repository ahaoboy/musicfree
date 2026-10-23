use crate::{Error, Result, http};
use abv::av2bv;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, REFERER};
use url::Url;

const VIDEO_HOST: &str = "https://www.bilibili.com/video";

pub fn is_url(input: &str) -> bool {
    direct_bv(input).is_some()
        || direct_av(input).is_some()
        || Url::parse(input)
            .ok()
            .and_then(|url| url.host_str().map(is_host))
            .unwrap_or(false)
}

pub async fn video_id(input: &str) -> Result<String> {
    if let Some(bvid) = direct_bv(input) {
        return Ok(bvid);
    }
    if let Some(bvid) = direct_av(input) {
        return Ok(bvid);
    }

    let url = Url::parse(input).map_err(|error| Error::InvalidUrl(error.to_string()))?;
    let url = if is_short(&url) {
        resolve_short(&url).await?
    } else {
        url
    };

    find_bv(url.path())
        .or_else(|| find_av(url.path()))
        .or_else(|| query_value(&url, "bvid"))
        .or_else(|| {
            query_value(&url, "aid")
                .and_then(|aid| aid.parse().ok())
                .and_then(|aid| av2bv(aid).ok())
        })
        .ok_or_else(|| Error::InvalidUrl(format!("cannot extract Bilibili video ID from {input}")))
}

pub fn page(input: &str) -> usize {
    Url::parse(input)
        .ok()
        .and_then(|url| query_value(&url, "p"))
        .and_then(|page| page.parse().ok())
        .filter(|page| *page > 0)
        .unwrap_or(1)
}

pub fn video_url(bvid: &str, page: Option<usize>) -> String {
    match page {
        Some(page) => format!("{VIDEO_HOST}/{bvid}?p={page}"),
        None => format!("{VIDEO_HOST}/{bvid}"),
    }
}

fn direct_bv(input: &str) -> Option<String> {
    let id = input.strip_prefix("BV")?;
    (id.len() == 10 && id.chars().all(|c| c.is_ascii_alphanumeric())).then(|| input.to_owned())
}

fn direct_av(input: &str) -> Option<String> {
    let id = input.strip_prefix("av")?;
    let aid = id.parse().ok()?;
    av2bv(aid).ok()
}

fn find_bv(text: &str) -> Option<String> {
    let start = text.find("BV")?;
    let id = text.get(start..start + 12)?;
    id.chars()
        .all(|c| c.is_ascii_alphanumeric())
        .then(|| id.to_owned())
}

fn find_av(text: &str) -> Option<String> {
    text.split('/').find_map(|segment| {
        let id = segment.strip_prefix("av")?;
        let digits = id
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        let aid = digits.parse().ok()?;
        av2bv(aid).ok()
    })
}

fn query_value(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

fn is_host(host: &str) -> bool {
    host == "bilibili.com" || host.ends_with(".bilibili.com") || host == "b23.tv"
}

fn is_short(url: &Url) -> bool {
    url.host_str() == Some("b23.tv")
}

async fn resolve_short(url: &Url) -> Result<Url> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        ACCEPT,
        reqwest::header::HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        ),
    );
    headers.insert(
        ACCEPT_LANGUAGE,
        reqwest::header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
    );
    headers.insert(
        REFERER,
        reqwest::header::HeaderValue::from_static("https://www.bilibili.com/"),
    );
    Ok(http::head(url.as_str(), headers).await?.url().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_direct_ids() {
        assert_eq!(direct_bv("BV1aAV7zHEhJ").as_deref(), Some("BV1aAV7zHEhJ"));
        assert!(direct_av("av170001").is_some());
    }

    #[test]
    fn finds_ids_in_text() {
        assert_eq!(
            find_bv("https://www.bilibili.com/video/BV1aAV7zHEhJ?p=2").as_deref(),
            Some("BV1aAV7zHEhJ")
        );
        assert!(find_av("/video/av170001").is_some());
    }

    #[test]
    fn recognizes_hosts() {
        assert!(is_url("https://www.bilibili.com/video/BV1aAV7zHEhJ"));
        assert!(is_url("https://b23.tv/vBnRYHM"));
        assert!(!is_url("https://example.com/video/BV1aAV7zHEhJ"));
    }
}
