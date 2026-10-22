//! Arayüz, tarayıcı eklentisi, pano ve komut satırının ortak kullandığı indirme akışları.

use std::collections::HashMap;
use std::time::Duration;

use idc_core::ext::{DownloadCapture, ExtResponse, LinkItem, MediaRequest};
use idc_core::model::*;
use idc_core::util::{err_text, site_matches};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::{AppState, PendingAdd, PendingLinks, PendingMedia};
use crate::windows;

pub fn state(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}

/// Kullanıcı başlattıysa ve ayar açıksa ilerleme penceresini açar.
pub fn maybe_progress_window(app: &AppHandle, id: &str) {
    let st = state(app);
    let s = st.engine.settings();
    let silent = st.engine.get(id).map(|d| d.silent).unwrap_or(true);
    if silent || s.downloads.progress_dialog == "none" {
        return;
    }
    windows::open1(app, "progress", "id", id);
    if s.downloads.progress_dialog == "minimized" {
        let label = windows::label_for("progress", &HashMap::from([("id".to_string(), id.to_string())]));
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.minimize();
        }
    }
}

/// Onay penceresi için bekleyen indirme oluşturur ve dosya bilgisini arka planda sorar.
pub fn create_pending(app: &AppHandle, req: AddRequest, probe: Option<ProbeResult>) -> String {
    let st = state(app);
    let settings = st.engine.settings();
    let token = new_id();
    let url = req.request.url.clone();
    let name = req
        .file_name
        .clone()
        .or_else(|| probe.as_ref().map(|p| p.file_name.clone()))
        .unwrap_or_else(|| idc_core::filename::resolve(None, &url, &url, req.mime_hint.as_deref()));
    let category = req.category.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| settings.category_for(&name));
    let dir = req.save_dir.clone().filter(|d| !d.is_empty()).unwrap_or_else(|| settings.dir_for_category(&category));
    let duplicate_of = st.engine.list().into_iter().find(|d| d.url == url || (d.file_name == name && d.save_dir == dir)).map(|d| d.id);
    let need_probe = probe.is_none();
    let p = PendingAdd {
        token: token.clone(),
        request: req.clone(),
        probe,
        probing: need_probe,
        error: None,
        suggested_name: name,
        suggested_dir: dir,
        category,
        duplicate_of,
    };
    st.pending.lock().insert(token.clone(), p);
    if need_probe {
        let app2 = app.clone();
        let tok = token.clone();
        tauri::async_runtime::spawn(async move {
            let st = state(&app2);
            let r = tokio::time::timeout(Duration::from_secs(25), st.engine.probe(&req.request)).await;
            let settings = st.engine.settings();
            let updated = {
                let mut map = st.pending.lock();
                let Some(p) = map.get_mut(&tok) else { return };
                p.probing = false;
                match r {
                    Ok(Ok(pr)) => {
                        if req.file_name.is_none() {
                            p.suggested_name = pr.file_name.clone();
                            if req.category.is_none() {
                                p.category = pr.category.clone();
                                if req.save_dir.is_none() {
                                    p.suggested_dir = settings.dir_for_category(&pr.category);
                                }
                            }
                        }
                        if pr.status_code >= 400 {
                            p.error = Some(err_text("http_status", pr.status_code));
                        }
                        p.probe = Some(pr);
                    }
                    Ok(Err(e)) => p.error = Some(err_text("network", e)),
                    Err(_) => p.error = Some(err_text("timeout", "")),
                }
                p.clone()
            };
            let _ = app2.emit("idc://pending", updated);
        });
    }
    windows::open1(app, "fileinfo", "token", &token);
    token
}

/// Onaylanan bekleyen indirmeyi ekler.
pub fn confirm_pending(app: &AppHandle, token: &str, req: AddRequest) -> anyhow::Result<String> {
    let st = state(app);
    let p = st.pending.lock().remove(token);
    let mut req = req;
    if req.size_hint.is_none() {
        req.size_hint = p.as_ref().and_then(|p| p.probe.as_ref()).and_then(|x| x.size);
    }
    if req.mime_hint.is_none() {
        req.mime_hint = p.as_ref().and_then(|p| p.probe.as_ref()).and_then(|x| x.mime.clone());
    }
    let start = req.start;
    let id = st.engine.add(req)?;
    if start {
        maybe_progress_window(app, &id);
    }
    Ok(id)
}

