//! Small OS integrations: folder picker, "open folder", clipboard.

use std::path::{Path, PathBuf};
use std::process::Command;

pub fn open_folder(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    #[cfg(windows)]
    let _ = Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(path).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(path).spawn();
}

pub fn choose_folder(start: &Path) -> Option<PathBuf> {
    #[cfg(any(windows, target_os = "macos"))]
    {
        let mut d = rfd::FileDialog::new().set_title("Escolher pasta de destino");
        if start.exists() {
            d = d.set_directory(start);
        }
        d.pick_folder()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let start_s = format!("{}/", start.display());
        let attempts: [(&str, Vec<String>); 2] = [
            ("zenity", vec!["--file-selection".into(), "--directory".into(), format!("--filename={start_s}"), "--title=Escolher pasta de destino".into()]),
            ("kdialog", vec!["--getexistingdirectory".into(), start_s.clone()]),
        ];
        for (bin, args) in attempts {
            if let Ok(out) = Command::new(bin).args(&args).output() {
                if out.status.success() {
                    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !p.is_empty() {
                        return Some(PathBuf::from(p));
                    }
                }
                return None;
            }
        }
        None
    }
}

pub fn clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}
