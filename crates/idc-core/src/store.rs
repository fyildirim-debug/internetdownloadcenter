//! Ayar ve indirme listesinin diskte saklanması. Yazımlar atomiktir.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{de::DeserializeOwned, Serialize};

use crate::model::Download;
use crate::settings::Settings;

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn load_settings(&self) -> Settings {
        let mut s: Settings = read_json(&self.dir.join("settings.json")).unwrap_or_default();
        s.sanitize();
        s
    }

    pub fn save_settings(&self, s: &Settings) -> anyhow::Result<()> {
        write_json(&self.dir.join("settings.json"), s)
    }

    pub fn load_downloads(&self) -> Vec<Download> {
        read_json(&self.dir.join("downloads.json")).unwrap_or_default()
    }

    pub fn save_downloads(&self, list: &[Download]) -> anyhow::Result<()> {
        write_json(&self.dir.join("downloads.json"), &list)
    }

    pub fn settings_exist(&self) -> bool {
        self.dir.join("settings.json").exists()
    }
}

/// Okunamayan ya da bozuk dosyayı `.bak` olarak saklar ve `None` döner.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let data = std::fs::read(path).ok()?;
    match serde_json::from_slice(&data) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("{} okunamadı: {e}", path.display());
            let _ = std::fs::copy(path, path.with_extension("json.bak"));
            None
        }
    }
}

pub fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> anyhow::Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    // Eşzamanlı iki yazıcı aynı geçici dosyayı kullanmasın.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_extension(format!("json.{}.{}.tmp", std::process::id(), n));
    let data = serde_json::to_vec_pretty(value)?;
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp).with_context(|| format!("{} oluşturulamadı", tmp.display()))?;
        f.write_all(&data)?;
        f.sync_all()?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Site parolaları bulunabilir; yalnız kullanıcı okuyabilsin.
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    replace_file(&tmp, path)?;
    Ok(())
}

/// Windows'ta antivirüs kilidine karşı birkaç kez dener.
pub fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut last = None;
    for i in 0..10 {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last = Some(e);
                std::thread::sleep(std::time::Duration::from_millis(50 * (i + 1)));
            }
        }
    }
    Err(last.unwrap())
}
