//! Video ve akış indirmeleri: yt-dlp ile biçim sorgulama, indirme, ilerleme okuma.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::engine::task::Outcome;
use crate::engine::{Active, Inner};
use crate::model::*;
use crate::settings::Settings;
use crate::util::err_text;

/// yt-dlp ile indirilmesi gereken bilinen video siteleri.
const MEDIA_HOSTS: &[&str] = &[
    "youtube.com", "youtu.be", "youtube-nocookie.com", "vimeo.com", "dailymotion.com", "dai.ly", "twitter.com", "x.com",
    "instagram.com", "facebook.com", "fb.watch", "tiktok.com", "twitch.tv", "reddit.com", "v.redd.it", "soundcloud.com",
    "bilibili.com", "rumble.com", "streamable.com", "ok.ru", "vk.com", "bitchute.com", "odysee.com", "kick.com",
    "mixcloud.com", "bandcamp.com", "pinterest.com", "linkedin.com", "tumblr.com", "imdb.com", "ted.com",
    "9gag.com", "nicovideo.jp", "coub.com", "izlesene.com", "puhutv.com", "trtizle.com", "blutv.com", "dmax.com.tr",
    "tv8.com.tr", "kanald.com.tr", "showtv.com.tr", "startv.com.tr", "atv.com.tr",
];

pub fn is_media_site(url: &str) -> bool {
    let Ok(u) = url::Url::parse(url) else { return false };
    let host = u.host_str().unwrap_or("").to_lowercase();
    MEDIA_HOSTS.iter().any(|h| host == *h || host.ends_with(&format!(".{h}")))
}

/// Akış (HLS/DASH) adresi mi.
pub fn is_stream_url(url: &str) -> bool {
    let l = url.to_lowercase();
    let path = l.split(['?', '#']).next().unwrap_or("");
    path.ends_with(".m3u8") || path.ends_with(".mpd") || path.ends_with(".ism/manifest") || l.contains(".m3u8?") || l.contains("/manifest/dash")
}

/// Netscape biçiminde çerez dosyası yazar.
fn write_cookie_file(cookies: &[CookieSpec], raw: Option<&str>, url: &str) -> Option<PathBuf> {
    let mut lines = vec!["# Netscape HTTP Cookie File".to_string()];
    for c in cookies {
        if c.name.is_empty() || c.domain.is_empty() {
            continue;
        }
        let domain = if c.host_only { c.domain.trim_start_matches('.').to_string() } else if c.domain.starts_with('.') { c.domain.clone() } else { format!(".{}", c.domain) };
        let flag = if c.host_only { "FALSE" } else { "TRUE" };
        let exp = c.expires.map(|e| e as i64).unwrap_or(0);
        let domain = if c.http_only { format!("#HttpOnly_{domain}") } else { domain };
        lines.push(format!("{}\t{}\t{}\t{}\t{}\t{}\t{}", domain, flag, if c.path.is_empty() { "/" } else { &c.path }, if c.secure { "TRUE" } else { "FALSE" }, exp, c.name, c.value));
    }
    if let (Some(raw), Ok(u)) = (raw, url::Url::parse(url)) {
        let host = u.host_str().unwrap_or("");
        for (k, v) in crate::http::parse_cookie_header(raw) {
            if cookies.iter().any(|c| c.name == k) {
                continue;
            }
            lines.push(format!("{}\tFALSE\t/\tFALSE\t0\t{}\t{}", host, k, v));
        }
    }
    if lines.len() == 1 {
        return None;
    }
    let path = crate::paths::temp_dir().join(format!("cookies-{}.txt", new_id()));
    std::fs::write(&path, lines.join("\n") + "\n").ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Some(path)
}

