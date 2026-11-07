//! Filename handling, download directories, session log and URL-file helpers.

use anyhow::{Context, Result};
use chrono::Utc;
use regex::Regex;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::config::*;

static INVALID_CHARS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[<>:"/\\|?*\x00-\x1f]"#).unwrap());

pub fn remove_invalid_characters(s: &str) -> String {
    INVALID_CHARS.replace_all(s, "").into_owned()
}

/// Split into (stem, extension-with-dot) using Python `pathlib` semantics.
pub fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

fn take_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Sanitise and cap the filename length.
pub fn truncate_filename(filename: &str) -> String {
    let (stem, ext) = split_ext(filename);
    let mut name = remove_invalid_characters(stem);
    if name.chars().count() > MAX_FILENAME_LEN {
        name = take_chars(&name, MAX_FILENAME_LEN.saturating_sub(ext.chars().count()));
    }
    format!("{name}{ext}")
}

fn numbered_filename(filename: &str, index: usize) -> String {
    let (stem, ext) = split_ext(filename);
    let mut base = remove_invalid_characters(stem);
    if base.is_empty() {
        base = "file".into();
    }
    let suffix = if index == 0 { String::new() } else { format!(" ({index})") };
    let avail = MAX_FILENAME_LEN
        .saturating_sub(ext.chars().count() + suffix.chars().count())
        .max(1);
    format!("{}{suffix}{ext}", take_chars(&base, avail))
}

/// First `name (N).ext` not yet reserved in this run (files on disk are deliberately ignored,
/// so an item resolves to the same name on every run).
pub fn reserve_unique_filename(filename: &str, reserved: &HashSet<String>) -> String {
    (0..)
        .map(|i| numbered_filename(filename, i))
        .find(|c| !reserved.contains(c))
        .unwrap()
}

pub fn matches_ignore(filename: &str, ignore: &[String]) -> bool {
    ignore.iter().any(|w| filename.contains(w.as_str()))
}

/// True when `--include` is set and the filename matches none of it (i.e. should be skipped).
pub fn fails_include(filename: &str, include: &[String]) -> bool {
    !include.is_empty() && include.iter().all(|w| !filename.contains(w.as_str()))
}

pub fn format_directory_name(name: Option<&str>, id: Option<&str>) -> Option<String> {
    match (name, id) {
        (None, id) => id.map(str::to_owned),
        (Some(n), Some(i)) => Some(format!("{n} ({i})")),
        (Some(_), None) => None,
    }
}

fn sanitize_directory_name(name: &str) -> String {
    let bad: &[char] = if cfg!(windows) { &['\\', '/', ':', '*', '?', '"', '<', '>', '|'] } else { &['/', ':'] };
    name.chars().map(|c| if bad.contains(&c) { '_' } else { c }).collect()
}

pub fn create_download_directory(
    dir_name: Option<&str>,
    custom_path: Option<&Path>,
    no_download_folder: bool,
    create: bool,
) -> Result<PathBuf> {
    let mut base = custom_path.map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    if !no_download_folder {
        base.push(DOWNLOAD_FOLDER);
    }
    if let Some(n) = dir_name.map(sanitize_directory_name).filter(|n| !n.trim().is_empty()) {
        base.push(n.trim());
    }
    if create {
        std::fs::create_dir_all(&base).with_context(|| format!("creating directory {}", base.display()))?;
    }
    Ok(base)
}

pub fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Free disk space (bytes) on the filesystem holding `path` (walks up to an existing ancestor).
pub fn free_space(path: &Path) -> Option<u64> {
    let mut p = std::fs::canonicalize(path).ok().or_else(|| {
        path.ancestors().find_map(|a| std::fs::canonicalize(a).ok())
    })?;
    while !p.exists() {
        if !p.pop() {
            return None;
        }
    }
    fs4::available_space(&p).ok()
}

// ----------------------------------------------------------------- session log

pub fn session_log(line: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(SESSION_LOG) {
        let _ = writeln!(f, "{} {line}", Utc::now().format("%Y-%m-%dT%H:%M:%SZ"));
    }
}

pub fn session_start() {
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(SESSION_LOG) {
        let _ = writeln!(f, "\n--- Session started {} UTC ---", Utc::now().format("%Y-%m-%dT%H:%M:%S"));
    }
}

// ------------------------------------------------------------------- URL files

pub fn backup_urls_file(file: &Path) {
    let dir = Path::new(BACKUP_FOLDER);
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let ts = Utc::now().format("%d%m%Y_%H%M%S");
    let _ = std::fs::copy(file, dir.join(format!("URLs_{ts}.bak")));
}

pub fn read_urls(file: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_split() {
        assert_eq!(split_ext("a.tar.gz"), ("a.tar", ".gz"));
        assert_eq!(split_ext(".bashrc"), (".bashrc", ""));
        assert_eq!(split_ext("noext"), ("noext", ""));
    }

    #[test]
    fn sanitizes_and_truncates() {
        assert_eq!(truncate_filename("a<b>:c.mp4"), "abc.mp4");
        let long = format!("{}.mp4", "x".repeat(300));
        let t = truncate_filename(&long);
        assert_eq!(t.chars().count(), MAX_FILENAME_LEN + 0);
        assert!(t.ends_with(".mp4"));
    }

    #[test]
    fn unique_names() {
        let mut r = HashSet::new();
        for expect in ["a.jpg", "a (1).jpg", "a (2).jpg"] {
            let n = reserve_unique_filename("a.jpg", &r);
            assert_eq!(n, expect);
            r.insert(n);
        }
    }

    #[test]
    fn filters() {
        assert!(matches_ignore("x.zip", &[".zip".into()]));
        assert!(!fails_include("movie.mp4", &["mp4".into(), "mkv".into()]));
        assert!(fails_include("a.jpg", &["mp4".into()]));
        assert!(!fails_include("a.jpg", &[]));
    }

    #[test]
    fn dir_names() {
        assert_eq!(format_directory_name(Some("Pics"), Some("abc")).unwrap(), "Pics (abc)");
        assert_eq!(format_directory_name(None, Some("abc")).unwrap(), "abc");
        assert_eq!(format_directory_name(None, None), None);
    }
}