/// Eklentinin ya da başka kaynağın getirdiği çerez/başlıkları içeren istek.
fn request_from_capture(c: &DownloadCapture) -> RequestInfo {
    RequestInfo {
        url: c.url.clone(),
        referer: c.referrer.clone().or_else(|| c.page_url.clone()),
        headers: c.headers.clone(),
        cookies: c.cookies.clone(),
        raw_cookie: None,
        user_agent: c.user_agent.clone(),
        method: c.method.clone(),
        post_data: c.post_data.clone(),
    }
}

fn is_html_mime(m: Option<&str>) -> bool {
    m.map(|m| m.starts_with("text/html") || m.starts_with("application/xhtml")).unwrap_or(false)
}

/// Tarayıcıdan yakalanan indirme.
pub async fn handle_capture(app: &AppHandle, c: DownloadCapture) -> ExtResponse {
    let st = state(app);
    let s = st.engine.settings();
    let rej = |reason: &str, message: Option<String>| ExtResponse::Rejected { reason: reason.into(), message };

    if !s.general.browser_integration && !c.forced {
        return rej("disabled", None);
    }
    if !(c.url.starts_with("http://") || c.url.starts_with("https://")) {
        return rej("unsupported", None);
    }
    if c.method.as_deref().map(|m| !m.eq_ignore_ascii_case("GET")).unwrap_or(false) {
        return rej("unsupported", Some("POST".into()));
    }
    if !c.forced {
        let ft = &s.file_types;
        if ft.skip_sites.iter().any(|p| site_matches(p, &c.url) || c.page_url.as_deref().map(|u| site_matches(p, u)).unwrap_or(false)) {
            return rej("excluded", None);
        }
        if ft.skip_addresses.iter().any(|p| idc_core::util::wildcard_match(&p.to_lowercase(), &c.url.to_lowercase())) {
            return rej("excluded", None);
        }
        if let (Some(sz), true) = (c.file_size, ft.min_size_kib > 0) {
            if sz > 0 && sz < ft.min_size_kib * 1024 {
                return rej("excluded", Some("size".into()));
            }
        }
    }

    let req = request_from_capture(&c);

    // Adres yenileme: aynı ad ve boyuttaki yarım indirme yeni adresle sürer.
    if let (Some(name), Some(size)) = (c.file_name.as_deref(), c.file_size) {
        if let Some(id) = st.engine.find_unfinished(name, Some(size)) {
            st.engine.refresh_request(&id, req.clone());
            st.engine.start(std::slice::from_ref(&id));
            crate::notify(app, "refreshed", name);
            maybe_progress_window(app, &id);
            return ExtResponse::Accepted { id: Some(id) };
        }
    }

    // Sunucu bizi kabul ediyor mu? Etmiyorsa indirme tarayıcıda kalsın.
    let probe = tokio::time::timeout(Duration::from_secs(12), st.engine.probe(&req)).await;
    let (probe, blocked) = match probe {
        Ok(Ok(p)) => {
            let html_trap = p.is_html && !is_html_mime(c.mime.as_deref());
            let bad = p.status_code >= 400 || html_trap;
            let why = if p.status_code >= 400 { format!("HTTP {}", p.status_code) } else { "HTML".to_string() };
            (Some(p), if bad { Some(why) } else { None })
        }
        Ok(Err(e)) => (None, Some(e.to_string())),
        // Yavaş sunucu engel sayılmaz; motor yeniden dener.
        Err(_) => (None, None),
    };

    if let Some(why) = blocked {
        tracing::info!("indirme engellendi ({why}): {}", c.url);
        if s.media.browser_fallback && !c.forced {
            return rej("blocked", Some(why));
        }
        if s.media.impersonate_fallback {
            let spec = MediaSpec { page_url: c.url.clone(), format_id: String::new(), impersonate: true, ..Default::default() };
            let name = c.file_name.clone().or_else(|| idc_core::filename::from_url(&c.url));
            let add = AddRequest { request: req, file_name: name, media: Some(spec), start: true, size_hint: c.file_size, ..Default::default() };
            return match st.engine.add(add) {
                Ok(id) => {
                    maybe_progress_window(app, &id);
                    ExtResponse::Accepted { id: Some(id) }
                }
                Err(e) => rej("error", Some(e.to_string())),
            };
        }
    }

    let add = AddRequest {
        request: req,
        file_name: c.file_name.clone().filter(|n| !n.is_empty()),
        size_hint: c.file_size.or(probe.as_ref().and_then(|p| p.size)),
        mime_hint: c.mime.clone().or(probe.as_ref().and_then(|p| p.mime.clone())),
        description: c.tab_title.clone(),
        start: true,
        ..Default::default()
    };
    if s.downloads.show_file_info_dialog {
        let token = create_pending(app, AddRequest { start: false, ..add }, probe);
        return ExtResponse::Accepted { id: Some(token) };
    }
    match st.engine.add(add) {
        Ok(id) => {
            maybe_progress_window(app, &id);
            ExtResponse::Accepted { id: Some(id) }
        }
        Err(e) => rej("error", Some(e.to_string())),
    }
}

