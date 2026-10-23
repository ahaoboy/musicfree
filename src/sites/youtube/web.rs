use super::{api, download as media, url};
use crate::{Error, Result, http, http::HeaderMapExt};
use serde_youtube::types::{PlayerResponse, YtConfig};

/// Download through the web player API. Falls back to clients that still
/// expose direct URLs when WEB only returns SABR metadata.
pub async fn download(
    video_id: &str,
    html: &str,
    config: &YtConfig,
    sts: Option<u64>,
) -> Result<Vec<u8>> {
    let mut sts = sts;
    let mut player_js = None;

    if sts.is_none()
        && let Ok(js) = media::player_js(html).await
    {
        sts = api::sts_from_player_js(&js);
        player_js = Some(js);
    }

    let (player, user_agent) = player(video_id, html, config, sts).await?;
    let format = player
        .streaming_data
        .best_playable_format()
        .ok_or(Error::AudioNotFound)?;
    let media_url = media::resolve_url(format, html, player_js).await?;
    let headers = http::media_headers(user_agent, "https://www.youtube.com/")?
        .with_origin("https://www.youtube.com")
        .with_sec_fetch();

    http::download(&media_url, headers).await
}

async fn player(
    video_id: &str,
    html: &str,
    config: &YtConfig,
    sts: Option<u64>,
) -> Result<(PlayerResponse, &'static str)> {
    if let Ok(player) = api::player_from_html(html)
        && player.streaming_data.best_playable_format().is_some()
    {
        return Ok((player, url::WEB_UA));
    }

    if let Ok(player) = api::web_player(video_id, config, sts).await
        && player.streaming_data.best_playable_format().is_some()
    {
        return Ok((player, url::WEB_UA));
    }

    if let Ok(player) = api::visionos_player(video_id, config, sts).await
        && player.streaming_data.best_playable_format().is_some()
    {
        return Ok((player, url::VISIONOS_UA));
    }

    let player = api::mweb_player(video_id, config, sts).await?;
    if player.streaming_data.best_playable_format().is_some() {
        Ok((player, url::MWEB_UA))
    } else {
        Err(Error::AudioNotFound)
    }
}
