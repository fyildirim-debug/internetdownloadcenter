//! Arayüzün çağırdığı Tauri komutları.

use std::collections::HashMap;

use idc_core::model::*;
use idc_core::settings::{Queue, Settings};
use idc_core::system::{BrowserInfo, OsIntegration, ToolsStatus};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::flows;
use crate::state::{AppState, MediaChoice, PendingAdd, PendingLinks, PendingMedia};
use crate::windows;

type R<T> = Result<T, String>;

fn e(x: impl std::fmt::Display) -> String {
    x.to_string()
}

#[tauri::command]
pub fn list_downloads(st: State<AppState>) -> Vec<DownloadView> {
    st.engine.list()
}

#[tauri::command]
pub fn get_download(st: State<AppState>, id: String) -> R<DownloadDetail> {
    st.engine.detail(&id).ok_or_else(|| "not_found".into())
}

#[tauri::command]
pub async fn probe_url(st: State<'_, AppState>, url: String, headers: Option<Vec<(String, String)>>) -> R<ProbeResult> {
    let req = RequestInfo { url, headers: headers.unwrap_or_default(), ..Default::default() };
    st.engine.probe(&req).await.map_err(e)
}

#[tauri::command]
pub fn add_download(app: AppHandle, st: State<AppState>, req: AddRequest) -> R<String> {
    let start = req.start;
    let id = st.engine.add(req).map_err(e)?;
    if start {
        flows::maybe_progress_window(&app, &id);
    }
    Ok(id)
}

#[tauri::command]
pub fn add_many(st: State<AppState>, reqs: Vec<AddRequest>) -> R<Vec<String>> {
    let mut ids = vec![];
    for mut r in reqs {
        // Toplu eklemede ilerleme pencereleri açılmaz.
        r.silent = r.silent || r.start;
        ids.push(st.engine.add(r).map_err(e)?);
    }
    Ok(ids)
}

#[tauri::command]
pub fn start_downloads(app: AppHandle, st: State<AppState>, ids: Vec<String>) {
    st.engine.start(&ids);
    if ids.len() == 1 {
        flows::maybe_progress_window(&app, &ids[0]);
    }
}

#[tauri::command]
pub fn pause_downloads(st: State<AppState>, ids: Vec<String>) {
    st.engine.pause(&ids);
}

#[tauri::command]
pub async fn remove_downloads(app: AppHandle, st: State<'_, AppState>, ids: Vec<String>, delete_files: bool) -> R<()> {
    for id in &ids {
        for k in ["progress", "complete", "properties"] {
            windows::close(&app, &windows::label_for(k, &HashMap::from([("id".to_string(), id.clone())])));
        }
    }
    st.engine.remove(&ids, delete_files).await;
    Ok(())
}

#[tauri::command]
pub fn start_all(st: State<AppState>) {
    st.engine.start_all();
}

#[tauri::command]
pub fn stop_all(st: State<AppState>) {
    st.engine.stop_all();
}

#[tauri::command]
pub async fn delete_completed(st: State<'_, AppState>) -> R<()> {
    st.engine.delete_completed().await;
    Ok(())
}

#[tauri::command]
pub async fn redownload(app: AppHandle, st: State<'_, AppState>, id: String) -> R<()> {
    st.engine.redownload(&id).await;
    flows::maybe_progress_window(&app, &id);
    Ok(())
}

#[tauri::command]
pub fn update_download(st: State<AppState>, id: String, patch: DownloadPatch) -> R<()> {
    st.engine.update(&id, patch).map_err(e)
}

fn path_of(st: &AppState, id: &str) -> R<std::path::PathBuf> {
    let d = st.engine.get(id).ok_or("not_found")?;
    Ok(if d.status == Status::Completed { d.target_path() } else { d.part_path() })
}

#[tauri::command]
pub fn open_file(st: State<AppState>, id: String) -> R<()> {
    let p = path_of(&st, &id)?;
    tauri_plugin_opener::open_path(p.to_string_lossy().to_string(), None::<&str>).map_err(e)
}

#[tauri::command]
pub fn open_with(st: State<AppState>, id: String) -> R<()> {
    let p = path_of(&st, &id)?;
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32.exe").arg("shell32.dll,OpenAs_RunDLL").arg(&p).spawn().map_err(e)?;
        return Ok(());
    }
    #[cfg(unix)]
    {
        // Masaüstü portalının uygulama seçicisi (GNOME, KDE, XFCE'de xdg-desktop-portal ile).
        let uri = url::Url::from_file_path(&p).map(|u| u.to_string()).unwrap_or_default();
        let ok = std::process::Command::new("gdbus")
            .args([
                "call", "--session", "--dest", "org.freedesktop.portal.Desktop", "--object-path", "/org/freedesktop/portal/desktop",
                "--method", "org.freedesktop.portal.OpenURI.OpenURI", "", &uri, "{'ask': <true>}",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            // Portal yoksa dosyayı klasöründe göster; kullanıcı sağ tıkla "Birlikte aç"ı seçer.
            tauri_plugin_opener::reveal_item_in_dir(&p).map_err(e)?;
        }
        Ok(())
    }
}

