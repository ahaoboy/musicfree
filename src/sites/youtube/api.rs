use crate::{Error, Result, http};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, ORIGIN, USER_AGENT};
use serde_json::{Value, json};
use serde_youtube::types::{PlayerResponse, YtConfig};
use std::sync::LazyLock;

use super::url;

const ANDROID_CLIENT_ID: &str = "3";
const WEB_CLIENT_ID: &str = "1";
const MWEB_CLIENT_ID: &str = "2";
const VISIONOS_CLIENT_ID: &str = "101";

pub fn config(html: &str) -> Result<YtConfig> {
    serde_youtube::parser::extract_ytcfg(html).map_err(parse_error)
}

pub fn player_from_html(html: &str) -> Result<PlayerResponse> {
    serde_youtube::parser::extract_player_response(html).map_err(parse_error)
}

pub fn player_js_url(html: &str) -> Option<String> {
    serde_youtube::parser::player_js_url(html)
}

/// Signature timestamp from the page's `ytcfg`.
pub fn sts(html: &str) -> Option<u64> {
    static STS_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#""STS"\s*:\s*(\d{5,})"#).unwrap());
    find_number(&STS_RE, html)
}

/// Signature timestamp from the player `base.js` (used when `ytcfg` lacks it).
pub fn sts_from_player_js(player_js: &str) -> Option<u64> {
    static STS_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?:signatureTimestamp|playerSignatureTimestamp)\s*:\s*(\d{5,})")
            .unwrap()
    });
    find_number(&STS_RE, player_js)
}

pub async fn android_player(
    video_id: &str,
    config: &YtConfig,
    sts: Option<u64>,
) -> Result<PlayerResponse> {
    player(
        video_id,
        config,
        Client {
            id: ANDROID_CLIENT_ID,
            version: url::ANDROID_VERSION,
            ua: url::ANDROID_UA,
            context: json!({
                "clientName": "ANDROID",
                "clientVersion": url::ANDROID_VERSION,
                "androidSdkVersion": 30,
                "userAgent": url::ANDROID_UA,
                "osName": "Android",
                "osVersion": "11",
            }),
        },
        sts,
    )
    .await
}

pub async fn web_player(
    video_id: &str,
    config: &YtConfig,
    sts: Option<u64>,
) -> Result<PlayerResponse> {
    let mut context = config.innertube_context.client.clone();
    if let Some(client) = context.as_object_mut() {
        client.entry("hl").or_insert_with(|| json!("en"));
        client.entry("gl").or_insert_with(|| json!("US"));
    }
    player(
        video_id,
        config,
        Client {
            id: WEB_CLIENT_ID,
            version: &config.innertube_client_version,
            ua: url::WEB_UA,
            context,
        },
        sts,
    )
    .await
}

pub async fn mweb_player(
    video_id: &str,
    config: &YtConfig,
    sts: Option<u64>,
) -> Result<PlayerResponse> {
    player(
        video_id,
        config,
        Client {
            id: MWEB_CLIENT_ID,
            version: url::MWEB_VERSION,
            ua: url::MWEB_UA,
            context: json!({
                "clientName": "MWEB",
                "clientVersion": url::MWEB_VERSION,
                "userAgent": url::MWEB_UA,
                "osName": "iPad",
                "osVersion": "16.7.10",
                "hl": "en",
                "gl": "US",
            }),
        },
        sts,
    )
    .await
}

pub async fn visionos_player(
    video_id: &str,
    config: &YtConfig,
    sts: Option<u64>,
) -> Result<PlayerResponse> {
    player(
        video_id,
        config,
        Client {
            id: VISIONOS_CLIENT_ID,
            version: url::VISIONOS_VERSION,
            ua: url::VISIONOS_UA,
            context: json!({
                "clientName": "VISIONOS",
                "clientVersion": url::VISIONOS_VERSION,
                "deviceMake": "Apple",
                "deviceModel": "RealityDevice17,1",
                "userAgent": url::VISIONOS_UA,
                "osName": "visionOS",
                "osVersion": "26.5.23O471",
                "hl": "en",
                "gl": "US",
            }),
        },
        sts,
    )
    .await
}

struct Client<'a> {
    id: &'static str,
    version: &'a str,
    ua: &'static str,
    context: Value,
}

async fn player(
    video_id: &str,
    config: &YtConfig,
    client: Client<'_>,
    sts: Option<u64>,
) -> Result<PlayerResponse> {
    let url = format!(
        "https://www.youtube.com/youtubei/v1/player?key={}&prettyPrint=false",
        config.innertube_api_key
    );

    let mut playback = serde_json::Map::new();
    playback.insert("html5Preference".into(), json!("HTML5_PREF_WANTS"));
    playback.insert("pcm2".into(), json!("yes"));
    if let Some(sts) = sts {
        playback.insert("signatureTimestamp".into(), json!(sts));
    }

    let body = json!({
        "videoId": video_id,
        "context": { "client": client.context },
        "playbackContext": { "contentPlaybackContext": playback },
        "contentCheckOk": true,
        "racyCheckOk": true,
    });

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static(client.ua));
    headers.insert("X-YouTube-Client-Name", HeaderValue::from_static(client.id));
    headers.insert(
        "X-YouTube-Client-Version",
        HeaderValue::from_str(client.version)?,
    );
    headers.insert(ORIGIN, HeaderValue::from_static("https://www.youtube.com"));
    if let Some(visitor) = &config.visitor_data {
        headers.insert("X-Goog-Visitor-Id", HeaderValue::from_str(visitor)?);
    }

    http::post_json(&url, &body, headers).await
}

fn find_number(regex: &regex::Regex, text: &str) -> Option<u64> {
    regex
        .captures(text)
        .and_then(|captures| captures.get(1))
        .and_then(|value| value.as_str().parse().ok())
}

fn parse_error(error: serde_youtube::Error) -> Error {
    Error::Parse(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sts_from_page() {
        let html = r#"<script>ytcfg.set({"STS":20746});</script>"#;
        assert_eq!(sts(html), Some(20746));
    }

    #[test]
    fn parses_sts_from_player_js() {
        assert_eq!(
            sts_from_player_js("var x={signatureTimestamp:20746};"),
            Some(20746)
        );
    }
}
