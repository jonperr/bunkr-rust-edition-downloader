//! URL → album/item pipeline: resolve, filter, download (with retries), persist state.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use futures_util::{stream, StreamExt};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::config::*;
use crate::engine::{DlError, Engine};
use crate::fsutil::*;
use crate::net::{self, Net};
use crate::parse;
use crate::state::{self, AlbumState, StateStore};
use crate::ui::{Reason, Ui};
use crate::urlutil::{self, UrlKind};

/// Process-wide shared context.
pub struct Run {
    pub net: Net,
    pub cfg: Settings,
    pub engine: Engine,
}

/// Outcome of processing one URL.
#[derive(Debug, Clone)]
pub enum UrlResult {
    Ok,
    /// Some items permanently failed.
    Failed,
    /// The URL could not be processed at all.
    Error(String),
    /// Not enough free disk space (the CLI stops, the GUI just reports it).
    DiskFull(String),
}

impl UrlResult {
    pub fn needs_retry(&self) -> bool {
        !matches!(self, UrlResult::Ok)
    }
}

/// A fetched and classified URL, ready to be listed or downloaded.
pub struct Prepared {
    pub url: String,
    pub html: String,
    pub kind: UrlKind,
    pub identifier: String,
    pub name: Option<String>,
    pub dir: PathBuf,
}

pub async fn prepare(run: &Run, ui: &Ui, raw: &str, create_dir: bool) -> Result<Prepared, String> {
    let url = urlutil::normalize(raw);

    let Some(html) = net::fetch_page(&run.net, &url).await else {
        session_log(&format!("[{}] Request error for {url}", Reason::ServiceUnavailable.code()));
        ui.log("Service unavailable", &format!("The page {url} is currently unavailable. Verify the link and retry."));
        ui.record(Reason::ServiceUnavailable);
        return Err("Não foi possível acessar essa página. Verifique o link e sua conexão.".into());
    };
    let kind = match urlutil::kind(&url) {
        Ok(k) => k,
        Err(e) => {
            ui.log("Invalid URL", &e);
            return Err("Link em formato inválido.".into());
        }
    };

    let album_id = (kind == UrlKind::Album).then(|| urlutil::last_segment(&url));
    let album_name = parse::album_name(&html);
    let dir_name = if run.cfg.no_album_folder {
        None
    } else {
        format_directory_name(album_name.as_deref(), album_id.as_deref())
    };
    let dir = match create_download_directory(dir_name.as_deref(), run.cfg.custom_path.as_deref(), run.cfg.no_download_folder, create_dir) {
        Ok(d) => d,
        Err(e) => {
            ui.log("Directory error", &format!("{e:#}"));
            return Err(format!("Não foi possível criar a pasta de destino: {e:#}"));
        }
    };
    let identifier = album_id.unwrap_or_else(|| urlutil::media_slug(&url, &html));
    Ok(Prepared { url, html, kind, identifier, name: album_name, dir })
}

/// Collect every item page (and date) of an album, fetching listing pages concurrently.
async fn crawl_album(run: &Run, url: &str, first_html: &str) -> Result<(Vec<String>, HashMap<String, DateTime<Utc>>)> {
    let host = urlutil::host_page(url);
    let mut pages = parse::item_pages(first_html, &host);
    let mut dates: HashMap<String, DateTime<Utc>> = pages
        .iter()
        .cloned()
        .zip(parse::item_dates(first_html))
        .filter_map(|(p, d)| d.map(|d| (p, d)))
        .collect();

    if let Some(next) = parse::next_album_pages(first_html, url) {
        let fetched: Vec<(String, Option<String>)> = stream::iter(next)
            .map(|u| async move {
                let h = net::fetch_page(&run.net, &u).await;
                (u, h)
            })
            .buffered(6) // concurrent, but results stay in page order
            .collect()
            .await;
        for (u, h) in fetched {
            let h = h.ok_or_else(|| anyhow!("Failed to load paginated album page: {u}"))?;
            let ip = parse::item_pages(&h, &host);
            dates.extend(ip.iter().cloned().zip(parse::item_dates(&h)).filter_map(|(p, d)| d.map(|d| (p, d))));
            pages.extend(ip);
        }
    }
    let mut seen = HashSet::new();
    pages.retain(|p| seen.insert(p.clone()));
    Ok((pages, dates))
}

