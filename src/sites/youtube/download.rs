use super::{api, ejs, url};
use crate::{Error, Result, http};
use ::url::Url;
use serde_youtube::types::Format;

/// Fetch the player `base.js` referenced by a watch page.
pub async fn player_js(html: &str) -> Result<String> {
    let url = api::player_js_url(html).ok_or(Error::PlayerJsNotFound)?;
    let headers = http::media_headers(url::WEB_UA, "https://www.youtube.com/")?;
    http::get_text(&url, headers).await
}

/// Resolve a format's direct URL, applying the `n`/signature challenge when
/// necessary.
pub async fn resolve_url(format: &Format, html: &str, cached_js: Option<String>) -> Result<String> {
    if let Some(raw_url) = &format.url {
        if needs_n(raw_url) {
            let js = match cached_js {
                Some(js) => js,
                None => player_js(html).await?,
            };
            return ejs::solve_n(raw_url, &js);
        }
        return Ok(raw_url.clone());
    }

    if let Some(cipher) = &format.signature_cipher {
        let js = match cached_js {
            Some(js) => js,
            None => player_js(html).await?,
        };
        return ejs::solve_cipher(cipher, &js);
    }

    Err(Error::AudioNotFound)
}

fn needs_n(raw_url: &str) -> bool {
    Url::parse(raw_url)
        .ok()
        .is_some_and(|url| url.query_pairs().any(|(key, _)| key == "n"))
}
