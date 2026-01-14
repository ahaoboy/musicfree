use std::env;
use std::fs;
use std::path::Path;

use musicfree::{bilibili, detect_site, sanitize_filename, youtube, Site};

fn print_usage() {
    eprintln!("Usage: musicfree <url>");
    eprintln!();
    eprintln!("Supported sites:");
    eprintln!("  - Bilibili (bilibili.com/video/BVxxx or BVxxx)");
    eprintln!("  - YouTube  (youtube.com/watch?v=xxx or youtu.be/xxx)");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let url = &args[1];

    if url == "-h" || url == "--help" {
        print_usage();
        return;
    }

    println!("Downloading audio from: {}", url);

    let result = match detect_site(url) {
        Ok(Site::Bilibili) => download_bilibili(url).await,
        Ok(Site::YouTube) => download_youtube(url).await,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    match result {
        Ok(path) => println!("Saved to: {}", path),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn download_bilibili(url: &str) -> Result<String, musicfree::error::MusicFreeError> {
    println!("Detected: Bilibili");

    let info = bilibili::download_audio(url).await?;
    let filename = format!("{}.m4a", sanitize_filename(&info.title));
    let path = Path::new(".").join(&filename);

    fs::write(&path, &info.data)?;
    Ok(filename)
}

async fn download_youtube(url: &str) -> Result<String, musicfree::error::MusicFreeError> {
    println!("Detected: YouTube");

    let info = youtube::download_audio(url).await?;
    let filename = format!("{}.m4a", sanitize_filename(&info.title));
    let path = Path::new(".").join(&filename);

    fs::write(&path, &info.data)?;
    Ok(filename)
}
