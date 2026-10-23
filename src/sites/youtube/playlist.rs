use crate::{Error, Result};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug)]
pub struct PlaylistInfo {
    pub title: Option<String>,
    pub videos: Vec<VideoInfo>,
}

#[derive(Debug)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub duration: Option<u64>,
}

pub fn parse(html: &str) -> Result<PlaylistInfo> {
    let value: Value = serde_json::from_str(extract_json(html, "ytInitialData")?)?;
    let mut videos = Vec::new();
    let mut seen = HashSet::new();
    collect_videos(&value, &mut videos, &mut seen);

    if videos.is_empty() {
        return Err(Error::AudioNotFound);
    }

    Ok(PlaylistInfo {
        title: playlist_title(&value),
        videos,
    })
}

fn collect_videos(value: &Value, videos: &mut Vec<VideoInfo>, seen: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(renderer) = map.get("playlistVideoRenderer") {
                push_video(renderer, false, videos, seen);
            }
            if let Some(renderer) = map.get("playlistPanelVideoRenderer") {
                push_video(renderer, true, videos, seen);
            }
            if let Some(lockup) = map.get("lockupViewModel") {
                push_lockup(lockup, videos, seen);
            }
            for child in map.values() {
                collect_videos(child, videos, seen);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_videos(item, videos, seen);
            }
        }
        _ => {}
    }
}

fn push_video(value: &Value, panel: bool, videos: &mut Vec<VideoInfo>, seen: &mut HashSet<String>) {
    let id = if panel {
        value.pointer("/navigationEndpoint/watchEndpoint/videoId")
    } else {
        value.get("videoId")
    }
    .and_then(Value::as_str);
    let Some(id) = id else {
        return;
    };
    if !valid_video_id(id) || !seen.insert(id.to_owned()) {
        return;
    }

    let Some(title) = value.get("title").and_then(text) else {
        return;
    };
    videos.push(VideoInfo {
        id: id.to_owned(),
        title,
        duration: value
            .get("lengthSeconds")
            .and_then(Value::as_str)
            .and_then(|value| value.parse().ok())
            .or_else(|| value.get("lengthText").and_then(clock_duration)),
    });
}

fn push_lockup(value: &Value, videos: &mut Vec<VideoInfo>, seen: &mut HashSet<String>) {
    let Some(id) = value.get("contentId").and_then(Value::as_str) else {
        return;
    };
    if !valid_video_id(id) || !seen.insert(id.to_owned()) {
        return;
    }
    let Some(title) = value
        .pointer("/metadata/lockupMetadataViewModel/title")
        .and_then(text)
    else {
        return;
    };
    videos.push(VideoInfo {
        id: id.to_owned(),
        title,
        duration: badge_duration(value),
    });
}

fn valid_video_id(id: &str) -> bool {
    id.len() == 11
        && id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

fn badge_duration(value: &Value) -> Option<u64> {
    match value {
        Value::Object(map) => {
            if let Some(badge) = map.get("thumbnailBadgeViewModel")
                && let Some(duration) = badge
                    .get("text")
                    .and_then(text)
                    .and_then(|text| parse_clock(&text))
            {
                return Some(duration);
            }
            map.values().find_map(badge_duration)
        }
        Value::Array(items) => items.iter().find_map(badge_duration),
        _ => None,
    }
}

fn playlist_title(value: &Value) -> Option<String> {
    value
        .pointer("/header/playlistHeaderRenderer/title")
        .and_then(text)
        .or_else(|| {
            value
                .pointer("/header/pageHeaderRenderer/pageTitle")
                .and_then(text)
        })
        .or_else(|| {
            value
                .pointer("/metadata/playlistMetadataRenderer/title")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            value
                .pointer("/contents/twoColumnWatchNextResults/playlist/playlist/title")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
}

fn text(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_owned());
    }
    for key in ["simpleText", "content"] {
        if let Some(text) = value.get(key).and_then(Value::as_str) {
            return Some(text.to_owned());
        }
    }
    let runs = value.get("runs")?.as_array()?;
    let text = runs
        .iter()
        .filter_map(|run| run.get("text").and_then(Value::as_str))
        .collect::<String>();
    (!text.is_empty()).then_some(text)
}

fn clock_duration(value: &Value) -> Option<u64> {
    text(value).and_then(|text| parse_clock(&text))
}

fn parse_clock(text: &str) -> Option<u64> {
    let parts = text
        .split(':')
        .map(|part| part.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;
    match parts.as_slice() {
        [seconds] => Some(*seconds),
        [minutes, seconds] => Some(minutes * 60 + seconds),
        [hours, minutes, seconds] => Some(hours * 3600 + minutes * 60 + seconds),
        _ => None,
    }
}

fn extract_json<'a>(html: &'a str, marker: &str) -> Result<&'a str> {
    let mut offset = 0;
    while let Some(found) = html[offset..].find(marker) {
        let start = offset + found + marker.len();
        let Some(relative) = html[start..].find('{') else {
            break;
        };
        let start = start + relative;
        if let Some(end) = balanced_json_end(html, start) {
            return Ok(&html[start..end]);
        }
        offset = start + 1;
    }
    Err(Error::Parse(format!("missing {marker} JSON")))
}

fn balanced_json_end(text: &str, start: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, character) in text[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(start + offset + character.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_lockup_videos() {
        let html = r#"<script>var ytInitialData = {
            "metadata": {"playlistMetadataRenderer": {"title": "Mix"}},
            "contents": [{"lockupViewModel": {
                "contentId": "abcdefghijk",
                "metadata": {"lockupMetadataViewModel": {"title": {"content": "A Song"}}},
                "contentImage": {"thumbnailViewModel": {"overlays": [
                    {"thumbnailBottomOverlayViewModel": {"badges": [
                        {"thumbnailBadgeViewModel": {"text": "3:51"}}
                    ]}}
                ]}}
            }}]
        };</script>"#;
        let playlist = parse(html).unwrap();
        assert_eq!(playlist.title.as_deref(), Some("Mix"));
        assert_eq!(playlist.videos.len(), 1);
        assert_eq!(playlist.videos[0].title, "A Song");
        assert_eq!(playlist.videos[0].duration, Some(231));
    }

    #[test]
    fn collects_playlist_videos_from_mixed_sections() {
        let html = r#"<script>var ytInitialData = {
            "header": {"pageHeaderRenderer": {"pageTitle": "Mix"}},
            "contents": {"sectionListRenderer": {"contents": [
                {"itemSectionRenderer": {"contents": [
                    {"playlistVideoListRenderer": {"contents": [
                        {"playlistVideoRenderer": {
                            "videoId": "abcdefghijk",
                            "lengthSeconds": "212",
                            "title": {"runs": [{"text": "A "}, {"text": "Song"}]}
                        }}
                    ]}}
                ]}},
                {"continuationItemRenderer": {}}
            ]}}
        };</script>"#;
        let playlist = parse(html).unwrap();
        assert_eq!(playlist.title.as_deref(), Some("Mix"));
        assert_eq!(playlist.videos.len(), 1);
        assert_eq!(playlist.videos[0].id, "abcdefghijk");
        assert_eq!(playlist.videos[0].title, "A Song");
        assert_eq!(playlist.videos[0].duration, Some(212));
    }
}
