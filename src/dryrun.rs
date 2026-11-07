//! `--dry-run`: crawl, resolve names/sizes and apply filters, but download nothing.

use console::style;
use futures_util::{stream, StreamExt};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::album::{prepare, resolve_pages, Run};
use crate::ui::Ui;
use crate::fsutil::*;
use crate::net;
use crate::urlutil;

struct Row {
    filename: String,
    size: Option<u64>,
    status: &'static str,
}

fn fmt_size(n: Option<u64>) -> String {
    let Some(n) = n else { return "unknown".into() };
    let mut v = n as f64;
    for unit in ["B", "KB", "MB", "GB"] {
        if v < 1024.0 {
            return format!("{v:.1} {unit}");
        }
        v /= 1024.0;
    }
    format!("{v:.1} TB")
}

fn label(status: &str) -> console::StyledObject<&'static str> {
    match status {
        "would_download" => style("Would download").green(),
        "already_downloaded" => style("Already downloaded").dim(),
        "filtered_ignore" => style("Filtered (ignore list)").yellow(),
        "filtered_include" => style("Filtered (include list)").yellow(),
        "unresolved" => style("Could not resolve URL").red(),
        _ => style("Failed to fetch page").red(),
    }
}

pub async fn run(run: &Arc<Run>, ui: &Ui, raw: &str) {
    let Ok(prep) = prepare(run, ui, raw, false).await else {
        eprintln!("{}", style(format!("Could not fetch {raw}")).red());
        return;
    };
    let resolved = match resolve_pages(run, &prep).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", style(format!("{e:#}")).red());
            return;
        }
    };
    let cached = resolved.cached.as_ref().map(|c| c.items.clone()).unwrap_or_default();
    let reserved = Mutex::new(HashSet::<String>::new());
    let cfg = &run.cfg;

    let rows: Vec<Row> = stream::iter(resolved.pages.iter().cloned())
        .map(|page| {
            let (prep, cached, reserved) = (&prep, &cached, &reserved);
            async move {
                // Previously completed and still on disk.
                if let Some(c) = cached.get(&page).filter(|c| c.status == "completed") {
                    let p = prep.dir.join(truncate_filename(&c.filename));
                    if let Ok(m) = std::fs::metadata(&p) {
                        return Row { filename: c.filename.clone(), size: Some(m.len()), status: "already_downloaded" };
                    }
                }
                let html = if page == prep.url { Some(prep.html.clone()) } else { net::fetch_page(&run.net, &page).await };
                let Some(html) = html else {
                    return Row { filename: page, size: None, status: "fetch_failed" };
                };
                let meta = net::item_meta(&page, &html);

                let (filename, link) = if cfg.clean_name {
                    let name = {
                        let mut r = reserved.lock().unwrap();
                        let n = reserve_unique_filename(&meta.page_filename, &r);
                        r.insert(n.clone());
                        n
                    };
                    // Filters need only the name: don't bother the signing API for filtered files.
                    if matches_ignore(&name, &cfg.ignore) {
                        return Row { filename: name, size: None, status: "filtered_ignore" };
                    }
                    if fails_include(&name, &cfg.include) {
                        return Row { filename: name, size: None, status: "filtered_include" };
                    }
                    let l = net::signed_link(&run.net, &page, &html).await;
                    (name, l)
                } else {
                    let l = net::signed_link(&run.net, &page, &html).await;
                    let name = match &l {
                        Some(l) => net::format_item_filename(&meta.page_filename, &urlutil::url_based_filename(l)),
                        None => meta.page_filename.clone(),
                    };
                    (name, l)
                };
                let Some(link) = link else {
                    return Row { filename, size: None, status: "unresolved" };
                };
                if matches_ignore(&filename, &cfg.ignore) {
                    return Row { filename, size: None, status: "filtered_ignore" };
                }
                if fails_include(&filename, &cfg.include) {
                    return Row { filename, size: None, status: "filtered_include" };
                }
                if let Ok(m) = std::fs::metadata(prep.dir.join(truncate_filename(&filename))) {
                    return Row { filename, size: Some(m.len()), status: "already_downloaded" };
                }
                let (_, len) = run.engine.probe(&link).await;
                Row { filename, size: len.filter(|l| *l > 0), status: "would_download" }
            }
        })
        .buffered(cfg.workers * 3) // ordered output, concurrent resolution
        .collect()
        .await;

    // ---- table
    let width = rows.iter().map(|r| r.filename.chars().count()).max().unwrap_or(8).clamp(8, 70);
    println!("\n{}", style(format!("Dry run for ID: {}", prep.identifier)).bold());
    println!("{:<width$}  {:>10}  {}", style("Filename").bold(), style("Size").bold(), style("Status").bold());
    let (mut total, mut would, mut already, mut filtered, mut unresolved) = (0u64, 0, 0, 0, 0);
    for r in &rows {
        let mut name: String = r.filename.chars().take(width).collect();
        if r.filename.chars().count() > width {
            name.pop();
            name.push('…');
        }
        println!("{name:<width$}  {:>10}  {}", fmt_size(r.size), label(r.status));
        match r.status {
            "would_download" => {
                would += 1;
                total += r.size.unwrap_or(0);
            }
            "already_downloaded" => already += 1,
            "filtered_ignore" | "filtered_include" => filtered += 1,
            _ => unresolved += 1,
        }
    }
    println!("\nWould download: {would} file(s), {} total\n", fmt_size(Some(total)));
    if already > 0 {
        println!("Already downloaded: {already} file(s)");
    }
    if filtered > 0 {
        println!("Filtered out: {filtered} file(s)");
    }
    if unresolved > 0 {
        println!("{}", style(format!("Could not resolve: {unresolved} file(s)")).red());
    }
}
