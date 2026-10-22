//! Yardımcı araçların (yt-dlp, ffmpeg, deno) bulunması, indirilmesi ve güncellenmesi.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;

/// Kurulum paketiyle gelen araçların klasörü (uygulama açılışta bildirir).
static BUNDLED_DIR: once_cell::sync::OnceCell<PathBuf> = once_cell::sync::OnceCell::new();

/// Paketle gelen araçların yerini kaydeder (ör. Tauri kaynak klasöründeki `tools`).
pub fn set_bundled_dir(dir: PathBuf) {
    let _ = BUNDLED_DIR.set(dir);
}

use crate::system::{ToolInfo, ToolsStatus};

#[derive(Clone)]
pub struct Tools {
    busy: Arc<Mutex<bool>>,
    message: Arc<Mutex<Option<String>>>,
    progress: Arc<Mutex<Option<(String, u64, u64)>>>,
}

fn exe(name: &str) -> String {
    if cfg!(windows) { format!("{name}.exe") } else { name.to_string() }
}

/// PATH içinde ara.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(exe(name));
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn version_of(path: &Path, arg: &str) -> Option<String> {
    let mut c = std::process::Command::new(path);
    c.arg(arg);
    hide_window(&mut c);
    let out = c.output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    let line = s.lines().next()?.trim().to_string();
    if line.is_empty() { None } else { Some(line.chars().take(80).collect()) }
}

/// Windows'ta konsol penceresi açılmasın.
pub fn hide_window(c: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let _ = c;
}

impl Default for Tools {
    fn default() -> Self {
        Self::new()
    }
}

impl Tools {
    pub fn new() -> Self {
        Self { busy: Arc::new(Mutex::new(false)), message: Arc::new(Mutex::new(None)), progress: Arc::new(Mutex::new(None)) }
    }

    pub fn dir(&self) -> PathBuf {
        crate::paths::tools_dir()
    }

    fn managed(&self, name: &str) -> PathBuf {
        self.dir().join(exe(name))
    }

    /// Önce IDC'nin indirdiği, yoksa sistemdeki araç.
    pub fn path(&self, name: &str) -> Option<PathBuf> {
        let m = self.managed(name);
        if m.is_file() {
            return Some(m);
        }
        // Kurulum paketinin kaynak klasöründeki araçlar.
        if let Some(d) = BUNDLED_DIR.get() {
            let cand = d.join(exe(name));
            if cand.is_file() {
                return Some(cand);
            }
        }
        // Paketle gelen araçlar (çalıştırılabilir dosyanın yanında).
        if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())) {
            for cand in [dir.join(exe(name)), dir.join("tools").join(exe(name))] {
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
        which(name)
    }

    pub fn ytdlp(&self) -> Option<PathBuf> {
        self.path("yt-dlp")
    }
    pub fn ffmpeg(&self) -> Option<PathBuf> {
        self.path("ffmpeg")
    }
    pub fn deno(&self) -> Option<PathBuf> {
        self.path("deno")
    }

    fn info(&self, name: &str, arg: &str) -> ToolInfo {
        match self.path(name) {
            Some(p) => ToolInfo {
                installed: true,
                version: version_of(&p, arg).map(|v| {
                    let v = v.trim_start_matches("ffmpeg version ").trim_start_matches("deno ");
                    v.split(' ').next().unwrap_or("").to_string()
                }),
                source: if p.starts_with(self.dir()) {
                    "managed".into()
                } else if BUNDLED_DIR.get().map(|d| p.starts_with(d)).unwrap_or(false) {
                    "bundled".into()
                } else {
                    "system".into()
                },
                path: Some(p.to_string_lossy().into()),
            },
            None => ToolInfo { installed: false, version: None, path: None, source: "none".into() },
        }
    }

    pub fn status(&self) -> ToolsStatus {
        let msg = self.message.lock().clone();
        let prog = self.progress.lock().clone();
        ToolsStatus {
            ytdlp: self.info("yt-dlp", "--version"),
            ffmpeg: self.info("ffmpeg", "-version"),
            deno: self.info("deno", "--version"),
            busy: *self.busy.lock(),
            message: match (prog, msg) {
                (Some((n, d, t)), _) if t > 0 => Some(format!("{n}: {}%", d * 100 / t)),
                (_, m) => m,
            },
        }
    }

    pub fn is_busy(&self) -> bool {
        *self.busy.lock()
    }

    /// Eksik araçları indirir; `update` ise yt-dlp'yi de günceller.
    pub async fn ensure(&self, update: bool) -> anyhow::Result<ToolsStatus> {
        {
            let mut b = self.busy.lock();
            if *b {
                anyhow::bail!("araç kurulumu zaten sürüyor");
            }
            *b = true;
        }
        let r = self.ensure_inner(update).await;
        *self.busy.lock() = false;
        *self.progress.lock() = None;
        match &r {
            Ok(_) => *self.message.lock() = None,
            Err(e) => *self.message.lock() = Some(e.to_string()),
        }
        r.map(|_| self.status())
    }

    async fn ensure_inner(&self, update: bool) -> anyhow::Result<()> {
        let dir = self.dir();
        tokio::fs::create_dir_all(&dir).await?;
        let client = reqwest::Client::builder().user_agent("InternetDownloadCenter").build()?;

        // yt-dlp: resmi tek dosya sürümü (ejs ve curl_cffi dahil).
        let yt = self.managed("yt-dlp");
        // Paketle gelen sürüm varsa yalnız güncellemede indir; güncel sürüm veri klasörüne yazılır ve önceliklidir.
        if update || self.ytdlp().is_none() {
            let asset = ytdlp_asset();
            let url = format!("https://github.com/yt-dlp/yt-dlp/releases/latest/download/{asset}");
            self.fetch(&client, "yt-dlp", &url, &yt).await?;
            make_executable(&yt);
        }

        // deno: YouTube imza çözümü için.
        let deno = self.managed("deno");
        if !deno.is_file() && which("deno").is_none() {
            if let Some(asset) = deno_asset() {
                let url = format!("https://github.com/denoland/deno/releases/latest/download/{asset}");
                let zip = crate::paths::temp_dir().join(asset);
                self.fetch(&client, "deno", &url, &zip).await?;
                let target = deno.clone();
                tokio::task::spawn_blocking(move || extract_zip_member(&zip, &exe("deno"), &target)).await??;
                make_executable(&deno);
            }
        }

        // ffmpeg: sistemde yoksa yt-dlp'nin ffmpeg derlemeleri.
        let ff = self.managed("ffmpeg");
        if !ff.is_file() && which("ffmpeg").is_none() {
            if let Some((asset, is_zip)) = ffmpeg_asset() {
                let url = format!("https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/{asset}");
                let arc = crate::paths::temp_dir().join(asset);
                self.fetch(&client, "ffmpeg", &url, &arc).await?;
                let d2 = dir.clone();
                tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                    for name in ["ffmpeg", "ffprobe"] {
                        let target = d2.join(exe(name));
                        if is_zip {
                            extract_zip_member(&arc, &exe(name), &target)?;
                        } else {
                            extract_tar_xz_member(&arc, name, &target)?;
                        }
                        make_executable(&target);
                    }
                    let _ = std::fs::remove_file(&arc);
                    Ok(())
                })
                .await??;
            }
        }
        Ok(())
    }

    async fn fetch(&self, client: &reqwest::Client, label: &str, url: &str, to: &Path) -> anyhow::Result<()> {
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;
        tracing::info!("{label} indiriliyor: {url}");
        let resp = client.get(url).send().await?.error_for_status()?;
        let total = resp.content_length().unwrap_or(0);
        let tmp = to.with_extension("download");
        let mut f = tokio::fs::File::create(&tmp).await?;
        let mut done = 0u64;
        let mut s = resp.bytes_stream();
        while let Some(chunk) = s.next().await {
            let chunk = chunk?;
            f.write_all(&chunk).await?;
            done += chunk.len() as u64;
            *self.progress.lock() = Some((label.to_string(), done, total));
        }
        f.flush().await?;
        drop(f);
        if to.exists() {
            let _ = tokio::fs::remove_file(to).await;
        }
        tokio::fs::rename(&tmp, to).await?;
        Ok(())
    }
}

