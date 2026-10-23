use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("request timed out: {0}")]
    Timeout(String),

    #[error("HTTP {status} for {url}")]
    Http { status: u16, url: String },

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("unsupported URL: {0}")]
    Unsupported(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("audio stream not found")]
    AudioNotFound,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid header value: {0}")]
    Header(#[from] reqwest::header::InvalidHeaderValue),

    #[error("cookie error: {0}")]
    Cookie(#[from] serde_cookie::Error),

    #[error("download failed: {0}")]
    Download(String),

    #[error("JavaScript execution failed: {0}")]
    Js(String),

    #[error("cipher error: {0}")]
    Cipher(String),

    #[error("player JavaScript not found")]
    PlayerJsNotFound,
}

impl Error {
    pub(crate) fn http(status: reqwest::StatusCode, url: &str) -> Self {
        Self::Http {
            status: status.as_u16(),
            url: url.to_owned(),
        }
    }

    pub(crate) fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_) | Self::Timeout(_))
            || matches!(self, Self::Http { status, .. } if matches!(*status, 408 | 429) || (500..600).contains(status))
    }
}
