//! Sistem entegrasyonu ile ilgili arayüz tipleri (tarayıcılar, araçlar, işletim sistemi).

use serde::{Deserialize, Serialize};

/// Bilgisayarda bulunan bir tarayıcı ve IDC entegrasyon durumu.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BrowserInfo {
    /// `chrome`, `chromium`, `edge`, `brave`, `vivaldi`, `opera`, `firefox`, `zen`, `librewolf`, `waterfox`, `floorp`.
    pub id: String,
    pub name: String,
    /// `chromium` ya da `firefox`.
    pub family: String,
    pub installed: bool,
    /// Yerel mesajlaşma bildirimi kayıtlı mı.
    pub host_registered: bool,
    /// Eklentinin yükleneceği klasör ya da dosya.
    pub extension_path: String,
    /// `chrome://extensions` gibi eklenti sayfası.
    pub extensions_page: String,
    pub executable: Option<String>,
    /// Flatpak/Snap kum havuzunda: yerel mesajlaşma ek ayar ister.
    pub sandboxed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    /// `managed` (IDC indirdi), `system`, `none`.
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolsStatus {
    pub ytdlp: ToolInfo,
    pub ffmpeg: ToolInfo,
    pub deno: ToolInfo,
    /// Kurulum/güncelleme sürüyor.
    pub busy: bool,
    /// Son işlemin mesajı.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct OsIntegration {
    pub autostart: bool,
    /// `idc://` adres işleyicisi ve "Birlikte aç" kaydı.
    pub url_handler: bool,
    /// KDE (Konqueror/Dolphin) varsayılan indirme yöneticisi. KDE dışında `None`.
    pub kde_download_manager: Option<bool>,
    /// Windows'ta sağ tık menüsü / Linux'ta dosya yöneticisi eylemi.
    pub context_menu: bool,
    /// `kde`, `gnome`, `xfce`, `cinnamon`, `mate`, `lxqt`, `windows`, `other`.
    pub desktop: String,
    /// `x11`, `wayland`, `windows`.
    pub session: String,
    /// GNOME'da tepsi simgesi için AppIndicator uzantısı gerekir.
    pub tray_supported: bool,
}
