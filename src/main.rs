//! Bunkr downloader — Rust port tuned for throughput.

mod cli;

use clap::Parser;
use futures_util::{stream, StreamExt};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use bunkr_core::album::{self, Run, UrlResult};
use bunkr_core::config::*;
use bunkr_core::engine::Engine;
use bunkr_core::ratelimit::RateLimiter;
use bunkr_core::ui::Ui;
use bunkr_core::{dryrun, fsutil, net};

use crate::cli::Cli;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let cfg = cli.settings();

    // ---- collect URLs: command line, plus an explicit --file, or URLs.txt when nothing was given
    let mut urls = cli.urls.clone();
    let file = match (&cli.file, urls.is_empty()) {
        (Some(f), _) => Some(f.clone()),
        (None, true) => Some(PathBuf::from(URLS_FILE)),
        (None, false) => None,
    };
    if let Some(f) = &file {
        match fsutil::read_urls(f) {
            Ok(mut u) => {
                fsutil::backup_urls_file(f);
                urls.append(&mut u);
            }
            Err(e) => {
                eprintln!("{e:#}\n\nUsage: bunkr [OPTIONS] <URL>...   (or list URLs in {URLS_FILE})");
                return ExitCode::from(2);
            }
        }
    }
    if urls.is_empty() {
        eprintln!("No URLs to process.");
        return ExitCode::from(2);
    }

    let net = match net::Net::new() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("failed to initialise HTTP clients: {e:#}");
            return ExitCode::from(1);
        }
    };
    let limiter = cfg.rate_limit_kbps.map(|kb| Arc::new(RateLimiter::new(kb * KB as f64)));
    let concurrent = cfg.max_concurrent_urls > 1 && urls.len() > 1;
    let ui = Arc::new(Ui::new(!cfg.disable_ui && !concurrent && !cfg.dry_run));
    let run = Arc::new(Run {
        engine: Engine { client: net.dl.clone(), limiter, connections: cfg.connections },
        net,
        cfg: cfg.clone(),
    });

    fsutil::session_start();

    let work = {
        let run = run.clone();
        let ui = ui.clone();
        async move {
            if run.cfg.dry_run {
                for u in &urls {
                    dryrun::run(&run, &ui, u).await;
                }
                return Vec::new();
            }
            let limit = if concurrent { run.cfg.max_concurrent_urls } else { 1 };
            stream::iter(urls)
                .map(|u| {
                    let (run, ui) = (run.clone(), ui.clone());
                    async move {
                        let res = album::process_url(&run, &ui, &u).await;
                        if let UrlResult::DiskFull(_) = res {
                            std::process::exit(1);
                        }
                        (u, res.needs_retry())
                    }
                })
                .buffer_unordered(limit)
                .filter_map(|(u, failed)| async move { failed.then_some(u) })
                .collect::<Vec<String>>()
                .await
        }
    };

    let failed = tokio::select! {
        f = work => f,
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\nInterrupted. Partial downloads were kept; run the same command again to resume.");
            return ExitCode::from(130);
        }
    };

    ui.print_summary();
    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("\n{} URL(s) failed and may need attention:", failed.len());
        for u in &failed {
            eprintln!("  {u}");
        }
        ExitCode::from(1)
    }
}
