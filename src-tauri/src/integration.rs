//! Tarayıcı (yerel mesajlaşma) ve işletim sistemi entegrasyonu.

use std::path::{Path, PathBuf};

use idc_core::system::{BrowserInfo, OsIntegration};
use tauri::{AppHandle, Manager};

pub const HOST_NAME: &str = "com.furkanyildirim.idc";
pub const CHROMIUM_ID: &str = "khmbnekbmjoegchdepkfehmenkbfmbpd";
pub const GECKO_ID: &str = "idc@furkanyildirim.com";

// Alanların bir kısmı yalnız Linux'ta, bir kısmı yalnız Windows'ta okunur.
#[allow(dead_code)]
struct Known {
    id: &'static str,
    name: &'static str,
    family: &'static str,
    /// Linux'ta PATH içinde aranacak komutlar.
    bins: &'static [&'static str],
    /// Linux kullanıcı yapılandırma klasörü (~/ altında); varlığı kurulu olduğunu gösterir.
    conf: &'static [&'static str],
    /// Linux yerel mesajlaşma klasörleri (~/ altında).
    hosts: &'static [&'static str],
    flatpak: &'static str,
    page: &'static str,
    /// Windows kayıt defteri anahtarı (HKCU\Software\...).
    win_key: &'static str,
    /// Windows'ta olası çalıştırılabilir yollar (%ProgramFiles%, %LOCALAPPDATA% göreli).
    win_paths: &'static [&'static str],
}

const BROWSERS: &[Known] = &[
    Known { id: "chrome", name: "Google Chrome", family: "chromium", bins: &["google-chrome", "google-chrome-stable", "google-chrome-beta"], conf: &[".config/google-chrome"], hosts: &[".config/google-chrome/NativeMessagingHosts", ".config/google-chrome-beta/NativeMessagingHosts", ".config/google-chrome-unstable/NativeMessagingHosts"], flatpak: "com.google.Chrome", page: "chrome://extensions", win_key: r"Software\Google\Chrome\NativeMessagingHosts", win_paths: &[r"Google\Chrome\Application\chrome.exe"] },
    Known { id: "chromium", name: "Chromium", family: "chromium", bins: &["chromium", "chromium-browser"], conf: &[".config/chromium"], hosts: &[".config/chromium/NativeMessagingHosts"], flatpak: "org.chromium.Chromium", page: "chrome://extensions", win_key: r"Software\Chromium\NativeMessagingHosts", win_paths: &[r"Chromium\Application\chrome.exe"] },
    Known { id: "edge", name: "Microsoft Edge", family: "chromium", bins: &["microsoft-edge", "microsoft-edge-stable", "microsoft-edge-beta", "microsoft-edge-dev"], conf: &[".config/microsoft-edge"], hosts: &[".config/microsoft-edge/NativeMessagingHosts", ".config/microsoft-edge-beta/NativeMessagingHosts", ".config/microsoft-edge-dev/NativeMessagingHosts"], flatpak: "com.microsoft.Edge", page: "edge://extensions", win_key: r"Software\Microsoft\Edge\NativeMessagingHosts", win_paths: &[r"Microsoft\Edge\Application\msedge.exe"] },
    Known { id: "brave", name: "Brave", family: "chromium", bins: &["brave-browser", "brave", "brave-browser-stable"], conf: &[".config/BraveSoftware/Brave-Browser"], hosts: &[".config/BraveSoftware/Brave-Browser/NativeMessagingHosts"], flatpak: "com.brave.Browser", page: "brave://extensions", win_key: r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts", win_paths: &[r"BraveSoftware\Brave-Browser\Application\brave.exe"] },
    Known { id: "vivaldi", name: "Vivaldi", family: "chromium", bins: &["vivaldi", "vivaldi-stable"], conf: &[".config/vivaldi"], hosts: &[".config/vivaldi/NativeMessagingHosts"], flatpak: "com.vivaldi.Vivaldi", page: "vivaldi://extensions", win_key: r"Software\Vivaldi\NativeMessagingHosts", win_paths: &[r"Vivaldi\Application\vivaldi.exe"] },
    Known { id: "opera", name: "Opera", family: "chromium", bins: &["opera"], conf: &[".config/opera"], hosts: &[".config/opera/NativeMessagingHosts"], flatpak: "com.opera.Opera", page: "opera://extensions", win_key: r"Software\Opera Software\NativeMessagingHosts", win_paths: &[r"Programs\Opera\opera.exe"] },
    Known { id: "firefox", name: "Mozilla Firefox", family: "firefox", bins: &["firefox", "firefox-esr", "firefox-developer-edition"], conf: &[".mozilla/firefox", "snap/firefox"], hosts: &[".mozilla/native-messaging-hosts", "snap/firefox/common/.mozilla/native-messaging-hosts"], flatpak: "org.mozilla.firefox", page: "about:addons", win_key: r"Software\Mozilla\NativeMessagingHosts", win_paths: &[r"Mozilla Firefox\firefox.exe"] },
    Known { id: "zen", name: "Zen Browser", family: "firefox", bins: &["zen-browser", "zen", "zen-bin"], conf: &[".zen"], hosts: &[".zen/native-messaging-hosts", ".mozilla/native-messaging-hosts"], flatpak: "app.zen_browser.zen", page: "about:addons", win_key: r"Software\Mozilla\NativeMessagingHosts", win_paths: &[r"Zen Browser\zen.exe"] },
    Known { id: "librewolf", name: "LibreWolf", family: "firefox", bins: &["librewolf"], conf: &[".librewolf"], hosts: &[".librewolf/native-messaging-hosts"], flatpak: "io.gitlab.librewolf-community", page: "about:addons", win_key: r"Software\LibreWolf\NativeMessagingHosts", win_paths: &[r"LibreWolf\librewolf.exe"] },
    Known { id: "floorp", name: "Floorp", family: "firefox", bins: &["floorp"], conf: &[".floorp"], hosts: &[".floorp/native-messaging-hosts", ".mozilla/native-messaging-hosts"], flatpak: "one.ablaze.floorp", page: "about:addons", win_key: r"Software\Mozilla\NativeMessagingHosts", win_paths: &[r"Ablaze Floorp\floorp.exe"] },
    Known { id: "waterfox", name: "Waterfox", family: "firefox", bins: &["waterfox"], conf: &[".waterfox"], hosts: &[".waterfox/native-messaging-hosts"], flatpak: "net.waterfox.waterfox", page: "about:addons", win_key: r"Software\Waterfox\NativeMessagingHosts", win_paths: &[r"Waterfox\waterfox.exe"] },
];

#[cfg_attr(windows, allow(dead_code))]
fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(bin)).find(|p| p.is_file())
}

