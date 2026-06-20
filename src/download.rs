use rand::{Rng, SeedableRng, rngs::StdRng};
use reqwest::header::USER_AGENT;
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue, RANGE},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{sync::OnceLock, time::Duration};

use crate::error::{MusicFreeError, Result};

pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
pub(crate) const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36 Edg/143.0.0.0";

pub fn get_http_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    let cookie_jar = recommended_cookies();

    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .connect_timeout(DEFAULT_TIMEOUT)
            .tcp_keepalive(Duration::from_secs(60))
            .cookie_provider(std::sync::Arc::new(cookie_jar))
            .cookie_store(true)
            .http1_only()
            .build()
            .expect("Failed to create HTTP client")
    })
}

pub fn recommended_cookies() -> reqwest::cookie::Jar {
    let cookie =
        "CONSENT=YES+; Path=/; Domain=youtube.com; Secure; Expires=Fri, 01 Jan 2038 00:00:00 GMT;";
    let url = "https://youtube.com".parse().unwrap();

    let jar = reqwest::cookie::Jar::default();
    jar.add_cookie_str(cookie, &url);
    jar
}

/// Get default headers for API/page requests.
fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
    headers.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        HeaderValue::from_static("en-US,en"),
    );
    headers
}

/// Merge user-provided headers with defaults.
/// User-provided values OVERWRITE defaults (uses insert, not append).
fn merge_headers(user_headers: HeaderMap) -> HeaderMap {
    let mut merged = default_headers();
    for (key, value) in user_headers.into_iter() {
        if let Some(name) = key {
            merged.insert(name, value); // insert = overwrite, NOT append
        }
    }
    merged
}

/// Execute HTTP request with error handling.
async fn execute_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    headers: HeaderMap,
) -> Result<reqwest::Response> {
    let all_headers = merge_headers(headers);
    let request = client.request(method, url).headers(all_headers);

    let response = request.send().await.map_err(|e| {
        if e.is_timeout() {
            MusicFreeError::RequestTimeout(url.to_string())
        } else {
            MusicFreeError::NetworkError(e)
        }
    })?;

    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(MusicFreeError::HttpError {
            status: status.as_u16(),
            url: url.to_string(),
        })
    }
}

/// Execute a download request WITHOUT merging default headers.
/// For download requests, the caller provides the exact headers to use
/// (UA, Referer, Accept-Encoding, etc.) — we don't want defaults added.
async fn execute_download_request(
    client: &reqwest::Client,
    url: &str,
    headers: HeaderMap,
) -> Result<reqwest::Response> {
    let request = client.get(url).headers(headers);

    let response = request.send().await.map_err(|e| {
        if e.is_timeout() {
            MusicFreeError::RequestTimeout(url.to_string())
        } else {
            MusicFreeError::NetworkError(e)
        }
    })?;

    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(MusicFreeError::HttpError {
            status: status.as_u16(),
            url: url.to_string(),
        })
    }
}

/// Download and parse JSON response with custom headers
pub async fn download_json<T: DeserializeOwned>(url: &str, headers: HeaderMap) -> Result<T> {
    let response = get_response(url, headers).await?;
    response.json::<T>().await.map_err(MusicFreeError::from)
}

/// Download binary data from URL (simple, no chunking)
pub async fn download_binary(url: &str, headers: HeaderMap) -> Result<Vec<u8>> {
    let response = get_response(url, headers).await?;
    let bytes = response.bytes().await.map_err(MusicFreeError::from)?;
    Ok(bytes.to_vec())
}

// ── Chunked download (yt-dlp style) ──────────────────────────────────────────

/// Default chunk size: 10 MB (same as yt-dlp).
const CHUNK_SIZE: usize = 10 * 1024 * 1024;

/// Maximum retries for each chunk.
const MAX_RETRIES: u32 = 5;

/// Minimum backoff between retries (milliseconds).
const RETRY_BASE_DELAY_MS: u64 = 500;

/// Build download-specific headers.
/// NOTE: We intentionally do NOT set `Accept-Encoding: identity`.
/// Browsers always advertise gzip/br support, and `identity`-only requests
/// are a strong anti-bot signal on googlevideo CDN.
fn build_download_headers(base: &HeaderMap) -> HeaderMap {
    base.clone()
}

