//! Bildirim alanı simgesi ve menüsü.

use std::sync::atomic::Ordering;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

use crate::flows;

const TRAY_ID: &str = "idc-tray";

fn tr(app: &AppHandle, key: &str) -> String {
    let s = flows::state(app).engine.settings();
    let tr = crate::lang(&s) == "tr";
    let (t, e) = match key {
        "show" => ("Internet Download Center'ı aç", "Open Internet Download Center"),
        "add" => ("URL ekle…", "Add URL…"),
        "resume" => ("Tümünü sürdür", "Resume all"),
        "stop" => ("Tümünü durdur", "Stop all"),
        "limit" => ("Hız sınırlayıcı", "Speed limiter"),
        "options" => ("Seçenekler…", "Options…"),
        "quit" => ("Çıkış", "Exit"),
        "check" => ("Güncellemeleri denetle", "Check for updates"),
        _ => (key, key),
    };
    if tr { t.into() } else { e.into() }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let s = flows::state(app).engine.settings();
    let m = Menu::new(app)?;
    m.append(&MenuItem::with_id(app, "show", tr(app, "show"), true, None::<&str>)?)?;
    m.append(&MenuItem::with_id(app, "add", tr(app, "add"), true, None::<&str>)?)?;
    m.append(&PredefinedMenuItem::separator(app)?)?;
    m.append(&MenuItem::with_id(app, "resume", tr(app, "resume"), true, None::<&str>)?)?;
    m.append(&MenuItem::with_id(app, "stop", tr(app, "stop"), true, None::<&str>)?)?;
    m.append(&CheckMenuItem::with_id(app, "limit", tr(app, "limit"), true, s.connection.speed_limit_enabled, None::<&str>)?)?;
    m.append(&PredefinedMenuItem::separator(app)?)?;
    m.append(&MenuItem::with_id(app, "options", tr(app, "options"), true, None::<&str>)?)?;
    match crate::updates::available() {
        Some(u) => {
            let label = if crate::lang(&s) == "tr" { format!("Yeni sürümü indir ({})", u.latest) } else { format!("Download new version ({})", u.latest) };
            m.append(&MenuItem::with_id(app, "update", label, true, None::<&str>)?)?;
        }
        None => m.append(&MenuItem::with_id(app, "check", tr(app, "check"), true, None::<&str>)?)?,
    }
    m.append(&MenuItem::with_id(app, "quit", tr(app, "quit"), true, None::<&str>)?)?;
    Ok(m)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let icon = app.default_window_icon().cloned().unwrap_or_else(|| tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")).expect("simge"));
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Internet Download Center")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| {
            let st = flows::state(app);
            match ev.id().as_ref() {
                "show" => crate::windows::show_main(app),
                "add" => crate::windows::open1(app, "addurl", "from", "tray"),
                "resume" => st.engine.start_all(),
                "stop" => st.engine.stop_all(),
                "limit" => {
                    let s = st.engine.settings();
                    let on = !s.connection.speed_limit_enabled;
                    let _ = st.engine.set_global_speed_limit(if on { Some(s.connection.speed_limit_kib.max(1)) } else { None });
                    refresh(app);
                }
                "options" => crate::windows::open1(app, "options", "tab", "general"),
                "update" => {
                    let url = crate::updates::available().map(|u| u.url).unwrap_or_else(|| crate::updates::RELEASES_PAGE.into());
                    let _ = tauri_plugin_opener::open_url(url, None::<&str>);
                }
                "check" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if crate::updates::check(&app, true).await.is_none() {
                            crate::notify(&app, "update_failed", "");
                        }
                    });
                }
                "quit" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move { crate::quit(&app).await });
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                crate::windows::show_main(tray.app_handle());
            }
        })
        .build(app)?;
    flows::state(app).tray_ok.store(true, Ordering::Relaxed);
    Ok(())
}

/// Menüyü (dil, hız sınırı işareti) yeniler.
pub fn refresh(app: &AppHandle) {
    if let Some(t) = app.tray_by_id(TRAY_ID) {
        if let Ok(m) = build_menu(app) {
            let _ = t.set_menu(Some(m));
        }
    }
}

/// İpucu metninde toplam hız.
pub fn set_tooltip(app: &AppHandle, text: &str) {
    if let Some(t) = app.tray_by_id(TRAY_ID) {
        let _ = t.set_tooltip(Some(text));
    }
}
