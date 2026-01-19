use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewResponse {
    pub data: ViewData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewData {
    pub cid: u64,
    pub bvid: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aid: Option<u64>,
    pub videos: u64,
    pub desc: String,
    pub duration: u64,
    pub pages: Vec<EpisodePage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ugc_season: Option<UgcSession>,
    pub owner: Owner,
    pub pic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub mid: u64,
    pub name: String,
    pub face: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodePage {
    pub cid: u64,
    pub part: String,
    pub duration: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeArc {
    pub title: String,
    pub pic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: u64,
    pub aid: u64,
    pub cid: u64,
    pub title: String,
    pub page: EpisodePage,
    pub pages: Vec<EpisodePage>,
    pub bvid: String,
    pub arc: EpisodeArc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    #[serde(rename = "season_id")]
    pub season_id: u64,
    pub id: u64,
    pub title: String,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UgcSession {
    pub id: u64,
    pub title: String,
    pub cover: String,
    pub mid: u64,
    pub intro: String,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayUrlResponse {
    pub data: PlayData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dash: Option<Dash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub durl: Option<Vec<Durl>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dash {
    pub audio: Vec<Audio>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
    pub bandwidth: u64,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Durl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInfo {
    pub cid: u64,
    pub title: String,
    pub bvid: String,
    pub cover: String,
    pub duration: u64,
}