#[tauri::command]
pub fn open_folder(st: State<AppState>, id: String) -> R<()> {
    let d = st.engine.get(&id).ok_or("not_found")?;
    let p = if d.status == Status::Completed { d.target_path() } else { d.part_path() };
    if p.exists() {
        tauri_plugin_opener::reveal_item_in_dir(&p).map_err(e)
    } else {
        tauri_plugin_opener::open_path(d.save_dir.clone(), None::<&str>).map_err(e)
    }
}

#[tauri::command]
pub fn get_settings(st: State<AppState>) -> Settings {
    st.engine.settings()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, st: State<AppState>, settings: Settings) -> R<Settings> {
    let old = st.engine.settings();
    let s = st.engine.save_settings(settings).map_err(e)?;
    let _ = app.emit("idc://settings", &s);
    if old.general.launch_on_startup != s.general.launch_on_startup {
        let al = tauri_plugin_autostart::ManagerExt::autolaunch(&app);
        let _ = if s.general.launch_on_startup { al.enable() } else { al.disable() };
    }
    if old.general.clipboard_monitor != s.general.clipboard_monitor {
        crate::clipboard::reset();
    }
    crate::tray::refresh(&app);
    Ok(s)
}

#[tauri::command]
pub async fn media_probe(app: AppHandle, url: String, headers: Option<Vec<(String, String)>>) -> R<MediaProbe> {
    let r = idc_core::ext::MediaRequest { page_url: url, headers: headers.unwrap_or_default(), ..Default::default() };
    flows::media_probe(&app, &r).await
}

#[tauri::command]
pub fn tools_status(st: State<AppState>) -> ToolsStatus {
    st.engine.tools().status()
}

#[tauri::command]
pub async fn tools_update(app: AppHandle, st: State<'_, AppState>) -> R<ToolsStatus> {
    let tools = st.engine.tools().clone();
    // İlerleme mesajlarını yayınla.
    let app2 = app.clone();
    let t2 = tools.clone();
    let ticker = tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let s = t2.status();
            let _ = app2.emit("idc://tools", &s);
            if !s.busy {
                break;
            }
        }
    });
    let r = tools.ensure(true).await;
    ticker.abort();
    let s = tools.status();
    let _ = app.emit("idc://tools", &s);
    r.map_err(e)
}

#[tauri::command]
pub fn browsers_status(app: AppHandle) -> Vec<BrowserInfo> {
    crate::integration::status(&app)
}

#[tauri::command]
pub fn browsers_install(app: AppHandle, ids: Option<Vec<String>>) -> R<Vec<BrowserInfo>> {
    crate::integration::install(&app, ids).map_err(e)
}

#[tauri::command]
pub fn browser_open_extensions(app: AppHandle, id: String) -> R<()> {
    crate::integration::open_extensions_page(&app, &id).map_err(e)
}

#[tauri::command]
pub fn os_integration_status(app: AppHandle) -> OsIntegration {
    crate::integration::os_status(&app)
}

#[tauri::command]
pub fn os_integration_apply(app: AppHandle, opts: OsIntegration) -> R<OsIntegration> {
    crate::integration::os_apply(&app, &opts).map_err(e)
}

#[tauri::command]
pub fn open_window(app: AppHandle, kind: String, params: Option<HashMap<String, String>>) -> R<()> {
    windows::open(&app, &kind, params.unwrap_or_default()).map_err(e)
}

#[tauri::command]
pub fn start_queue(st: State<AppState>, queue_id: String) {
    st.engine.start_queue(&queue_id);
}

#[tauri::command]
pub fn stop_queue(st: State<AppState>, queue_id: String) {
    st.engine.stop_queue(&queue_id);
}

#[tauri::command]
pub fn save_queue(app: AppHandle, st: State<AppState>, queue: Queue) -> R<()> {
    st.engine.save_queue(queue).map_err(e)?;
    let _ = app.emit("idc://settings", st.engine.settings());
    Ok(())
}

#[tauri::command]
pub fn delete_queue(app: AppHandle, st: State<AppState>, id: String) -> R<()> {
    st.engine.delete_queue(&id).map_err(e)?;
    let _ = app.emit("idc://settings", st.engine.settings());
    Ok(())
}

#[tauri::command]
pub fn move_in_queue(app: AppHandle, st: State<AppState>, queue_id: String, ids: Vec<String>, delta: i32) {
    st.engine.move_in_queue(&queue_id, &ids, delta);
    let _ = app.emit("idc://settings", st.engine.settings());
}

#[tauri::command]
pub fn batch_preview(pattern: String, from: String, to: String, step: u32, padding: u32) -> Vec<String> {
    crate::grabber::batch(&pattern, &from, &to, step.max(1), padding as usize)
}

#[tauri::command]
pub async fn grab_links(st: State<'_, AppState>, url: String, depth: u32, extensions: Vec<String>) -> R<Vec<GrabbedLink>> {
    let s = st.engine.settings();
    crate::grabber::grab(&s, &url, depth.clamp(1, 3), &extensions).await.map_err(e)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    os: String,
    data_dir: String,
    first_run: bool,
}

