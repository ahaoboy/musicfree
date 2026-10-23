use crate::{Error, Result};
use reqwest::{
    Client, Response, StatusCode,
    header::{
        ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CONNECTION, CONTENT_RANGE, HeaderMap,
        HeaderValue, ORIGIN, RANGE, REFERER, USER_AGENT,
    },
};
use serde::{Serialize, de::DeserializeOwned};
use serde_cookie::{Cookie, CookieJar};
use std::{
    path::Path,
    sync::{Arc, OnceLock},
    time::Duration,
};

pub const DEFAULT_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";
const TIMEOUT: Duration = Duration::from_secs(300);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const CHUNK_SIZE: usize = 10 * 1024 * 1024;
const MAX_RETRIES: u32 = 5;
const RETRY_BASE_MS: u64 = 500;

static CUSTOM_COOKIES: OnceLock<CookieJar> = OnceLock::new();

/// Shared HTTP client. Cookies must be loaded before the first request.
pub fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();

    CLIENT.get_or_init(|| {
        let jar = build_cookie_jar();
        Client::builder()
            .default_headers(default_headers())
            .timeout(TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            .cookie_provider(Arc::new(jar))
            .cookie_store(true)
            .build()
            .expect("failed to build HTTP client")
    })
}

/// Load a Netscape-format `cookies.txt` file (yt-dlp style).
pub fn load_cookies_from_file(path: impl AsRef<Path>) -> Result<()> {
    let jar = CookieJar::from_path(path.as_ref())?;
    if jar.is_empty() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "cookie file {} contains no cookies",
                path.as_ref().display()
            ),
        )));
    }
    let _ = CUSTOM_COOKIES.set(jar);
    Ok(())
}

fn build_cookie_jar() -> reqwest::cookie::Jar {
    let jar = reqwest::cookie::Jar::default();
    let mut cookies = CookieJar::new();
    cookies.push(Cookie {
        domain: ".youtube.com".into(),
        include_subdomains: true,
        path: "/".into(),
        secure: true,
        expiry: 0,
        name: "CONSENT".into(),
        value: "YES+".into(),
    });
    if let Some(custom) = CUSTOM_COOKIES.get() {
        cookies.extend(custom.iter().cloned());
    }
    cookies.add_to_reqwest_jar(&jar);
    jar
}

fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_UA));
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers
}

/// Browser-like headers for page and media requests.
pub fn media_headers(ua: &'static str, referer: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(ua));
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert(REFERER, HeaderValue::from_str(referer)?);
    Ok(headers)
}

