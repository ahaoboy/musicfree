use crate::{Error, Result, http};
use reqwest::header::HeaderMap;
use serde::{Deserialize, de::DeserializeOwned};
use tokio::sync::OnceCell;

const VIEW_API: &str = "https://api.bilibili.com/x/web-interface/view";
const PLAY_API: &str = "https://api.bilibili.com/x/player/playurl";

pub async fn view(bvid: &str) -> Result<ViewData> {
    get(&format!("{VIEW_API}?bvid={bvid}")).await
}

pub async fn play(bvid: &str, cid: u64) -> Result<PlayData> {
    get(&format!("{PLAY_API}?bvid={bvid}&cid={cid}&fnval=16")).await
}

/// Establish the cookies Bilibili expects before calling its APIs.
pub async fn warm_up() -> Result<()> {
    static WARMED: OnceCell<()> = OnceCell::const_new();

    WARMED
        .get_or_try_init(|| async {
            http::get_text("https://www.bilibili.com", HeaderMap::new()).await?;
            http::get_text(
                "https://api.bilibili.com/x/frontend/finger/spi",
                HeaderMap::new(),
            )
            .await?;
            Ok::<(), Error>(())
        })
        .await?;
    Ok(())
}

async fn get<T: DeserializeOwned>(url: &str) -> Result<T> {
    let response: ApiResponse<T> = http::get_json(url, HeaderMap::new()).await?;
    if response.code != 0 {
        return Err(Error::Parse(format!(
            "Bilibili API {}: {}",
            response.code, response.message
        )));
    }
    response
        .data
        .ok_or_else(|| Error::Parse("Bilibili API returned no data".into()))
}

#[derive(Deserialize)]
struct ApiResponse<T> {
    code: i64,
    #[serde(default)]
    message: String,
    data: Option<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ViewData {
    pub bvid: String,
    pub title: String,
    #[serde(default)]
    pub pic: String,
    #[serde(default)]
    pub pages: Vec<Page>,
    #[serde(default)]
    pub ugc_season: Option<Season>,
    #[serde(default)]
    pub season_id: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Page {
    pub cid: u64,
    #[serde(default)]
    pub part: String,
    #[serde(default)]
    pub duration: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Season {
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Section {
    #[serde(default)]
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Episode {
    pub bvid: String,
    #[serde(default)]
    pub pages: Vec<Page>,
    #[serde(default)]
    pub arc: Arc,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Arc {
    #[serde(default)]
    pub pic: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayData {
    #[serde(default)]
    pub dash: Option<Dash>,
    #[serde(default)]
    pub durl: Option<Vec<Durl>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dash {
    #[serde(default)]
    pub audio: Vec<AudioStream>,
    #[serde(default)]
    pub video: Vec<VideoStream>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AudioStream {
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub bandwidth: u64,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(default, rename = "mimeType")]
    pub mime_type: String,
    #[serde(default)]
    pub codecs: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideoStream {
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub bandwidth: u64,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(default, rename = "mimeType")]
    pub mime_type: String,
    #[serde(default)]
    pub codecs: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub codecid: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Durl {
    pub url: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub length: Option<u64>,
}
