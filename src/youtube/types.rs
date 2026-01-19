use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerResponse {
    #[serde(rename = "streamingData")]
    pub streaming_data: StreamingData,
    #[serde(rename = "videoDetails")]
    pub video_details: VideoDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDetails {
    #[serde(rename = "videoId")]
    pub video_id: String,
    pub title: String,
    #[serde(rename = "lengthSeconds")]
    pub length_seconds: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingData {
    pub formats: Vec<Format>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Format {
    pub itag: u64,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "lastModified")]
    pub last_modified: String,
    pub quality: String,
    #[serde(rename = "signatureCipher")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_cipher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtConfig {
    #[serde(rename = "INNERTUBE_CONTEXT")]
    pub innertube_context: InnertubeContext,
    #[serde(rename = "VISITOR_DATA")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visitor_data: Option<String>,
    #[serde(rename = "INNERTUBE_API_KEY")]
    pub innertube_api_key: String,
    #[serde(rename = "PLAYER_JS_URL")]
    pub player_js_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnertubeContext {
    pub client: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct InnertubeRequest {
    #[serde(rename = "videoId")]
    pub video_id: String,
    pub context: InnertubeContext,
    #[serde(rename = "playbackContext")]
    pub playback_context: PlaybackContext,
    #[serde(rename = "contentCheckOk")]
    pub content_check_ok: bool,
    #[serde(rename = "racyCheckOk")]
    pub racy_check_ok: bool,
}

#[derive(Debug, Serialize)]
pub struct PlaybackContext {
    #[serde(rename = "contentPlaybackContext")]
    pub content_playback_context: ContentPlaybackContext,
}

#[derive(Debug, Serialize)]
pub struct ContentPlaybackContext {
    #[serde(rename = "html5Preference")]
    pub html5_preference: String,
}
