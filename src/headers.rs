//! Shared download header builders.
//!
//! Provides a `download_headers()` base function plus `with_*()` modifiers
//! that build browser-like HTTP headers for binary download requests.
//! Used by YouTube (android/web) and Bilibili download modules.

use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

/// Build a base set of browser-like headers for binary download requests.
/// Mimics a real browser: UA, Accept, Accept-Language, Connection, Referer.
pub fn download_headers(ua: &'static str, referer: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(USER_AGENT, HeaderValue::from_static(ua));
    h.insert(reqwest::header::ACCEPT, HeaderValue::from_static("*/*"));
    h.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        HeaderValue::from_static("en-US,en;q=0.9"),
    );
    h.insert(
        reqwest::header::CONNECTION,
        HeaderValue::from_static("keep-alive"),
    );
    h.insert(
        reqwest::header::REFERER,
        HeaderValue::from_str(referer).expect("invalid referer URL"),
    );
    h
}

/// Add `Origin` header for cross-origin requests.
pub fn with_origin(h: &mut HeaderMap, origin: &'static str) {
    h.insert(reqwest::header::ORIGIN, HeaderValue::from_static(origin));
}

/// Add `Accept-Encoding` header (e.g. "gzip, deflate, br").
pub fn with_accept_encoding(h: &mut HeaderMap, enc: &'static str) {
    h.insert(
        reqwest::header::ACCEPT_ENCODING,
        HeaderValue::from_static(enc),
    );
}

/// Add `Accept-Language` header, overwriting the default.
pub fn with_accept_language(h: &mut HeaderMap, lang: &'static str) {
    h.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        HeaderValue::from_static(lang),
    );
}

/// Add `sec-fetch-*` headers for cross-origin media requests (YouTube style).
pub fn with_sec_fetch(h: &mut HeaderMap) {
    h.insert(
        reqwest::header::HeaderName::from_static("sec-fetch-mode"),
        HeaderValue::from_static("no-cors"),
    );
    h.insert(
        reqwest::header::HeaderName::from_static("sec-fetch-site"),
        HeaderValue::from_static("cross-site"),
    );
}