/// Tüm yt-dlp çağrılarında ortak argümanlar.
fn common_args(cmd: &mut tokio::process::Command, inner_tools: &crate::tools::Tools, req: &RequestInfo, impersonate: bool) -> Option<PathBuf> {
    cmd.arg("--ignore-config").arg("--no-playlist").arg("--encoding").arg("utf-8");
    if let Some(deno) = inner_tools.deno() {
        cmd.arg("--js-runtimes").arg(format!("deno:{}", deno.display()));
    }
    if let Some(ff) = inner_tools.ffmpeg() {
        cmd.arg("--ffmpeg-location").arg(ff);
    }
    if let Some(ua) = req.user_agent.as_deref().filter(|s| !s.is_empty()) {
        cmd.arg("--user-agent").arg(ua);
    }
    if let Some(r) = req.referer.as_deref().filter(|s| !s.is_empty()) {
        cmd.arg("--referer").arg(r);
    }
    for (k, v) in &req.headers {
        let lk = k.to_ascii_lowercase();
        if matches!(lk.as_str(), "cookie" | "user-agent" | "referer" | "range" | "host" | "content-length" | "accept-encoding" | "connection")
            || lk.starts_with("sec-fetch")
            || lk.starts_with(':')
        {
            continue;
        }
        cmd.arg("--add-header").arg(format!("{}:{}", k, v));
    }
    if impersonate {
        cmd.arg("--impersonate").arg("chrome");
    }
    let cookie_file = write_cookie_file(&req.cookies, req.raw_cookie.as_deref(), &req.url);
    if let Some(p) = &cookie_file {
        cmd.arg("--cookies").arg(p);
    }
    cmd.env("PYTHONIOENCODING", "utf-8");
    cookie_file
}

fn configure_process(cmd: &mut tokio::process::Command) {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(unix)]
    {
        // Kendi süreç grubu: durdururken ffmpeg alt süreçleri de kapanır.
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000);
    }
}

fn kill_tree(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGTERM);
    }
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("taskkill");
        c.args(["/PID", &pid.to_string(), "/T", "/F"]);
        crate::tools::hide_window(&mut c);
        let _ = c.status();
    }
}

/// Sayfadaki videonun biçimlerini sorgular.
pub async fn probe(tools: &crate::tools::Tools, settings: &Settings, req: &RequestInfo) -> anyhow::Result<MediaProbe> {
    let yt = tools.ytdlp().ok_or_else(|| anyhow::anyhow!(err_text("tools_missing", "yt-dlp")))?;
    let mut cmd = tokio::process::Command::new(yt);
    let cookie = common_args(&mut cmd, tools, req, false);
    cmd.arg("-J").arg("--no-warnings").arg("--").arg(&req.url);
    configure_process(&mut cmd);
    let out = tokio::time::timeout(std::time::Duration::from_secs(90), cmd.output()).await;
    if let Some(c) = cookie {
        let _ = std::fs::remove_file(c);
    }
    let out = out.map_err(|_| anyhow::anyhow!(err_text("timeout", "yt-dlp")))??;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let line = err.lines().rev().find(|l| l.contains("ERROR")).unwrap_or(err.lines().last().unwrap_or("")).trim().to_string();
        anyhow::bail!(err_text("ytdlp_failed", line.trim_start_matches("ERROR: ")));
    }
    let v: Value = serde_json::from_slice(&out.stdout)?;
    Ok(parse_probe(&v, &req.url, settings))
}

fn f64_of(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_f64())
}
fn str_of<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(|x| x.as_str()).filter(|s| !s.is_empty() && *s != "none")
}

fn size_of(f: &Value, duration: Option<f64>) -> Option<u64> {
    f64_of(f, "filesize")
        .or_else(|| f64_of(f, "filesize_approx"))
        .or_else(|| match (f64_of(f, "tbr"), duration) {
            (Some(t), Some(d)) => Some(t * 1000.0 / 8.0 * d),
            _ => None,
        })
        .map(|x| x as u64)
}

fn codec_label(c: &str) -> &'static str {
    let c = c.to_lowercase();
    if c.starts_with("avc") || c.starts_with("h264") {
        "H.264"
    } else if c.starts_with("hev") || c.starts_with("hvc") || c.starts_with("h265") {
        "H.265"
    } else if c.starts_with("vp09") || c.starts_with("vp9") {
        "VP9"
    } else if c.starts_with("av01") || c.starts_with("av1") {
        "AV1"
    } else if c.starts_with("vp8") {
        "VP8"
    } else {
        ""
    }
}

