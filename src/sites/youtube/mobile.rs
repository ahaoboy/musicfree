use super::{api, download as media, url};
use crate::{Error, Result, http, http::HeaderMapExt};
use serde_youtube::types::YtConfig;

/// Download through the Android/mobile player API.
pub async fn download(
    video_id: &str,
    config: &YtConfig,
    html: &str,
    sts: Option<u64>,
) -> Result<Vec<u8>> {
    let player = api::android_player(video_id, config, sts).await?;
    let format = player
        .streaming_data
        .best_playable_format()
        .ok_or(Error::AudioNotFound)?;
    let media_url = media::resolve_url(format, html, None).await?;
    let headers = http::media_headers(url::ANDROID_UA, "https://www.youtube.com/")?
        .with_origin("https://www.youtube.com")
        .with_sec_fetch();

    http::download(&media_url, headers).await
}
