//! Download engine.
//!
//! * Large files with Range support are split into work units fetched by up to N parallel
//!   connections (work-stealing through a semaphore). Every unit is written **directly at
//!   its offset** in a preallocated `.temp` file, so there is no `.partN` merge pass.
//!   A tiny `.bunkrparts` sidecar records finished units, giving resume across runs.
//! * A failed unit resumes from the last byte it persisted instead of restarting.
//! * Everything else streams into `.temp` (resuming with `Range: bytes=N-` when possible).
//! * The finished file is atomically renamed into place.

use futures_util::StreamExt;
use indicatif::ProgressBar;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::config::*;
use crate::fsutil::append_suffix;
use crate::ratelimit::RateLimiter;

#[derive(Debug, Clone)]
pub enum DlError {
    /// Could not connect / host reported 503 or 521.
    ServerDown(String),
    RateLimited,
    BadGateway,
    Http(u16),
    Net(String),
    Io(String),
    Incomplete,
    /// Server ignored the Range header.
    NoRange,
}

impl std::fmt::Display for DlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DlError::ServerDown(e) => write!(f, "server unreachable ({e})"),
            DlError::RateLimited => write!(f, "rate limited (HTTP 429)"),
            DlError::BadGateway => write!(f, "bad gateway (HTTP 502)"),
            DlError::Http(c) => write!(f, "HTTP {c}"),
            DlError::Net(e) => write!(f, "network error ({e})"),
            DlError::Io(e) => write!(f, "disk error ({e})"),
            DlError::Incomplete => write!(f, "incomplete transfer"),
            DlError::NoRange => write!(f, "server ignored Range request"),
        }
    }
}

impl DlError {
    fn retriable(&self) -> bool {
        matches!(self, DlError::RateLimited | DlError::Net(_) | DlError::Incomplete | DlError::ServerDown(_))
    }
}

fn from_status(code: u16) -> DlError {
    match code {
        429 => DlError::RateLimited,
        502 => DlError::BadGateway,
        503 | 521 => DlError::ServerDown(format!("HTTP {code}")),
        c => DlError::Http(c),
    }
}

fn classify(e: reqwest::Error) -> DlError {
    if let Some(s) = e.status() {
        return from_status(s.as_u16());
    }
    if e.is_connect() {
        DlError::ServerDown(e.to_string())
    } else {
        DlError::Net(e.to_string())
    }
}

fn io(e: std::io::Error) -> DlError {
    DlError::Io(e.to_string())
}

#[derive(Serialize, Deserialize)]
struct Plan {
    content_length: u64,
    ranges: Vec<(u64, u64)>,
    done: Vec<bool>,
}

impl Plan {
    fn new(len: u64, connections: usize) -> Self {
        let target = (connections as u64 * UNITS_PER_CONNECTION).max(1);
        let unit = (len / target).clamp(MIN_WORK_UNIT, MAX_WORK_UNIT).max(1);
        let n = len.div_ceil(unit);
        let ranges: Vec<(u64, u64)> = (0..n)
            .map(|i| (i * unit, if i + 1 < n { (i + 1) * unit - 1 } else { len - 1 }))
            .collect();
        Self { content_length: len, done: vec![false; ranges.len()], ranges }
    }

    fn done_bytes(&self) -> u64 {
        self.ranges.iter().zip(&self.done).filter(|(_, d)| **d).map(|((s, e), _)| e - s + 1).sum()
    }

    fn save(&self, path: &Path) {
        if let Ok(b) = serde_json::to_vec(self) {
            let _ = std::fs::write(path, b);
        }
    }
}

#[derive(Clone)]
pub struct Engine {
    pub client: Client,
    pub limiter: Option<Arc<RateLimiter>>,
    pub connections: usize,
}