pub struct Resolved {
    pub pages: Vec<String>,
    pub dates: HashMap<String, DateTime<Utc>>,
    pub cached: Option<AlbumState>,
}

/// Item pages of the URL, reusing the persisted listing when it still looks current.
pub async fn resolve_pages(run: &Run, p: &Prepared) -> Result<Resolved> {
    if p.kind == UrlKind::Media {
        return Ok(Resolved { pages: vec![p.url.clone()], dates: HashMap::new(), cached: None });
    }
    let cached = state::load(&p.dir).filter(|s| s.album_id == p.identifier && !s.item_pages.is_empty());
    if let (Some(c), false) = (&cached, run.cfg.refresh) {
        // Page 1 is already in memory: use it to detect albums that changed since the last crawl.
        let first = parse::item_pages(&p.html, &urlutil::host_page(&p.url));
        let multi = parse::next_album_pages(&p.html, &p.url).is_some();
        let prefix_ok = c.item_pages.len() >= first.len() && c.item_pages[..first.len()] == first[..];
        let size_ok = if multi { c.item_pages.len() > first.len() } else { c.item_pages.len() == first.len() };
        if prefix_ok && size_ok {
            return Ok(Resolved { pages: c.item_pages.clone(), dates: state::parse_dates(&c.item_dates), cached: cached.clone() });
        }
    }
    let (pages, dates) = crawl_album(run, &p.url, &p.html).await?;
    // Keep per-item results of earlier runs when the listing is refreshed.
    let items = cached.map(|c| c.items).unwrap_or_default();
    let item_dates = dates.iter().map(|(k, v)| (k.clone(), Some(v.to_rfc3339()))).collect();
    let st = AlbumState { album_id: p.identifier.clone(), item_pages: pages.clone(), items, item_dates };
    Ok(Resolved { pages, dates, cached: Some(st) })
}

// ------------------------------------------------------------------ items

struct Job {
    run: Arc<Run>,
    ui: Arc<Ui>,
    dir: PathBuf,
    label: String,
    state: Option<StateStore>,
    reserved: Mutex<HashSet<String>>,
    dates: HashMap<String, DateTime<Utc>>,
    dl_slots: Semaphore,
}

enum Outcome {
    Done,
    Failed,
    /// Failed after all attempts; to be retried once more after the others finish.
    Retry { filename: String },
}

impl Job {
    fn reserve(&self, name: &str, page: &str) -> String {
        let cached = self.state.as_ref().and_then(|s| s.item(page)).map(|i| i.filename);
        let mut r = self.reserved.lock().unwrap();
        let unique = match cached {
            Some(c) if !r.contains(&c) => c,
            _ => reserve_unique_filename(name, &r),
        };
        r.insert(unique.clone());
        unique
    }

    fn skip_reason(&self, filename: &str) -> Option<Reason> {
        if self.dir.join(truncate_filename(filename)).exists() {
            Some(Reason::AlreadyDownloaded)
        } else if matches_ignore(filename, &self.run.cfg.ignore) {
            Some(Reason::IgnoreList)
        } else if fails_include(filename, &self.run.cfg.include) {
            Some(Reason::IncludeList)
        } else {
            None
        }
    }

    fn skip(&self, page: &str, filename: &str, reason: Reason) -> Outcome {
        let msg = match reason {
            Reason::AlreadyDownloaded => format!("{filename} has already been downloaded."),
            Reason::IgnoreList => format!("{filename} matches the ignore list."),
            _ => format!("No included words found for {filename}."),
        };
        self.ui.log("Skipped download", &msg);
        self.ui.record(reason);
        self.ui.overall_inc();
        if reason == Reason::AlreadyDownloaded {
            if let Some(s) = &self.state {
                s.set_item(page, filename, true);
            }
        }
        Outcome::Done
    }

    fn fail(&self, page: &str, filename: &str, msg: &str) -> Outcome {
        self.ui.log("Download failed", msg);
        self.ui.record(Reason::Failed);
        session_log(&format!("[{}] {msg} — {page}", Reason::Failed.code()));
        if let Some(s) = &self.state {
            s.set_item(page, filename, false);
        }
        self.ui.overall_inc();
        Outcome::Failed
    }
}

fn backoff(attempt: u32) -> Duration {
    Duration::from_secs_f64(1.5f64.powi(attempt as i32 + 1) + 1.0 + rand::random::<f64>())
}

