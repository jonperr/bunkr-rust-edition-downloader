//! Persistent GUI settings (same keys and file name as the Qt version's INI).

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct GuiSettings {
    pub destination: String,
    pub clean_filenames: bool,
    pub create_downloads_subfolder: bool,
    pub disable_server_check: bool,
    pub max_retries: u32,
    pub connections: u32,
    pub workers: u32,
    pub max_concurrent_albums: u32,
    pub rate_limit_kbps: f64,
    pub ignore_text: String,
    pub include_text: String,
    pub dark_mode: bool,
}

fn default_destination() -> String {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("Downloads").join("BunkrDownloader").display().to_string()
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            destination: default_destination(),
            clean_filenames: true,
            create_downloads_subfolder: false,
            disable_server_check: false,
            max_retries: 5,
            connections: 8,
            workers: 4,
            max_concurrent_albums: 1,
            rate_limit_kbps: 0.0,
            ignore_text: String::new(),
            include_text: String::new(),
            dark_mode: true,
        }
    }
}

pub fn ini_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("bunkr_gui_settings.ini")))
        .unwrap_or_else(|| PathBuf::from("bunkr_gui_settings.ini"))
}

impl GuiSettings {
    pub fn load() -> Self {
        let mut s = Self::default();
        let Ok(text) = std::fs::read_to_string(ini_path()) else { return s };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let v = v.trim().replace("\\\\", "\\");
            let b = |d: bool| match v.to_ascii_lowercase().as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => d,
            };
            match k.trim() {
                "destination" if !v.is_empty() => s.destination = v.clone(),
                "clean_filenames" => s.clean_filenames = b(s.clean_filenames),
                "create_downloads_subfolder" => s.create_downloads_subfolder = b(s.create_downloads_subfolder),
                "disable_server_check" => s.disable_server_check = b(s.disable_server_check),
                "max_retries" => s.max_retries = v.parse().unwrap_or(s.max_retries).min(20),
                "connections" => s.connections = v.parse().unwrap_or(s.connections).clamp(1, 16),
                "workers" => s.workers = v.parse().unwrap_or(s.workers).clamp(1, 16),
                "max_concurrent_albums" => s.max_concurrent_albums = v.parse().unwrap_or(1).clamp(1, 10),
                "rate_limit_kbps" => s.rate_limit_kbps = v.parse().unwrap_or(0.0f64).max(0.0),
                "ignore_text" => s.ignore_text = v.clone(),
                "include_text" => s.include_text = v.clone(),
                "dark_mode" => s.dark_mode = b(s.dark_mode),
                _ => {}
            }
        }
        s
    }

    pub fn save(&self) {
        let esc = |t: &str| t.replace('\\', "\\\\");
        let out = format!(
            "[General]\r\ndestination={}\r\ncreate_downloads_subfolder={}\r\ndisable_server_check={}\r\nmax_retries={}\r\nconnections={}\r\nworkers={}\r\nmax_concurrent_albums={}\r\nrate_limit_kbps={}\r\nignore_text={}\r\ninclude_text={}\r\ndark_mode={}\r\nclean_filenames={}\r\n",
            esc(&self.destination),
            self.create_downloads_subfolder,
            self.disable_server_check,
            self.max_retries,
            self.connections,
            self.workers,
            self.max_concurrent_albums,
            self.rate_limit_kbps,
            esc(&self.ignore_text),
            esc(&self.include_text),
            self.dark_mode,
            self.clean_filenames,
        );
        let _ = std::fs::write(ini_path(), out);
    }

    /// Translate into the backend's settings (Python: `build_args_namespace`).
    pub fn to_backend(&self) -> bunkr_core::config::Settings {
        let words = |t: &str| t.split(|c: char| c.is_whitespace() || c == ',').filter(|w| !w.is_empty()).map(str::to_owned).collect();
        bunkr_core::config::Settings {
            custom_path: (!self.destination.trim().is_empty()).then(|| PathBuf::from(self.destination.trim())),
            no_download_folder: !self.create_downloads_subfolder,
            no_album_folder: false, // every album always goes into its own folder
            disable_ui: true,
            disable_disk_check: std::env::var_os("BUNKR_GUI_NODISK").is_some(),
            disable_server_check: self.disable_server_check,
            clean_name: self.clean_filenames,
            max_retries: self.max_retries,
            connections: self.connections.max(1) as usize,
            rate_limit_kbps: (self.rate_limit_kbps > 0.0).then_some(self.rate_limit_kbps),
            dry_run: false,
            refresh: false,
            max_concurrent_urls: self.max_concurrent_albums.max(1) as usize,
            workers: self.workers.max(1) as usize,
            ignore: words(&self.ignore_text),
            include: words(&self.include_text),
        }
    }
}
