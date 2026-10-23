use super::{Mode, api, url};
use crate::{AudioFormat, Error, MediaFormat, MediaKind, Result};
use serde_json::Value;
use serde_youtube::types::{Format, PlayerResponse};
use std::collections::HashSet;

/// List all directly downloadable YouTube formats.
pub async fn list(input: &str) -> Result<Vec<MediaFormat>> {
    let video_id = url::video_id(input)?;
    let html = super::fetch_page(&video_id).await?;
    let config = api::config(&html)?;
    let sts = api::sts(&html);
    let mut formats = Vec::new();

    if let Ok(player) = api::visionos_player_json(&video_id, &config, sts).await {
        formats.extend(parse_player(&player));
    }
    if let Ok(player) = api::android_player_json(&video_id, &config, sts).await {
        formats.extend(parse_player(&player));
    }

    let mut seen = HashSet::new();
    formats.retain(|format| seen.insert(format.id.clone()));
    crate::sort_formats(&mut formats);

    if formats.is_empty() {
        return Err(Error::AudioNotFound);
    }
    Ok(formats)
}

/// Find a specific format and return the player JS HTML needed to resolve its
/// URL, together with the matching download user agent.
pub(crate) async fn find_format(
    input: &str,
    format_id: &str,
    mode: Mode,
) -> Result<(Format, &'static str, String)> {
    let video_id = url::video_id(input)?;
    let html = super::fetch_page(&video_id).await?;
    let config = api::config(&html)?;
    let sts = api::sts(&html);

    if mode != Mode::Mobile
        && let Ok(player) = api::player_from_html(&html)
        && let Some(format) = find_in(&player, format_id)
    {
        return Ok((format, url::WEB_UA, html));
    }

    if mode == Mode::Mobile {
        if let Some(format) = find_android(&video_id, &config, sts, format_id).await {
            return Ok((format, url::ANDROID_UA, html));
        }
        if let Some(format) = find_visionos(&video_id, &config, sts, format_id).await {
            return Ok((format, url::VISIONOS_UA, html));
        }
        if let Some(format) = find_web(&video_id, &config, sts, format_id).await {
            return Ok((format, url::WEB_UA, html));
        }
        if let Some(format) = find_mweb(&video_id, &config, sts, format_id).await {
            return Ok((format, url::MWEB_UA, html));
        }
    } else if mode == Mode::Web {
        if let Some(format) = find_visionos(&video_id, &config, sts, format_id).await {
            return Ok((format, url::VISIONOS_UA, html));
        }
        if let Some(format) = find_web(&video_id, &config, sts, format_id).await {
            return Ok((format, url::WEB_UA, html));
        }
        if let Some(format) = find_mweb(&video_id, &config, sts, format_id).await {
            return Ok((format, url::MWEB_UA, html));
        }
        if let Some(format) = find_android(&video_id, &config, sts, format_id).await {
            return Ok((format, url::ANDROID_UA, html));
        }
    } else {
        if let Some(format) = find_android(&video_id, &config, sts, format_id).await {
            return Ok((format, url::ANDROID_UA, html));
        }
        if let Some(format) = find_visionos(&video_id, &config, sts, format_id).await {
            return Ok((format, url::VISIONOS_UA, html));
        }
        if let Some(format) = find_web(&video_id, &config, sts, format_id).await {
            return Ok((format, url::WEB_UA, html));
        }
        if let Some(format) = find_mweb(&video_id, &config, sts, format_id).await {
            return Ok((format, url::MWEB_UA, html));
        }
    }

    Err(Error::Unsupported(format!(
        "YouTube format not found: {format_id}"
    )))
}

async fn find_android(
    video_id: &str,
    config: &serde_youtube::types::YtConfig,
    sts: Option<u64>,
    format_id: &str,
) -> Option<Format> {
    api::android_player(video_id, config, sts)
        .await
        .ok()
        .and_then(|player| find_in(&player, format_id))
}

async fn find_visionos(
    video_id: &str,
    config: &serde_youtube::types::YtConfig,
    sts: Option<u64>,
    format_id: &str,
) -> Option<Format> {
    api::visionos_player(video_id, config, sts)
        .await
        .ok()
        .and_then(|player| find_in(&player, format_id))
}

async fn find_web(
    video_id: &str,
    config: &serde_youtube::types::YtConfig,
    sts: Option<u64>,
    format_id: &str,
) -> Option<Format> {
    api::web_player(video_id, config, sts)
        .await
        .ok()
        .and_then(|player| find_in(&player, format_id))
}