#[cfg_attr(windows, allow(dead_code))]
fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(unix)]
fn flatpak_installed(id: &str) -> bool {
    !id.is_empty()
        && [home().join(".local/share/flatpak/app").join(id), PathBuf::from("/var/lib/flatpak/app").join(id)].iter().any(|p| p.exists())
}

/// Köprü programının yolu. AppImage'da geçici bağlama noktası yerine kalıcı kopya kullanılır.
pub fn host_path(app: &AppHandle) -> PathBuf {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_default();
    let name = if cfg!(windows) { "idc-host.exe" } else { "idc-host" };
    let bundled = exe_dir.join(name);
    if std::env::var_os("APPIMAGE").is_some() {
        let dir = idc_core::paths::config_dir().join("bin");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join(name);
        let need = match (std::fs::metadata(&bundled), std::fs::metadata(&target)) {
            (Ok(a), Ok(b)) => a.len() != b.len(),
            (Ok(_), Err(_)) => true,
            _ => false,
        };
        if need {
            let _ = std::fs::copy(&bundled, &target);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
            }
        }
        if let Ok(appimage) = std::env::var("APPIMAGE") {
            let _ = std::fs::write(idc_core::paths::config_dir().join("app-path.txt"), appimage);
        }
        return target;
    }
    let _ = app;
    bundled
}

