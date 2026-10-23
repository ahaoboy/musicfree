use crate::{
    Site, Track,
    sites::{self, youtube},
};
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub async fn download(
    track: &Track,
    dir: Option<&Path>,
    output: Option<&Path>,
    mode: youtube::Mode,
) -> Result<()> {
    let path = directory(dir).join(audio_name(track, output));
    if path.exists() {
        println!("⏭ File already exists, skipping: {}", path.display());
        return Ok(());
    }

    let data = match track.site {
        Site::Youtube => youtube::download(&track.url, mode).await,
        _ => sites::extractor(track.site).download(&track.url).await,
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
    let mut path = PathBuf::from(stem(track, output));
    path.set_extension(track.extension());
    path
}

fn cover_name(track: &Track, output: Option<&Path>) -> PathBuf {
    let stem = match output.and_then(Path::file_stem) {
        Some(stem) => sanitize(&stem.to_string_lossy()),
        None => sanitize(&format!("{}_{}", track.title, track.id)),
    };
    let stem = if stem.is_empty() {
        "cover".into()
    } else {
        stem
    };
    PathBuf::from(format!("{stem}.jpg"))
}

fn stem(track: &Track, output: Option<&Path>) -> String {
    output
        .and_then(Path::file_stem)
        .map(|stem| sanitize(&stem.to_string_lossy()))
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| {
            let title = sanitize(&track.title);
            if title.is_empty() {
                "audio".into()
            } else {
                title
            }
        })
}

fn sanitize(value: &str) -> String {
    sanitize_filename::sanitize(value)
}

fn write(path: &Path, data: Vec<u8>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create directory {}", parent.display()))?;
    }
    fs::write(path, data).with_context(|| format!("write {}", path.display()))
}
