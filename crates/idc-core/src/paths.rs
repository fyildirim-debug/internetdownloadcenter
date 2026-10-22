//! Platforma göre ayar, veri ve indirme dizinleri.

use std::path::PathBuf;

pub const APP_DIR_LINUX: &str = "internet-download-center";
pub const APP_DIR_WINDOWS: &str = "InternetDownloadCenter";

fn app_dir_name() -> &'static str {
    if cfg!(windows) { APP_DIR_WINDOWS } else { APP_DIR_LINUX }
}

/// `IDC_PORTABLE` ya da çalıştırılabilir dosyanın yanında `portable.txt` varsa her şey oradadır.
pub fn portable_root() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("IDC_DATA_DIR") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    if dir.join("portable.txt").exists() {
        return Some(dir.join("data"));
    }
    None
}

/// Ayarlar ve indirme listesi.
pub fn config_dir() -> PathBuf {
    if let Some(p) = portable_root() {
        return p;
    }
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join(app_dir_name())
}

/// İndirilen yardımcı araçlar (yt-dlp, ffmpeg, deno).
pub fn tools_dir() -> PathBuf {
    if let Some(p) = portable_root() {
        return p.join("tools");
    }
    dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join(app_dir_name()).join("tools")
}

/// Geçici dosyalar (çerez dosyaları, araç arşivleri).
pub fn temp_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("idc-{}", whoami_suffix()));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn whoami_suffix() -> String {
    std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "user".into())
}

pub fn default_download_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn ipc_file() -> PathBuf {
    config_dir().join("ipc.json")
}