/// Eklenti dosyalarını kalıcı veri klasörüne kopyalar (güncellemelerde tazelenir).
pub fn extension_dir(app: &AppHandle) -> PathBuf {
    let dest = idc_core::paths::config_dir().join("extension");
    let src = app
        .path()
        .resource_dir()
        .ok()
        .map(|r| r.join("extension"))
        .filter(|p| p.join("chromium").exists())
        .or_else(|| {
            // Geliştirme: depodaki derleme çıktısı.
            let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../extension/build");
            if dev.join("chromium").exists() { Some(dev) } else { None }
        });
    if let Some(src) = src {
        let stamp_src = std::fs::metadata(src.join("chromium/manifest.json")).and_then(|m| m.modified()).ok();
        let stamp_dst = std::fs::metadata(dest.join("chromium/manifest.json")).and_then(|m| m.modified()).ok();
        if stamp_src != stamp_dst || !dest.join("chromium/manifest.json").exists() {
            let _ = std::fs::remove_dir_all(dest.join("chromium"));
            let _ = copy_dir(&src.join("chromium"), &dest.join("chromium"));
            for f in ["idc-firefox.xpi", "idc-chromium.zip"] {
                let _ = std::fs::copy(src.join(f), dest.join(f));
            }
            if let (Some(t), Ok(f)) = (stamp_src, std::fs::File::options().write(true).open(dest.join("chromium/manifest.json"))) {
                let _ = f.set_modified(t);
            }
        }
    }
    dest
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let p = e.path();
        let t = dst.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &t)?;
        } else {
            std::fs::copy(&p, &t)?;
        }
    }
    Ok(())
}

fn manifest(family: &str, host: &Path) -> serde_json::Value {
    let mut m = serde_json::json!({
        "name": HOST_NAME,
        "description": "Internet Download Center",
        "path": host.to_string_lossy(),
        "type": "stdio",
    });
    if family == "firefox" {
        m["allowed_extensions"] = serde_json::json!([GECKO_ID]);
    } else {
        m["allowed_origins"] = serde_json::json!([format!("chrome-extension://{}/", CHROMIUM_ID)]);
    }
    m
}

/// Bildirim dosyalarının yazıldığı yer (Windows'ta kayıt defteri bunlara işaret eder).
fn manifest_file(family: &str) -> PathBuf {
    idc_core::paths::config_dir().join("native-hosts").join(format!("{family}.json"))
}

fn write_manifest_file(family: &str, host: &Path) -> std::io::Result<PathBuf> {
    let p = manifest_file(family);
    std::fs::create_dir_all(p.parent().unwrap())?;
    std::fs::write(&p, serde_json::to_vec_pretty(&manifest(family, host)).unwrap())?;
    Ok(p)
}

/// Kullanıcı düzeyinde kayıt var mı (uygulamanın kendisinin yazdığı).
fn user_registered(k: &Known) -> bool {
    #[cfg(unix)]
    {
        k.hosts.iter().any(|h| home().join(h).join(format!("{HOST_NAME}.json")).exists())
    }
    #[cfg(windows)]
    {
        is_registered(k)
    }
}

/// Kullanıcı ya da sistem düzeyinde (deb/rpm paketinin yazdığı) kayıt var mı.
fn is_registered(k: &Known) -> bool {
    #[cfg(unix)]
    {
        let sys: &[&str] = match (k.family, k.id) {
            ("firefox", _) => &["/usr/lib/mozilla/native-messaging-hosts", "/usr/lib64/mozilla/native-messaging-hosts"],
            (_, "chromium") => &["/etc/chromium/native-messaging-hosts"],
            (_, "edge") => &["/etc/opt/edge/native-messaging-hosts"],
            (_, "brave") => &["/etc/opt/brave/native-messaging-hosts", "/etc/opt/chrome/native-messaging-hosts"],
            (_, "vivaldi") => &["/etc/opt/vivaldi/native-messaging-hosts", "/etc/opt/chrome/native-messaging-hosts"],
            (_, "opera") => &["/etc/opt/opera/native-messaging-hosts", "/etc/opt/chrome/native-messaging-hosts"],
            _ => &["/etc/opt/chrome/native-messaging-hosts"],
        };
        user_registered(k) || sys.iter().any(|d| Path::new(d).join(format!("{HOST_NAME}.json")).exists())
    }
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        let hk = winreg::RegKey::predef(HKEY_CURRENT_USER);
        hk.open_subkey(format!(r"{}\{}", k.win_key, HOST_NAME)).is_ok()
    }
}

#[cfg(unix)]
fn detect(k: &Known) -> (bool, Option<String>, bool) {
    if let Some(p) = k.bins.iter().find_map(|b| which(b)) {
        return (true, Some(p.to_string_lossy().into()), false);
    }
    if k.conf.iter().any(|c| home().join(c).exists()) {
        return (true, None, k.conf.iter().any(|c| c.starts_with("snap/")));
    }
    if flatpak_installed(k.flatpak) {
        return (true, Some(format!("flatpak run {}", k.flatpak)), true);
    }
    (false, None, false)
}