pub fn parse_probe(v: &Value, page_url: &str, settings: &Settings) -> MediaProbe {
    let duration = f64_of(v, "duration");
    let formats = v.get("formats").and_then(|f| f.as_array()).cloned().unwrap_or_default();
    let want_mp4 = settings.media.container == "mp4";

    let is_video = |f: &Value| str_of(f, "vcodec").is_some() || (f.get("vcodec").is_none() && f64_of(f, "height").is_some());
    let has_audio = |f: &Value| str_of(f, "acodec").is_some();
    let audio: Vec<&Value> = formats.iter().filter(|f| !is_video(f) && has_audio(f)).collect();
    let best_audio = |prefer_m4a: bool| -> Option<&Value> {
        let mut a = audio.clone();
        a.sort_by(|x, y| {
            let px = (prefer_m4a && str_of(x, "ext") == Some("m4a")) as i32;
            let py = (prefer_m4a && str_of(y, "ext") == Some("m4a")) as i32;
            (px, f64_of(x, "abr").or(f64_of(x, "tbr")).unwrap_or(0.0) as i64).cmp(&(py, f64_of(y, "abr").or(f64_of(y, "tbr")).unwrap_or(0.0) as i64))
        });
        a.last().copied()
    };

    // Yükseklik + fps + HDR başına en iyi görüntü biçimi.
    let mut best: std::collections::BTreeMap<(u32, u32, bool), &Value> = Default::default();
    for f in formats.iter().filter(|f| is_video(f)) {
        let Some(h) = f64_of(f, "height").map(|x| x as u32) else { continue };
        if str_of(f, "format_note").map(|n| n.contains("storyboard")).unwrap_or(false) || str_of(f, "ext") == Some("mhtml") {
            continue;
        }
        let fps = f64_of(f, "fps").unwrap_or(30.0);
        let fps_key = if fps > 32.0 { fps.round() as u32 } else { 30 };
        let hdr = str_of(f, "dynamic_range").map(|d| d != "SDR").unwrap_or(false);
        let key = (h, fps_key, hdr);
        let score = |x: &Value| {
            let ext_ok = if want_mp4 { str_of(x, "ext") == Some("mp4") } else { true };
            (ext_ok as i32, f64_of(x, "tbr").unwrap_or(0.0) as i64)
        };
        match best.get(&key) {
            Some(cur) if score(cur) >= score(f) => {}
            _ => {
                best.insert(key, f);
            }
        }
    }

    let mut out = vec![];
    for ((h, fps, hdr), f) in best.iter().rev() {
        let id = str_of(f, "format_id").unwrap_or("").to_string();
        let vcodec = str_of(f, "vcodec").map(String::from);
        let ext = str_of(f, "ext").unwrap_or("mp4").to_string();
        let (sel, size, acodec) = if has_audio(f) {
            (id.clone(), size_of(f, duration), str_of(f, "acodec").map(String::from))
        } else {
            let a = best_audio(ext == "mp4");
            match a {
                Some(a) => (
                    format!("{}+{}", id, str_of(a, "format_id").unwrap_or("ba")),
                    size_of(f, duration).map(|s| s + size_of(a, duration).unwrap_or(0)),
                    str_of(a, "acodec").map(String::from),
                ),
                None => (format!("{id}+ba/{id}"), size_of(f, duration), None),
            }
        };
        let codec = vcodec.as_deref().map(codec_label).unwrap_or("");
        let shown_ext = if !has_audio(f) && settings.media.container != ext { settings.media.container.clone() } else { ext.clone() };
        let label = format!(
            "{}p{}{} {}{}",
            h,
            if *fps > 30 { fps.to_string() } else { String::new() },
            if *hdr { " HDR" } else { "" },
            shown_ext.to_uppercase(),
            if codec.is_empty() { String::new() } else { format!(" · {codec}") }
        );
        out.push(MediaFormat {
            format_id: sel,
            label,
            height: Some(*h),
            width: f64_of(f, "width").map(|x| x as u32),
            fps: f64_of(f, "fps"),
            ext: shown_ext,
            vcodec,
            acodec,
            filesize: size,
            tbr: f64_of(f, "tbr"),
            audio_only: false,
            hdr: *hdr,
        });
    }

    // Biçim listesi olmayan (tek dosyalı) kaynaklar.
    if out.is_empty() && audio.is_empty() {
        let ext = str_of(v, "ext").unwrap_or("mp4").to_string();
        out.push(MediaFormat {
            format_id: "b/bv*+ba".into(),
            label: format!("{} {}", str_of(v, "resolution").unwrap_or("Original"), ext.to_uppercase()),
            height: f64_of(v, "height").map(|x| x as u32),
            ext,
            filesize: size_of(v, duration),
            ..Default::default()
        });
    }

    if let Some(a) = best_audio(true) {
        let size = size_of(a, duration);
        for fmt in ["m4a", "mp3", "opus"] {
            out.push(MediaFormat {
                format_id: if fmt == "m4a" { "ba[ext=m4a]/ba".into() } else { "ba".into() },
                label: format!("{} {}", fmt.to_uppercase(), f64_of(a, "abr").map(|x| format!("{}k", x.round())).unwrap_or_default()).trim().to_string(),
                ext: fmt.into(),
                acodec: str_of(a, "acodec").map(String::from),
                filesize: size,
                tbr: f64_of(a, "abr"),
                audio_only: true,
                ..Default::default()
            });
        }
    }

    let subtitles = v
        .get("subtitles")
        .and_then(|s| s.as_object())
        .map(|o| o.keys().filter(|k| *k != "live_chat").cloned().collect())
        .unwrap_or_default();

    MediaProbe {
        page_url: page_url.to_string(),
        title: str_of(v, "title").unwrap_or("video").to_string(),
        thumbnail: str_of(v, "thumbnail").map(String::from),
        duration,
        uploader: str_of(v, "uploader").or(str_of(v, "channel")).map(String::from),
        extractor: str_of(v, "extractor_key").or(str_of(v, "extractor")).unwrap_or("").to_string(),
        formats: out,
        subtitles,
        is_live: v.get("is_live").and_then(|x| x.as_bool()).unwrap_or(false),
        playlist_count: v.get("playlist_count").and_then(|x| x.as_u64()).map(|x| x as u32),
    }
}

