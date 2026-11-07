//! Command-line interface. Precedence: explicit CLI flag > bunkr.toml > built-in default.

use clap::Parser;
use std::path::PathBuf;

use bunkr_core::config::*;

#[derive(Parser, Debug)]
#[command(
    name = "bunkr",
    version,
    about = "Fast Bunkr album/file downloader (Rust port of BunkrDownloader)",
    after_help = "With no URL given, URLs are read from URLs.txt (one per line), like the old main.py."
)]
pub struct Cli {
    /// Album (/a/) or file (/f/ /i/ /v/) URLs to download.
    pub urls: Vec<String>,

    /// Read URLs from this file instead of the command line (default when no URL is given: URLs.txt).
    #[arg(short = 'f', long, value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// Directory where downloaded content is saved.
    #[arg(long, value_name = "DIR")]
    pub custom_path: Option<PathBuf>,

    /// Save files without a 'Downloads' subfolder.
    #[arg(long)]
    pub no_download_folder: bool,

    /// Save files without an 'ALBUM_TITLE (ALBUM_ID)' subfolder.
    #[arg(long)]
    pub no_album_folder: bool,

    /// Disable the live progress UI (plain log lines instead).
    #[arg(long)]
    pub disable_ui: bool,

    /// Skip the free-disk-space check.
    #[arg(long)]
    pub disable_disk_check: bool,

    /// Download from hosts that were marked offline earlier in this session anyway.
    #[arg(long)]
    pub disable_server_check: bool,

    /// Keep the original filename (adds a (N) suffix to prevent duplicates).
    #[arg(long)]
    pub clean_name: bool,

    /// Maximum retries for a single file [default: 5].
    #[arg(long, value_name = "N")]
    pub max_retries: Option<u32>,

    /// Parallel connections per file for chunked downloads; 1 disables chunking [default: 8].
    #[arg(long, value_name = "N")]
    pub connections: Option<usize>,

    /// Number of files downloaded at the same time within an album [default: 4].
    #[arg(long, value_name = "N")]
    pub workers: Option<usize>,

    /// Maximum total download speed in KB/s, shared by every connection (default: unlimited).
    #[arg(long, value_name = "KB/S")]
    pub rate_limit: Option<f64>,

    /// List what would be downloaded (names, sizes, status) without downloading anything.
    #[arg(long)]
    pub dry_run: bool,

    /// Number of URLs processed concurrently in batch mode [default: 1].
    #[arg(long, value_name = "N")]
    pub max_concurrent_urls: Option<usize>,

    /// Skip files whose name contains this substring (repeat the flag for several).
    #[arg(long, value_name = "TEXT")]
    pub ignore: Vec<String>,

    /// Only download files whose name contains this substring (repeat the flag for several).
    #[arg(long, value_name = "TEXT")]
    pub include: Vec<String>,

    /// Ignore the cached album listing and crawl the album again.
    #[arg(long)]
    pub refresh: bool,

    /// TOML config file (default: ./bunkr.toml).
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

impl Cli {
    pub fn settings(&self) -> Settings {
        let t = load_toml(self.config.as_deref());
        let or_vec = |cli: &Vec<String>, key: &str| {
            if cli.is_empty() { t_strs(&t, key).unwrap_or_default() } else { cli.clone() }
        };
        Settings {
            custom_path: self.custom_path.clone().or_else(|| t_str(&t, "custom_path").map(PathBuf::from)),
            no_download_folder: self.no_download_folder || t_bool(&t, "no_download_folder").unwrap_or(false),
            no_album_folder: self.no_album_folder || t_bool(&t, "no_album_folder").unwrap_or(false),
            disable_ui: self.disable_ui || t_bool(&t, "disable_ui").unwrap_or(false),
            disable_disk_check: self.disable_disk_check || t_bool(&t, "disable_disk_check").unwrap_or(false),
            disable_server_check: self.disable_server_check || t_bool(&t, "disable_server_check").unwrap_or(false),
            clean_name: self.clean_name || t_bool(&t, "clean_name").unwrap_or(false),
            max_retries: self.max_retries.or(t_uint(&t, "max_retries").map(|v| v as u32)).unwrap_or(DEFAULT_RETRIES),
            connections: self.connections.or(t_uint(&t, "connections").map(|v| v as usize)).unwrap_or(DEFAULT_CONNECTIONS).max(1),
            workers: self.workers.or(t_uint(&t, "workers").map(|v| v as usize)).unwrap_or(DEFAULT_WORKERS).max(1),
            rate_limit_kbps: self.rate_limit.or(t_float(&t, "rate_limit")).filter(|v| *v > 0.0),
            refresh: self.refresh,
            dry_run: self.dry_run || t_bool(&t, "dry_run").unwrap_or(false),
            max_concurrent_urls: self.max_concurrent_urls.or(t_uint(&t, "max_concurrent_urls").map(|v| v as usize)).unwrap_or(1).max(1),
            ignore: or_vec(&self.ignore, "ignore"),
            include: or_vec(&self.include, "include"),
        }
    }
}
