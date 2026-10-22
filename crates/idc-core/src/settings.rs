//! Kullanıcı ayarları (`settings.json`). Eksik alanlar varsayılanla dolar; eski dosyalar bozulmaz.

use serde::{Deserialize, Serialize};

use crate::model::AfterAction;

/// IDM'in varsayılan yakalama listesinden genişletilmiş uzantılar.
pub const DEFAULT_CAPTURE_EXTENSIONS: &[&str] = &[
    "3gp", "7z", "aac", "ace", "aif", "aiff", "apk", "appimage", "arj", "asf", "avi", "bin", "bz2",
    "cab", "deb", "dmg", "doc", "docx", "epub", "exe", "flac", "flv", "gz", "gzip", "img", "iso",
    "jar", "lzh", "lz", "m4a", "m4v", "mkv", "mov", "mp3", "mp4", "mpa", "mpe", "mpeg", "mpg", "msi",
    "msix", "msu", "odp", "ods", "odt", "ogg", "ogv", "opus", "pdf", "pps", "ppt", "pptx", "qt",
    "r0*", "r1*", "ra", "rar", "rm", "rmvb", "rpm", "sea", "sit", "sitx", "snap", "tar", "tbz2",
    "tgz", "tif", "tiff", "torrent", "ts", "txz", "vob", "wav", "webm", "wma", "wmv", "xls", "xlsx",
    "xz", "z", "zip", "zst",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Category {
    pub id: String,
    /// Yerleşik kategorilerde arayüz bu adı dil dosyasından çevirir.
    pub name: String,
    pub extensions: Vec<String>,
    pub save_dir: String,
    pub builtin: bool,
}

impl Default for Category {
    fn default() -> Self {
        Self { id: String::new(), name: String::new(), extensions: vec![], save_dir: String::new(), builtin: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Schedule {
    pub enabled: bool,
    /// "HH:MM" yerel saat.
    pub start_time: String,
    pub stop_enabled: bool,
    pub stop_time: String,
    /// 0 = Pazartesi ... 6 = Pazar. Boşsa her gün.
    pub days: Vec<u8>,
    /// Tek seferlik: "YYYY-MM-DD". Boşsa haftalık tekrar.
    pub once_date: Option<String>,
    /// Kuyruk bitince yapılacak işlem.
    pub after_all: AfterAction,
    /// Başarısız indirmeleri bu kadar kez yeniden dene.
    pub retry_failed: u32,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            enabled: false,
            start_time: "02:00".into(),
            stop_enabled: false,
            stop_time: "08:00".into(),
            days: vec![],
            once_date: None,
            after_all: AfterAction::None,
            retry_failed: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Queue {
    pub id: String,
    pub name: String,
    /// Aynı anda en çok kaç indirme.
    pub max_concurrent: u32,
    pub schedule: Schedule,
    /// Kuyruktaki indirmelerin sırası.
    pub order: Vec<String>,
    pub builtin: bool,
}

impl Default for Queue {
    fn default() -> Self {
        Self { id: String::new(), name: String::new(), max_concurrent: 3, schedule: Schedule::default(), order: vec![], builtin: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SiteLogin {
    /// Joker destekli adres kalıbı: `*.example.com/files/*`.
    pub pattern: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ServerLimit {
    pub host: String,
    pub connections: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct GeneralSettings {
    pub launch_on_startup: bool,
    /// Pencere kapatılınca tepsiye küçül.
    pub close_to_tray: bool,
    pub clipboard_monitor: bool,
    /// Tarayıcı eklentisinden gelen indirmeleri yakala.
    pub browser_integration: bool,
    /// Videoların üstünde "Bu videoyu indir" düğmesi.
    pub video_panel: bool,
    /// Basılıyken tıklanan bağlantıyı tarayıcıya bırakan tuş: `alt`, `shift`, `ctrl`, `none`.
    pub bypass_key: String,
    /// Basılıyken tıklanan bağlantıyı zorla IDC'ye veren tuş.
    pub force_key: String,
    pub play_sounds: bool,
    pub check_updates: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            launch_on_startup: true,
            close_to_tray: true,
            clipboard_monitor: true,
            browser_integration: true,
            video_panel: true,
            bypass_key: "alt".into(),
            force_key: "none".into(),
            play_sounds: true,
            check_updates: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadSettings {
    /// Yeni indirmelerde bağlantı sayısı (varsayılan 8).
    pub default_connections: u8,
    /// Kuyruk dışındaki elle başlatılanlar dahil aynı anda en çok indirme; 0 = sınırsız.
    pub max_simultaneous: u32,
    /// Yakalanan indirmede "İndirme Dosya Bilgisi" penceresini göster.
    pub show_file_info_dialog: bool,
    /// Pencere açıkken indirmeye arka planda başla.
    pub start_immediately_with_dialog: bool,
    /// `normal`, `minimized`, `none`.
    pub progress_dialog: String,
    pub show_complete_dialog: bool,
    /// Aynı adda dosya: `ask`, `rename`, `overwrite`.
    pub duplicate_action: String,
    pub retry_count: u32,
    pub retry_delay_sec: u32,
    /// Elle eklenen indirmelerde kullanıcı aracısı.
    pub user_agent: String,
    pub preallocate: bool,
    /// Dosyanın değişiklik zamanını sunucudaki tarihe ayarla.
    pub use_server_time: bool,
    /// Bitince virüs tarayıcı komutu; `[File]` dosya yoluyla değişir.
    pub antivirus_command: String,
}

impl Default for DownloadSettings {
    fn default() -> Self {
        Self {
            default_connections: 8,
            max_simultaneous: 0,
            show_file_info_dialog: true,
            start_immediately_with_dialog: false,
            progress_dialog: "normal".into(),
            show_complete_dialog: true,
            duplicate_action: "rename".into(),
            retry_count: 20,
            retry_delay_sec: 5,
            user_agent: String::new(),
            preallocate: true,
            use_server_time: false,
            antivirus_command: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ConnectionSettings {
    pub speed_limit_enabled: bool,
    /// KiB/sn.
    pub speed_limit_kib: u64,
    pub timeout_sec: u64,
    pub server_limits: Vec<ServerLimit>,
    /// HTTP/2 kullan (kapalıyken her parça ayrı TCP bağlantısıdır, çoğu sunucuda daha hızlıdır).
    pub http2: bool,
    /// Ek bağlantılar reddedilince bağlantı sayısını kendiliğinden düşür.
    pub adaptive: bool,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            speed_limit_enabled: false,
            speed_limit_kib: 1024,
            timeout_sec: 30,
            server_limits: vec![],
            http2: false,
            adaptive: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxySettings {
    /// `system`, `none`, `manual`.
    pub mode: String,
    /// `http://host:port`, `socks5://host:port`.
    pub url: String,
    pub username: String,
    pub password: String,
    pub bypass: Vec<String>,
}

impl Default for ProxySettings {
    fn default() -> Self {
        Self { mode: "system".into(), url: String::new(), username: String::new(), password: String::new(), bypass: vec!["localhost".into(), "127.0.0.1".into()] }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FileTypeSettings {
    /// Tarayıcıdan otomatik yakalanacak uzantılar (`r0*` gibi joker olabilir).
    pub capture_extensions: Vec<String>,
    /// Asla yakalanmayacak uzantılar.
    pub skip_extensions: Vec<String>,
    /// Bu sitelerden yakalama (joker: `*.example.com`).
    pub skip_sites: Vec<String>,
    /// Bu adreslerden yakalama (joker destekli tam adres).
    pub skip_addresses: Vec<String>,
    /// Bundan küçük dosyalar tarayıcıda kalır (KiB, 0 = sınırsız).
    pub min_size_kib: u64,
}

impl Default for FileTypeSettings {
    fn default() -> Self {
        Self {
            capture_extensions: DEFAULT_CAPTURE_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            skip_extensions: vec![],
            skip_sites: vec![
                "*.update.microsoft.com".into(),
                "download.windowsupdate.com".into(),
                "*.googleusercontent.com/*/clients*".into(),
            ],
            skip_addresses: vec![],
            min_size_kib: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct MediaSettings {
    /// `best`, `2160`, `1440`, `1080`, `720`, `480`, `360`, `audio`.
    pub default_quality: String,
    /// `mp4`, `mkv`, `webm`.
    pub container: String,
    /// `m4a`, `mp3`, `opus`.
    pub audio_format: String,
    pub auto_update_tools: bool,
    /// Engellenen doğrudan indirmeleri yt-dlp kimliğe bürünmesiyle dene.
    pub impersonate_fallback: bool,
    /// Her şey başarısız olursa indirmeyi tarayıcıya geri ver.
    pub browser_fallback: bool,
    /// Altyazıları da indir.
    pub download_subtitles: bool,
    pub subtitle_langs: Vec<String>,
    pub embed_thumbnail: bool,
    pub sponsorblock: bool,
}

impl Default for MediaSettings {
    fn default() -> Self {
        Self {
            default_quality: "best".into(),
            container: "mp4".into(),
            audio_format: "m4a".into(),
            auto_update_tools: true,
            impersonate_fallback: true,
            browser_fallback: true,
            download_subtitles: false,
            subtitle_langs: vec!["tr".into(), "en".into()],
            embed_thumbnail: false,
            sponsorblock: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    /// `auto`, `tr`, `en`.
    pub language: String,
    /// `light`, `dark`, `system`.
    pub theme: String,
    pub first_run_done: bool,
    pub default_save_dir: String,
    /// Kategori klasörlerini `default_save_dir` altında aç.
    pub per_category_dirs: bool,
    pub general: GeneralSettings,
    pub downloads: DownloadSettings,
    pub connection: ConnectionSettings,
    pub proxy: ProxySettings,
    pub file_types: FileTypeSettings,
    pub categories: Vec<Category>,
    pub queues: Vec<Queue>,
    pub site_logins: Vec<SiteLogin>,
    pub media: MediaSettings,
    /// Ana pencerede görünen sütunlar ve genişlikleri.
    pub columns: Vec<ColumnSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ColumnSetting {
    pub id: String,
    pub width: u32,
    pub visible: bool,
}

pub const MAIN_QUEUE: &str = "main";

impl Default for Settings {
    fn default() -> Self {
        let base = crate::paths::default_download_dir();
        let cat = |id: &str, name: &str, exts: &[&str], sub: &str| Category {
            id: id.into(),
            name: name.into(),
            extensions: exts.iter().map(|s| s.to_string()).collect(),
            save_dir: if sub.is_empty() { base.to_string_lossy().into() } else { base.join(sub).to_string_lossy().into() },
            builtin: true,
        };
        Self {
            version: 1,
            language: "auto".into(),
            theme: "light".into(),
            first_run_done: false,
            default_save_dir: base.to_string_lossy().into(),
            per_category_dirs: true,
            general: GeneralSettings::default(),
            downloads: DownloadSettings::default(),
            connection: ConnectionSettings::default(),
            proxy: ProxySettings::default(),
            file_types: FileTypeSettings::default(),
            categories: vec![
                cat("general", "General", &[], ""),
                cat("compressed", "Compressed", &["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "zst", "tgz", "txz", "tbz2", "lz", "lzh", "arj", "cab", "ace", "z", "sit", "sitx", "sea", "r0*", "r1*", "iso", "img"], "Compressed"),
                cat("documents", "Documents", &["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pps", "odt", "ods", "odp", "txt", "rtf", "epub", "csv", "md"], "Documents"),
                cat("music", "Music", &["mp3", "m4a", "aac", "flac", "wav", "wma", "ogg", "opus", "aif", "aiff", "ra", "mpa"], "Music"),
                cat("programs", "Programs", &["exe", "msi", "msix", "msu", "apk", "deb", "rpm", "appimage", "snap", "flatpakref", "dmg", "jar", "bin", "run", "sh"], "Programs"),
                cat("video", "Video", &["mp4", "mkv", "webm", "avi", "mov", "wmv", "flv", "m4v", "mpg", "mpeg", "mpe", "3gp", "ogv", "ts", "vob", "rm", "rmvb", "asf", "qt"], "Video"),
            ],
            queues: vec![Queue { id: MAIN_QUEUE.into(), name: "Main queue".into(), builtin: true, ..Default::default() }],
            site_logins: vec![],
            media: MediaSettings::default(),
            columns: vec![],
        }
    }
}

impl Settings {
    /// Dosya adına göre kategori kimliği.
    pub fn category_for(&self, file_name: &str) -> String {
        let ext = crate::util::extension_of(file_name);
        if ext.is_empty() {
            return "general".into();
        }
        for c in &self.categories {
            if c.id == "general" {
                continue;
            }
            if c.extensions.iter().any(|p| crate::util::ext_matches(p, &ext)) {
                return c.id.clone();
            }
        }
        "general".into()
    }

    /// Kategorinin kayıt klasörü (boşsa varsayılan klasör).
    pub fn dir_for_category(&self, id: &str) -> String {
        self.categories
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.save_dir.clone())
            .filter(|d| !d.trim().is_empty())
            .unwrap_or_else(|| self.default_save_dir.clone())
    }

    /// Ayar dosyasından gelen değerleri güvenli aralıklara çeker.
    pub fn sanitize(&mut self) {
        self.downloads.default_connections = self.downloads.default_connections.clamp(1, crate::MAX_CONNECTIONS);
        if self.connection.timeout_sec < 5 {
            self.connection.timeout_sec = 5;
        }
        if !self.queues.iter().any(|q| q.id == MAIN_QUEUE) {
            self.queues.insert(0, Queue { id: MAIN_QUEUE.into(), name: "Main queue".into(), builtin: true, ..Default::default() });
        }
        if !self.categories.iter().any(|c| c.id == "general") {
            let d = Settings::default();
            self.categories.insert(0, d.categories[0].clone());
        }
        for q in &mut self.queues {
            q.max_concurrent = q.max_concurrent.clamp(1, 32);
        }
    }

    pub fn connections_for_host(&self, host: &str, requested: u8) -> u8 {
        let mut n = requested.clamp(1, crate::MAX_CONNECTIONS);
        for l in &self.connection.server_limits {
            if crate::util::wildcard_match(&l.host.to_lowercase(), &host.to_lowercase()) && l.connections > 0 {
                n = n.min(l.connections);
            }
        }
        n
    }
}
