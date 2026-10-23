#[tokio::main]
async fn main() {
    if let Err(error) = musicfree::cli::run().await {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
