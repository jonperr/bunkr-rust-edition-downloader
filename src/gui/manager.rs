//! Bridge between the GUI and the download core: a persistent queue (like the Qt
//! version's DownloadManager). URLs can be added at any moment, bounded by the
//! "albums at once" limit; "Cancelar Tudo" aborts everything immediately.

use std::sync::{Arc, Mutex};

use bunkr_core::album::{process_url, Run, UrlResult};
use bunkr_core::config::KB;
use bunkr_core::engine::Engine;
use bunkr_core::net::Net;
use bunkr_core::ratelimit::RateLimiter;
use bunkr_core::ui::{LogBuf, Status, Ui};
use tokio::sync::Semaphore;
use tokio::task::AbortHandle;

use super::settings::GuiSettings;

/// The expensive-to-build parts (thread pool + TLS/HTTP clients). Built on a background
/// thread right after the first frame so that the first click on "Baixar" is instant.
pub struct Backend {
    pub rt: tokio::runtime::Runtime,
    pub net: Net,
}

pub fn build_backend() -> Result<Backend, String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("bunkr-worker")
        .build()
        .map_err(|e| format!("não foi possível iniciar o motor de download: {e}"))?;
    let net = Net::new().map_err(|e| format!("não foi possível iniciar o cliente HTTP: {e:#}"))?;
    Ok(Backend { rt, net })
}

pub struct Manager {
    backend: Backend,
    limiter: Option<Arc<RateLimiter>>,
    sem: Arc<Semaphore>,
    tasks: Mutex<Vec<(AbortHandle, Arc<Ui>)>>,
}

impl Manager {
    /// Concurrency and the speed limit are fixed here (first download), like the original.
    pub fn new(backend: Backend, s: &GuiSettings) -> Self {
        Self {
            backend,
            limiter: (s.rate_limit_kbps > 0.0).then(|| Arc::new(RateLimiter::new(s.rate_limit_kbps * KB as f64))),
            sem: Arc::new(Semaphore::new(s.max_concurrent_albums.max(1) as usize)),
            tasks: Mutex::new(Vec::new()),
        }
    }

    pub fn add(&self, url: String, s: &GuiSettings, ui: Arc<Ui>, log: Arc<LogBuf>) {
        let cfg = s.to_backend();
        let (net, sem, limiter) = (self.backend.net.clone(), self.sem.clone(), self.limiter.clone());
        let ui2 = ui.clone();
        let handle = self.backend.rt.spawn(async move {
            let _permit = sem.acquire_owned().await.ok();
            let run = Arc::new(Run {
                engine: Engine { client: net.dl.clone(), limiter, connections: cfg.connections },
                net,
                cfg,
            });
            let res = process_url(&run, &ui2, &url).await;
            match res {
                UrlResult::Ok => ui2.set_status(Status::Ok),
                UrlResult::Failed => ui2.set_status(Status::Warn),
                UrlResult::Error(m) | UrlResult::DiskFull(m) => {
                    log.push(format!("{}: {m} ({url})", if bunkr_core::i18n::Lang::detect() == bunkr_core::i18n::Lang::Pt { "Erro" } else { "Error" }));
                    ui2.set_note(m);
                    ui2.set_status(Status::Error);
                }
            }
        });
        let mut t = self.tasks.lock().unwrap();
        t.retain(|(h, _)| !h.is_finished());
        t.push((handle.abort_handle(), ui));
    }

    pub fn cancel_all(&self) {
        for (h, ui) in self.tasks.lock().unwrap().drain(..) {
            if h.is_finished() {
                continue;
            }
            h.abort();
            let mut v = ui.view.lock().unwrap();
            for b in v.bars.drain(..) {
                b.finish_and_clear();
            }
            v.status = Status::Cancelled;
            v.note = Some("Cancelado.".into());
        }
    }

    pub fn shutdown(self) {
        self.cancel_all();
        self.backend.rt.shutdown_background();
    }
}