fn make_executable(p: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755));
    }
    let _ = p;
}

fn ytdlp_asset() -> &'static str {
    if cfg!(windows) {
        if cfg!(target_arch = "aarch64") { "yt-dlp_arm64.exe" } else { "yt-dlp.exe" }
    } else if cfg!(target_arch = "aarch64") {
        "yt-dlp_linux_aarch64"
    } else {
        "yt-dlp_linux"
    }
}

fn deno_asset() -> Option<&'static str> {
    if cfg!(windows) {
        Some("deno-x86_64-pc-windows-msvc.zip")
    } else if cfg!(target_arch = "aarch64") {
        Some("deno-aarch64-unknown-linux-gnu.zip")
    } else if cfg!(target_arch = "x86_64") {
        Some("deno-x86_64-unknown-linux-gnu.zip")
    } else {
        None
    }
}

fn ffmpeg_asset() -> Option<(&'static str, bool)> {
    if cfg!(windows) {
        Some(("ffmpeg-master-latest-win64-gpl.zip", true))
    } else if cfg!(target_arch = "aarch64") {
        Some(("ffmpeg-master-latest-linuxarm64-gpl.tar.xz", false))
    } else if cfg!(target_arch = "x86_64") {
        Some(("ffmpeg-master-latest-linux64-gpl.tar.xz", false))
    } else {
        None
    }
}

/// Zip içinden dosya adı `member` ile biten ilk girdiyi çıkarır.
fn extract_zip_member(zip: &Path, member: &str, to: &Path) -> anyhow::Result<()> {
    let f = std::fs::File::open(zip)?;
    let mut a = zip::ZipArchive::new(f)?;
    for i in 0..a.len() {
        let mut e = a.by_index(i)?;
        let name = e.name().replace('\\', "/");
        if name.rsplit('/').next() == Some(member) && !e.is_dir() {
            let mut out = std::fs::File::create(to)?;
            std::io::copy(&mut e, &mut out)?;
            return Ok(());
        }
    }
    anyhow::bail!("{member} arşivde yok")
}

fn extract_tar_xz_member(arc: &Path, member: &str, to: &Path) -> anyhow::Result<()> {
    let f = std::fs::File::open(arc)?;
    let dec = xz2::read::XzDecoder::new(f);
    let mut t = tar::Archive::new(dec);
    for e in t.entries()? {
        let mut e = e?;
        let p = e.path()?.to_string_lossy().to_string();
        if p.ends_with(&format!("/bin/{member}")) {
            let mut out = std::fs::File::create(to)?;
            std::io::copy(&mut e, &mut out)?;
            return Ok(());
        }
    }
    anyhow::bail!("{member} arşivde yok")
}