enum Dl {
    Success,
    Failed,
}

/// Download one resolved file with the retry policy of the original tool.
async fn download_file(job: &Job, filename: &str, link: &str, date: Option<DateTime<Utc>>, retries: u32) -> Dl {
    let run = &job.run;
    let ui = &job.ui;
    let dest = job.dir.join(truncate_filename(filename));
    let bar = ui.file_bar(filename);
    if !ui.is_live() {
        ui.log("Downloading", filename);
    }
    let started = std::time::Instant::now();
    let retries = retries.max(1);

    for attempt in 0..retries {
        match run.engine.download(link, &dest, &bar).await {
            Ok(()) => {
                bar.finish_and_clear();
                if let Some(d) = date {
                    let _ = filetime::set_file_mtime(&dest, filetime::FileTime::from_unix_time(d.timestamp(), 0));
                }
                if !ui.is_live() {
                    let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
                    let secs = started.elapsed().as_secs_f64().max(0.001);
                    ui.log("Downloaded", &format!("{filename} ({:.1} MB, {:.1} MB/s)", size as f64 / MB as f64, size as f64 / MB as f64 / secs));
                }
                ui.add_done_bytes(bar.position());
                return Dl::Success;
            }
            Err(e) => {
                let last = attempt + 1 == retries;
                match &e {
                    DlError::BadGateway | DlError::Http(_) | DlError::Io(_) | DlError::NoRange => {
                        ui.log("Request error", &format!("{filename}: {e}"));
                        break;
                    }
                    _ => {
                        ui.log("Retrying download", &format!("{filename}: {e} ({}/{retries})", attempt + 1));
                        if !last {
                            tokio::time::sleep(backoff(attempt)).await;
                        }
                    }
                }
            }
        }
    }
    bar.finish_and_clear();
    Dl::Failed
}

/// Resolve and download one item. `retry_name` is Some(..) on the final retry pass.
async fn process_item(job: &Job, page: &str, prefetched: Option<String>, retry_name: Option<String>) -> Outcome {
    let run = &job.run;
    let ui = &job.ui;
    let cfg = &run.cfg;
    let is_final = retry_name.is_some();

    // Fast path: finished in a previous run and still on disk -> no network at all.
    if !is_final {
        if let Some(st) = job.state.as_ref().and_then(|s| s.item(page)) {
            if st.status == "completed" && job.dir.join(truncate_filename(&st.filename)).exists() {
                ui.log("Skipped download", &format!("{} has already been downloaded (cached from a previous run).", st.filename));
                ui.record(Reason::AlreadyDownloaded);
                ui.overall_inc();
                return Outcome::Done;
            }
        }
    }

    let html = match prefetched {
        Some(h) => h,
        None => match net::fetch_page(&run.net, page).await {
            Some(h) => h,
            None => return job.fail(page, &urlutil::last_segment(page), &format!("Unable to load page {page}")),
        },
    };

    let meta = net::item_meta(page, &html);
    let date = job.dates.get(page).copied().or(meta.date);

    let (filename, link) = if let Some(name) = retry_name {
        // Re-sign: the previous token may have expired while the item waited for its retry.
        (name, net::signed_link(&run.net, page, &html).await)
    } else if cfg.clean_name {
        // Name is known from the page alone, so filtered/existing files never hit the signing API.
        let name = job.reserve(&meta.page_filename, page);
        if let Some(r) = job.skip_reason(&name) {
            return job.skip(page, &name, r);
        }
        let link = net::signed_link(&run.net, page, &html).await;
        (name, link)
    } else {
        let link = net::signed_link(&run.net, page, &html).await;
        let name = match &link {
            Some(l) => net::format_item_filename(&meta.page_filename, &urlutil::url_based_filename(l)),
            None => meta.page_filename.clone(),
        };
        if let Some(r) = job.skip_reason(&name) {
            return job.skip(page, &name, r);
        }
        (name, link)
    };

    let Some(link) = link else {
        return job.fail(page, &filename, &format!("Could not resolve a download URL for {filename}."));
    };

    let _slot = job.dl_slots.acquire().await.ok();
    let retries = if is_final { 2 } else { cfg.max_retries };
    match download_file(job, &filename, &link, date, retries).await {
        Dl::Success => {
            ui.record(Reason::Completed);
            if let Some(s) = &job.state {
                s.set_item(page, &filename, job.dir.join(truncate_filename(&filename)).exists());
            }
            ui.overall_inc();
            Outcome::Done
        }
        Dl::Failed if !is_final => {
            ui.log("Exceeded retry attempts", &format!("Max retries reached for {filename}. It will be retried once more after all other tasks."));
            Outcome::Retry { filename }
        }
        Dl::Failed => job.fail(page, &filename, &format!("Failed to download {filename}. Check the log file.")),
    }
}

