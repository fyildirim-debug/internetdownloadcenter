//! Internet Download Center masaüstü uygulaması.

mod api;
mod clipboard;
mod commands;
mod flows;
mod grabber;
mod integration;
mod state;
mod tray;
mod windows;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use idc_core::model::{AfterAction, Status};
use idc_core::{Engine, EngineEvent, Settings};
use tauri::{AppHandle, Emitter, Manager, RunEvent};
use tauri_plugin_notification::NotificationExt;

use state::AppState;

/// İşletim sisteminin dili: `tr` ya da `en`.
pub fn sys_lang() -> &'static str {
    let loc = sys_locale::get_locale().unwrap_or_default().to_lowercase();
    if loc.starts_with("tr") { "tr" } else { "en" }
}

/// Arayüz dili: `tr` ya da `en`.
pub fn lang(s: &Settings) -> String {
    match s.language.as_str() {
        "tr" | "en" => s.language.clone(),
        _ => sys_lang().into(),
    }
}

fn t(app: &AppHandle, tr: &str, en: &str) -> String {
    if lang(&flows::state(app).engine.settings()) == "tr" { tr.into() } else { en.into() }
}

pub fn notify(app: &AppHandle, kind: &str, detail: &str) {
    let body = match kind {
        "refreshed" => t(app, "Adres yenilendi, indirme sürüyor", "Address refreshed, download resumed"),
        "completed" => t(app, "İndirme tamamlandı", "Download complete"),
        "failed" => t(app, "İndirme başarısız", "Download failed"),
        "queue" => t(app, "Kuyruk tamamlandı", "Queue finished"),
        "checksum" => t(app, "Sağlama değeri UYUŞMUYOR", "Checksum MISMATCH"),
        _ => kind.to_string(),
    };
    let _ = app.notification().builder().title("Internet Download Center").body(format!("{body}: {detail}")).show();
}

/// Görevleri durdurup kaydeder ve uygulamadan çıkar.
pub async fn quit(app: &AppHandle) {
    let st = flows::state(app);
    st.quitting.store(true, Ordering::Relaxed);
    st.engine.shutdown().await;
    api::cleanup();
    app.exit(0);
}

fn run_after(app: &AppHandle, action: AfterAction) {
    let (tr, en) = match action {
        AfterAction::None => return,
        AfterAction::Exit => ("Internet Download Center kapatılacak", "Internet Download Center will exit"),
        AfterAction::Shutdown => ("Bilgisayar 60 saniye içinde kapatılacak", "The computer will shut down in 60 seconds"),
        AfterAction::Sleep => ("Bilgisayar 60 saniye içinde uyku moduna geçecek", "The computer will sleep in 60 seconds"),
        AfterAction::Hibernate => ("Bilgisayar 60 saniye içinde hazırda bekletilecek", "The computer will hibernate in 60 seconds"),
    };
    if action == AfterAction::Exit {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { quit(&app).await });
        return;
    }
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
    let cancelled = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    let (c2, d2, app2) = (cancelled.clone(), done.clone(), app.clone());
    app.dialog()
        .message(t(app, tr, en))
        .title("Internet Download Center")
        .buttons(MessageDialogButtons::OkCancelCustom(t(app, "Şimdi", "Now"), t(app, "İptal", "Cancel")))
        .show(move |ok| {
            if ok {
                if !d2.swap(true, Ordering::SeqCst) {
                    let app = app2.clone();
                    tauri::async_runtime::spawn(async move { power(&app, action).await });
                }
            } else {
                c2.store(true, Ordering::SeqCst);
            }
        });
    let app3 = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
        if !cancelled.load(Ordering::SeqCst) && !done.swap(true, Ordering::SeqCst) {
            power(&app3, action).await;
        }
    });
}

async fn power(app: &AppHandle, action: AfterAction) {
    flows::state(app).engine.shutdown().await;
    let cmds: &[&[&str]] = if cfg!(windows) {
        match action {
            AfterAction::Shutdown => &[&["shutdown", "/s", "/t", "0"]],
            AfterAction::Sleep => &[&["rundll32.exe", "powrprof.dll,SetSuspendState", "0,1,0"]],
            AfterAction::Hibernate => &[&["shutdown", "/h"]],
            _ => &[],
        }
    } else {
        match action {
            AfterAction::Shutdown => &[&["systemctl", "poweroff"], &["loginctl", "poweroff"], &["shutdown", "-h", "now"]],
            AfterAction::Sleep => &[&["systemctl", "suspend"], &["loginctl", "suspend"]],
            AfterAction::Hibernate => &[&["systemctl", "hibernate"], &["loginctl", "hibernate"]],
            _ => &[],
        }
    };
    for c in cmds {
        if std::process::Command::new(c[0]).args(&c[1..]).status().map(|s| s.success()).unwrap_or(false) {
            break;
        }
    }
}