/// Ayardaki varsayılan kaliteye karşılık gelen seçici.
pub fn default_selector(settings: &Settings) -> (String, bool) {
    let q = settings.media.default_quality.as_str();
    let mp4 = settings.media.container == "mp4";
    match q {
        "audio" => ("ba[ext=m4a]/ba".into(), true),
        "best" | "" => (if mp4 { "bv*[ext=mp4]+ba[ext=m4a]/bv*+ba/b".into() } else { "bv*+ba/b".into() }, false),
        h => (format!("bv*[height<={h}]+ba/b[height<={h}]/bv*+ba/b"), false),
    }
}

/// yt-dlp ile indirme görevi.
pub async fn run(inner: Arc<Inner>, active: Arc<Active>, d: Download) -> Outcome {
    let DownloadKind::Media(spec) = &d.kind else { return Outcome::Failed(err_text("unknown", "")) };
    let settings = inner.settings.read().clone();
    if inner.tools.ytdlp().is_none() {
        // İlk kullanımda araçları kendiliğinden kur.
        if let Err(e) = inner.tools.ensure(false).await {
            return Outcome::Failed(err_text("tools_missing", e));
        }
    }
    let Some(yt) = inner.tools.ytdlp() else { return Outcome::Failed(err_text("tools_missing", "yt-dlp")) };
    let _ = std::fs::create_dir_all(&d.save_dir);

    let mut req = d.request.clone();
    if req.url.is_empty() {
        req.url = spec.page_url.clone();
    }
    let source = spec.media_url.clone().filter(|_| spec.page_url.is_empty()).unwrap_or_else(|| if spec.page_url.is_empty() { req.url.clone() } else { spec.page_url.clone() });

    let mut cmd = tokio::process::Command::new(yt);
    let cookie = common_args(&mut cmd, &inner.tools, &req, spec.impersonate);
    let (default_sel, default_audio) = default_selector(&settings);
    let format = if spec.format_id.is_empty() { default_sel } else { spec.format_id.clone() };
    let audio_only = spec.audio_only || (spec.format_id.is_empty() && default_audio);
    let stem = media_stem(&d.file_name);
    cmd.arg("-f").arg(&format);
    if audio_only {
        let af = spec.audio_format.clone().unwrap_or_else(|| settings.media.audio_format.clone());
        cmd.arg("-x").arg("--audio-format").arg(af);
    } else if !spec.impersonate {
        let cont = spec.container.clone().unwrap_or_else(|| settings.media.container.clone());
        cmd.arg("--merge-output-format").arg(cont);
    }
    if !spec.subtitles.is_empty() || settings.media.download_subtitles {
        let langs = if spec.subtitles.is_empty() { settings.media.subtitle_langs.join(",") } else { spec.subtitles.join(",") };
        cmd.arg("--write-subs").arg("--sub-langs").arg(langs);
        if !audio_only {
            cmd.arg("--embed-subs");
        }
    }
    if settings.media.embed_thumbnail {
        cmd.arg("--embed-thumbnail");
    }
    if settings.media.sponsorblock && is_media_site(&source) && source.contains("youtu") {
        cmd.arg("--sponsorblock-remove").arg("sponsor");
    }
    if let Some(l) = d.speed_limit {
        cmd.arg("--limit-rate").arg(l.to_string());
    } else if settings.connection.speed_limit_enabled {
        cmd.arg("--limit-rate").arg(format!("{}K", settings.connection.speed_limit_kib));
    }
    match settings.proxy.mode.as_str() {
        "manual" if !settings.proxy.url.is_empty() => {
            cmd.arg("--proxy").arg(&settings.proxy.url);
        }
        "none" => {
            cmd.arg("--proxy").arg("");
        }
        _ => {}
    }
    cmd.arg("-N").arg(d.max_connections.clamp(1, crate::MAX_CONNECTIONS).to_string())
        .arg("--retries").arg(settings.downloads.retry_count.to_string())
        .arg("--fragment-retries").arg(settings.downloads.retry_count.to_string())
        .arg("--newline")
        .arg("--progress")
        .arg("--progress-template")
        .arg("download:IDCP|%(progress.status)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(info.format_id)s")
        .arg("--print").arg("after_move:IDCFILE|%(filepath)s")
        .arg("--no-mtime")
        .arg("-P").arg(&d.save_dir)
        .arg("-o").arg(format!("{}.%(ext)s", stem.replace('%', "%%")))
        .arg("--")
        .arg(&source);
    configure_process(&mut cmd);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Outcome::Failed(err_text("ytdlp_failed", e)),
    };
    let pid = child.id().unwrap_or(0);
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let id = d.id.clone();
    let hint = d.size;

    // Hata satırlarını topla.
    let errs = Arc::new(parking_lot::Mutex::new(Vec::<String>::new()));
    let errs2 = errs.clone();
    let inner2 = inner.clone();
    let id2 = id.clone();
    let err_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            let l = l.trim().to_string();
            if l.starts_with("ERROR") {
                errs2.lock().push(l.trim_start_matches("ERROR: ").to_string());
            } else if l.contains("[Merger]") || l.contains("[ExtractAudio]") || l.contains("[VideoConvertor]") {
                inner2.update(&id2, |d| d.status = Status::Processing);
            }
        }
    });

    let mut final_path: Option<PathBuf> = None;
    let mut base_done: u64 = 0;
    let mut cur_fmt = String::new();
    let mut cur_total: u64 = 0;
    let mut lines = BufReader::new(stdout).lines();
    let cancel = active.cancel.clone();
    let cancelled = loop {
        let line = tokio::select! {
            _ = cancel.cancelled() => break true,
            l = lines.next_line() => l,
        };
        let Ok(Some(line)) = line else { break false };
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("IDCFILE|") {
            final_path = Some(PathBuf::from(rest));
            continue;
        }
        if line.contains("[Merger]") || line.contains("[ExtractAudio]") || line.contains("[FixupM3u8]") {
            inner.update(&id, |d| d.status = Status::Processing);
            continue;
        }
        let Some(rest) = line.strip_prefix("IDCP|") else { continue };
        let p: Vec<&str> = rest.split('|').collect();
        if p.len() < 7 {
            continue;
        }
        let num = |s: &str| s.parse::<f64>().ok().map(|x| x as u64);
        let status = p[0];
        let done = num(p[1]).unwrap_or(0);
        let total = num(p[2]).or(num(p[3])).unwrap_or(0);
        let fmt = p[6].to_string();
        // Ayrı görüntü + ses akışları sırayla iner: biten akışın boyutunu tabana ekle.
        if !cur_fmt.is_empty() && fmt != cur_fmt {
            base_done += cur_total;
        }
        cur_fmt = fmt;
        cur_total = total.max(done);
        let recv = base_done + done;
        active.recv.store(recv, Ordering::Relaxed);
        let est = (base_done + total).max(hint.unwrap_or(0)).max(recv);
        active.media_total.store(est, Ordering::Relaxed);
        active.media_speed.store(num(p[4]).unwrap_or(0), Ordering::Relaxed);
        active.media_eta.store(num(p[5]).unwrap_or(u64::MAX), Ordering::Relaxed);
        active.conns.store(d.max_connections, Ordering::Relaxed);
        if status == "finished" {
            active.media_speed.store(0, Ordering::Relaxed);
        }
    };

    if cancelled {
        if pid != 0 {
            kill_tree(pid);
        }
        let _ = child.kill().await;
    }
    let status = child.wait().await;
    let _ = err_task.await;
    if let Some(c) = cookie {
        let _ = std::fs::remove_file(c);
    }
    active.conns.store(0, Ordering::Relaxed);
    if cancelled {
        let r = active.recv.load(Ordering::Relaxed);
        inner.update(&id, |d| d.downloaded = r);
        return Outcome::Paused;
    }
    let ok = status.map(|s| s.success()).unwrap_or(false);
    if !ok || final_path.is_none() {
        let msg = errs.lock().last().cloned().unwrap_or_else(|| "yt-dlp".into());
        return Outcome::Failed(err_text("ytdlp_failed", msg));
    }
    let path = final_path.unwrap();
    let size = std::fs::metadata(&path).map(|m| m.len()).ok();
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let dir = path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    crate::engine::mark_origin_pub(&path, &source, d.request.referer.as_deref());
    inner.update(&id, |x| {
        x.file_name = name.clone();
        if !dir.is_empty() {
            x.save_dir = dir.clone();
        }
        x.size = size;
        x.downloaded = size.unwrap_or(0);
        x.status = Status::Completed;
        x.completed_at = Some(now_ms());
        x.error = None;
    });
    Outcome::Completed
}

