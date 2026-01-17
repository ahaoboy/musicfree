use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ViewResponse {
    pub data: ViewData,
}

#[derive(Debug, Deserialize)]
pub struct ViewData {
    pub cid: u64,
    pub bvid: String,
    pub title: String,
    pub aid: Option<u64>,
    pub videos: u64,
    pub desc: String,
    pub duration: u64,
    pub pages: Vec<EpisodePage>,
    pub ugc_season: Option<UgcSesson>,
    pub owner: Owner,
    pub pic: String,
}

#[derive(Debug, Deserialize)]
pub struct Owner {
    pub mid: u64,
    pub name: String,
    pub face: String,
}

#[derive(Debug, Deserialize)]
pub struct EpisodePage {
    pub cid: u64,
    pub part: String,
    pub first_frame: String,
    pub duration: u64,
}

#[derive(Debug, Deserialize)]
pub struct Episode {
    pub id: u64,
    pub aid: u64,
    pub cid: u64,
    pub title: String,
    pub page: EpisodePage,
    pub pages: Vec<EpisodePage>,
    pub bvid: String,
}

#[derive(Debug, Deserialize)]
pub struct Section {
    pub season_id: u64,
    pub id: u64,
    pub title: String,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Deserialize)]
pub struct UgcSesson {
    pub id: u64,
    pub title: String,
    pub cover: String,
    pub mid: u64,
    pub intro: String,
    pub sections: Vec<Section>,
}

#[derive(Debug, Deserialize)]
pub struct PlayUrlResponse {
    pub data: PlayData,
}

#[derive(Debug, Deserialize)]
pub struct PlayData {
    pub dash: Option<Dash>,
    pub durl: Option<Vec<Durl>>,
}

#[derive(Debug, Deserialize)]
pub struct Dash {
    pub audio: Vec<Audio>,
}

#[derive(Debug, Deserialize)]
pub struct Audio {
    pub bandwidth: u64,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Durl {
    pub url: String,
}
