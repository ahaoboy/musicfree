use super::YoutubeMode;
use crate::{AudioFormat, MediaFormat, Track, safe_title};
#[cfg(feature = "youtube")]
use crate::{Site, sites::youtube};
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub async fn download(
    track: &Track,
    dir: Option<&Path>,
    output: Option<&Path>,
    mode: YoutubeMode,
    selected: Option<&MediaFormat>,
) -> Result<()> {
    let format = selected
        .map(|format| format.format)
        .unwrap_or_else(|| track.format.unwrap_or(AudioFormat::Mp3));
    let path = directory(dir).join(audio_name(
        track,
        output,
        format,
        selected.map(|format| format.id.as_str()),
    ));
    if path.exists() {
        println!("⏭ File already exists, skipping: {}", path.display());
        return Ok(());
    }

    #[cfg(not(feature = "youtube"))]
    let _ = mode;

    #[cfg(feature = "youtube")]
    let data = if let Some(format) = selected {
        if matches!(track.site, Site::Youtube) {
            youtube::download_format(&track.url, mode.into(), &format.id).await
        } else {
            track.download_format(&format.id).await
        }
    } else if matches!(track.site, Site::Youtube) {
        youtube::download(&track.url, mode.into()).await
    } else {
        track.download().await
    }
    .with_context(|| format!("download {}", track.title))?;

    #[cfg(not(feature = "youtube"))]
    let data = if let Some(format) = selected {
        track.download_format(&format.id).await
    } else {
        track.download().await
    }
    .with_context(|| format!("download {}", track.title))?;

    write(&path, data)?;
    println!("✓ Saved to: {}", path.display());
    Ok(())
}

pub async fn download_cover(
    track: &Track,
    dir: Option<&Path>,
    output: Option<&Path>,
) -> Result<()> {
    let Some(cover) = &track.cover else {
        println!("✗ No cover available for this audio");
        return Ok(());
    };
    let path = directory(dir).join(cover_name(track, output));
    if path.exists() {
        println!("⏭ Cover file already exists, skipping: {}", path.display());
        return Ok(());
    }

    let data = crate::download_cover(track.site, cover)
        .await
        .with_context(|| format!("download cover for {}", track.title))?;
    write(&path, data)?;
    println!("✓ Cover saved to: {}", path.display());
    Ok(())
}

fn directory(dir: Option<&Path>) -> &Path {
    dir.unwrap_or_else(|| Path::new("."))
}

fn audio_name(
    track: &Track,
    output: Option<&Path>,
    format: AudioFormat,
    selected_id: Option<&str>,
) -> PathBuf {
    match output.and_then(Path::file_stem) {
        Some(stem) => PathBuf::from(format!(
            "{}.{}",
            safe_title(&stem.to_string_lossy()),
            format.extension()
        )),
        None => {
            let title = safe_title(&track.title);
            let stem = match selected_id {
                Some(id) => format!("{title} [{}]", safe_title(id)),
                None => title,
            };
            let mut name = PathBuf::from(stem);
            name.set_extension(format.extension());
            name
        }
    }
}

fn cover_name(track: &Track, output: Option<&Path>) -> PathBuf {
    match output.and_then(Path::file_stem) {
        Some(stem) => PathBuf::from(format!("{}.jpg", safe_title(&stem.to_string_lossy()))),
        None => PathBuf::from(
            track
                .cover_file_name()
                .unwrap_or_else(|| "cover.jpg".into()),
        ),
    }
}

fn write(path: &Path, data: Vec<u8>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create directory {}", parent.display()))?;
    }
    fs::write(path, data).with_context(|| format!("write {}", path.display()))
}
