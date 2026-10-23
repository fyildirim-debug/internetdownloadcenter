//! Programın yeni sürümünü GitHub Sürümler sayfasından denetler.

use std::time::Duration;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

const RELEASES_API: &str = "https://api.github.com/repos/fyildirim-debug/internetdownloadcenter/releases/latest";
pub const RELEASES_PAGE: &str = "https://github.com/fyildirim-debug/internetdownloadcenter/releases/latest";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub url: String,
    pub newer: bool,
}

static LATEST: Mutex<Option<UpdateInfo>> = Mutex::new(None);

/// Bulunan yeni sürüm (tepsi menüsü için).
pub fn available() -> Option<UpdateInfo> {
    LATEST.lock().clone().filter(|u| u.newer)
}

fn parse(v: &str) -> Vec<u64> {
    v.trim_start_matches('v').split(['.', '-']).take(3).map(|p| p.parse().unwrap_or(0)).collect()
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    parse(latest) > parse(current)
}

/// Son sürümü sorar; yeniyse bir kez bildirim gösterir. `manual` ise sonuç her durumda bildirilir.
pub async fn check(app: &AppHandle, manual: bool) -> Option<UpdateInfo> {
    let client = reqwest::Client::builder().user_agent("InternetDownloadCenter").timeout(Duration::from_secs(20)).build().ok()?;
    let resp = client.get(RELEASES_API).header("Accept", "application/vnd.github+json").send().await.ok()?;
    if !resp.status().is_success() {
        tracing::info!("sürüm denetimi: HTTP {}", resp.status());
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&resp.bytes().await.ok()?).ok()?;
    let tag = v.get("tag_name")?.as_str()?.to_string();
    let url = v.get("html_url").and_then(|u| u.as_str()).unwrap_or(RELEASES_PAGE).to_string();
    let current = env!("CARGO_PKG_VERSION").to_string();
    let info = UpdateInfo { newer: is_newer(&tag, &current), latest: tag.trim_start_matches('v').to_string(), current, url };
    *LATEST.lock() = Some(info.clone());
    let _ = app.emit("idc://update", &info);
    crate::tray::refresh(app);
    if info.newer {
        // Aynı sürüm için bildirimi yalnız bir kez göster.
        let stamp = idc_core::paths::config_dir().join("update-notified.txt");
        let seen = std::fs::read_to_string(&stamp).unwrap_or_default();
        if manual || seen.trim() != info.latest {
            crate::notify(app, "update", &info.latest);
            let _ = std::fs::write(stamp, &info.latest);
        }
    } else if manual {
        crate::notify(app, "uptodate", &info.current);
    }
    Some(info)
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions() {
        assert!(super::is_newer("v1.0.1", "1.0.0"));
        assert!(super::is_newer("v1.10.0", "1.9.9"));
        assert!(!super::is_newer("v1.0.0", "1.0.0"));
        assert!(!super::is_newer("0.9.0", "1.0.0"));
    }
}
