//! Bunkr Downloader — native GUI (egui). Built for the fastest possible startup:
//! no network, TLS or thread pool is touched before the first frame is on screen.

#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

#![allow(float_literal_f32_fallback)]

mod app;
mod icon_data;
mod manager;
mod settings;
mod sys;
mod theme;
mod widgets;

use std::time::Instant;


fn main() -> eframe::Result {
    let t0 = Instant::now();
    let mut opts = app::Opts { shot: None, shot_after_ms: 1500, bench: false, urls: vec![], dest: None, expand: false, log: false, cancel_after_ms: None, t0 };
    let mut size = [900.0f32, 780.0f32];
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bench-startup" => opts.bench = true,
            "--shot" => opts.shot = args.next().map(Into::into),
            "--shot-after" => opts.shot_after_ms = args.next().and_then(|v| v.parse().ok()).unwrap_or(1500),
            "--url" => opts.urls.extend(args.next()),
            "--dest" => opts.dest = args.next(),
            "--expand" => opts.expand = true,
            "--log" => opts.log = true,
            "--cancel-after" => opts.cancel_after_ms = args.next().and_then(|v| v.parse().ok()),
            "--size" => {
                if let Some((w, h)) = args.next().as_deref().and_then(|v| v.split_once('x')) {
                    size = [w.parse().unwrap_or(900.0), h.parse().unwrap_or(780.0)];
                }
            }
            _ => {}
        }
    }

    let native = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Bunkr Downloader")
            .with_inner_size(size)
            .with_min_inner_size([700.0, 560.0])
            .with_icon(icon_data::app_icon()),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native("Bunkr Downloader", native, Box::new(move |cc| Ok(Box::new(app::App::new(cc, opts)))))
}
