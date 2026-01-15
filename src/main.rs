use std::env;
use std::fs;
use std::path::Path;

use musicfree::extract;

fn print_usage() {
    eprintln!("Usage: musicfree <url>");
    eprintln!();
    eprintln!("Supported sites:");
    eprintln!("  - Bilibili (bilibili.com/video/BVxxx or BVxxx)");
    eprintln!("  - YouTube  (youtube.com/watch?v=xxx or youtu.be/xxx)");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  musicfree https://www.bilibili.com/video/BV1234567890");
    eprintln!("  musicfree https://www.youtube.com/watch?v=dQw4w9WgXcQ");
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

    println!("Extracting audio from: {}", url);

    // Use the new trait-based extraction system
    match extract(url).await {
        Ok(audios) => {
            let audios_len = audios.len();
            for (index, audio) in audios.into_iter().enumerate() {
                println!("Found audio: {}", audio.title);
                println!("Platform: {:?}", audio.platform);

                if let Some(duration) = audio.duration {
                    println!("Duration: {} seconds", duration);
                }

                // Save audio to file
                if let Some(binary_data) = audio.binary {
                    let filename = sanitize_filename::sanitize(&audio.title)
                        + audio
                            .format
                            .unwrap_or(musicfree::core::AudioFormat::Mp3)
                            .extension();
                    let path = Path::new(".").join(&filename);

                    match fs::write(&path, binary_data) {
                        Ok(_) => println!("Saved to: {}", path.display()),
                        Err(e) => {
                            eprintln!("Error saving file: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    println!("No binary data available for download");
                }

                if index < audios_len - 1 {
                    println!("---");
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
