//! HTTP clients, page fetching and the Bunkr signing/download API.

use anyhow::Result;
use rand::Rng;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;

use crate::config::*;
use crate::fsutil::session_log;
use crate::parse;
use crate::urlutil;

#[derive(Clone)]
pub struct Net {
    /// Pages + JSON APIs (HTTP/2, connection pooling, gzip).
    pub web: Client,
    /// File data: HTTP/1.1 only so every ranged request gets its *own* TCP connection
    /// (HTTP/2 would multiplex them over one socket and defeat parallel chunking),
    /// and no transparent decompression so byte ranges stay exact.
    pub dl: Client,
}

impl Net {
    pub fn new() -> Result<Self> {
        let web = Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(60))
            .tcp_nodelay(true)
            .build()?;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::REFERER, DOWNLOAD_REFERER.parse()?);
        let dl = Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .http1_only()
            .no_gzip()
            .no_deflate()
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_nodelay(true)
            .build()?;
        Ok(Self { web, dl })
    }
}

fn backoff(attempt: u32) -> Duration {
    let base = 0.75 * 2f64.powi(attempt as i32);
    Duration::from_secs_f64(base + rand::rng().random_range(0.0..1.0))
}

/// Fetch a page's HTML, retrying transient failures. On a 403 it retries once on the
/// fallback domain. Returns None when the page is unavailable.
pub async fn fetch_page(net: &Net, url: &str) -> Option<String> {
    let mut url = url.to_string();
    let mut tried_fallback = false;
    for attempt in 0..5u32 {
        match net.web.get(&url).send().await {
            Ok(resp) => {
                let st = resp.status();
                if st == StatusCode::FORBIDDEN {
                    if !tried_fallback {
                        tried_fallback = true;
                        url = urlutil::replace_domain_with_fallback(&url);
                        continue;
                    }
                    session_log(&format!("DDoSGuard blocked the request to {url}"));
                    return None;
                }
                if st == StatusCode::INTERNAL_SERVER_ERROR || st == StatusCode::BAD_GATEWAY {
                    session_log(&format!("Server error {st} when fetching {url}"));
                    return None;
                }
                if st == StatusCode::TOO_MANY_REQUESTS || st == StatusCode::SERVICE_UNAVAILABLE {
                    tokio::time::sleep(backoff(attempt)).await;
                    continue;
                }
                if !st.is_success() {
                    return None;
                }
                match resp.text().await {
                    Ok(t) => return Some(t),
                    Err(_) => tokio::time::sleep(backoff(attempt)).await,
                }
            }
            Err(_) => tokio::time::sleep(backoff(attempt)).await,
        }
    }
    None
}

async fn request_json(net: &Net, post: bool, url: &str, body: Option<Value>, query: Option<(&str, &str)>) -> Option<Value> {
    const RETRIES: u32 = 3;
    for attempt in 1..=RETRIES {
        let mut rb = if post { net.web.post(url) } else { net.web.get(url) };
        rb = rb.timeout(Duration::from_secs(10));
        if let Some(b) = &body {
            rb = rb.json(b);
        }
        if let Some(q) = query {
            rb = rb.query(&[q]);
        }
        let res: Result<Value, reqwest::Error> = async { rb.send().await?.error_for_status()?.json::<Value>().await }.await;
        match res {
            Ok(v) => return Some(v),
            Err(_) if attempt < RETRIES => {
                tokio::time::sleep(Duration::from_secs_f64(2.0 * 2f64.powi(attempt as i32 - 1))).await
            }
            Err(_) => {}
        }
    }
    None
}

fn json_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Unsigned download URL for assets without a CDN landing page (archives, some videos).
async fn download_response(net: &Net, file_id: &str) -> Option<String> {
    let data = request_json(net, true, &download_api(), Some(json!({ "id": file_id })), None).await?;
    let base = data.get("mediafiles").and_then(json_str)?;
    let path = data.get("path").and_then(json_str)?;
    let mut u = Url::parse(&base).ok()?;
    u.set_path(&path);
    Some(u.to_string())
}

/// Resolve and sign the direct download URL of an item page.
pub async fn signed_link(net: &Net, item_url: &str, html: &str) -> Option<String> {
    let vars = parse::page_vars(html);
    let cdn_url = vars.get("jsCDN").filter(|s| !s.is_empty()).cloned();
    let file_id = if vars.is_empty() { parse::file_id(html) } else { None };
    let unsigned = match file_id {
        Some(id) => download_response(net, &id).await,
        None => None,
    };
    if cdn_url.is_none() && unsigned.is_none() {
        return None;
    }

    let slug_src = unsigned.as_deref().unwrap_or(item_url);
    let media_slug = urlutil::url_based_filename(slug_src);
    let media_path = match &cdn_url {
        Some(c) => Url::parse(c).map(|u| u.path().to_string()).unwrap_or_default(),
        None => format!("/storage/media/{media_slug}"),
    };

    let data = request_json(net, false, &sign_api(), None, Some(("path", &media_path))).await?;
    let token = data.get("token").and_then(json_str);
    let expires = data.get("ex").and_then(json_str);
    let base = cdn_url.clone().or_else(|| unsigned.clone());
    match (token, expires, base) {
        (Some(t), Some(e), Some(b)) => Some(format!("{b}?token={t}&ex={e}")),
        _ => cdn_url.or(unsigned), // API gave no token: fall back to the plain URL
    }
}

/// Everything needed to download one item, resolved from its page.
pub struct ItemMeta {
    pub page_filename: String,
    pub date: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn item_meta(item_url: &str, html: &str) -> ItemMeta {
    ItemMeta {
        page_filename: parse::item_filename(html).unwrap_or_else(|| urlutil::last_segment(item_url)),
        date: parse::item_date(html),
    }
}

/// Combine the page filename with the link-derived one while keeping the page extension.
pub fn format_item_filename(original: &str, url_based: &str) -> String {
    use crate::fsutil::{remove_invalid_characters, split_ext};
    if url_based.is_empty() || original == url_based {
        return original.to_string();
    }
    let (o_stem, o_ext) = split_ext(original);
    let (u_stem, _) = split_ext(url_based);
    let prefixed = u_stem.starts_with(&format!("{o_stem}-")) || u_stem.starts_with(&format!("{o_stem}_"));
    if u_stem == o_stem || prefixed {
        return url_based.to_string();
    }
    format!("{}-{u_stem}{o_ext}", remove_invalid_characters(o_stem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_merge() {
        assert_eq!(format_item_filename("a.mp4", "a.mp4"), "a.mp4");
        assert_eq!(format_item_filename("a.mp4", "a-xYz.mp4"), "a-xYz.mp4");
        assert_eq!(format_item_filename("clip.mp4", "Zq81.mp4"), "clip-Zq81.mp4");
    }
}