/// Builder-style helpers for request headers.
pub trait HeaderMapExt {
    fn with_origin(self, origin: &'static str) -> Self;
    fn with_sec_fetch(self) -> Self;
    fn with_accept_encoding(self, encoding: &'static str) -> Self;
    fn with_accept_language(self, language: &'static str) -> Self;
}

impl HeaderMapExt for HeaderMap {
    fn with_origin(mut self, origin: &'static str) -> Self {
        self.insert(ORIGIN, HeaderValue::from_static(origin));
        self
    }

    fn with_sec_fetch(mut self) -> Self {
        self.insert("sec-fetch-mode", HeaderValue::from_static("no-cors"));
        self.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        self
    }

    fn with_accept_encoding(mut self, encoding: &'static str) -> Self {
        self.insert(ACCEPT_ENCODING, HeaderValue::from_static(encoding));
        self
    }

    fn with_accept_language(mut self, language: &'static str) -> Self {
        self.insert(ACCEPT_LANGUAGE, HeaderValue::from_static(language));
        self
    }
}

async fn send(request: reqwest::RequestBuilder, url: &str) -> Result<Response> {
    let response = request.send().await.map_err(|error| {
        if error.is_timeout() {
            Error::Timeout(url.to_owned())
        } else {
            Error::Network(error)
        }
    })?;
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(Error::http(response.status(), url))
    }
}

pub async fn get(url: &str, headers: HeaderMap) -> Result<Response> {
    send(client().get(url).headers(headers), url).await
}

pub async fn head(url: &str, headers: HeaderMap) -> Result<Response> {
    send(client().head(url).headers(headers), url).await
}

pub async fn get_text(url: &str, headers: HeaderMap) -> Result<String> {
    get(url, headers).await?.text().await.map_err(Error::from)
}

pub async fn get_json<T: DeserializeOwned>(url: &str, headers: HeaderMap) -> Result<T> {
    get(url, headers).await?.json().await.map_err(Error::from)
}

pub async fn get_bytes(url: &str, headers: HeaderMap) -> Result<Vec<u8>> {
    get(url, headers)
        .await?
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(Error::from)
}

pub async fn post_json<T: DeserializeOwned, B: Serialize>(
    url: &str,
    body: &B,
    headers: HeaderMap,
) -> Result<T> {
    send(client().post(url).headers(headers).json(body), url)
        .await?
        .json()
        .await
        .map_err(Error::from)
}

struct Chunk {
    data: Vec<u8>,
    total: Option<usize>,
    partial: bool,
}

/// Download a media file using ranged requests with retries and jittered
/// chunk boundaries.
pub async fn download(url: &str, headers: HeaderMap) -> Result<Vec<u8>> {
    let first = match fetch_chunk_retry(url, &headers, Some((0, CHUNK_SIZE - 1)), None).await {
        Ok(chunk) => chunk,
        Err(_) => fetch_chunk_retry(url, &headers, None, None).await?,
    };

    if !first.partial {
        return Ok(first.data);
    }
    let Some(total) = first.total else {
        return fetch_chunk_retry(url, &headers, None, None)
            .await
            .map(|chunk| chunk.data);
    };
    if first.data.len() >= total {
        return Ok(first.data);
    }

    let mut data = Vec::new();
    data.try_reserve(total)
        .map_err(|error| Error::Download(format!("cannot allocate {total} bytes: {error}")))?;
    data.extend_from_slice(&first.data);

    while data.len() < total {
        let start = data.len();
        let remaining = total - start;
        let nominal = CHUNK_SIZE.min(remaining);
        let jitter = nominal / 20;
        let low = nominal.saturating_sub(jitter).max(1);
        let high = (nominal + jitter).min(remaining);
        let len = rand::random_range(low..=high);
        let end = start + len - 1;

        let chunk = fetch_chunk_retry(url, &headers, Some((start, end)), Some(len)).await?;
        data.extend_from_slice(&chunk.data);
    }

    Ok(data)
}

async fn fetch_chunk_retry(
    url: &str,
    headers: &HeaderMap,
    range: Option<(usize, usize)>,
    expected: Option<usize>,
) -> Result<Chunk> {
    let mut last_error = None;

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(RETRY_BASE_MS * 2u64.pow(attempt - 1))).await;
        }

        match fetch_chunk(url, headers, range).await {
            Ok(chunk) if expected.is_none_or(|len| chunk.data.len() == len) => return Ok(chunk),
            Ok(chunk) => {
                last_error = Some(Error::Download(format!(
                    "expected {} bytes, received {}",
                    expected.unwrap_or_default(),
                    chunk.data.len()
                )));
            }
            Err(error) if error.is_retryable() && attempt < MAX_RETRIES => {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| Error::Download("max retries exceeded".into())))
}

async fn fetch_chunk(
    url: &str,
    headers: &HeaderMap,
    range: Option<(usize, usize)>,
) -> Result<Chunk> {
    let mut request = client().get(url).headers(headers.clone());
    if let Some((start, end)) = range {
        request = request.header(RANGE, format!("bytes={start}-{end}"));
    }

    let response = send(request, url).await?;
    let partial = response.status() == StatusCode::PARTIAL_CONTENT;
    let total = response
        .headers()
        .get(CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_total);
    let data = response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(Error::from)?;

    Ok(Chunk {
        data,
        total,
        partial,
    })
}

fn parse_total(content_range: &str) -> Option<usize> {
    let total = content_range.rsplit_once('/')?.1;
    total.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_range_total() {
        assert_eq!(parse_total("bytes 0-99/1234"), Some(1234));
        assert_eq!(parse_total("bytes 0-99/*"), None);
        assert_eq!(parse_total("invalid"), None);
    }
}
