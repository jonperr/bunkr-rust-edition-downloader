//! HTML scraping. All functions are synchronous and return owned data, because
//! `scraper::Html` is not `Send` and must never be held across an `.await`.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;
use std::sync::LazyLock;
use url::Url;

static JS_VARS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)var\s+(\w+)\s*=\s*(".*?"|'.*?'|[^;]+);"#).unwrap());
static DIGITS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+").unwrap());
static AGO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bago\b|\byesterday\b|\btoday\b").unwrap());

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("static selector")
}

fn has_classes(el: &ElementRef, wanted: &[&str]) -> bool {
    wanted.iter().all(|w| el.value().classes().any(|c| c == *w))
}

// ------------------------------------------------------------------ album pages

/// Absolute URLs of every item on an album listing page, in document order.
pub fn item_pages(html: &str, host_page: &str) -> Vec<String> {
    let doc = Html::parse_document(html);
    let base = Url::parse(host_page).ok();
    doc.select(&sel("a[href]"))
        .filter(|a| has_classes(a, &["after:absolute", "after:z-10", "after:inset-0"]))
        .filter_map(|a| {
            let href = a.value().attr("href")?;
            Some(match &base {
                Some(b) => b.join(href).map(|u| u.to_string()).unwrap_or_else(|_| format!("{host_page}{href}")),
                None => format!("{host_page}{href}"),
            })
        })
        .collect()
}

/// `?page=N` URLs for pages 2..=last, or None when the album has a single page.
pub fn next_album_pages(html: &str, url: &str) -> Option<Vec<String>> {
    let doc = Html::parse_document(html);
    let nav = doc.select(&sel("nav.pagination")).next()?;
    let text = nav.text().collect::<Vec<_>>().join(" ");
    let last = DIGITS.find_iter(&text).filter_map(|m| m.as_str().parse::<u32>().ok()).max()?;
    Some((2..=last).map(|p| format!("{url}?page={p}")).collect())
}

/// Item upload dates from the listing (`.theDate`), positionally paired with `item_pages`.
pub fn item_dates(html: &str) -> Vec<Option<DateTime<Utc>>> {
    let doc = Html::parse_document(html);
    doc.select(&sel(".theDate"))
        .map(|t| parse_date(t.text().collect::<String>().trim()))
        .collect()
}

pub fn album_name(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let h1 = sel("h1");
    let container = doc
        .select(&sel("div"))
        .find(|d| has_classes(d, &["text-subs", "font-semibold", "flex", "text-base", "sm:text-lg"]))?;
    let raw: String = container.select(&h1).next()?.text().collect();
    Some(fix_mojibake(&decode_entities(raw.trim())))
}

fn decode_entities(s: &str) -> String {
    // html5ever already decoded entities in text nodes; kept as a seam for double-escaped titles.
    if s.contains('&') {
        let frag = Html::parse_fragment(s);
        frag.root_element().text().collect()
    } else {
        s.to_string()
    }
}

/// Repair UTF-8 that was mis-decoded as Latin-1 (returns the input when it isn't mojibake).
pub fn fix_mojibake(s: &str) -> String {
    if s.chars().all(|c| (c as u32) < 256) {
        let bytes: Vec<u8> = s.chars().map(|c| c as u8).collect();
        if let Ok(fixed) = String::from_utf8(bytes) {
            return fixed;
        }
    }
    s.to_string()
}

// ------------------------------------------------------------------ item pages

fn collect_text(el: ElementRef, out: &mut String) {
    for child in el.children() {
        if let Some(t) = child.value().as_text() {
            out.push_str(t);
        } else if let Some(ce) = ElementRef::wrap(child) {
            if ce.value().classes().any(|c| c == "__cf_email__") {
                if let Some(plain) = ce.value().attr("data-cfemail").and_then(decrypt_cf_email) {
                    out.push_str(&plain);
                    continue;
                }
            }
            collect_text(ce, out);
        }
    }
}

