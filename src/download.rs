use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;

use crate::error::{MusicFreeError, Result};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

/// Initialize HTTP client with default configuration
fn get_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(DEFAULT_TIMEOUT)
        .connect_timeout(DEFAULT_TIMEOUT)
        .build()
        .expect("Failed to create HTTP client")
}

/// Get default headers for requests
fn get_default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
    headers
}

/// Create custom headers with additional values
fn create_custom_headers(additional_headers: Option<HeaderMap>) -> Result<HeaderMap> {
    let mut headers = get_default_headers().clone();

    if let Some(custom) = additional_headers {
        headers.extend(custom);
    }

    Ok(headers)
}

/// Execute HTTP request with error handling
async fn execute_request(
    client: reqwest::Client,
    method: reqwest::Method,
    url: &str,
    headers: Option<HeaderMap>,
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

/// Download and parse JSON response from URL
pub async fn download_json<T: DeserializeOwned>(url: &str) -> Result<T> {
    let client = get_http_client();
    let response = execute_request(client, reqwest::Method::GET, url, None).await?;
    response.json::<T>().await.map_err(MusicFreeError::from)
}

/// Download and parse JSON response with custom headers
pub async fn download_json_with_headers<T: DeserializeOwned>(
    url: &str,
    headers: HeaderMap,
) -> Result<T> {
    let client = get_http_client();
    let response = execute_request(client, reqwest::Method::GET, url, Some(headers)).await?;
    response.json::<T>().await.map_err(MusicFreeError::from)
}

/// Download binary data from URL
pub async fn download_binary(url: &str) -> Result<Vec<u8>> {
    let client = get_http_client();
    let response = execute_request(client, reqwest::Method::GET, url, None).await?;
    let bytes = response.bytes().await.map_err(MusicFreeError::from)?;
    Ok(bytes.to_vec())
}

/// Download binary data from URL with custom headers
pub async fn download_binary_with_headers(url: &str, headers: HeaderMap) -> Result<Vec<u8>> {
    let client = get_http_client();
    let response = execute_request(client, reqwest::Method::GET, url, Some(headers)).await?;
    let bytes = response.bytes().await.map_err(MusicFreeError::from)?;
    Ok(bytes.to_vec())
}

/// Get HTTP response from URL
pub async fn get_response(url: &str) -> Result<reqwest::Response> {
    let client = get_http_client();
    execute_request(client, reqwest::Method::GET, url, None).await
}

/// Get HTTP response from URL with custom headers
pub async fn get_response_with_headers(url: &str, headers: HeaderMap) -> Result<reqwest::Response> {
    let client = get_http_client();
    execute_request(client, reqwest::Method::GET, url, Some(headers)).await
}

/// Download text content from URL
pub async fn download_text(url: &str) -> Result<String> {
    let client = get_http_client();
    let response = execute_request(client, reqwest::Method::GET, url, None).await?;
    response.text().await.map_err(MusicFreeError::from)
}

/// Download text content from URL with custom headers
pub async fn download_text_with_headers(url: &str, headers: HeaderMap) -> Result<String> {
    let client = get_http_client();
    let response = execute_request(client, reqwest::Method::GET, url, Some(headers)).await?;
    response.text().await.map_err(MusicFreeError::from)
}

/// Execute POST request with JSON body and custom headers
pub async fn post_json_with_headers<T: DeserializeOwned, B: Serialize>(
    url: &str,
    body: &B,
    headers: HeaderMap,
) -> Result<T> {
    let client = get_http_client();
    let request_headers = create_custom_headers(Some(headers))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderMap;

    #[tokio::test]
    async fn test_download_json() {
        let url = "https://httpbin.org/json";
        let result: serde_json::Value = download_json(url).await.unwrap();
        assert!(result.is_object());
    }

    #[tokio::test]
    async fn test_download_binary() {
        let url = "https://httpbin.org/bytes/100";
        let result = download_binary(url).await.unwrap();
        assert_eq!(result.len(), 100);
    }

    #[tokio::test]
    async fn test_download_text() {
        let url = "https://httpbin.org/robots.txt";
        let result = download_text(url).await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_custom_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Test", "value".parse().unwrap());

        let url = "https://httpbin.org/headers";
        let result: serde_json::Value = download_json_with_headers(url, headers).await.unwrap();
        assert!(result.is_object());
    }
}
