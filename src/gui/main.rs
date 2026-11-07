//! Bunkr Downloader — native GUI (egui). Built for the fastest possible startup:
//! no network, TLS or thread pool is touched before the first frame is on screen.

#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

#![allow(float_literal_f32_fallback)]

mod app;
mod manager;
mod settings;
mod sys;
mod theme;
mod widgets;

use std::time::Instant;

fn make_icon() -> eframe::egui::IconData {
    const N: usize = 64;
    let mut rgba = vec![0u8; N * N * 4];
    let (r, c0, c1) = (14.0f32, (0x3F, 0xB6, 0xFF), (0x00, 0x72, 0xD6));
    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let (dx, dy) = ((fx - 32.0).abs() - (32.0 - r), (fy - 32.0).abs() - (32.0 - r));
            let d = dx.max(0.0).hypot(dy.max(0.0)) - r;
            if d > 0.5 {
                continue;
            }
            let t = (fx + fy) / (2.0 * N as f32);
            let mix = |a: i32, b: i32| (a as f32 + (b - a) as f32 * t) as u8;
            let i = (y * N + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[mix(c0.0, c1.0), mix(c0.1, c1.1), mix(c0.2, c1.2), 255]);
            // white download arrow
            let (ax, ay) = (fx - 32.0, fy - 30.0);
            let shaft = ax.abs() < 3.0 && (-16.0..6.0).contains(&ay);
            let head = ay > -2.0 && ay < 12.0 && ax.abs() < (12.0 - ay).max(0.0) * 0.9 && ax.abs() >= 0.0 && (ay > 0.0 || ax.abs() < 3.0);
            let base = ay > 14.0 && ay < 18.0 && ax.abs() < 15.0;
            if shaft || head || base {
                rgba[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    eframe::egui::IconData { rgba, width: N as u32, height: N as u32 }
}

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
            .with_icon(make_icon()),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native("Bunkr Downloader", native, Box::new(move |cc| Ok(Box::new(app::App::new(cc, opts)))))
}
