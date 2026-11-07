//! URL helpers: normalisation, type detection, identifiers.

use regex::Regex;
use std::sync::LazyLock;
use url::Url;

use crate::config::FALLBACK_DOMAIN;

static VALID_SLUG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());
static MEDIA_SLUG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"const\s+slug\s*=\s*"([a-zA-Z0-9_-]+)""#).unwrap());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlKind {
    Album,
    Media,
}

/// Force https (unless the user explicitly typed http://) and drop the `page` query param.
pub fn normalize(raw: &str) -> String {
    let raw = raw.trim();
    let with_scheme = if raw.starts_with("https://") || raw.starts_with("http://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };
    match Url::parse(&with_scheme) {
        Ok(mut u) => {
            let kept: Vec<(String, String)> = u
                .query_pairs()
                .filter(|(k, _)| k != "page")
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect();
            if kept.is_empty() {
                u.set_query(None);
            } else {
                u.query_pairs_mut().clear().extend_pairs(kept);
            }
            u.set_fragment(None);
            u.to_string()
        }
        Err(_) => with_scheme,
    }
}

/// `scheme://host[:port]` of a URL.
pub fn host_page(url: &str) -> String {
    match Url::parse(url) {
        Ok(u) => {
            let host = u.host_str().unwrap_or_default();
            match u.port() {
                Some(p) => format!("{}://{host}:{p}", u.scheme()),
                None => format!("{}://{host}", u.scheme()),
            }
        }
        Err(_) => String::new(),
    }
}

pub fn replace_domain_with_fallback(url: &str) -> String {
    match Url::parse(url) {
        Ok(mut u) => {
            let _ = u.set_host(Some(FALLBACK_DOMAIN));
            u.to_string()
        }
        Err(_) => url.to_string(),
    }
}

fn path_segments(url: &str) -> Vec<&str> {
    let no_q = url.split(['?', '#']).next().unwrap_or(url);
    no_q.trim_end_matches('/').split('/').collect()
}

pub fn kind(url: &str) -> Result<UrlKind, String> {
    let segs = path_segments(url);
    let t = segs.len().checked_sub(2).map(|i| segs[i]).unwrap_or("");
    match t {
        "a" => Ok(UrlKind::Album),
        "f" | "i" | "v" => Ok(UrlKind::Media),
        other => Err(format!("Invalid URL format for {url}: unexpected URL type '{other}'")),
    }
}

pub fn last_segment(url: &str) -> String {
    path_segments(url).last().copied().unwrap_or("").to_string()
}

/// Media slug from the URL, or from an inline `const slug = "..."` script as a fallback.
pub fn media_slug(url: &str, html: &str) -> String {
    let slug = last_segment(url);
    if VALID_SLUG.is_match(&slug) {
        return slug;
    }
    MEDIA_SLUG.captures(html).map(|c| c[1].to_string()).unwrap_or(slug)
}

/// Filename part of a (signed) download link: last path segment, no query.
pub fn url_based_filename(link: &str) -> String {
    match Url::parse(link) {
        Ok(u) => u.path().trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string(),
        Err(_) => last_segment(link),
    }
}

/// First DNS label of the download host (used to track offline CDN nodes).
pub fn subdomain(link: &str) -> String {
    Url::parse(link)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.split('.').next().unwrap_or(h).to_string()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes() {
        assert_eq!(normalize("bunkr.si/a/PUK068QE?page=3"), "https://bunkr.si/a/PUK068QE");
        assert_eq!(normalize("https://bunkr.si/a/X/"), "https://bunkr.si/a/X/");
    }

    #[test]
    fn kinds() {
        assert_eq!(kind("https://bunkr.si/a/PUK068QE"), Ok(UrlKind::Album));
        assert_eq!(kind("https://bunkr.fi/f/gBrv5f8tAGlGW"), Ok(UrlKind::Media));
        assert!(kind("https://bunkr.fi/x/abc").is_err());
    }

    #[test]
    fn link_name() {
        assert_eq!(url_based_filename("https://c.bunkr.ru/a/b-xyz.mp4?token=1&ex=2"), "b-xyz.mp4");
        assert_eq!(subdomain("https://c12.bunkr.ru/x"), "c12");
    }
}
