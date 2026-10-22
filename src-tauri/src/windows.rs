//! Pencerelerin açılması. Her pencere aynı arayüz paketinin bir rotasıdır.

use std::collections::HashMap;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

struct Spec {
    width: f64,
    height: f64,
    min: Option<(f64, f64)>,
    resizable: bool,
    /// Kimliğe göre ayrı pencere (her indirme için bir ilerleme penceresi gibi).
    per: Option<&'static str>,
}

fn spec(kind: &str) -> Spec {
    let s = |w, h, min, r, per| Spec { width: w, height: h, min, resizable: r, per };
    match kind {
        "main" => s(1040.0, 640.0, Some((760.0, 420.0)), true, None),
        "progress" => s(470.0, 345.0, None, true, Some("id")),
        "fileinfo" => s(580.0, 350.0, None, true, Some("token")),
        "complete" => s(480.0, 270.0, None, false, Some("id")),
        "addurl" => s(540.0, 290.0, None, true, None),
        "options" => s(780.0, 580.0, Some((640.0, 460.0)), true, None),
        "scheduler" => s(780.0, 540.0, Some((640.0, 420.0)), true, None),
        "batch" => s(600.0, 500.0, None, true, None),
        "grabber" => s(800.0, 580.0, Some((600.0, 420.0)), true, None),
        "media" => s(640.0, 600.0, Some((520.0, 420.0)), true, Some("token")),
        "about" => s(460.0, 380.0, None, false, None),
        "wizard" => s(760.0, 560.0, Some((640.0, 480.0)), true, None),
        "properties" => s(580.0, 520.0, None, true, Some("id")),
        "links" => s(800.0, 580.0, Some((600.0, 400.0)), true, Some("token")),
        _ => s(600.0, 400.0, None, true, None),
    }
}

fn encode(v: &str) -> String {
    url::form_urlencoded::byte_serialize(v.as_bytes()).collect()
}

pub fn label_for(kind: &str, params: &HashMap<String, String>) -> String {
    let sp = spec(kind);
    match sp.per.and_then(|k| params.get(k)) {
        Some(v) => format!("{}-{}", kind, v.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>()),
        None => kind.to_string(),
    }
}

/// Pencereyi açar; zaten açıksa öne getirir.
pub fn open(app: &AppHandle, kind: &str, params: HashMap<String, String>) -> tauri::Result<()> {
    let label = label_for(kind, &params);
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    let sp = spec(kind);
    let query: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", encode(k), encode(v))).collect();
    let url = format!("index.html#/{}{}{}", kind, if query.is_empty() { "" } else { "?" }, query.join("&"));
    let mut b = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
        .title("Internet Download Center")
        .inner_size(sp.width, sp.height)
        .resizable(sp.resizable)
        .center()
        .focused(true);
    if let Some((w, h)) = sp.min {
        b = b.min_inner_size(w, h);
    }
    if kind == "main" {
        b = b.visible(true);
    }
    let w = b.build()?;
    if kind == "main" {
        // Kapatınca tepsiye küçül (ayar açıksa).
        let app2 = app.clone();
        w.on_window_event(move |e| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                let st = app2.state::<crate::state::AppState>();
                let to_tray = st.engine.settings().general.close_to_tray && st.tray_ok.load(std::sync::atomic::Ordering::Relaxed);
                if to_tray && !st.quitting.load(std::sync::atomic::Ordering::Relaxed) {
                    api.prevent_close();
                    if let Some(w) = app2.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
            }
        });
    }
    Ok(())
}

pub fn open1(app: &AppHandle, kind: &str, key: &str, value: &str) {
    let mut p = HashMap::new();
    p.insert(key.to_string(), value.to_string());
    if let Err(e) = open(app, kind, p) {
        tracing::error!("{kind} penceresi açılamadı: {e}");
    }
}

pub fn show_main(app: &AppHandle) {
    if let Err(e) = open(app, "main", HashMap::new()) {
        tracing::error!("ana pencere açılamadı: {e}");
    }
}

pub fn close(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.close();
    }
}
