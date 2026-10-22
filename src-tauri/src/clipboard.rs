//! Pano izleme: kopyalanan indirme bağlantısını yakalar (IDM'deki gibi).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use parking_lot::Mutex;
use tauri::AppHandle;

static RESET: AtomicBool = AtomicBool::new(false);
static LAST: Mutex<String> = Mutex::new(String::new());

/// Ayar değişince son içeriği yeniden oku (eski içerik tetiklemesin).
pub fn reset() {
    RESET.store(true, Ordering::Relaxed);
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut cb = None;
        let mut first = true;
        loop {
            std::thread::sleep(Duration::from_millis(1200));
            let st = crate::flows::state(&app);
            let s = st.engine.settings();
            if !s.general.clipboard_monitor {
                first = true;
                continue;
            }
            if cb.is_none() {
                cb = arboard::Clipboard::new().ok();
            }
            let Some(c) = cb.as_mut() else { continue };
            let Ok(text) = c.get_text() else { continue };
            let text = text.trim().to_string();
            let reset = RESET.swap(false, Ordering::Relaxed);
            let changed = {
                let mut last = LAST.lock();
                if *last == text {
                    false
                } else {
                    *last = text.clone();
                    true
                }
            };
            // İlk okumada ya da ayar yeni açıldığında var olan içerik yakalanmaz.
            if first || reset || !changed {
                first = false;
                continue;
            }
            if text.contains(char::is_whitespace) || !(text.starts_with("http://") || text.starts_with("https://")) || text.len() > 4096 {
                continue;
            }
            let name = idc_core::filename::from_url(&text).unwrap_or_default();
            let ext = idc_core::util::extension_of(&name);
            let capture = !ext.is_empty() && s.file_types.capture_extensions.iter().any(|p| idc_core::util::ext_matches(p, &ext));
            if capture {
                let app2 = app.clone();
                let url = text.clone();
                let _ = app.run_on_main_thread(move || crate::flows::handle_url_arg(&app2, &url));
            }
        }
    });
}
