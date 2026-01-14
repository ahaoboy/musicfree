use thiserror::Error;

#[derive(Error, Debug)]
pub enum MusicFreeError {
    #[error("Network request failed: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid URL format: {0}")]
    InvalidUrl(String),

    #[error("Unsupported site: {0}")]
    UnsupportedSite(String),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Audio stream not found")]
    AudioNotFound,

    #[error("Video not found or unavailable")]
    VideoNotFound,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("YouTube extractor failed: {0}")]
    YoutubeError(String),

    #[error("External command failed: {0}")]
    CommandError(String),

    #[error("Invalid header value: {0}")]
    HeaderError(#[from] reqwest::header::InvalidHeaderValue),
}

pub type Result<T> = std::result::Result<T, MusicFreeError>;