fn media_request_info(r: &MediaRequest, url: &str) -> RequestInfo {
    RequestInfo {
        url: url.to_string(),
        referer: r.referrer.clone().or_else(|| if url != r.page_url { Some(r.page_url.clone()) } else { None }),
        headers: r.headers.clone(),
        cookies: r.cookies.clone(),
        user_agent: r.user_agent.clone(),
        ..Default::default()
    }
}

/// Sorgulanacak adres: bilinen video sitesinde sayfa, diğerlerinde yakalanan akış.
fn media_source(r: &MediaRequest) -> String {
    match &r.media_url {
        Some(m) if !m.is_empty() && !idc_core::media::is_media_site(&r.page_url) => m.clone(),
        _ => r.page_url.clone(),
    }
}

pub async fn media_probe(app: &AppHandle, r: &MediaRequest) -> Result<MediaProbe, String> {
    let st = state(app);
    if st.engine.tools().ytdlp().is_none() {
        let _ = app.emit("idc://tools", st.engine.tools().status());
        st.engine.tools().ensure(false).await.map_err(|e| err_text("tools_missing", e))?;
        let _ = app.emit("idc://tools", st.engine.tools().status());
    }
    let src = media_source(r);
    let settings = st.engine.settings();
    let res = idc_core::media::probe(st.engine.tools(), &settings, &media_request_info(r, &src)).await;
    match res {
        Ok(p) => Ok(p),
        // Sayfa sorgusu başarısızsa yakalanan akışı dene.
        Err(e) => match &r.media_url {
            Some(m) if !m.is_empty() && *m != src => idc_core::media::probe(st.engine.tools(), &settings, &media_request_info(r, m)).await.map_err(|e| e.to_string()),
            _ => Err(e.to_string()),
        },
    }
}

/// Medya indirmesini doğrudan ekler.
pub fn add_media(app: &AppHandle, r: &MediaRequest, format_id: String, audio_only: bool, file_name: Option<String>, save_dir: Option<String>, subtitles: Vec<String>, size_hint: Option<u64>, thumbnail: Option<String>) -> anyhow::Result<String> {
    let st = state(app);
    let src = media_source(r);
    let settings = st.engine.settings();
    let title = file_name.clone().or_else(|| r.title.clone()).filter(|t| !t.trim().is_empty()).unwrap_or_else(|| "video".into());
    let spec = MediaSpec {
        page_url: src.clone(),
        media_url: r.media_url.clone(),
        format_id,
        audio_only,
        audio_format: if audio_only { Some(settings.media.audio_format.clone()) } else { None },
        container: None,
        title: Some(title.clone()),
        thumbnail,
        impersonate: false,
        subtitles,
    };
    let category = if audio_only { "music" } else { "video" };
    let add = AddRequest {
        request: media_request_info(r, &src),
        file_name: Some(title),
        save_dir,
        category: Some(category.into()),
        size_hint,
        media: Some(spec),
        start: true,
        ..Default::default()
    };
    let id = st.engine.add(add)?;
    maybe_progress_window(app, &id);
    Ok(id)
}

