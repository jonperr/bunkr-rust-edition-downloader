//! Per-album sidecar state (`.bunkr_state.json`), compatible with the Python version.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::STATE_FILE;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ItemState {
    pub filename: String,
    pub status: String, // "completed" | "failed"
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct AlbumState {
    pub album_id: String,
    #[serde(default)]
    pub item_pages: Vec<String>,
    #[serde(default)]
    pub items: HashMap<String, ItemState>,
    #[serde(default)]
    pub item_dates: HashMap<String, Option<String>>,
}

pub fn load(dir: &Path) -> Option<AlbumState> {
    let text = std::fs::read_to_string(dir.join(STATE_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn parse_dates(map: &HashMap<String, Option<String>>) -> HashMap<String, DateTime<Utc>> {
    map.iter()
        .filter_map(|(k, v)| {
            let d = DateTime::parse_from_rfc3339(v.as_deref()?).ok()?;
            Some((k.clone(), d.with_timezone(&Utc)))
        })
        .collect()
}

/// Thread-safe, write-throttled holder of the album state.
pub struct StateStore {
    path: PathBuf,
    inner: Mutex<(AlbumState, Instant, bool)>, // state, last save, dirty
}

impl StateStore {
    pub fn new(dir: &Path, state: AlbumState) -> Self {
        Self {
            path: dir.join(STATE_FILE),
            inner: Mutex::new((state, Instant::now() - Duration::from_secs(60), true)),
        }
    }

    pub fn item(&self, page: &str) -> Option<ItemState> {
        self.inner.lock().unwrap().0.items.get(page).cloned()
    }

    pub fn set_item(&self, page: &str, filename: &str, completed: bool) {
        let mut g = self.inner.lock().unwrap();
        g.0.items.insert(
            page.to_string(),
            ItemState { filename: filename.into(), status: if completed { "completed" } else { "failed" }.into() },
        );
        g.2 = true;
        if g.1.elapsed() >= Duration::from_secs(1) {
            Self::write(&self.path, &mut g);
        }
    }

    /// Write now if there are unsaved changes.
    pub fn flush(&self) {
        let mut g = self.inner.lock().unwrap();
        if g.2 {
            Self::write(&self.path, &mut g);
        }
    }

    fn write(path: &Path, g: &mut (AlbumState, Instant, bool)) {
        let Ok(bytes) = serde_json::to_vec(&g.0) else { return };
        let tmp = path.with_extension("json.tmp");
        // Write + rename so a crash never leaves a truncated state file behind.
        if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, path).is_ok() {
            g.1 = Instant::now();
            g.2 = false;
        }
    }
}

impl Drop for StateStore {
    fn drop(&mut self) {
        // Cancelled runs (GUI "Cancelar Tudo") still persist what they finished.
        if let Ok(mut g) = self.inner.lock() {
            if g.2 {
                Self::write(&self.path, &mut g);
            }
        }
    }
}