/// Parse Content-Range header value like "bytes 0-1048575/5000000"
fn parse_content_range(content_range: &str) -> Option<usize> {
    // "bytes START-END/TOTAL"
    let total = content_range.rsplit('/').next()?;
    // "*" means unknown
    if total == "*" {
        return None;
    }
    total.parse().ok()
}

/// Retry a single chunk download with exponential backoff.
/// Only retries on 5xx server errors and network errors (like yt-dlp).
async fn download_single_chunk(url: &str, headers: &HeaderMap) -> Result<Vec<u8>> {
    let mut last_error = None;

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            let delay_ms = RETRY_BASE_DELAY_MS * 2u64.pow(attempt - 1);
            #[cfg(debug_assertions)]
            eprintln!("[debug] Retry attempt {attempt}/{MAX_RETRIES}, sleeping {delay_ms}ms");
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        match download_chunk_inner(url, headers).await {
            Ok(data) => return Ok(data),
            Err(e) => {
                #[cfg(debug_assertions)]
                eprintln!("[debug] Chunk download error (attempt {attempt}): {e}");
                let should_retry = matches!(
                    &e,
                    MusicFreeError::HttpError { status, .. } if *status >= 500 && *status < 600
                ) || matches!(
                    &e,
                    MusicFreeError::NetworkError(..) | MusicFreeError::RequestTimeout(..)
                );
                if should_retry && attempt < MAX_RETRIES {
                    last_error = Some(e);
                    continue;
                }
                return Err(e);
            }
        }
    }

    Err(last_error.unwrap_or(MusicFreeError::DownloadFailed(
        "max retries exceeded".to_string(),
    )))
}

/// Execute a single chunk download and return the bytes.
/// Returns (data, is_partial_content).
async fn download_chunk_inner(url: &str, headers: &HeaderMap) -> Result<Vec<u8>> {
    let client = get_http_client();
    let response = execute_download_request(client, url, headers.clone()).await?;
    let status = response.status();

    #[cfg(debug_assertions)]
    {
        let content_range = response
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("none");
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("none");
        eprintln!(
            "[debug] Chunk response: status={status}, \
             content-range={content_range}, content-length={content_length}"
        );
    }

    if status.is_success() {
        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(MusicFreeError::from)
    } else {
        Err(MusicFreeError::HttpError {
            status: status.as_u16(),
            url: url.to_string(),
        })
    }
}

