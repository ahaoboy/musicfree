use thiserror::Error;

#[derive(Error, Debug)]
pub enum MusicFreeError {
    #[error("Network request failed: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Request timeout for URL: {0}")]
    RequestTimeout(String),

    #[error("HTTP error {status} for URL: {url}")]
    HttpError { status: u16, url: String },

    #[error("Invalid URL format: {0}")]
    InvalidUrl(String),

    #[error("Unsupported site: {0}")]
    UnsupportedSite(String),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),

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

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Extractor not found for URL: {0}")]
    ExtractorNotFound(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Failed to extract audio: {0}")]
    ExtractionFailed(String),

    #[error("Timeout during extraction: {0}")]
    ExtractionTimeout(String),

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("JS decryption failed: {0}")]
    JsDecryptionFailed(String),

    #[error("Cipher parse error: {0}")]
    CipherParseError(String),

    #[error("Player.js not found")]
    PlayerJsNotFound,

    #[error("Config parse error: {0}")]
    ConfigParseError(String),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}

pub type Result<T> = std::result::Result<T, MusicFreeError>;