/// Download every item. Returns true if at least one item permanently failed.
async fn run_items(job: &Job, pages: Vec<String>, prefetched: Option<(String, String)>) -> bool {
    let workers = job.run.cfg.workers;
    job.ui.start_overall(&job.label, pages.len() as u64);

    // Up to 2x workers items are in flight: half download while the rest are being
    // resolved (page fetch + signing), so the connection slots never sit idle.
    let first: Vec<(String, Outcome)> = stream::iter(pages)
        .map(|p| {
            let pre = prefetched.as_ref().filter(|(u, _)| *u == p).map(|(_, h)| h.clone());
            async move {
                let o = process_item(job, &p, pre, None).await;
                (p, o)
            }
        })
        .buffer_unordered(workers * 2)
        .collect()
        .await;

    let mut failed = first.iter().filter(|(_, o)| matches!(o, Outcome::Failed)).count();
    let retries: Vec<(String, String)> = first
        .into_iter()
        .filter_map(|(p, o)| match o {
            Outcome::Retry { filename } => Some((p, filename)),
            _ => None,
        })
        .collect();

    if !retries.is_empty() {
        let second: Vec<Outcome> = stream::iter(retries)
            .map(|(p, name)| async move { process_item(job, &p, None, Some(name)).await })
            .buffer_unordered(workers)
            .collect()
            .await;
        failed += second.iter().filter(|o| matches!(o, Outcome::Failed)).count();
    }

    if let Some(s) = &job.state {
        s.flush();
    }
    job.ui.finish_overall();
    failed > 0
}

/// Process one URL end to end.
pub async fn process_url(run: &Arc<Run>, ui: &Arc<Ui>, raw: &str) -> UrlResult {
    ui.set_status(crate::ui::Status::Running);

    if !run.cfg.disable_disk_check {
        let root = run.cfg.custom_path.clone().unwrap_or_else(|| PathBuf::from("."));
        if let Some(free) = free_space(&root) {
            if free < MIN_DISK_SPACE {
                let msg = format!("Only {:.2} GB available on {}. Stopped to prevent data loss.", free as f64 / GB as f64, root.display());
                ui.log("Insufficient disk space", &msg);
                return UrlResult::DiskFull(msg);
            }
        }
    }

    let prep = match prepare(run, ui, raw, true).await {
        Ok(p) => p,
        Err(e) => return UrlResult::Error(e),
    };
    ui.set_album(prep.name.clone(), prep.dir.display().to_string());

    let resolved = match resolve_pages(run, &prep).await {
        Ok(r) => r,
        Err(e) => {
            ui.log("Crawl failed", &format!("{e:#}"));
            session_log(&format!("[FAILED] {e:#} — {}", prep.url));
            return UrlResult::Error(format!("{e:#}"));
        }
    };
    if resolved.pages.is_empty() {
        ui.log("Nothing to download", &format!("No items found at {}", prep.url));
        return UrlResult::Ok;
    }

    let state = resolved.cached.map(|st| {
        let from_cache = !st.items.is_empty();
        let store = StateStore::new(&prep.dir, st);
        store.flush();
        if from_cache {
            ui.log("Using cached album state", &format!("Reusing {} previously crawled item page(s)", resolved.pages.len()));
        }
        store
    });

    let job = Job {
        run: run.clone(),
        ui: ui.clone(),
        dir: prep.dir.clone(),
        label: prep.identifier.clone(),
        state,
        reserved: Mutex::new(HashSet::new()),
        dates: resolved.dates,
        dl_slots: Semaphore::new(run.cfg.workers),
    };
    let prefetched = (prep.kind == UrlKind::Media).then(|| (prep.url.clone(), prep.html.clone()));
    if run_items(&job, resolved.pages, prefetched).await { UrlResult::Failed } else { UrlResult::Ok }
}