/// Download binary data in chunks using HTTP Range requests, then concatenate.
///
/// **No HEAD request** — we discover the total size from the first Range GET's
/// Content-Range response header. This avoids the HEAD→GET double-request pattern
/// that triggers bot detection on googlevideo CDN.
///
/// Key anti-403 strategies (from yt-dlp):
/// - **No HEAD request** — go straight to Range GET
/// - **No default headers merged** — caller's UA/Referer are used as-is
/// - **`Accept-Encoding: identity`** for reliable Content-Length
/// - **Randomized chunk boundaries (±5 %)** to avoid detection patterns
/// - **Retry on 5xx / network errors** with exponential backoff
pub async fn download_binary_chunked(url: &str, headers: HeaderMap) -> Result<Vec<u8>> {
    let download_headers = build_download_headers(&headers);

    #[cfg(debug_assertions)]
    {
        eprintln!("[debug] download_binary_chunked: url={url}");
        for (key, val) in download_headers.iter() {
            eprintln!(
                "[debug]   header: {} = {}",
                key,
                val.to_str().unwrap_or("<binary>")
            );
        }
    }

    // ── First request: try Range to discover if server supports it ─────────
    let mut rng = StdRng::from_entropy();
    let first_chunk_size = CHUNK_SIZE.min(CHUNK_SIZE);
    let mut first_headers = download_headers.clone();
    first_headers.insert(
        RANGE,
        HeaderValue::from_str(&format!("bytes=0-{}", first_chunk_size - 1))?,
    );

    let client = get_http_client();
    let first_response = match execute_download_request(client, url, first_headers).await {
        Ok(resp) => resp,
        #[allow(unused_variables)]
        Err(e) => {
            // If Range fails, try without Range
            #[cfg(debug_assertions)]
            eprintln!("[debug] Range request failed: {e}, trying without Range");
            let resp = execute_download_request(client, url, download_headers.clone()).await?;
            let data = resp.bytes().await.map_err(MusicFreeError::from)?;
            return Ok(data.to_vec());
        }
    };

    let status = first_response.status();
    let content_range = first_response
        .headers()
        .get("content-range")
        .and_then(|v| v.to_str().ok());

    let total_size = content_range.and_then(parse_content_range);

    #[cfg(debug_assertions)]
    eprintln!(
        "[debug] First response: status={status}, content-range={content_range:?}, total={total_size:?}"
    );

    if status == reqwest::StatusCode::PARTIAL_CONTENT {
        let chunk_data = first_response
            .bytes()
            .await
            .map_err(MusicFreeError::from)?
            .to_vec();

        match total_size {
            Some(total) if total > CHUNK_SIZE => {
                #[cfg(debug_assertions)]
                eprintln!("[debug] Chunked mode: total={total} bytes, chunk_size={CHUNK_SIZE}");

                let mut result = Vec::with_capacity(total);
                result.extend_from_slice(&chunk_data);
                let mut offset = chunk_data.len();

                while offset < total {
                    let remaining = total - offset;

                    // Randomize chunk boundary within ±5 % (yt-dlp style)
                    let nominal = CHUNK_SIZE.min(remaining);
                    let min_chunk = ((nominal as f64) * 0.95) as usize;
                    let max_chunk = ((nominal as f64) * 1.05) as usize;
                    let chunk_len = if min_chunk < max_chunk {
                        rng.gen_range(min_chunk..=max_chunk).min(remaining)
                    } else {
                        nominal
                    };

                    let start = offset;
                    let end = offset + chunk_len - 1;

                    #[cfg(debug_assertions)]
                    eprintln!(
                        "[debug] Chunk bytes={start}-{end} ({chunk_len} bytes, {:.1}%)",
                        (offset as f64 / total as f64) * 100.0
                    );

                    let mut chunk_headers = download_headers.clone();
                    chunk_headers.insert(
                        RANGE,
                        HeaderValue::from_str(&format!("bytes={start}-{end}"))?,
                    );

                    let chunk_data = download_single_chunk(url, &chunk_headers).await?;

                    #[cfg(debug_assertions)]
                    eprintln!("[debug] Chunk received: {} bytes", chunk_data.len());

                    if chunk_data.len() > chunk_len + 1024 || chunk_data.len() < chunk_len / 2 {
                        return Ok(chunk_data);
                    }

                    result.extend_from_slice(&chunk_data);
                    offset = end + 1;
                }

                Ok(result)
            }
            _ => {
                // Small file, already got it in the first chunk
                #[cfg(debug_assertions)]
                eprintln!("[debug] Small file (got all in first Range chunk)");
                Ok(chunk_data)
            }
        }
    } else {
        // Server returned 200 (doesn't support Range) — take full body
        let data = first_response
            .bytes()
            .await
            .map_err(MusicFreeError::from)?
            .to_vec();
        Ok(data)
    }
}

/// Get HTTP response from URL with custom headers
pub async fn get_response(url: &str, headers: HeaderMap) -> Result<reqwest::Response> {
    let client = get_http_client();
    execute_request(client, reqwest::Method::GET, url, headers).await
}

/// Download text content from URL with custom headers
pub async fn download_text(url: &str, headers: HeaderMap) -> Result<String> {
    let response = get_response(url, headers).await?;
    response.text().await.map_err(MusicFreeError::from)
}

/// Execute POST request with JSON body and custom headers
pub async fn post_json<T: DeserializeOwned, B: Serialize>(
    url: &str,
    body: &B,
    headers: HeaderMap,
) -> Result<T> {
    let client = get_http_client();
    let request_headers = merge_headers(headers);
    let request = client.post(url).headers(request_headers).json(body);

    let response = request.send().await.map_err(|e| {
        if e.is_timeout() {
            MusicFreeError::RequestTimeout(url.to_string())
        } else {
            MusicFreeError::NetworkError(e)
        }
    })?;

    let status = response.status();
    if status.is_success() {
        response.json::<T>().await.map_err(MusicFreeError::from)
    } else {
        Err(MusicFreeError::HttpError {
            status: status.as_u16(),
            url: url.to_string(),
        })
    }
}
