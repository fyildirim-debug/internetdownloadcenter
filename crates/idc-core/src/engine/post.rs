//! İndirme sonrası işler: yeniden adlandırma, tarih, kaynak işareti, sağlama ve virüs taraması.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::task::Outcome;
use super::Inner;
use crate::model::*;
use crate::util::err_text;

pub async fn finish_file(inner: &Arc<Inner>, id: &str, part: &Path) -> Outcome {
    let Some(d) = inner.with(id, |d| d.clone()) else { return Outcome::Failed(err_text("cancelled", "")) };
    let settings = inner.settings.read().clone();

    // Hedefte aynı adlı dosya belirdiyse benzersiz ad seç (üzerine yazma ayarı hariç).
    let dir = PathBuf::from(&d.save_dir);
    let mut name = d.file_name.clone();
    if dir.join(&name).exists() && settings.downloads.duplicate_action != "overwrite" {
        name = crate::filename::unique_name(&dir, &name, &|_| false);
    }
    let target = dir.join(&name);
    let p = part.to_path_buf();
    let t = target.clone();
    let moved = tokio::task::spawn_blocking(move || {
        if t.exists() {
            let _ = std::fs::remove_file(&t);
        }
        super::move_file(&p, &t)
    })
    .await;
    match moved {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Outcome::Failed(err_text("write_failed", e)),
        Err(e) => return Outcome::Failed(err_text("write_failed", e)),
    }
    if name != d.file_name {
        inner.update(id, |x| x.file_name = name.clone());
    }

    if settings.downloads.use_server_time {
        if let Some(t) = d.last_modified.as_deref().and_then(|s| httpdate::parse_http_date(s).ok()) {
            if let Ok(f) = std::fs::OpenOptions::new().write(true).open(&target) {
                let _ = f.set_modified(t);
            }
        }
    }
    mark_origin(&target, d.final_url.as_deref().unwrap_or(&d.request.url), d.request.referer.as_deref());

    let mut checksum_ok = None;
    if let Some(c) = d.checksum.clone() {
        inner.update(id, |x| x.status = Status::Processing);
        let path = target.clone();
        let algo = c.algo.clone();
        let got = tokio::task::spawn_blocking(move || hash_file(&path, &algo)).await.ok().flatten();
        let ok = got.as_deref().map(|g| g.eq_ignore_ascii_case(c.expected.trim())).unwrap_or(false);
        checksum_ok = Some(ok);
    }

    if !settings.downloads.antivirus_command.trim().is_empty() {
        run_antivirus(&settings.downloads.antivirus_command, &target).await;
    }

    inner.update(id, |x| {
        x.status = Status::Completed;
        x.completed_at = Some(now_ms());
        x.error = None;
        x.checksum_ok = checksum_ok;
        x.segments.clear();
        if let Ok(m) = std::fs::metadata(&target) {
            x.size = Some(m.len());
            x.downloaded = m.len();
        }
    });
    if checksum_ok == Some(false) {
        inner.update(id, |x| x.error = Some(err_text("checksum_mismatch", "")));
    }
    Outcome::Completed
}

/// İnternetten gelen dosyayı işaretler: Windows'ta Zone.Identifier, Linux'ta xdg kaynak özniteliği.
pub fn mark_origin(path: &Path, url: &str, referer: Option<&str>) {
    #[cfg(windows)]
    {
        let ads = format!("{}:Zone.Identifier", path.display());
        let mut body = String::from("[ZoneTransfer]\r\nZoneId=3\r\n");
        if let Some(r) = referer {
            body.push_str(&format!("ReferrerUrl={}\r\n", r));
        }
        body.push_str(&format!("HostUrl={}\r\n", url));
        let _ = std::fs::write(ads, body);
    }
    #[cfg(unix)]
    {
        let _ = xattr::set(path, "user.xdg.origin.url", url.as_bytes());
        if let Some(r) = referer {
            let _ = xattr::set(path, "user.xdg.referrer.url", r.as_bytes());
        }
    }
}

pub fn hash_file(path: &Path, algo: &str) -> Option<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; 1 << 20];
    macro_rules! digest {
        ($h:expr) => {{
            use sha2::Digest;
            let mut h = $h;
            loop {
                let n = f.read(&mut buf).ok()?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Some(hex::encode(h.finalize()))
        }};
    }
    match algo.to_ascii_lowercase().replace('-', "").as_str() {
        "md5" => digest!(md5::Md5::new()),
        "sha1" => digest!(sha1::Sha1::new()),
        "sha512" => digest!(sha2::Sha512::new()),
        _ => digest!(sha2::Sha256::new()),
    }
}

async fn run_antivirus(cmd: &str, file: &Path) {
    let line = cmd.replace("[File]", &format!("\"{}\"", file.display()));
    let mut c = if cfg!(windows) {
        let mut c = tokio::process::Command::new("cmd");
        c.arg("/C").arg(&line);
        c
    } else {
        let mut c = tokio::process::Command::new("sh");
        c.arg("-c").arg(&line);
        c
    };
    let _ = tokio::time::timeout(std::time::Duration::from_secs(600), c.status()).await;
}