/// Cloudflare "email protection" decoding (XOR with the first byte).
pub fn decrypt_cf_email(hex: &str) -> Option<String> {
    if hex.len() < 4 || hex.len() % 2 != 0 {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect();
    let bytes = bytes?;
    let key = bytes[0];
    String::from_utf8(bytes[1..].iter().map(|b| b ^ key).collect()).ok()
}

pub fn item_filename(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let h1 = doc
        .select(&sel("h1"))
        .find(|h| has_classes(h, &["text-subs", "font-semibold", "text-base", "sm:text-lg", "truncate"]))?;
    let mut text = String::new();
    collect_text(h1, &mut text);
    let text = text.trim();
    (!text.is_empty()).then(|| fix_mojibake(text))
}

/// Best-effort upload date of a single item page.
pub fn item_date(html: &str) -> Option<DateTime<Utc>> {
    let doc = Html::parse_document(html);
    if let Some(d) = doc.select(&sel("time[datetime]")).find_map(|t| parse_date(t.value().attr("datetime")?)) {
        return Some(d);
    }
    doc.select(&sel("[title]")).find_map(|t| {
        let text: String = t.text().collect();
        if AGO.is_match(text.trim()) {
            parse_date(t.value().attr("title")?)
        } else {
            None
        }
    })
}

/// CDN/runtime variables from the inline script containing `var jsCDN`.
pub fn page_vars(html: &str) -> HashMap<String, String> {
    let doc = Html::parse_document(html);
    for script in doc.select(&sel("script")) {
        let text: String = script.text().collect();
        if text.contains("var jsCDN") {
            return JS_VARS
                .captures_iter(&text)
                .map(|c| {
                    let v = c[2].replace(r"\/", "/").replace(r"\\", "\\");
                    (c[1].to_string(), v.trim_matches(|ch| ch == '\'' || ch == '"').to_string())
                })
                .collect();
        }
    }
    HashMap::new()
}

/// `data-file-id` of the first `<script>` tag.
pub fn file_id(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    doc.select(&sel("script")).next()?.value().attr("data-file-id").map(str::to_owned)
}

// ------------------------------------------------------------------------ dates

pub fn parse_date(text: &str) -> Option<DateTime<Utc>> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(d) = DateTime::parse_from_rfc3339(text) {
        return Some(d.with_timezone(&Utc));
    }
    for f in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f", "%H:%M:%S %d/%m/%Y"] {
        if let Ok(n) = NaiveDateTime::parse_from_str(text, f) {
            return Some(Utc.from_utc_datetime(&n));
        }
    }
    for f in ["%Y-%m-%d", "%d/%m/%Y", "%B %d, %Y", "%b %d, %Y"] {
        if let Ok(d) = NaiveDate::parse_from_str(text, f) {
            return Some(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALBUM: &str = r#"
    <html><body>
    <div class="text-subs font-semibold flex text-base sm:text-lg"><h1>My Album</h1></div>
    <a class="after:absolute after:z-10 after:inset-0" href="/f/aaa111"></a><span class="theDate">10:22:03 15/03/2024</span>
    <a class="after:absolute after:z-10 after:inset-0" href="/f/bbb222"></a><span class="theDate">11:00:00 16/03/2024</span>
    <a class="other" href="/nope"></a>
    <nav class="pagination"><a>1</a><a>2</a><a>3</a></nav>
    </body></html>"#;

    #[test]
    fn album_listing() {
        let pages = item_pages(ALBUM, "https://bunkr.si");
        assert_eq!(pages, ["https://bunkr.si/f/aaa111", "https://bunkr.si/f/bbb222"]);
        assert_eq!(album_name(ALBUM).as_deref(), Some("My Album"));
        let dates = item_dates(ALBUM);
        assert_eq!(dates.len(), 2);
        assert_eq!(dates[0].unwrap().to_rfc3339(), "2024-03-15T10:22:03+00:00");
        let next = next_album_pages(ALBUM, "https://bunkr.si/a/X").unwrap();
        assert_eq!(next, ["https://bunkr.si/a/X?page=2", "https://bunkr.si/a/X?page=3"]);
    }

    #[test]
    fn item_page() {
        let html = r#"<html><head>
        <script>var jsCDN = "https:\/\/cdn.example\/storage\/abc.mp4"; var other = 'x';</script></head><body>
        <h1 class="text-subs font-semibold text-base sm:text-lg truncate">clip<span class="__cf_email__" data-cfemail="26434a67634e4f42">[email&#160;protected]</span>.mp4</h1></body></html>"#;
        let vars = page_vars(html);
        assert_eq!(vars["jsCDN"], "https://cdn.example/storage/abc.mp4");
        assert_eq!(vars["other"], "x");
        let name = item_filename(html).unwrap();
        assert!(name.starts_with("clip") && name.ends_with(".mp4"), "{name}");
    }

    #[test]
    fn cf_email() {
        // key 0x26, payload "a@b.c" ^ key
        let key = 0x26u8;
        let hex: String = std::iter::once(key).chain("a@b.c".bytes().map(|b| b ^ key)).map(|b| format!("{b:02x}")).collect();
        assert_eq!(decrypt_cf_email(&hex).unwrap(), "a@b.c");
    }

    #[test]
    fn dates_and_mojibake() {
        assert!(parse_date("2024-03-15T10:22:03Z").is_some());
        assert!(parse_date("2024-03-15").is_some());
        assert!(parse_date("15/03/2024").is_some());
        assert!(parse_date("March 15, 2024").is_some());
        assert!(parse_date("garbage").is_none());
        assert_eq!(fix_mojibake("CafÃ©"), "Café");
        assert_eq!(fix_mojibake("Café"), "Café");
    }

    #[test]
    fn file_id_first_script() {
        let html = r#"<html><head><script data-file-id="123"></script></head></html>"#;
        assert_eq!(file_id(html).as_deref(), Some("123"));
    }
}