#[cfg(windows)]
fn detect(k: &Known) -> (bool, Option<String>, bool) {
    let roots = [std::env::var("ProgramFiles").ok(), std::env::var("ProgramFiles(x86)").ok(), std::env::var("LOCALAPPDATA").ok()];
    for r in roots.iter().flatten() {
        for p in k.win_paths {
            let full = Path::new(r).join(p);
            if full.is_file() {
                return (true, Some(full.to_string_lossy().into()), false);
            }
        }
    }
    (false, None, false)
}

pub fn status(app: &AppHandle) -> Vec<BrowserInfo> {
    let ext = extension_dir(app);
    let mut out = vec![];
    let mut seen_win_keys = std::collections::HashSet::new();
    for k in BROWSERS {
        let (installed, exe, sandboxed) = detect(k);
        if cfg!(windows) && !installed && !seen_win_keys.insert(k.win_key) {
            continue;
        }
        out.push(BrowserInfo {
            id: k.id.into(),
            name: k.name.into(),
            family: k.family.into(),
            installed,
            host_registered: is_registered(k),
            extension_path: if k.family == "firefox" { ext.join("idc-firefox.xpi") } else { ext.join("chromium") }.to_string_lossy().into(),
            extensions_page: k.page.into(),
            executable: exe,
            sandboxed,
        });
    }
    out
}

/// Seçilen (ya da bulunan tüm) tarayıcılar için köprüyü kaydeder.
pub fn install(app: &AppHandle, ids: Option<Vec<String>>) -> anyhow::Result<Vec<BrowserInfo>> {
    let host = host_path(app);
    if !host.exists() {
        tracing::warn!("köprü bulunamadı: {}", host.display());
    }
    for fam in ["chromium", "firefox"] {
        write_manifest_file(fam, &host)?;
    }
    for k in BROWSERS {
        let wanted = match &ids {
            Some(list) => list.iter().any(|i| i == k.id),
            None => detect(k).0,
        };
        if !wanted {
            continue;
        }
        register(k, &host)?;
    }
    Ok(status(app))
}

/// Uygulama her açıldığında kayıtlı köprülerin yolunu tazeler (güncelleme sonrası yol değişebilir).
pub fn refresh(app: &AppHandle) {
    let host = host_path(app);
    let _ = extension_dir(app);
    for fam in ["chromium", "firefox"] {
        let _ = write_manifest_file(fam, &host);
    }
    for k in BROWSERS {
        if user_registered(k) {
            let _ = register(k, &host);
        }
    }
}

#[cfg(unix)]
fn register(k: &Known, host: &Path) -> anyhow::Result<()> {
    let data = serde_json::to_vec_pretty(&manifest(k.family, host))?;
    let has_bin = k.bins.iter().any(|b| which(b).is_some());
    let mut wrote = false;
    for (i, h) in k.hosts.iter().enumerate() {
        let dir = home().join(h);
        // Yalnız var olan tarayıcı klasörlerine yaz; hiç açılmamış ama kurulu tarayıcı için ilk klasörü oluştur.
        let parent_exists = dir.parent().map(|p| p.exists()).unwrap_or(false);
        if !(parent_exists || (i == 0 && has_bin)) {
            continue;
        }
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(format!("{HOST_NAME}.json")), &data)?;
        wrote = true;
    }
    if !wrote && !flatpak_installed(k.flatpak) {
        // Tarayıcı algılanamadı ama kullanıcı yine de istedi: ilk klasöre yaz.
        let dir = home().join(k.hosts[0]);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(format!("{HOST_NAME}.json")), &data)?;
    }
    // Flatpak tarayıcılar kendi kum havuzlarındaki klasörü okur.
    if flatpak_installed(k.flatpak) {
        let base = home().join(".var/app").join(k.flatpak);
        let sub = if k.family == "firefox" { ".mozilla/native-messaging-hosts".to_string() } else { k.hosts[0].replacen(".config/", "config/", 1) };
        let dir = base.join(sub);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(format!("{HOST_NAME}.json")), &data);
    }
    Ok(())
}

