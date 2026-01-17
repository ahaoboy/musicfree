use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{sync::OnceLock, time::Duration};

use crate::error::{MusicFreeError, Result};

pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);
pub(crate) const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

pub fn get_http_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .connect_timeout(DEFAULT_TIMEOUT)
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(60))
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client")
    })
}

/// Get default headers for requests
fn get_default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
    headers
}

/// Create custom headers with additional values
fn create_custom_headers(additional_headers: HeaderMap) -> Result<HeaderMap> {
    let mut headers = get_default_headers().clone();
    headers.extend(additional_headers);
    Ok(headers)
}

/// Execute HTTP request with error handling
async fn execute_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    headers: HeaderMap,
) -> Result<reqwest::Response> {
    let request_headers = create_custom_headers(headers)?;
    let request = client.request(method.clone(), url).headers(request_headers);

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

/// Download binary data from URL
pub async fn download_binary(url: &str, headers: HeaderMap) -> Result<Vec<u8>> {
    let response = get_response(url, headers).await?;
    let bytes = response.bytes().await.map_err(MusicFreeError::from)?;
    Ok(bytes.to_vec())
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
    let request_headers = create_custom_headers(headers)?;
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
