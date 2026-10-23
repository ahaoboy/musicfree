#![forbid(unsafe_code)]

pub mod error;
pub mod http;
pub mod model;
pub mod sites;

#[cfg(feature = "cli")]
pub mod cli;

pub use error::{Error, Result};
pub use http::load_cookies_from_file;
pub use model::{AudioFormat, Playlist, Site, Track};
pub use sites::{Extractor, bilibili::Bilibili, extract, extractor, file::File, youtube::Youtube};
