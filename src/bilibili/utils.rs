use crate::download::get_http_client;
use crate::error::{MusicFreeError, Result};
use abv::av2bv;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
use url::Url;

/// Resolve short link by following redirects
pub async fn resolve_short_link(short_url: &str) -> Result<String> {
    let mut headers = HeaderMap::new();
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
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));
    headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static("zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7"),
    );
    headers.insert(
        "sec-ch-ua",
        HeaderValue::from_static(
            "\"Not(A:Brand\";v=\"8\", \"Chromium\";v=\"144\", \"Google Chrome\";v=\"144\"",
        ),
    );
    headers.insert(
        "sec-ch-ua-platform",
        HeaderValue::from_static("\"Windows\""),
    );

    let client = get_http_client();
    let response = client.head(short_url).send().await?;
    let final_url = response.url().to_string();
    Ok(final_url)
}

/// Parse BV ID from Bilibili URL
pub async fn parse_id(url: &str) -> Result<String> {
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

                    // If still can't extract, try using query parameters
                    if let Some(query) = resolved_parsed.query()
                        && let Some(bv_match) = query.split('&').find(|p| p.starts_with("bvid="))
                        && let Some(bvid) = bv_match.split('=').nth(1)
                    {
                        return Ok(bvid.to_string());
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

/// Check if URL is a Bilibili link
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

/// Check if URL is a Bilibili short URL
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
