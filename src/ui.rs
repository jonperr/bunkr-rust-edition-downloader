//! Terminal UI: live progress bars when attached to a TTY, plain log lines otherwise,
//! plus the end-of-run summary counters.

use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Pending,
    Running,
    Ok,
    Warn,
    Error,
    Cancelled,
}

/// Bounded, shared log used by the GUI.
#[derive(Default)]
pub struct LogBuf {
    lines: Mutex<VecDeque<String>>,
    version: AtomicU64,
}

impl LogBuf {
    pub fn push(&self, line: String) {
        let mut l = self.lines.lock().unwrap();
        if l.len() >= 2000 {
            l.pop_front();
        }
        l.push_back(line);
        self.version.fetch_add(1, Ordering::Relaxed);
    }
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }
    pub fn snapshot(&self) -> String {
        self.lines.lock().unwrap().iter().map(String::as_str).collect::<Vec<_>>().join("\n")
    }
}

/// Live state of one URL, read by the GUI every frame (nothing is pushed through channels).
#[derive(Default)]
pub struct View {
    pub status: Status,
    pub album_name: Option<String>,
    pub path: Option<String>,
    pub note: Option<String>,
    pub files_total: u64,
    pub files_done: u64,
    pub bars: Vec<ProgressBar>,
    pub done_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Completed = 0,
    Failed,
    AlreadyDownloaded,
    IgnoreList,
    IncludeList,
    DomainOffline,
    ServiceUnavailable,
}

impl Reason {
    pub fn code(self) -> &'static str {
        match self {
            Reason::Completed => "COMPLETED",
            Reason::Failed => "FAILED",
            Reason::AlreadyDownloaded => "SKIPPED:ALREADY_DOWNLOADED",
            Reason::IgnoreList => "SKIPPED:IGNORE_LIST",
            Reason::IncludeList => "SKIPPED:INCLUDE_LIST",
            Reason::DomainOffline => "SKIPPED:DOMAIN_OFFLINE",
            Reason::ServiceUnavailable => "SKIPPED:SERVICE_UNAVAILABLE",
        }
    }
}

pub struct Ui {
    mp: Option<MultiProgress>,
    gui_log: Option<Arc<LogBuf>>,
    pub view: Mutex<View>,
    overall: Mutex<Option<ProgressBar>>,
    counts: [AtomicU64; 7],
    start: Instant,
}

impl Ui {
    /// `live` requests the progress-bar UI; it is only honoured when stderr is a terminal.
    pub fn new(live: bool) -> Self {
        let live = live && console::Term::stderr().is_term();
        Self {
            mp: live.then(MultiProgress::new),
            gui_log: None,
            view: Mutex::new(View::default()),
            overall: Mutex::new(None),
            counts: Default::default(),
            start: Instant::now(),
        }
    }

    /// GUI mode: no terminal output; events go to `log` and the per-URL `view`.
    pub fn gui(log: Arc<LogBuf>) -> Self {
        Self {
            mp: None,
            gui_log: Some(log),
            view: Mutex::new(View::default()),
            overall: Mutex::new(None),
            counts: Default::default(),
            start: Instant::now(),
        }
    }

    pub fn is_live(&self) -> bool {
        self.mp.is_some() || self.gui_log.is_some()
    }

    pub fn set_status(&self, st: Status) {
        self.view.lock().unwrap().status = st;
    }

    pub fn set_note(&self, note: impl Into<String>) {
        self.view.lock().unwrap().note = Some(note.into());
    }

    pub fn set_album(&self, name: Option<String>, path: String) {
        let mut v = self.view.lock().unwrap();
        v.album_name = name;
        v.path = Some(path);
    }

    pub fn add_done_bytes(&self, n: u64) {
        if self.gui_log.is_some() {
            self.view.lock().unwrap().done_bytes += n;
        }
    }

    pub fn log(&self, event: &str, details: &str) {
        if let Some(l) = &self.gui_log {
            l.push(format!("{event}: {details}"));
            return;
        }
        let ts = chrono::Local::now().format("%H:%M:%S");
        let line = format!("{} {} {}", style(ts).dim(), style(event).cyan().bold(), style(details).dim());
        match &self.mp {
            Some(mp) => {
                let _ = mp.println(line);
            }
            None => println!("{line}"),
        }
    }

