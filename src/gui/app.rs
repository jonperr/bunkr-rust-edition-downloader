//! The main window. Layout and texts mirror gui/main_window.py of the Qt version.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bunkr_core::ui::{LogBuf, Reason, Status, Ui};
use eframe::egui::{self, vec2, Align, Color32, CursorIcon, FontId, Layout, RichText, Sense, Stroke, TextEdit, ViewportCommand};

use bunkr_core::i18n::{t, Lang};
use super::manager::{build_backend, Backend, Manager};
use super::settings::GuiSettings;
use super::sys;
use super::theme::{self, Palette, DARK, LIGHT};
use super::widgets::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Opts {
    pub shot: Option<PathBuf>,
    pub shot_after_ms: u64,
    pub bench: bool,
    pub urls: Vec<String>,
    pub dest: Option<String>,
    pub expand: bool,
    pub log: bool,
    pub cancel_after_ms: Option<u64>,
    pub t0: Instant,
}

struct Row {
    url: String,
    ui: Arc<Ui>,
    speed: f64,
    last_t: Instant,
    last_bytes: u64,
}

struct Snap {
    status: Status,
    title: String,
    note: String,
    files_total: u64,
    files_done: u64,
    active_pct: Option<f32>,
    speed: f64,
    counts: (u64, u64, u64),
}

pub struct App {
    l: Lang,
    s: GuiSettings,
    p: Palette,
    url_text: String,
    rows: Vec<Row>,
    log: Arc<LogBuf>,
    log_cache: (u64, String),
    warm: Arc<Mutex<Option<Result<Backend, String>>>>,
    manager: Option<Manager>,
    settings_open: bool,
    log_open: bool,
    dialog: Option<(String, String)>,
    frame: u32,
    opts: Opts,
    shot_sent: bool,
    autostarted: bool,
}

fn classify(raw: &str) -> (String, bool) {
    let text = raw.trim();
    if text.is_empty() {
        return (text.into(), false);
    }
    let cand = if text.starts_with("http://") || text.starts_with("https://") { text.to_string() } else { format!("https://{text}") };
    let Ok(u) = url::Url::parse(&cand) else { return (text.into(), false) };
    let host = u.host_str().unwrap_or("").to_lowercase();
    // BUNKR_GUI_ANYHOST lets tests point the GUI at a local mirror.
    if !host.contains("bunkr") && std::env::var_os("BUNKR_GUI_ANYHOST").is_none() {
        return (text.into(), false);
    }
    let segs: Vec<&str> = u.path().split('/').filter(|s| !s.is_empty()).collect();
    if segs.len() < 2 || !matches!(segs[segs.len() - 2].to_lowercase().as_str(), "a" | "f" | "i" | "v") {
        return (text.into(), false);
    }
    (cand, true)
}

fn split_urls(raw: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    raw.split_whitespace().filter(|s| seen.insert(s.to_string())).map(str::to_owned).collect()
}

