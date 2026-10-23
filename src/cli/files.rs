use super::YoutubeMode;
#[cfg(feature = "youtube")]
use crate::{Site, sites::youtube};
use crate::{Track, safe_title, sites};
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
) -> Result<()> {
    let path = directory(dir).join(audio_name(track, output));
    if path.exists() {
        println!("⏭ File already exists, skipping: {}", path.display());
        return Ok(());
    }

    #[cfg(not(feature = "youtube"))]
    let _ = mode;

    #[cfg(feature = "youtube")]
    let data = if matches!(track.site, Site::Youtube) {
        youtube::download(&track.url, mode.into()).await
    } else {
        sites::extractor(track.site).download(&track.url).await
    }
    .with_context(|| format!("download {}", track.title))?;

    #[cfg(not(feature = "youtube"))]
    let data = sites::extractor(track.site)
        .download(&track.url)
        .await
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

    let data = sites::extractor(track.site)
        .download_cover(cover)
        .await
        .with_context(|| format!("download cover for {}", track.title))?;
    write(&path, data)?;
    println!("✓ Cover saved to: {}", path.display());
    Ok(())
}

fn directory(dir: Option<&Path>) -> &Path {
    dir.unwrap_or_else(|| Path::new("."))
}

fn audio_name(track: &Track, output: Option<&Path>) -> PathBuf {
    match output.and_then(Path::file_stem) {
        Some(stem) => PathBuf::from(format!(
            "{}.{}",
            safe_title(&stem.to_string_lossy()),
            track.extension()
        )),
        None => PathBuf::from(track.file_name()),
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
