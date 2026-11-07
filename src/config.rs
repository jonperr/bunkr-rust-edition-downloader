//! Constants and the merged (CLI + bunkr.toml + defaults) runtime settings.

use std::path::{Path, PathBuf};

pub const KB: u64 = 1024;
pub const MB: u64 = 1024 * KB;
pub const GB: u64 = 1024 * MB;

pub const BACKUP_FOLDER: &str = "Backups";
pub const DOWNLOAD_FOLDER: &str = "Downloads";
pub const URLS_FILE: &str = "URLs.txt";
pub const SESSION_LOG: &str = "session.log";
pub const STATE_FILE: &str = ".bunkr_state.json";

const DEFAULT_SIGN_API: &str = "https://glb-apisign.cdn.cr/sign";
const DEFAULT_DOWNLOAD_API: &str = "https://dl.bunkr.cr/api/_001_v2";
pub const DOWNLOAD_REFERER: &str = "https://dl.bunkrr.cr/";
pub const FALLBACK_DOMAIN: &str = "bunkr.cr";
pub const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:136.0) Gecko/20100101 Firefox/136.0";

/// Signature API endpoint (overridable with `BUNKR_SIGN_API`, handy for tests/mirrors).
pub fn sign_api() -> String {
    std::env::var("BUNKR_SIGN_API").unwrap_or_else(|_| DEFAULT_SIGN_API.to_string())
}

/// Direct download API endpoint (overridable with `BUNKR_DOWNLOAD_API`).
pub fn download_api() -> String {
    std::env::var("BUNKR_DOWNLOAD_API").unwrap_or_else(|_| DEFAULT_DOWNLOAD_API.to_string())
}

pub const MAX_FILENAME_LEN: usize = 120;
pub const DEFAULT_WORKERS: usize = 4;
pub const DEFAULT_RETRIES: u32 = 5;
pub const DEFAULT_CONNECTIONS: usize = 8;
pub const CHUNK_MAX_RETRIES: u32 = 4;
pub const CHUNK_BASE_DELAY: f64 = 1.5;
pub const MIN_PARALLEL_SIZE: u64 = 16 * MB;
pub const MIN_DISK_SPACE: u64 = 4 * GB;
pub const UNITS_PER_CONNECTION: u64 = 4;
pub const MIN_WORK_UNIT: u64 = 4 * MB;
pub const MAX_WORK_UNIT: u64 = 32 * MB;
/// Bytes accumulated in memory before a single positional write hits the disk.
pub const WRITE_BUF: usize = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Settings {
    pub custom_path: Option<PathBuf>,
    pub no_download_folder: bool,
    pub no_album_folder: bool,
    pub disable_ui: bool,
    pub disable_disk_check: bool,
    pub disable_server_check: bool,
    pub clean_name: bool,
    pub max_retries: u32,
    pub connections: usize,
    pub rate_limit_kbps: Option<f64>,
    pub dry_run: bool,
    pub refresh: bool,
    pub max_concurrent_urls: usize,
    pub workers: usize,
    pub ignore: Vec<String>,
    pub include: Vec<String>,
}

/// Locate and parse bunkr.toml (explicit `--config`, else ./bunkr.toml). Never fails hard.
pub fn load_toml(explicit: Option<&Path>) -> toml::Table {
    let path = match explicit {
        Some(p) if p.is_file() => p.to_path_buf(),
        Some(p) => {
            eprintln!("warning: config file '{}' not found", p.display());
            return toml::Table::new();
        }
        None => match std::env::current_dir() {
            Ok(d) if d.join("bunkr.toml").is_file() => d.join("bunkr.toml"),
            _ => return toml::Table::new(),
        },
    };
    match std::fs::read_to_string(&path).map(|s| s.parse::<toml::Table>()) {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => {
            eprintln!("warning: could not parse '{}': {e}", path.display());
            toml::Table::new()
        }
        Err(e) => {
            eprintln!("warning: could not read '{}': {e}", path.display());
            toml::Table::new()
        }
    }
}

fn bad(key: &str) {
    eprintln!("warning: bunkr.toml '{key}' has an invalid value; using the default instead");
}

pub fn t_bool(t: &toml::Table, k: &str) -> Option<bool> {
    t.get(k).and_then(|v| v.as_bool().or_else(|| { bad(k); None }))
}
pub fn t_uint(t: &toml::Table, k: &str) -> Option<u64> {
    t.get(k).and_then(|v| v.as_integer().and_then(|i| u64::try_from(i).ok()).or_else(|| { bad(k); None }))
}
pub fn t_float(t: &toml::Table, k: &str) -> Option<f64> {
    t.get(k).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)).or_else(|| { bad(k); None }))
}
pub fn t_str(t: &toml::Table, k: &str) -> Option<String> {
    t.get(k).and_then(|v| v.as_str().map(str::to_owned).or_else(|| { bad(k); None }))
}
pub fn t_strs(t: &toml::Table, k: &str) -> Option<Vec<String>> {
    t.get(k).and_then(|v| {
        v.as_array()
            .and_then(|a| a.iter().map(|x| x.as_str().map(str::to_owned)).collect::<Option<Vec<_>>>())
            .or_else(|| { bad(k); None })
    })
}