#[cfg(windows)]
fn register(k: &Known, _host: &Path) -> anyhow::Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    let file = manifest_file(k.family);
    let hk = winreg::RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hk.create_subkey(format!(r"{}\{}", k.win_key, HOST_NAME))?;
    key.set_value("", &file.to_string_lossy().to_string())?;
    // Chromium türevleri çoğunlukla Chrome anahtarını da okur.
    if k.family == "chromium" {
        let (key, _) = hk.create_subkey(format!(r"Software\Google\Chrome\NativeMessagingHosts\{}", HOST_NAME))?;
        key.set_value("", &file.to_string_lossy().to_string())?;
    }
    Ok(())
}

/// Tarayıcının eklenti sayfasını açar (Chromium tabanlılarda `chrome://` adresleri komut satırından açılabilir).
pub fn open_extensions_page(app: &AppHandle, id: &str) -> anyhow::Result<()> {
    let k = BROWSERS.iter().find(|k| k.id == id).ok_or_else(|| anyhow::anyhow!("bilinmeyen tarayıcı"))?;
    let (_, exe, _) = detect(k);
    let target = if k.family == "firefox" { extension_dir(app).join("idc-firefox.xpi").to_string_lossy().to_string() } else { k.page.to_string() };
    let Some(exe) = exe else { anyhow::bail!("tarayıcı yolu bulunamadı") };
    let mut parts = exe.split_whitespace();
    let prog = parts.next().unwrap_or_default().to_string();
    let mut c = std::process::Command::new(prog);
    c.args(parts).arg(target);
    c.spawn()?;
    Ok(())
}

// ---------------- İşletim sistemi ----------------

pub fn desktop() -> (String, String) {
    if cfg!(windows) {
        return ("windows".into(), "windows".into());
    }
    let de = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    let d = if de.contains("kde") {
        "kde"
    } else if de.contains("gnome") || de.contains("ubuntu") || de.contains("pop") {
        "gnome"
    } else if de.contains("xfce") {
        "xfce"
    } else if de.contains("cinnamon") || de.contains("x-cinnamon") {
        "cinnamon"
    } else if de.contains("mate") {
        "mate"
    } else if de.contains("lxqt") {
        "lxqt"
    } else if de.contains("budgie") {
        "budgie"
    } else {
        "other"
    };
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| if std::env::var_os("WAYLAND_DISPLAY").is_some() { "wayland".into() } else { "x11".into() });
    (d.into(), session)
}

#[cfg(unix)]
fn kde_config_tool() -> Option<&'static str> {
    ["kwriteconfig6", "kwriteconfig5"].into_iter().find(|t| which(t).is_some())
}

#[cfg(unix)]
fn kde_read(key: &str) -> Option<String> {
    let tool = ["kreadconfig6", "kreadconfig5"].into_iter().find(|t| which(t).is_some())?;
    let out = std::process::Command::new(tool).args(["--file", "konquerorrc", "--group", "HTML Settings", "--key", key]).output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn app_exe() -> String {
    std::env::var("APPIMAGE").ok().unwrap_or_else(|| std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default())
}

#[cfg(unix)]
fn handler_desktop_path() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| home().join(".local/share")).join("applications/internet-download-center-handler.desktop")
}

pub fn os_status(app: &AppHandle) -> OsIntegration {
    use tauri_plugin_autostart::ManagerExt;
    let (desktop, session) = desktop();
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    #[cfg(unix)]
    let url_handler = {
        let out = std::process::Command::new("xdg-mime").args(["query", "default", "x-scheme-handler/idc"]).output();
        out.map(|o| String::from_utf8_lossy(&o.stdout).contains("internet-download-center")).unwrap_or(false)
            || mimeapps_get().map(|v| v.contains("internet-download-center")).unwrap_or(false)
    };
    #[cfg(windows)]
    let url_handler = {
        use winreg::enums::HKEY_CURRENT_USER;
        winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Classes\idc\shell\open\command").is_ok()
    };
    #[cfg(unix)]
    let kde = if desktop == "kde" || kde_config_tool().is_some() {
        Some(kde_read("DownloadManager").map(|v| v.contains("internet-download-center")).unwrap_or(false))
    } else {
        None
    };
    #[cfg(windows)]
    let kde = None;
    let tray_supported = desktop != "gnome" || has_appindicator_extension();
    OsIntegration { autostart, url_handler, kde_download_manager: kde, context_menu: false, desktop, session, tray_supported }
}