/// Motor olaylarını arayüze ve bildirimlere aktarır.
fn forward_events(app: AppHandle, engine: Engine) {
    let mut rx = engine.subscribe();
    tauri::async_runtime::spawn(async move {
        let mut last_tip = std::time::Instant::now();
        loop {
            let ev = match rx.recv().await {
                Ok(e) => e,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            };
            match ev {
                EngineEvent::Progress(ticks) => {
                    if last_tip.elapsed() > Duration::from_secs(1) {
                        last_tip = std::time::Instant::now();
                        let total: u64 = ticks.iter().map(|t| t.speed).sum();
                        let tip = format!("Internet Download Center\n{} · {}/s", ticks.len(), idc_core::util::human_size(total));
                        tray::set_tooltip(&app, &tip);
                    }
                    let _ = app.emit("idc://progress", ticks);
                }
                EngineEvent::Changed(v) => {
                    let _ = app.emit("idc://changed", v);
                }
                EngineEvent::Removed(ids) => {
                    let _ = app.emit("idc://removed", ids);
                }
                EngineEvent::Settings(s) => {
                    let _ = app.emit("idc://settings", s);
                }
                EngineEvent::Completed(id) => {
                    let Some(d) = engine.get(&id) else { continue };
                    let s = engine.settings();
                    windows::close(&app, &windows::label_for("progress", &std::collections::HashMap::from([("id".to_string(), id.clone())])));
                    let show = d.show_complete_dialog.unwrap_or(s.downloads.show_complete_dialog) && !d.silent;
                    if show {
                        windows::open1(&app, "complete", "id", &id);
                    } else {
                        notify(&app, "completed", &d.file_name);
                    }
                    if d.checksum_ok == Some(false) {
                        notify(&app, "checksum", &d.file_name);
                    }
                    if engine.list().iter().all(|v| !v.status.is_active() && v.status != Status::Queued) {
                        tray::set_tooltip(&app, "Internet Download Center");
                    }
                }
                EngineEvent::Failed(id, msg) => {
                    if let Some(d) = engine.get(&id) {
                        if !d.silent {
                            let detail = msg.split('|').nth(1).unwrap_or("").to_string();
                            notify(&app, "failed", format!("{} {}", d.file_name, detail).trim());
                        }
                    }
                }
                EngineEvent::QueueFinished(q) => {
                    let name = engine.settings().queues.iter().find(|x| x.id == q).map(|x| x.name.clone()).unwrap_or(q);
                    notify(&app, "queue", &name);
                }
                EngineEvent::After(a) => run_after(&app, a),
            }
        }
    });
}

/// yt-dlp haftada bir güncellenir (YouTube değişikliklerine ayak uydurmak için).
fn maybe_update_tools(engine: Engine) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(20)).await;
        let s = engine.settings();
        let tools = engine.tools();
        let Some(yt) = tools.ytdlp() else { return };
        if !s.media.auto_update_tools || !yt.starts_with(tools.dir()) {
            return;
        }
        let stamp = tools.dir().join(".last-update");
        let old = std::fs::metadata(&stamp).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).map(|e| e > Duration::from_secs(7 * 86400)).unwrap_or(true);
        if old && tools.ensure(true).await.is_ok() {
            let _ = std::fs::write(stamp, chrono::Utc::now().to_rfc3339());
        }
    });
}

fn handle_args(app: &AppHandle, args: &[String], from_second: bool) {
    let mut had_url = false;
    for a in args.iter().skip(1) {
        if a.starts_with("--") {
            continue;
        }
        if a.starts_with("http://") || a.starts_with("https://") || a.starts_with("ftp://") || a.starts_with("idc://") {
            had_url = true;
            flows::handle_url_arg(app, a);
        }
    }
    if args.iter().any(|a| a == "--add-url") {
        windows::open1(app, "addurl", "from", "cli");
        return;
    }
    let background = args.iter().any(|a| a == "--background" || a == "--minimized");
    if from_second && !had_url && !background {
        windows::show_main(app);
    }
}