pub async fn handle_media_download(app: &AppHandle, r: MediaRequest) -> ExtResponse {
    let quick = r.quick;
    if let Some(fid) = r.format_id.clone().filter(|f| !f.is_empty()) {
        return match add_media(app, &r, fid, r.audio_only, None, None, vec![], None, None) {
            Ok(id) => ExtResponse::Accepted { id: Some(id) },
            Err(e) => ExtResponse::Rejected { reason: "error".into(), message: Some(e.to_string()) },
        };
    }
    if quick {
        return match add_media(app, &r, String::new(), r.audio_only, None, None, vec![], None, None) {
            Ok(id) => ExtResponse::Accepted { id: Some(id) },
            Err(e) => ExtResponse::Rejected { reason: "error".into(), message: Some(e.to_string()) },
        };
    }
    let token = open_media_chooser(app, r);
    ExtResponse::Accepted { id: Some(token) }
}

/// Kalite seçme penceresini açar ve biçimleri arka planda sorgular.
pub fn open_media_chooser(app: &AppHandle, r: MediaRequest) -> String {
    let st = state(app);
    let token = new_id();
    st.pending_media.lock().insert(token.clone(), PendingMedia { token: token.clone(), request: r.clone(), probe: None, probing: true, error: None });
    let app2 = app.clone();
    let tok = token.clone();
    tauri::async_runtime::spawn(async move {
        let res = media_probe(&app2, &r).await;
        let st = state(&app2);
        let updated = {
            let mut map = st.pending_media.lock();
            let Some(p) = map.get_mut(&tok) else { return };
            p.probing = false;
            match res {
                Ok(pr) => p.probe = Some(pr),
                Err(e) => p.error = Some(e),
            }
            p.clone()
        };
        let _ = app2.emit("idc://pending-media", updated);
    });
    windows::open1(app, "media", "token", &token);
    token
}

pub fn handle_links(app: &AppHandle, links: Vec<LinkItem>, referrer: Option<String>) -> ExtResponse {
    let st = state(app);
    let settings = st.engine.settings();
    let mut seen = std::collections::HashSet::new();
    let list: Vec<GrabbedLink> = links
        .into_iter()
        .filter(|l| (l.url.starts_with("http://") || l.url.starts_with("https://")) && seen.insert(l.url.clone()))
        .map(|l| {
            let name = idc_core::filename::from_url(&l.url).unwrap_or_else(|| "download".into());
            GrabbedLink { category: settings.category_for(&name), file_name: name, text: l.text, url: l.url, depth: 0 }
        })
        .collect();
    let token = new_id();
    st.pending_links.lock().insert(token.clone(), PendingLinks { token: token.clone(), links: list, referrer });
    windows::open1(app, "links", "token", &token);
    ExtResponse::Accepted { id: Some(token) }
}

/// Komut satırı, `idc://` adresi ya da panodan gelen adres.
pub fn handle_url_arg(app: &AppHandle, raw: &str) {
    let raw = raw.trim();
    let url = if let Some(rest) = raw.strip_prefix("idc://") {
        // idc://add?url=... ya da idc://https://...
        let parsed = url::Url::parse(raw).ok();
        let from_query = parsed.as_ref().and_then(|u| u.query_pairs().find(|(k, _)| k == "url").map(|(_, v)| v.to_string()));
        from_query.unwrap_or_else(|| rest.trim_start_matches("add/").to_string())
    } else {
        raw.to_string()
    };
    if !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("ftp://")) {
        return;
    }
    if idc_core::media::is_media_site(&url) {
        open_media_chooser(app, MediaRequest { page_url: url, ..Default::default() });
        return;
    }
    let req = AddRequest { request: RequestInfo { url, ..Default::default() }, start: false, ..Default::default() };
    create_pending(app, req, None);
}
