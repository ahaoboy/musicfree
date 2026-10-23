#![forbid(unsafe_code)]

pub mod error;
pub mod http;
pub mod model;
pub mod sites;

#[cfg(feature = "cli")]
pub mod cli;

pub use error::{Error, Result};
pub use http::load_cookies_from_file;
pub use model::{AudioFormat, Playlist, Site, Track, cover_file_name, safe_title};
pub use sites::{Extractor, download, download_cover, extract, extractor};

#[cfg(feature = "bilibili")]
pub use sites::bilibili::Bilibili;
#[cfg(feature = "file")]
pub use sites::file::File;
#[cfg(feature = "youtube")]
pub use sites::youtube::Youtube;