async fn find_mweb(
    video_id: &str,
    config: &serde_youtube::types::YtConfig,
    sts: Option<u64>,
    format_id: &str,
) -> Option<Format> {
    api::mweb_player(video_id, config, sts)
        .await
        .ok()
        .and_then(|player| find_in(&player, format_id))
}

fn find_in(player: &PlayerResponse, format_id: &str) -> Option<Format> {
    player
        .streaming_data
        .formats
        .iter()
        .chain(player.streaming_data.adaptive_formats.iter())
        .find(|format| format.itag.to_string() == format_id && format.is_downloadable())
        .cloned()
}

fn parse_player(player: &Value) -> Vec<MediaFormat> {
    let duration = player
        .pointer("/videoDetails/lengthSeconds")
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok());
    let Some(streaming) = player.get("streamingData") else {
        return Vec::new();
    };

    let mut formats = Vec::new();
    for key in ["formats", "adaptiveFormats"] {
        let Some(items) = streaming.get(key).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            if item.get("url").is_none() && item.get("signatureCipher").is_none() {
                continue;
            }
            if let Some(format) = parse_format(item, duration) {
                formats.push(format);
            }
        }
    }
    formats
}

fn parse_format(item: &Value, fallback_duration: Option<u64>) -> Option<MediaFormat> {
    let id = number(item.get("itag")?)?.to_string();
    let mime_type = item.get("mimeType").and_then(Value::as_str)?.to_owned();
    let codec = mime_type
        .split("codecs=")
        .nth(1)
        .map(|codec| codec.trim_matches('"').to_owned());
    let is_audio = mime_type.starts_with("audio/");
    let is_video = mime_type.starts_with("video/");
    let has_audio = is_audio
        || codec.as_deref().is_some_and(|codec| {
            ["mp4a", "opus", "vorbis", "ac-3", "ec-3"]
                .iter()
                .any(|name| codec.contains(name))
        });
    let kind = if is_video {
        if has_audio {
            MediaKind::Muxed
        } else {
            MediaKind::Video
        }
    } else {
        MediaKind::Audio
    };
    let duration = item
        .get("approxDurationMs")
        .and_then(number)
        .map(|ms| ms / 1000)
        .or(fallback_duration);
    let quality = if kind == MediaKind::Audio {
        item.get("audioQuality")
            .and_then(Value::as_str)
            .map(|quality| {
                quality
                    .strip_prefix("AUDIO_QUALITY_")
                    .unwrap_or(quality)
                    .to_ascii_lowercase()
            })
    } else {
        item.get("qualityLabel")
            .and_then(Value::as_str)
            .or_else(|| item.get("quality").and_then(Value::as_str))
            .map(str::to_owned)
    };

    Some(MediaFormat {
        id,
        kind,
        format: AudioFormat::from_mime(&mime_type),
        mime_type,
        codec,
        bitrate: item.get("bitrate").and_then(number),
        size: item.get("contentLength").and_then(number),
        width: item
            .get("width")
            .and_then(number)
            .and_then(|value| u32::try_from(value).ok()),
        height: item
            .get("height")
            .and_then(number)
            .and_then(|value| u32::try_from(value).ok()),
        quality,
        duration,
    })
}

fn number(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_muxed_and_audio_formats() {
        let player = json!({
            "videoDetails": {"lengthSeconds": "120"},
            "streamingData": {
                "formats": [{
                    "itag": 18,
                    "mimeType": "video/mp4; codecs=\"avc1.42001E, mp4a.40.2\"",
                    "bitrate": 500000,
                    "width": 640,
                    "height": 360,
                    "contentLength": "1000",
                    "qualityLabel": "360p",
                    "url": "https://example.com/video"
                }],
                "adaptiveFormats": [{
                    "itag": 140,
                    "mimeType": "audio/mp4; codecs=\"mp4a.40.2\"",
                    "bitrate": 128000,
                    "audioQuality": "AUDIO_QUALITY_MEDIUM",
                    "url": "https://example.com/audio"
                }]
            }
        });
        let formats = parse_player(&player);
        assert_eq!(formats.len(), 2);
        assert_eq!(formats[0].kind, MediaKind::Muxed);
        assert_eq!(formats[0].size, Some(1000));
        assert_eq!(formats[1].kind, MediaKind::Audio);
        assert_eq!(formats[1].quality.as_deref(), Some("medium"));
        assert_eq!(formats[1].size_or_estimate(), Some(1_920_000));
    }

    #[test]
    fn skips_formats_without_direct_url() {
        let player = json!({
            "streamingData": {"adaptiveFormats": [{
                "itag": 140,
                "mimeType": "audio/mp4"
            }]}
        });
        assert!(parse_player(&player).is_empty());
    }
}