fn has_appindicator_extension() -> bool {
    #[cfg(unix)]
    {
        let out = std::process::Command::new("gnome-extensions").arg("list").arg("--enabled").output();
        if let Ok(o) = out {
            let s = String::from_utf8_lossy(&o.stdout);
            return s.contains("appindicator") || s.contains("ubuntu-appindicators");
        }
    }
    false
}

pub fn os_apply(app: &AppHandle, o: &OsIntegration) -> anyhow::Result<OsIntegration> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    if o.autostart {
        let _ = al.enable();
    } else {
        let _ = al.disable();
    }
    set_url_handler(o.url_handler)?;
    #[cfg(unix)]
    if let (Some(want), Some(tool)) = (o.kde_download_manager, kde_config_tool()) {
        let value = if want { app_exe() } else { String::new() };
        let mut c = std::process::Command::new(tool);
        c.args(["--file", "konquerorrc", "--group", "HTML Settings", "--key", "DownloadManager"]);
        if value.is_empty() {
            c.arg("--delete");
        } else {
            c.arg(value);
        }
        c.status()?;
    }
    Ok(os_status(app))
}

#[cfg(unix)]
fn set_url_handler(on: bool) -> anyhow::Result<()> {
    let path = handler_desktop_path();
    if on {
        std::fs::create_dir_all(path.parent().unwrap())?;
        let body = format!(
            "[Desktop Entry]\nType=Application\nName=Internet Download Center\nComment=Bağlantıyı IDC ile indir\nExec=\"{}\" %u\nIcon=internet-download-center\nNoDisplay=true\nTerminal=false\nMimeType=x-scheme-handler/idc;\n",
            app_exe()
        );
        std::fs::write(&path, body)?;
        let _ = std::process::Command::new("update-desktop-database").arg(path.parent().unwrap()).status();
        let _ = std::process::Command::new("xdg-mime").args(["default", "internet-download-center-handler.desktop", "x-scheme-handler/idc"]).status();
        // xdg-utils kurulu olmayabilir: mimeapps.list'e doğrudan da yaz.
        mimeapps_set(Some("internet-download-center-handler.desktop"))?;
    } else {
        let _ = std::fs::remove_file(&path);
        mimeapps_set(None)?;
    }
    Ok(())
}

#[cfg(unix)]
fn mimeapps_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| home().join(".config")).join("mimeapps.list")
}

#[cfg(unix)]
fn mimeapps_get() -> Option<String> {
    let text = std::fs::read_to_string(mimeapps_path()).ok()?;
    let mut in_default = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_default = l == "[Default Applications]";
        } else if in_default {
            if let Some(v) = l.strip_prefix("x-scheme-handler/idc=") {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// `[Default Applications]` bölümündeki `x-scheme-handler/idc` satırını yazar ya da siler.
#[cfg(unix)]
fn mimeapps_set(desktop: Option<&str>) -> std::io::Result<()> {
    let path = mimeapps_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut out: Vec<String> = vec![];
    let mut in_default = false;
    let mut found_section = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            if in_default {
                if let Some(d) = desktop {
                    out.push(format!("x-scheme-handler/idc={d}"));
                }
            }
            in_default = l == "[Default Applications]";
            found_section |= in_default;
            out.push(line.to_string());
            continue;
        }
        if in_default && l.starts_with("x-scheme-handler/idc=") {
            continue;
        }
        out.push(line.to_string());
    }
    if in_default {
        if let Some(d) = desktop {
            out.push(format!("x-scheme-handler/idc={d}"));
        }
    }
    if !found_section {
        if let Some(d) = desktop {
            out.push("[Default Applications]".into());
            out.push(format!("x-scheme-handler/idc={d}"));
        }
    }
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(&path, out.join("\n") + "\n")
}

#[cfg(windows)]
fn set_url_handler(on: bool) -> anyhow::Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    let hk = winreg::RegKey::predef(HKEY_CURRENT_USER);
    if on {
        let (k, _) = hk.create_subkey(r"Software\Classes\idc")?;
        k.set_value("", &"URL:Internet Download Center")?;
        k.set_value("URL Protocol", &"")?;
        let (c, _) = hk.create_subkey(r"Software\Classes\idc\shell\open\command")?;
        c.set_value("", &format!("\"{}\" \"%1\"", app_exe()))?;
    } else {
        let _ = hk.delete_subkey_all(r"Software\Classes\idc");
    }
    Ok(())
}