fn fmt_speed(bps: f64) -> String {
    if bps >= 1024.0 * 1024.0 { format!("{:.1} MB/s", bps / 1048576.0) } else { format!("{:.0} KB/s", bps / 1024.0) }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, opts: Opts) -> Self {
        let mut s = GuiSettings::load();
        if let Some(d) = &opts.dest {
            s.destination = d.clone();
        }
        let p = if s.dark_mode { DARK } else { LIGHT };
        theme::apply(&cc.egui_ctx, &p);
        if opts.bench {
            println!("window + GL context ready, App::new at {:.1} ms", opts.t0.elapsed().as_secs_f64() * 1000.0);
        }
        let expand = opts.expand;
        let log = opts.log;
        Self {
            l: Lang::detect(),
            s,
            p,
            url_text: opts.urls.join("\n"),
            rows: Vec::new(),
            log: Arc::new(LogBuf::default()),
            log_cache: (u64::MAX, String::new()),
            warm: Arc::new(Mutex::new(None)),
            manager: None,
            settings_open: expand,
            log_open: expand || log,
            dialog: None,
            frame: 0,
            opts,
            shot_sent: false,
            autostarted: false,
        }
    }

    fn append_log(&self, t: String) {
        self.log.push(t);
    }

    fn set_theme(&mut self, ctx: &egui::Context) {
        self.p = if self.s.dark_mode { DARK } else { LIGHT };
        theme::apply(ctx, &self.p);
        ctx.send_viewport_cmd(ViewportCommand::SetTheme(if self.s.dark_mode { egui::SystemTheme::Dark } else { egui::SystemTheme::Light }));
        self.s.save();
    }

    fn manager(&mut self) -> Option<&Manager> {
        if self.manager.is_none() {
            let ready = self.warm.lock().unwrap().take();
            let backend = match ready {
                Some(r) => r,
                None => build_backend(), // user was faster than the warm-up thread
            };
            match backend {
                Ok(b) => self.manager = Some(Manager::new(b, &self.s)),
                Err(e) => {
                    self.dialog = Some((t(self.l, "Erro", "Error").into(), e));
                    return None;
                }
            }
        }
        self.manager.as_ref()
    }

    fn submit_links(&mut self) {
        let (mut valid, mut remaining, mut invalid) = (Vec::new(), Vec::new(), 0);
        for line in self.url_text.lines() {
            let st = line.trim();
            if st.is_empty() {
                remaining.push(line.to_string());
                continue;
            }
            let (n, ok) = classify(st);
            if ok {
                valid.push(n);
            } else {
                invalid += 1;
                remaining.push(line.to_string());
            }
        }
        if valid.is_empty() {
            self.dialog = Some((t(self.l, "Nenhum link válido", "No valid links").into(), t(self.l, "Cole ao menos um link válido do Bunkr (ex.: https://bunkr.si/a/XXXXXXXX).", "Paste at least one valid Bunkr link (e.g. https://bunkr.si/a/XXXXXXXX).").into()));
            return;
        }
        let dest = self.s.destination.trim().to_string();
        if dest.is_empty() {
            self.dialog = Some((t(self.l, "Pasta de destino", "Destination folder").into(), "Escolha uma pasta de destino antes de baixar.".into()));
            return;
        }
        let _ = std::fs::create_dir_all(&dest);
        self.s.save();
        self.url_text = remaining.join("\n").trim().to_string();
        if invalid > 0 {
            self.append_log(format!("{invalid} invalid link(s) were ignored."));
        }
        self.submit_urls(valid);
    }

    fn submit_urls(&mut self, urls: Vec<String>) {
        if self.manager().is_none() {
            return;
        }
        let s = self.s.clone();
        for url in urls {
            let existing = self.rows.iter().position(|r| r.url == url);
            if let Some(i) = existing {
                let st = self.rows[i].ui.view.lock().unwrap().status;
                if matches!(st, Status::Pending | Status::Running) {
                    self.append_log(format!("Already queued: {}", elide(&url, 60)));
                    continue;
                }
            }
            let ui = Arc::new(Ui::gui(self.log.clone()));
            match existing {
                Some(i) => {
                    let r = &mut self.rows[i];
                    r.ui = ui.clone();
                    r.speed = 0.0;
                    r.last_bytes = 0;
                }
                None => self.rows.push(Row { url: url.clone(), ui: ui.clone(), speed: 0.0, last_t: Instant::now(), last_bytes: 0 }),
            }
            self.manager.as_ref().unwrap().add(url, &s, ui, self.log.clone());
        }
    }

    fn snapshot(r: &mut Row) -> Snap {
        let v = r.ui.view.lock().unwrap();
        let (mut pos, mut len, mut active) = (0u64, 0u64, 0usize);
        let mut bytes = v.done_bytes;
        for b in v.bars.iter().filter(|b| !b.is_finished()) {
            active += 1;
            pos += b.position();
            len += b.length().unwrap_or(0);
            bytes += b.position();
        }
        let now = Instant::now();
        let dt = now.duration_since(r.last_t).as_secs_f64();
        if dt >= 0.4 {
            let inst = bytes.saturating_sub(r.last_bytes) as f64 / dt;
            r.speed = if r.speed == 0.0 { inst } else { r.speed * 0.6 + inst * 0.4 };
            r.last_t = now;
            r.last_bytes = bytes;
        }
        if v.status != Status::Running {
            r.speed = 0.0;
        }
        let title = v.album_name.clone().unwrap_or_else(|| r.url.clone());
        let note = v.note.clone().or_else(|| v.path.clone()).unwrap_or_else(|| r.url.clone());
        let c = |x| r.ui.count(x);
        let counts = (
            c(Reason::Completed),
            c(Reason::AlreadyDownloaded) + c(Reason::IgnoreList) + c(Reason::IncludeList) + c(Reason::DomainOffline) + c(Reason::ServiceUnavailable),
            c(Reason::Failed),
        );
        Snap {
            status: v.status,
            title,
            note,
            files_total: v.files_total,
            files_done: v.files_done,
            active_pct: (active > 0 && len > 0).then(|| pos as f32 / len as f32),
            speed: r.speed,
            counts,
        }
    }

    // ------------------------------------------------------------------ UI parts

    fn app_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let p = self.p;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            icon_badge(ui, &p, 40.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new("Bunkr Downloader").size(19.0).color(p.on_surface).strong());
                ui.label(RichText::new(format!("{} • v{VERSION}", if self.l == Lang::Pt { "GUI nativa em Rust" } else { "Native Rust GUI" })).size(12.0).color(p.muted));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut dark = self.s.dark_mode;
                if switch(ui, &p, &mut dark, t(self.l, "Modo escuro", "Dark mode")) {
                    self.s.dark_mode = dark;
                    self.set_theme(ctx);
                }
            });
        });
    }

    fn url_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p;
        card(ui, &p, |ui| {
            let mut paste = false;
            card_header(ui, &p, Icon::Link, t(self.l, "Adicionar Links", "Add Links"), Some(t(self.l, "Lista de URLs", "URL list")), |ui| {
                paste = button(ui, &p, Some(Icon::Clipboard), t(self.l, "Colar Links", "Paste Links"), BtnKind::Tonal, false, true).clicked();
            });
            if paste {
                if let Some(t) = sys::clipboard_text().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
                    self.url_text = if self.url_text.trim().is_empty() { t } else { format!("{}\n{t}", self.url_text.trim_end()) };
                }
            }
            ui.add_space(2.0);
            let fr = egui::Frame::new().fill(p.input_bg).stroke(Stroke::new(1.0, p.input_border)).corner_radius(10.0).inner_margin(egui::Margin::symmetric(10, 8));
            let out = fr.show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical().max_height(84.0).min_scrolled_height(84.0).show(ui, |ui| {
                    ui.add(
                        TextEdit::multiline(&mut self.url_text)
                            .frame(false)
                            .desired_width(f32::INFINITY)
                            .desired_rows(5)
                            .text_color(p.on_surface)
                            .hint_text(RichText::new(format!("{}\nhttps://bunkr.si/a/PUK068QE\nhttps://bunkr.fi/f/gBrv5f8tAGlGW", t(self.l, "Cole um ou vários links, um por linha:", "Paste one or more links, one per line:"))).color(p.placeholder)),
                    )
                })
                .inner
            });
            if out.inner.has_focus() {
                ui.painter().rect_stroke(out.response.rect, 10.0, Stroke::new(1.5, p.primary), egui::StrokeKind::Inside);
            }
            let urls = split_urls(&self.url_text);
            let valid = urls.iter().filter(|u| classify(u).1).count();
            let invalid = urls.len() - valid;
            let label = if urls.is_empty() {
                String::new()
            } else if invalid > 0 {
                if self.l == Lang::Pt { format!("{valid} link(s) válido(s) • {invalid} inválido(s) (serão ignorados)") } else { format!("{valid} valid link(s) • {invalid} invalid (ignored)") }
            } else {
                if self.l == Lang::Pt { format!("{valid} link(s) válido(s)") } else { format!("{valid} valid link(s)") }
            };
            if !label.is_empty() {
                hint(ui, &p, &label);
            }
        });
    }

    fn dest_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p;
        card(ui, &p, |ui| {
            card_header(ui, &p, Icon::Folder, t(self.l, "Pasta de Destino", "Destination Folder"), None, |_| {});
            let (mut change, mut open) = (false, false);
            let row_w = ui.available_width();
            ui.allocate_ui_with_layout(vec2(row_w, 34.0), Layout::right_to_left(Align::Center), |ui| {
                open = button(ui, &p, Some(Icon::FolderOpen), t(self.l, "Abrir Pasta", "Open Folder"), BtnKind::Tonal, false, true).clicked();
                change = button(ui, &p, Some(Icon::Folder), t(self.l, "Alterar Pasta", "Change Folder"), BtnKind::Tonal, false, true).clicked();
                // the path field takes exactly the space that is left
                let fr = egui::Frame::new().fill(p.tonal_bg).stroke(Stroke::new(1.0, p.input_border)).corner_radius(10.0).inner_margin(egui::Margin::symmetric(12, 8));
                fr.show(ui, |ui| {
                    ui.set_width((ui.available_width() - 2.0).max(60.0));
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.add(egui::Label::new(RichText::new(&self.s.destination).size(13.0).color(p.muted)).truncate());
                    });
                });
            });
            hint(ui, &p, t(self.l, "Cada álbum é salvo em uma subpasta com o nome do álbum, automaticamente.", "Each album is saved into a subfolder named after the album, automatically."));
            if change {
                let start = PathBuf::from(if self.s.destination.is_empty() { "." } else { &self.s.destination });
                if let Some(d) = sys::choose_folder(&start) {
                    self.s.destination = d.display().to_string();
                    self.s.save();
                }
            }
            if open && !self.s.destination.trim().is_empty() {
                sys::open_folder(&PathBuf::from(self.s.destination.trim()));
            }
        });
    }

    fn settings_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p;
        card(ui, &p, |ui| {
            // clickable header
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click());
            let col = if resp.hovered() { p.primary } else { p.on_surface };
            paint_icon(ui.painter(), rect.left_center() + vec2(9.0, 0.0), 14.0, Icon::Gear, p.muted);
            ui.painter().text(rect.left_center() + vec2(26.0, 0.0), egui::Align2::LEFT_CENTER, t(self.l, "Configurações avançadas", "Advanced settings"), FontId::proportional(13.5), col);
            chevron(ui, rect.right_center() - vec2(8.0, 0.0), self.settings_open, p.muted);
            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                self.settings_open = !self.settings_open;
            }
            if !self.settings_open {
                return;
            }
            ui.add_space(4.0);
            let s = &mut self.s;
            switch(ui, &p, &mut s.clean_filenames, t(self.l, "Manter nomes originais dos arquivos", "Keep original filenames"));
            switch(ui, &p, &mut s.create_downloads_subfolder, t(self.l, "Criar subpasta \"Downloads\" dentro do destino", "Create a \"Downloads\" subfolder inside the destination"));
            switch(ui, &p, &mut s.disable_server_check, t(self.l, "Ignorar verificação de servidor offline", "Skip offline server check"));
            ui.add_space(2.0);
            let lbl = |ui: &mut egui::Ui, t: &str| {
                ui.label(RichText::new(t).size(12.0).color(p.muted));
            };
            egui::Grid::new("adv").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
                lbl(ui, t(self.l, "Tentativas em caso de falha", "Retries on failure"));
                ui.add_sized([110.0, 28.0], egui::DragValue::new(&mut s.max_retries).range(0..=20));
                ui.end_row();
                lbl(ui, t(self.l, "Conexões paralelas por arquivo", "Parallel connections per file"));
                ui.add_sized([110.0, 28.0], egui::DragValue::new(&mut s.connections).range(1..=16));
                ui.end_row();
                lbl(ui, t(self.l, "Arquivos simultâneos por álbum", "Concurrent files per album"));
                ui.add_sized([110.0, 28.0], egui::DragValue::new(&mut s.workers).range(1..=16));
                ui.end_row();
                lbl(ui, t(self.l, "Álbuns simultâneos", "Concurrent albums"));
                ui.add_sized([110.0, 28.0], egui::DragValue::new(&mut s.max_concurrent_albums).range(1..=10));
                ui.end_row();
                lbl(ui, t(self.l, "Limite de velocidade (KB/s, 0 = sem limite)", "Speed limit (KB/s, 0 = unlimited)"));
                ui.add_sized([110.0, 28.0], egui::DragValue::new(&mut s.rate_limit_kbps).range(0.0..=1_000_000.0).speed(25.0).max_decimals(0));
                ui.end_row();
                lbl(ui, t(self.l, "Ignorar arquivos que contenham", "Skip files containing"));
                ui.add(TextEdit::singleline(&mut s.ignore_text).desired_width(240.0).margin(egui::Margin::symmetric(10, 6)).hint_text(RichText::new(".zip .rar").color(p.placeholder)));
                ui.end_row();
                lbl(ui, t(self.l, "Baixar somente arquivos que contenham", "Only download files containing"));
                ui.add(TextEdit::singleline(&mut s.include_text).desired_width(240.0).margin(egui::Margin::symmetric(10, 6)).hint_text(RichText::new(t(self.l, "opcional", "optional")).color(p.placeholder)));
                ui.end_row();
            });
        });
    }

    fn downloads_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p;
        let snaps: Vec<(String, Snap)> = self.rows.iter_mut().map(|r| (r.url.clone(), Self::snapshot(r))).collect();
        let total = snaps.len();
        let processed = snaps.iter().filter(|(_, s)| matches!(s.status, Status::Ok | Status::Warn | Status::Error | Status::Cancelled)).count();
        let (ok, skipped, failed) = snaps.iter().fold((0, 0, 0), |a, (_, s)| (a.0 + s.counts.0, a.1 + s.counts.1, a.2 + s.counts.2));
        let busy = processed < total;

        card(ui, &p, |ui| {
            let right = if total == 0 { t(self.l, "Nenhum link adicionado ainda.", "No links added yet.").to_string() } else { if self.l == Lang::Pt { format!("{processed} de {total} processados") } else { format!("{processed} of {total} processed") } };
            card_header(ui, &p, Icon::Download, t(self.l, "Downloads em Curso", "Downloads in Progress"), None, |ui| {
                ui.label(RichText::new(right).size(12.0).color(p.muted));
            });
            let pct = if total > 0 { processed as f32 / total as f32 } else { 0.0 };
            progress(ui, &p, pct, 18.0, Some(&format!("{}%", (pct * 100.0) as u32)));
            let caption = if total == 0 {
                t(self.l, "Pronto para iniciar", "Ready to start").to_string()
            } else if busy && ok + skipped + failed == 0 {
                t(self.l, "Baixando…", "Downloading…").to_string()
            } else {
                if self.l == Lang::Pt { format!("{ok} baixado(s) • {skipped} ignorado(s) • {failed} com falha") } else { format!("{ok} downloaded • {skipped} skipped • {failed} failed") }
            };
            hint(ui, &p, &caption);

            for (i, (url, s)) in snaps.iter().enumerate() {
                self.draw_row(ui, &p, url, s, i == 0);
            }
        });
    }

    fn draw_row(&self, ui: &mut egui::Ui, p: &Palette, url: &str, s: &Snap, first: bool) {
        if !first {
            let y = ui.cursor().top();
            ui.painter().hline(ui.min_rect().x_range(), y, Stroke::new(1.0, p.divider));
        }
        ui.add_space(8.0);
        let (label, color) = match s.status {
            Status::Pending => (t(self.l, "Aguardando", "Waiting"), p.muted),
            Status::Running => (t(self.l, "Baixando", "Downloading"), p.primary),
            Status::Ok => (t(self.l, "Concluído", "Done"), p.success),
            Status::Warn => (t(self.l, "Concluído c/ falhas", "Done with errors"), p.warning),
            Status::Error => (t(self.l, "Erro", "Error"), p.error),
            Status::Cancelled => (t(self.l, "Cancelado", "Cancelled"), p.muted),
        };
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.horizontal(|ui| {
            ui.label(RichText::new(elide(&s.title, 58)).size(12.5).color(p.on_surface).strong()).on_hover_text(url);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(label).size(11.0).color(color).strong());
            });
        });
        ui.label(RichText::new(elide(&s.note, 78)).size(12.0).color(p.muted));
        let frac = if s.files_total > 0 {
            let cur = s.active_pct.unwrap_or(0.0);
            ((s.files_done as f32) + cur.min(0.999)) / s.files_total as f32
        } else {
            0.0
        };
        let frac = if matches!(s.status, Status::Ok | Status::Warn) { 1.0 } else { frac };
        progress(ui, p, frac, 8.0, None);
        let caption = match s.status {
            Status::Running if s.files_total > 0 => {
                let mut t = if self.l == Lang::Pt { format!("{} de {} arquivo(s)", s.files_done, s.files_total) } else { format!("{} of {} file(s)", s.files_done, s.files_total) };
                if let Some(a) = s.active_pct {
                    t += &format!(" • current item: {:.0}%", a * 100.0);
                }
                if s.speed > 0.0 {
                    t += &format!(" • {}", fmt_speed(s.speed));
                }
                t
            }
            Status::Ok | Status::Warn | Status::Error | Status::Cancelled => {
                let (a, b, c) = s.counts;
                let mut parts = Vec::new();
                if a > 0 { parts.push(format!("{a} downloaded")); }
                if b > 0 { parts.push(format!("{b} skipped")); }
                if c > 0 { parts.push(format!("{c} failed")); }
                parts.join(" • ")
            }
            _ => String::new(),
        };
        hint(ui, p, &caption);
        ui.add_space(4.0);
    }

    fn log_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p;
        if self.log.version() != self.log_cache.0 {
            self.log_cache = (self.log.version(), self.log.snapshot());
        }
        card(ui, &p, |ui| {
            card_header(ui, &p, Icon::Log, t(self.l, "Log de eventos", "Event log"), None, |_| {});
            egui::Frame::new().fill(p.input_bg).stroke(Stroke::new(1.0, p.input_border)).corner_radius(10.0).inner_margin(egui::Margin::symmetric(12, 8)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical().max_height(140.0).min_scrolled_height(140.0).stick_to_bottom(true).show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(&self.log_cache.1).monospace().size(11.5).color(p.muted)).selectable(true));
                });
            });
        });
    }

    fn screenshot_logic(&mut self, ctx: &egui::Context) {
        let Some(path) = self.opts.shot.clone() else { return };
        if !self.shot_sent && self.opts.t0.elapsed() >= Duration::from_millis(self.opts.shot_after_ms) {
            self.shot_sent = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let img = ctx.input(|i| {
            i.events.iter().find_map(|e| if let egui::Event::Screenshot { image, .. } = e { Some(image.clone()) } else { None })
        });
        if let Some(img) = img {
            let mut out = format!("P6\n{} {}\n255\n", img.size[0], img.size[1]).into_bytes();
            for c in &img.pixels {
                out.extend_from_slice(&[c.r(), c.g(), c.b()]);
            }
            let _ = std::fs::write(path, out);
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        self.p.page_bg.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        self.frame += 1;
        if self.frame == 1 && self.opts.bench {
            println!("first update() at {:.1} ms", self.opts.t0.elapsed().as_secs_f64() * 1000.0);
        }
        if self.frame == 2 {
            if self.opts.bench {
                println!("first frame painted + presented in {:.1} ms after process start", self.opts.t0.elapsed().as_secs_f64() * 1000.0);
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            // Warm up runtime + TLS on a background thread, *after* the window is on screen.
            let slot = self.warm.clone();
            std::thread::spawn(move || *slot.lock().unwrap() = Some(build_backend()));
        }
        if self.frame == 3 && !self.opts.urls.is_empty() && !self.autostarted {
            self.autostarted = true;
            self.submit_links();
        }

        let p = self.p;
        egui::TopBottomPanel::top("bar")
            .frame(egui::Frame::new().fill(p.page_bg).inner_margin(egui::Margin { left: 20, right: 20, top: 16, bottom: 10 }))
            .show_separator_line(false)
            .show(ctx, |ui| self.app_bar(ui, ctx));

        egui::CentralPanel::default().frame(egui::Frame::new().fill(p.page_bg)).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: 4, bottom: 20 }).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 8.0;
                    section_label(ui, &p, t(self.l, "Links do Bunkr", "Bunkr links"));
                    self.url_card(ui);
                    ui.add_space(8.0);
                    section_label(ui, &p, t(self.l, "Pasta de destino", "Destination folder"));
                    self.dest_card(ui);
                    ui.add_space(8.0);
                    self.settings_card(ui);
                    ui.add_space(4.0);

                    let w = ui.available_width();
                    let go = ui.allocate_ui(vec2(w, 46.0), |ui| button(ui, &p, Some(Icon::Download), t(self.l, "BAIXAR AGORA", "DOWNLOAD NOW"), BtnKind::Cta, false, true).clicked()).inner;
                    if go {
                        self.submit_links();
                    }
                    if self.manager.is_some() {
                        ui.vertical_centered(|ui| {
                            if button(ui, &p, None, t(self.l, "Cancelar Tudo", "Cancel All"), BtnKind::Danger, false, true)
                                .on_hover_text(t(self.l, "Interrompe todos os downloads pendentes e em andamento agora.", "Stops every pending and running download now."))
                                .clicked()
                            {
                                if let Some(m) = &self.manager {
                                    m.cancel_all();
                                }
                            }
                        });
                    }
                    ui.add_space(4.0);
                    section_label(ui, &p, t(self.l, "Progresso geral", "Overall progress"));
                    self.downloads_card(ui);
                    ui.add_space(6.0);
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            let w = 2.0 * 150.0 + 8.0;
                            ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
                            let a = button(ui, &p, Some(Icon::Gear), t(self.l, "Configurações", "Settings"), BtnKind::Nav, self.settings_open, true).clicked();
                            let b = button(ui, &p, Some(Icon::Log), t(self.l, "Log de Eventos", "Event Log"), BtnKind::Nav, self.log_open, true).clicked();
                            if a { self.settings_open = !self.settings_open; }
                            if b { self.log_open = !self.log_open; }
                        });
                    });
                    if self.log_open {
                        self.log_card(ui);
                    }
                });
            });
        });

        if let Some((title, body)) = self.dialog.clone() {
            let mut close = false;
            egui::Window::new(RichText::new(title).strong())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_max_width(380.0);
                    ui.label(RichText::new(body).color(p.on_surface));
                    ui.add_space(6.0);
                    if button(ui, &p, None, "OK", BtnKind::Tonal, false, true).clicked() {
                        close = true;
                    }
                });
            if close {
                self.dialog = None;
            }
        }

        let active = self.rows.iter().any(|r| matches!(r.ui.view.lock().unwrap().status, Status::Pending | Status::Running));
        if active {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if let Some(ms) = self.opts.cancel_after_ms {
            if self.opts.t0.elapsed() >= Duration::from_millis(ms) {
                self.opts.cancel_after_ms = None;
                if let Some(m) = &self.manager {
                    m.cancel_all();
                }
            }
        }
        self.screenshot_logic(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.s.save();
        if let Some(m) = self.manager.take() {
            m.shutdown();
        }
    }
}

#[allow(dead_code)]
fn _unused(_: HashMap<u8, u8>, _: Color32) {}
