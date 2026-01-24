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
    #[serde(rename = "adaptiveFormats")]
    pub adaptive_formats: Vec<Format>,
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
    #[serde(rename = "INNERTUBE_CLIENT_VERSION")]
    pub innertube_client_version: String,
    #[serde(rename = "INNERTUBE_API_VERSION")]
    pub innertube_api_version: String,
    #[serde(rename = "INNERTUBE_CLIENT_NAME")]
    pub innertube_client_name: String,
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
    // #[serde(rename = "html5Preference")]
    // pub html5_preference: String,
    pub pcm2: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtInitialData {
    pub contents: Contents,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<PlaylistHeader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contents {
    #[serde(rename = "twoColumnWatchNextResults")]
    pub two_column_watch_next_results: Option<TwoColumnWatchNextResults>,

    #[serde(rename = "twoColumnBrowseResultsRenderer")]
    pub two_column_browse_results_renderer: Option<TwoColumnBrowseResultsRenderer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoColumnBrowseResultsRenderer {
    pub tabs: Vec<Tab>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistHeader {
    #[serde(rename = "playlistHeaderRenderer")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist_header_renderer: Option<PlaylistHeaderRenderer>,
    #[serde(rename = "pageHeaderRenderer")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_header_renderer: Option<PageHeaderRenderer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistHeaderRenderer {
    pub title: Option<Title>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageHeaderRenderer {
    #[serde(rename = "pageTitle")]
    pub page_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    #[serde(rename = "tabRenderer")]
    pub tab_renderer: TabRenderer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabRenderer {
    pub selected: bool,
    pub content: TabContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabContent {
    #[serde(rename = "sectionListRenderer")]
    pub section_list_renderer: SectionListRenderer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionListRenderer {
    pub contents: Vec<SectionContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionContent {
    #[serde(rename = "itemSectionRenderer")]
    pub item_section_renderer: Option<ItemSectionRenderer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSectionRenderer {
    pub contents: Vec<ItemContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemContent {
    #[serde(rename = "playlistVideoListRenderer")]
    pub playlist_video_list_renderer: PlaylistVideoListRenderer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistVideoListRenderer {
    pub contents: Vec<VideoElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoElement {
    #[serde(rename = "playlistVideoRenderer")]
    pub playlist_video_renderer: PlaylistVideoRenderer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistVideoRenderer {
    #[serde(rename = "videoId")]
    pub video_id: String,
    #[serde(rename = "lengthSeconds")]
    pub length_seconds: String,
    pub title: PlaylistVideoRendererTitle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistVideoRendererTitle {
    pub runs: Vec<PlaylistVideoRendererTitleRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistVideoRendererTitleRun {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoColumnWatchNextResults {
    #[serde(rename = "playlist")]
    pub playlist: Playlist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    #[serde(rename = "playlist")]
    pub playlist: PlaylistData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistData {
    pub title: Option<String>,
    pub contents: Vec<PlaylistContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlaylistContent {
    Video(PlaylistVideoContent),
    Other(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistVideoContent {
    #[serde(rename = "playlistPanelVideoRenderer")]
    pub playlist_panel_video_renderer: PlaylistPanelVideoRenderer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistPanelVideoRenderer {
    pub title: Title,
    #[serde(rename = "navigationEndpoint")]
    pub navigation_endpoint: NavigationEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Title {
    SimpleText {
        #[serde(rename = "simpleText")]
        simple_text: String,
    },
    Runs {
        runs: Vec<Run>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationEndpoint {
    #[serde(rename = "commandMetadata")]
    pub command_metadata: CommandMetadata,
    #[serde(rename = "watchEndpoint")]
    pub watch_endpoint: WatchEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandMetadata {
    #[serde(rename = "webCommandMetadata")]
    pub web_command_metadata: WebCommandMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebCommandMetadata {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEndpoint {
    #[serde(rename = "videoId")]
    pub video_id: String,
}
