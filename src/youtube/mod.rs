mod common;
mod android;
#[cfg(feature = "ytdlp-ejs")]
mod ejs;

pub use common::{extract_video_id, is_youtube_url, AudioFormat, AudioInfo};

use crate::error::Result;

/// Download audio from YouTube
///
/// - 默认使用 Android 实现
/// - 如果启用了 `ejs` feature，则优先尝试 EJS 的实现，失败时回退到 Android
pub async fn download_audio(url: &str) -> Result<AudioInfo> {
    let video_id = extract_video_id(url)?;

    #[cfg(feature = "ytdlp-ejs")]
    {
        match ejs::download_audio_ejs(&video_id).await {
            Ok(info) => return Ok(info),
            Err(e) => {
                eprintln!("Web(EJS) client failed: {e}, falling back to Android client...");
            }
        }
    }

    android::download_audio_android(&video_id).await
}

/// Get available audio formats without downloading
///
/// 使用 Android API 实现，与是否启用 `ejs` 无关
pub async fn get_audio_formats(url: &str) -> Result<(String, Vec<AudioFormat>)> {
    let video_id = extract_video_id(url)?;
    android::get_audio_formats_android(&video_id).await
}