impl Engine {
    /// Detect Range support and size with a HEAD request.
    pub async fn probe(&self, url: &str) -> (bool, Option<u64>) {
        match self.client.head(url).timeout(Duration::from_secs(10)).send().await {
            Ok(r) if r.status().is_success() => {
                let h = r.headers();
                let ranges = h
                    .get(ACCEPT_RANGES)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.eq_ignore_ascii_case("bytes"));
                let len = h.get(CONTENT_LENGTH).and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
                (ranges, len)
            }
            _ => (false, None),
        }
    }

    /// Download `url` to `dest` (one attempt; the caller owns the retry policy).
    pub async fn download(&self, url: &str, dest: &Path, bar: &ProgressBar) -> Result<(), DlError> {
        let (ranges, len) = self.probe(url).await;
        if let Some(l) = len {
            if ranges && self.connections > 1 && l >= MIN_PARALLEL_SIZE {
                match self.chunked(url, dest, l, bar).await {
                    Err(DlError::NoRange) => {
                        let _ = fs::remove_file(append_suffix(dest, ".temp")).await;
                        let _ = fs::remove_file(append_suffix(dest, ".bunkrparts")).await;
                        bar.set_position(0);
                    }
                    other => return other,
                }
            }
        }
        self.single(url, dest, ranges, len, bar).await
    }

    // ------------------------------------------------------------ chunked path

    async fn chunked(&self, url: &str, dest: &Path, len: u64, bar: &ProgressBar) -> Result<(), DlError> {
        let temp = append_suffix(dest, ".temp");
        let plan_path = append_suffix(dest, ".bunkrparts");

        let existing: Option<Plan> = std::fs::read(&plan_path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Plan>(&b).ok())
            .filter(|p| p.content_length == len && p.done.len() == p.ranges.len());
        let temp_ok = fs::metadata(&temp).await.map(|m| m.len() == len).unwrap_or(false);

        let plan = match existing {
            Some(p) if temp_ok => p,
            _ => {
                let f = File::create(&temp).await.map_err(io)?;
                f.set_len(len).await.map_err(io)?;
                let p = Plan::new(len, self.connections);
                p.save(&plan_path);
                p
            }
        };

        bar.set_length(len);
        bar.set_position(plan.done_bytes());

        let pending: Vec<usize> = (0..plan.ranges.len()).filter(|i| !plan.done[*i]).collect();
        let plan = Arc::new(Mutex::new(plan));
        let sem = Arc::new(Semaphore::new(self.connections));
        let mut set: JoinSet<Result<(), DlError>> = JoinSet::new();

        for idx in pending {
            let (sem, plan, engine) = (sem.clone(), plan.clone(), self.clone());
            let (url, temp, plan_path, bar) = (url.to_string(), temp.clone(), plan_path.clone(), bar.clone());
            set.spawn(async move {
                let _permit = sem.acquire_owned().await.map_err(|e| DlError::Net(e.to_string()))?;
                let (start, end) = plan.lock().unwrap().ranges[idx];
                engine.unit(&url, &temp, start, end, &bar).await?;
                let mut p = plan.lock().unwrap();
                p.done[idx] = true;
                p.save(&plan_path);
                Ok(())
            });
        }

        let mut first_err: Option<DlError> = None;
        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    if first_err.is_none() {
                        first_err = Some(e);
                        set.abort_all(); // stop wasting bandwidth; finished units stay recorded
                    }
                }
                Err(_) => {} // cancelled by abort_all
            }
        }
        if let Some(e) = first_err {
            return Err(e);
        }
        if !plan.lock().unwrap().done.iter().all(|d| *d) {
            return Err(DlError::Incomplete);
        }

        fs::rename(&temp, dest).await.map_err(io)?;
        let _ = fs::remove_file(&plan_path).await;
        Ok(())
    }

    /// Download bytes `start..=end` into `temp` at their own offset, resuming from the last
    /// persisted byte on transient errors.
    async fn unit(&self, url: &str, temp: &Path, start: u64, end: u64, bar: &ProgressBar) -> Result<(), DlError> {
        let mut offset = start;
        let mut last = DlError::Incomplete;
        for attempt in 1..=CHUNK_MAX_RETRIES {
            match self.unit_attempt(url, temp, &mut offset, end, bar).await {
                Ok(()) => return Ok(()),
                Err(e) if e.retriable() => {
                    last = e;
                    if attempt < CHUNK_MAX_RETRIES {
                        let d = CHUNK_BASE_DELAY * 2f64.powi(attempt as i32 - 1) + rand::random::<f64>();
                        tokio::time::sleep(Duration::from_secs_f64(d)).await;
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Err(last)
    }

    async fn unit_attempt(&self, url: &str, temp: &Path, offset: &mut u64, end: u64, bar: &ProgressBar) -> Result<(), DlError> {
        if *offset > end {
            return Ok(());
        }
        let resp = self
            .client
            .get(url)
            .header(RANGE, format!("bytes={}-{}", *offset, end))
            .send()
            .await
            .map_err(classify)?;
        let status = resp.status();
        if status.as_u16() != 206 {
            return Err(if status.is_success() { DlError::NoRange } else { from_status(status.as_u16()) });
        }

        let mut file = OpenOptions::new().write(true).open(temp).await.map_err(io)?;
        file.seek(SeekFrom::Start(*offset)).await.map_err(io)?;

        let mut buf: Vec<u8> = Vec::with_capacity(WRITE_BUF);
        let mut stream = resp.bytes_stream();
        let result: Result<(), DlError> = async {
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(classify)?;
                if *offset + (buf.len() + chunk.len()) as u64 > end + 1 {
                    return Err(DlError::Io("server sent more data than requested".into()));
                }
                if let Some(l) = &self.limiter {
                    l.consume(chunk.len()).await;
                }
                bar.inc(chunk.len() as u64);
                buf.extend_from_slice(&chunk);
                if buf.len() >= WRITE_BUF {
                    file.write_all(&buf).await.map_err(io)?;
                    *offset += buf.len() as u64;
                    buf.clear();
                }
            }
            Ok(())
        }
        .await;

        match result {
            Ok(()) => {
                file.write_all(&buf).await.map_err(io)?;
                file.flush().await.map_err(io)?;
                *offset += buf.len() as u64;
                if *offset == end + 1 { Ok(()) } else { Err(DlError::Incomplete) }
            }
            Err(e) => {
                // Bytes still in RAM were never persisted: take them back off the progress bar.
                bar.dec(buf.len() as u64);
                let _ = file.flush().await;
                Err(e)
            }
        }
    }

    // ------------------------------------------------------------- single path

    async fn single(&self, url: &str, dest: &Path, ranges: bool, known_len: Option<u64>, bar: &ProgressBar) -> Result<(), DlError> {
        let temp: PathBuf = append_suffix(dest, ".temp");
        let plan_path = append_suffix(dest, ".bunkrparts");
        if fs::metadata(&plan_path).await.is_ok() {
            // Leftovers of a chunked attempt (preallocated file): not resumable as a stream.
            let _ = fs::remove_file(&temp).await;
            let _ = fs::remove_file(&plan_path).await;
        }

        let existing = fs::metadata(&temp).await.map(|m| m.len()).unwrap_or(0);
        let mut start = 0u64;
        let mut req = self.client.get(url);
        if ranges && existing > 0 && known_len.is_some_and(|l| existing < l) {
            start = existing;
            req = req.header(RANGE, format!("bytes={start}-"));
        }

        let resp = req.send().await.map_err(classify)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(from_status(status.as_u16()));
        }
        if start > 0 && status.as_u16() != 206 {
            start = 0; // server ignored the Range header: start over
        }
        let total = resp.content_length().map(|l| l + start).or(known_len);

        let mut file = if start > 0 {
            OpenOptions::new().append(true).open(&temp).await.map_err(io)?
        } else {
            File::create(&temp).await.map_err(io)?
        };
        bar.set_length(total.unwrap_or(0));
        bar.set_position(start);

        let mut written = start;
        let mut buf: Vec<u8> = Vec::with_capacity(WRITE_BUF);
        let mut stream = resp.bytes_stream();
        let mut failure: Option<DlError> = None;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(c) => {
                    if let Some(l) = &self.limiter {
                        l.consume(c.len()).await;
                    }
                    bar.inc(c.len() as u64);
                    buf.extend_from_slice(&c);
                    if buf.len() >= WRITE_BUF {
                        file.write_all(&buf).await.map_err(io)?;
                        written += buf.len() as u64;
                        buf.clear();
                    }
                }
                Err(e) => {
                    failure = Some(classify(e));
                    break;
                }
            }
        }
        // Persist whatever we received, even on failure, so the next attempt can resume.
        file.write_all(&buf).await.map_err(io)?;
        file.flush().await.map_err(io)?;
        written += buf.len() as u64;
        drop(file);

        if let Some(e) = failure {
            return Err(e);
        }
        if total.is_some_and(|t| written != t) {
            return Err(DlError::Incomplete);
        }
        fs::rename(&temp, dest).await.map_err(io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_covers_file_exactly() {
        for len in [16 * MB, 64 * MB + 1, 1 * GB + 12345, 10 * GB] {
            let p = Plan::new(len, 8);
            assert_eq!(p.ranges.first().unwrap().0, 0);
            assert_eq!(p.ranges.last().unwrap().1, len - 1);
            for w in p.ranges.windows(2) {
                assert_eq!(w[0].1 + 1, w[1].0);
            }
            let total: u64 = p.ranges.iter().map(|(s, e)| e - s + 1).sum();
            assert_eq!(total, len);
        }
    }
}