/// Destek ve paketleme için komut satırı araçları:
/// `--diagnose` ortamı JSON olarak yazar; `--integrate browsers,url,kde,autostart` entegrasyonu kurar.
/// Uygulama zaten çalışıyorsa tek örnek kuralı gereği bu komutlar çalışan örneğe gider ve etkisiz kalır.
fn cli_tools(app: &AppHandle, args: &[String]) -> bool {
    let diagnose = args.iter().any(|a| a == "--diagnose");
    let integrate = args.iter().position(|a| a == "--integrate").map(|i| args.get(i + 1).cloned().unwrap_or_else(|| "all".into()));
    if !diagnose && integrate.is_none() {
        return false;
    }
    if let Some(list) = integrate {
        let all = list == "all";
        let has = |k: &str| all || list.split(',').any(|x| x.trim() == k);
        if has("browsers") {
            if let Err(e) = integration::install(app, None) {
                eprintln!("tarayıcı entegrasyonu: {e}");
            }
        }
        let mut o = integration::os_status(app);
        if has("url") {
            o.url_handler = true;
        }
        if has("autostart") {
            o.autostart = true;
        }
        if has("kde") {
            o.kde_download_manager = Some(true);
        }
        if let Err(e) = integration::os_apply(app, &o) {
            eprintln!("sistem entegrasyonu: {e}");
        }
    }
    let st = flows::state(app);
    let report = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        "dataDir": idc_core::paths::config_dir(),
        "language": sys_lang(),
        "integration": integration::os_status(app),
        "browsers": integration::status(app),
        "tools": st.engine.tools().status(),
        "hostPath": integration::host_path(app),
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
    api::cleanup();
    std::process::exit(0);
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_env("IDC_LOG").unwrap_or_else(|_| "info".into()))
        .with_writer(std::io::stderr)
        .init();

    // NVIDIA + Wayland üzerinde WebKitGTK boş pencere sorununa karşı.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() && std::env::var("XDG_SESSION_TYPE").map(|v| v == "wayland").unwrap_or(false) {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    let args: Vec<String> = std::env::args().collect();
    let background = args.iter().any(|a| a == "--background" || a == "--minimized");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            handle_args(app, &argv, true);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--background"])))
        .setup(move |app| {
            let handle = app.handle().clone();
            let engine = tauri::async_runtime::block_on(async { Engine::new(idc_core::paths::config_dir()) });
            app.manage(AppState::new(engine.clone()));
            if cli_tools(&handle, &args) {
                return Ok(());
            }
            if let Err(e) = tauri::async_runtime::block_on(api::start(handle.clone())) {
                tracing::error!("yerel API başlatılamadı: {e}");
            }
            if let Err(e) = tray::create(&handle) {
                tracing::warn!("tepsi simgesi oluşturulamadı: {e}");
            }
            forward_events(handle.clone(), engine.clone());
            clipboard::start(handle.clone());
            let h2 = handle.clone();
            std::thread::spawn(move || integration::refresh(&h2));
            maybe_update_tools(engine.clone());

            if engine.is_first_run() {
                // İlk kurulumda başlangıçta çalıştırma ayarını uygula.
                use tauri_plugin_autostart::ManagerExt;
                if engine.settings().general.launch_on_startup {
                    let _ = handle.autolaunch().enable();
                }
                windows::open1(&handle, "wizard", "step", "0");
            } else if !background {
                windows::show_main(&handle);
            }
            handle_args(&handle, &args, false);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_downloads,
            commands::get_download,
            commands::probe_url,
            commands::add_download,
            commands::add_many,
            commands::start_downloads,
            commands::pause_downloads,
            commands::remove_downloads,
            commands::start_all,
            commands::stop_all,
            commands::delete_completed,
            commands::redownload,
            commands::update_download,
            commands::open_file,
            commands::open_with,
            commands::open_folder,
            commands::get_settings,
            commands::save_settings,
            commands::media_probe,
            commands::tools_status,
            commands::tools_update,
            commands::browsers_status,
            commands::browsers_install,
            commands::browser_open_extensions,
            commands::os_integration_status,
            commands::os_integration_apply,
            commands::open_window,
            commands::start_queue,
            commands::stop_queue,
            commands::save_queue,
            commands::delete_queue,
            commands::move_in_queue,
            commands::batch_preview,
            commands::grab_links,
            commands::app_info,
            commands::quit_app,
            commands::create_pending,
            commands::get_pending,
            commands::confirm_pending,
            commands::cancel_pending,
            commands::get_pending_media,
            commands::confirm_pending_media,
            commands::get_pending_links,
            commands::set_global_speed_limit,
            commands::export_list,
            commands::import_list,
        ])
        .build(tauri::generate_context!())
        .expect("Internet Download Center başlatılamadı");

    app.run(|app, event| match event {
        RunEvent::ExitRequested { api, code, .. } => {
            let st = flows::state(app);
            // Son pencere kapandı ama tepsi varsa arka planda çalışmaya devam et.
            if code.is_none() && !st.quitting.load(Ordering::Relaxed) && st.tray_ok.load(Ordering::Relaxed) {
                api.prevent_exit();
            }
        }
        RunEvent::Exit => {
            let st = flows::state(app);
            st.engine.save_if_dirty();
            api::cleanup();
        }
        _ => {}
    });
}