/// Medya dosya adının uzantısız kökü ("Mr. Bean" gibi noktalı başlıklar bozulmaz).
pub fn media_stem(name: &str) -> String {
    const EXTS: &[&str] = &["mp4", "mkv", "webm", "m4a", "mp3", "opus", "ogg", "flv", "mov", "avi", "ts", "aac", "wav", "flac"];
    let (stem, ext) = crate::filename::split_ext(name);
    let s = if EXTS.contains(&ext.to_ascii_lowercase().as_str()) { stem } else { name.to_string() };
    if s.trim().is_empty() { "video".into() } else { s }
}

/// Silinen medya indirmesinin yarım parçalarını temizler.
pub fn cleanup_parts(d: &Download) {
    if !d.is_media() || d.status == Status::Completed {
        return;
    }
    let stem = media_stem(&d.file_name);
    let Ok(rd) = std::fs::read_dir(Path::new(&d.save_dir)) else { return };
    for e in rd.flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n.starts_with(&format!("{stem}.")) && (n.ends_with(".part") || n.ends_with(".ytdl") || n.contains(".part-Frag")) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_sites() {
        assert!(is_media_site("https://www.youtube.com/watch?v=x"));
        assert!(is_media_site("https://m.youtube.com/watch?v=x"));
        assert!(!is_media_site("https://example.com/a.mp4"));
        assert!(is_stream_url("https://cdn.x.com/a/master.m3u8?token=1"));
    }

    #[test]
    fn parse_formats() {
        let v: Value = serde_json::json!({
            "title": "Deneme", "duration": 100.0, "extractor_key": "Youtube",
            "formats": [
                {"format_id": "140", "ext": "m4a", "vcodec": "none", "acodec": "mp4a.40.2", "abr": 128.0, "filesize": 1600000},
                {"format_id": "251", "ext": "webm", "vcodec": "none", "acodec": "opus", "abr": 160.0, "filesize": 1800000},
                {"format_id": "137", "ext": "mp4", "vcodec": "avc1.640028", "acodec": "none", "height": 1080, "width": 1920, "fps": 30.0, "tbr": 4000.0, "filesize": 50000000},
                {"format_id": "248", "ext": "webm", "vcodec": "vp9", "acodec": "none", "height": 1080, "width": 1920, "fps": 30.0, "tbr": 3000.0},
                {"format_id": "299", "ext": "mp4", "vcodec": "avc1.64002a", "acodec": "none", "height": 1080, "fps": 60.0, "tbr": 6000.0},
                {"format_id": "18", "ext": "mp4", "vcodec": "avc1.42001E", "acodec": "mp4a.40.2", "height": 360, "fps": 30.0, "tbr": 500.0}
            ],
            "subtitles": {"tr": [], "en": []}
        });
        let p = parse_probe(&v, "https://youtube.com/watch?v=x", &Settings::default());
        assert_eq!(p.title, "Deneme");
        assert_eq!(p.formats[0].label, "1080p60 MP4 · H.264");
        assert_eq!(p.formats[0].format_id, "299+140");
        let f1080 = p.formats.iter().find(|f| f.label == "1080p MP4 · H.264").unwrap();
        assert_eq!(f1080.format_id, "137+140");
        assert_eq!(f1080.filesize, Some(51600000));
        let f360 = p.formats.iter().find(|f| f.height == Some(360)).unwrap();
        assert_eq!(f360.format_id, "18");
        assert!(p.formats.iter().any(|f| f.audio_only && f.ext == "mp3"));
        assert_eq!(p.subtitles.len(), 2);
    }
}
