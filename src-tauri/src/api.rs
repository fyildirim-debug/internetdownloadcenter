//! Tarayıcı köprüsünün (idc-host) bağlandığı yerel HTTP uç noktası.

use std::net::SocketAddr;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use idc_core::ext::{ExtConfig, ExtRequest, ExtResponse};
use tauri::AppHandle;

use crate::flows;

#[derive(Clone)]
struct Ctx {
    app: AppHandle,
    token: String,
}

pub fn ext_config(app: &AppHandle) -> ExtConfig {
    let s = flows::state(app).engine.settings();
    ExtConfig {
        enabled: s.general.browser_integration,
        capture_extensions: s.file_types.capture_extensions.clone(),
        skip_extensions: s.file_types.skip_extensions.clone(),
        skip_sites: s.file_types.skip_sites.clone(),
        skip_addresses: s.file_types.skip_addresses.clone(),
        min_size_kib: s.file_types.min_size_kib,
        video_panel: s.general.video_panel,
        bypass_key: s.general.bypass_key.clone(),
        force_key: s.general.force_key.clone(),
        language: crate::lang(&s),
        browser_fallback: s.media.browser_fallback,
    }
}

async fn handle(State(ctx): State<Ctx>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Result<Json<ExtResponse>, StatusCode> {
    let ok = headers.get("x-idc-token").and_then(|v| v.to_str().ok()).map(|t| constant_eq(t, &ctx.token)).unwrap_or(false);
    if !ok {
        return Err(StatusCode::FORBIDDEN);
    }
    let req: ExtRequest = match serde_json::from_value(body) {
        Ok(r) => r,
        Err(e) => return Ok(Json(ExtResponse::Error { message: format!("geçersiz mesaj: {e}") })),
    };
    let app = &ctx.app;
    let resp = match req {
        ExtRequest::Hello { .. } => ExtResponse::Config { config: ext_config(app), app_version: env!("CARGO_PKG_VERSION").into() },
        ExtRequest::Download(c) => flows::handle_capture(app, c).await,
        ExtRequest::MediaProbe(r) => match flows::media_probe(app, &r).await {
            Ok(probe) => ExtResponse::Media { probe },
            Err(e) => ExtResponse::Error { message: e },
        },
        ExtRequest::MediaDownload(r) => flows::handle_media_download(app, r).await,
        ExtRequest::Links { links, referrer } => flows::handle_links(app, links, referrer),
        ExtRequest::Show => {
            crate::windows::show_main(app);
            ExtResponse::Accepted { id: None }
        }
    };
    Ok(Json(resp))
}

fn constant_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Sunucuyu başlatır ve `ipc.json` dosyasını yazar.
pub async fn start(app: AppHandle) -> anyhow::Result<()> {
    use rand::RngCore;
    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    let token = hex::encode(raw);
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let port = listener.local_addr()?.port();
    let router = Router::new().route("/api/ext", post(handle)).with_state(Ctx { app, token: token.clone() });
    let info = serde_json::json!({ "port": port, "token": token, "pid": std::process::id() });
    idc_core::store::write_json(&idc_core::paths::ipc_file(), &info)?;
    tracing::info!("yerel API 127.0.0.1:{port}");
    tauri::async_runtime::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("yerel API durdu: {e}");
        }
    });
    Ok(())
}

pub fn cleanup() {
    let p = idc_core::paths::ipc_file();
    // Yalnız bu sürecin yazdığı dosyayı sil.
    if let Some(v) = idc_core::store::read_json::<serde_json::Value>(&p) {
        if v.get("pid").and_then(|x| x.as_u64()) == Some(std::process::id() as u64) {
            let _ = std::fs::remove_file(p);
        }
    }
}