    pub fn record(&self, r: Reason) {
        self.counts[r as usize].fetch_add(1, Ordering::Relaxed);
    }

    pub fn start_overall(&self, label: &str, total: u64) {
        if self.gui_log.is_some() {
            let mut v = self.view.lock().unwrap();
            v.files_total = total;
            v.files_done = 0;
            return;
        }
        let Some(mp) = &self.mp else { return };
        let mut g = self.overall.lock().unwrap();
        if let Some(old) = g.take() {
            old.finish_and_clear();
        }
        let pb = mp.add(ProgressBar::new(total));
        pb.set_style(
            ProgressStyle::with_template("{prefix:.bold} [{bar:40.green/dim}] {pos}/{len} items • {elapsed_precise}")
                .unwrap()
                .progress_chars("█▉▊▋▌▍▎▏ "),
        );
        pb.set_prefix(label.to_string());
        pb.enable_steady_tick(Duration::from_millis(250));
        *g = Some(pb);
    }

    pub fn overall_inc(&self) {
        if self.gui_log.is_some() {
            self.view.lock().unwrap().files_done += 1;
            return;
        }
        if let Some(pb) = self.overall.lock().unwrap().as_ref() {
            pb.inc(1);
        }
    }

    pub fn finish_overall(&self) {
        if let Some(pb) = self.overall.lock().unwrap().take() {
            pb.finish_and_clear();
        }
    }

    /// A per-file byte progress bar (hidden in plain-log mode).
    pub fn file_bar(&self, name: &str) -> ProgressBar {
        if self.gui_log.is_some() {
            // Hidden bars still track position/length; the GUI polls them each frame.
            let pb = ProgressBar::hidden();
            pb.set_message(name.to_string());
            let mut v = self.view.lock().unwrap();
            v.bars.retain(|b| !b.is_finished());
            v.bars.push(pb.clone());
            return pb;
        }
        let Some(mp) = &self.mp else { return ProgressBar::hidden() };
        let pb = ProgressBar::new(0);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.cyan} {msg:<38!} [{bar:24.cyan/blue}] {binary_bytes:>10}/{binary_total_bytes:<10} {binary_bytes_per_sec:>12} eta {eta}",
            )
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏ "),
        );
        pb.set_message(name.to_string());
        let pb = match self.overall.lock().unwrap().as_ref() {
            Some(o) => mp.insert_before(o, pb),
            None => mp.add(pb),
        };
        pb.enable_steady_tick(Duration::from_millis(120));
        pb
    }

    pub fn count(&self, r: Reason) -> u64 {
        self.counts[r as usize].load(Ordering::Relaxed)
    }

    pub fn print_summary(&self) {
        self.finish_overall();
        let c = |r| self.count(r);
        let skipped = c(Reason::AlreadyDownloaded) + c(Reason::IgnoreList) + c(Reason::IncludeList) + c(Reason::DomainOffline) + c(Reason::ServiceUnavailable);
        let total = c(Reason::Completed) + c(Reason::Failed) + skipped;
        if total == 0 {
            return;
        }
        let out = format!(
            "\n{}  {} completed • {} failed • {} skipped  (already: {}, ignore: {}, include: {}, offline: {}, unavailable: {})  in {:.1}s",
            style("Summary").bold(),
            style(c(Reason::Completed)).green(),
            if c(Reason::Failed) > 0 { style(c(Reason::Failed)).red() } else { style(c(Reason::Failed)) },
            style(skipped).yellow(),
            c(Reason::AlreadyDownloaded),
            c(Reason::IgnoreList),
            c(Reason::IncludeList),
            c(Reason::DomainOffline),
            c(Reason::ServiceUnavailable),
            self.start.elapsed().as_secs_f64(),
        );
        match &self.mp {
            Some(mp) => {
                let _ = mp.println(out);
            }
            None => println!("{out}"),
        }
    }
}