#[tauri::command]
pub fn app_info(st: State<AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        data_dir: idc_core::paths::config_dir().to_string_lossy().into(),
        first_run: st.engine.is_first_run(),
    }
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> R<()> {
    crate::quit(&app).await;
    Ok(())
}

#[tauri::command]
pub fn create_pending(app: AppHandle, req: AddRequest) -> String {
    flows::create_pending(&app, req, None)
}

#[tauri::command]
pub fn get_pending(st: State<AppState>, token: String) -> R<PendingAdd> {
    st.pending.lock().get(&token).cloned().ok_or_else(|| "not_found".into())
}

#[tauri::command]
pub async fn confirm_pending(app: AppHandle, token: String, req: AddRequest) -> R<String> {
    flows::confirm_pending(&app, &token, req).await.map_err(e)
}

#[tauri::command]
pub async fn cancel_pending(st: State<'_, AppState>, token: String) -> R<()> {
    let p = st.pending.lock().remove(&token);
    // Arka planda başlamış indirme varsa iptal edip yarım dosyayı sil.
    if let Some(id) = p.and_then(|p| p.download_id) {
        st.engine.remove(&[id], true).await;
    }
    st.pending.lock().remove(&token);
    st.pending_media.lock().remove(&token);
    st.pending_links.lock().remove(&token);
    Ok(())
}

#[tauri::command]
pub fn get_pending_media(st: State<AppState>, token: String) -> R<PendingMedia> {
    st.pending_media.lock().get(&token).cloned().ok_or_else(|| "not_found".into())
}

#[tauri::command]
pub fn confirm_pending_media(app: AppHandle, st: State<AppState>, token: String, choice: MediaChoice) -> R<String> {
    let p = st.pending_media.lock().remove(&token).ok_or("not_found")?;
    let probe = p.probe.clone();
    let size = probe.as_ref().and_then(|pr| pr.formats.iter().find(|f| f.format_id == choice.format_id && f.audio_only == choice.audio_only).and_then(|f| f.filesize));
    let mut req = p.request.clone();
    if req.title.is_none() {
        req.title = probe.as_ref().map(|pr| pr.title.clone());
    }
    let name = Some(choice.file_name.clone()).filter(|n| !n.trim().is_empty());
    let dir = Some(choice.save_dir.clone()).filter(|n| !n.trim().is_empty());
    flows::add_media(&app, &req, choice.format_id, choice.audio_only, name, dir, choice.subtitles, size, probe.and_then(|p| p.thumbnail)).map_err(e)
}

#[tauri::command]
pub fn get_pending_links(st: State<AppState>, token: String) -> R<PendingLinks> {
    st.pending_links.lock().get(&token).cloned().ok_or_else(|| "not_found".into())
}

#[tauri::command]
pub fn set_global_speed_limit(app: AppHandle, st: State<AppState>, kib: Option<u64>) -> R<()> {
    st.engine.set_global_speed_limit(kib).map_err(e)?;
    let _ = app.emit("idc://settings", st.engine.settings());
    crate::tray::refresh(&app);
    Ok(())
}

#[derive(Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportItem {
    url: String,
    file_name: String,
    save_dir: String,
    referer: Option<String>,
    description: String,
}

#[tauri::command]
pub fn export_list(st: State<AppState>, path: String) -> R<()> {
    let items: Vec<ExportItem> = st
        .engine
        .list()
        .into_iter()
        .filter_map(|v| st.engine.get(&v.id))
        .map(|d| ExportItem { url: d.request.url.clone(), file_name: d.file_name.clone(), save_dir: d.save_dir.clone(), referer: d.request.referer.clone(), description: d.description.clone() })
        .collect();
    idc_core::store::write_json(std::path::Path::new(&path), &items).map_err(e)
}

#[tauri::command]
pub fn import_list(st: State<AppState>, path: String) -> R<u32> {
    let data = std::fs::read(&path).map_err(e)?;
    // JSON listesi ya da satır başına bir adres.
    let items: Vec<ExportItem> = match serde_json::from_slice(&data) {
        Ok(v) => v,
        Err(_) => String::from_utf8_lossy(&data)
            .lines()
            .map(|l| l.trim())
            .filter(|l| l.starts_with("http://") || l.starts_with("https://"))
            .map(|l| ExportItem { url: l.into(), file_name: String::new(), save_dir: String::new(), referer: None, description: String::new() })
            .collect(),
    };
    let mut n = 0;
    for it in items {
        let req = AddRequest {
            request: RequestInfo { url: it.url, referer: it.referer, ..Default::default() },
            file_name: Some(it.file_name).filter(|s| !s.is_empty()),
            save_dir: Some(it.save_dir).filter(|s| !s.is_empty()),
            description: Some(it.description),
            start: false,
            ..Default::default()
        };
        if st.engine.add(req).is_ok() {
            n += 1;
        }
    }
    Ok(n)
}
