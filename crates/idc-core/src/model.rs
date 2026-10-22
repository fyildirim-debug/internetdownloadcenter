//! Motorun kalıcı veri modeli ve arayüze giden görünüm tipleri.
//! JSON alan adları camelCase'dir; arayüzdeki `ui/src/lib/types.ts` bu dosyanın aynasıdır.

use serde::{Deserialize, Serialize};

pub type DownloadId = String;

/// İndirmenin yaşam döngüsündeki durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Kuyrukta, sırasını bekliyor.
    Queued,
    /// İlk bağlantı kuruluyor.
    Connecting,
    /// Veri alınıyor.
    Downloading,
    /// Kullanıcı durdurdu ya da hiç başlatılmadı.
    #[default]
    Paused,
    /// Video/ses birleştirme, sağlama doğrulama gibi son işlemler.
    Processing,
    Completed,
    Error,
}

impl Status {
    pub fn is_active(self) -> bool {
        matches!(self, Status::Connecting | Status::Downloading | Status::Processing)
    }
}

/// Tarayıcıdan gelen ya da kullanıcının girdiği çerez.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CookieSpec {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub host_only: bool,
    /// Unix saniyesi; yoksa oturum çerezi.
    pub expires: Option<f64>,
}

/// Sunucuya yapılacak isteğin tarayıcıdaki hali.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RequestInfo {
    pub url: String,
    pub referer: Option<String>,
    /// Tarayıcının gönderdiği başlıklar (Cookie, Range, Host hariç tutulur).
    pub headers: Vec<(String, String)>,
    pub cookies: Vec<CookieSpec>,
    /// Çerez API'si yoksa ham `Cookie` başlığı.
    pub raw_cookie: Option<String>,
    pub user_agent: Option<String>,
    /// GET dışındaki yöntemler yalnız tek bağlantıyla denenir.
    pub method: Option<String>,
    pub post_data: Option<String>,
}

/// Dosyanın bir parçası. `pos` o ana kadar diske yazılmış mutlak konumdur.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub start: u64,
    /// Hariç bitiş; `None` boyutu bilinmeyen akış.
    pub end: Option<u64>,
    pub pos: u64,
}

impl Segment {
    pub fn remaining(&self) -> Option<u64> {
        self.end.map(|e| e.saturating_sub(self.pos))
    }
    pub fn done(&self) -> bool {
        matches!(self.end, Some(e) if self.pos >= e)
    }
}

/// yt-dlp ile indirilecek medya.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct MediaSpec {
    pub page_url: String,
    pub media_url: Option<String>,
    /// yt-dlp biçim seçicisi (ör. `137+140`, `bv*[height<=1080]+ba/b`).
    pub format_id: String,
    pub audio_only: bool,
    /// Yalnız ses için hedef biçim: `m4a`, `mp3`, `opus`.
    pub audio_format: Option<String>,
    /// Kap biçimi: `mp4`, `mkv`, `webm`.
    pub container: Option<String>,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
    /// Doğrudan indirme engellendiği için yt-dlp kimliğe bürünmesi kullanılıyor.
    pub impersonate: bool,
    /// İndirilecek altyazı dilleri.
    pub subtitles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DownloadKind {
    #[default]
    Http,
    Media(MediaSpec),
}

/// İndirme bitince yapılacak sistem işlemi.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AfterAction {
    #[default]
    None,
    Exit,
    Shutdown,
    Sleep,
    Hibernate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Checksum {
    /// `md5`, `sha1`, `sha256`
    pub algo: String,
    pub expected: String,
}

/// Kalıcı indirme kaydı (`downloads.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Download {
    pub id: DownloadId,
    pub request: RequestInfo,
    /// Yönlendirmelerden sonraki son adres.
    pub final_url: Option<String>,
    pub file_name: String,
    pub save_dir: String,
    pub category: String,
    pub description: String,
    pub size: Option<u64>,
    pub downloaded: u64,
    pub resumable: bool,
    pub mime: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub status: Status,
    pub error: Option<String>,
    /// Bu indirme için en çok bağlantı (1–16).
    pub max_connections: u8,
    /// Bayt/sn; `None` sınırsız.
    pub speed_limit: Option<u64>,
    pub segments: Vec<Segment>,
    pub kind: DownloadKind,
    pub queue_id: Option<String>,
    /// Unix milisaniyesi.
    pub added_at: i64,
    pub last_try_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub after: AfterAction,
    pub checksum: Option<Checksum>,
    pub checksum_ok: Option<bool>,
    /// Bittiğinde tamamlandı penceresini göster (ayarı ezer).
    pub show_complete_dialog: Option<bool>,
    /// İndirme sırasında ilerleme penceresi açılmasın.
    pub silent: bool,
    /// Dosya adı sunucu cevabına göre belirlenebilir (kullanıcı ad vermedi).
    pub auto_name: bool,
    /// Tarayıcıdan yakalandı (engel durumunda tarayıcıya geri verilebilir).
    pub from_browser: bool,
}

impl Default for Download {
    fn default() -> Self {
        Self {
            id: String::new(),
            request: RequestInfo::default(),
            final_url: None,
            file_name: String::new(),
            save_dir: String::new(),
            category: "general".into(),
            description: String::new(),
            size: None,
            downloaded: 0,
            resumable: false,
            mime: None,
            etag: None,
            last_modified: None,
            status: Status::Paused,
            error: None,
            max_connections: 8,
            speed_limit: None,
            segments: Vec::new(),
            kind: DownloadKind::Http,
            queue_id: None,
            added_at: 0,
            last_try_at: None,
            completed_at: None,
            after: AfterAction::None,
            checksum: None,
            checksum_ok: None,
            show_complete_dialog: None,
            silent: false,
            auto_name: false,
            from_browser: false,
        }
    }
}

impl Download {
    pub fn target_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.save_dir).join(&self.file_name)
    }
    pub fn part_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.save_dir).join(format!("{}.idcpart", self.file_name))
    }
    pub fn is_media(&self) -> bool {
        matches!(self.kind, DownloadKind::Media(_))
    }
}

/// Arayüzdeki listenin bir satırı.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadView {
    pub id: DownloadId,
    pub url: String,
    pub file_name: String,
    pub save_dir: String,
    pub category: String,
    pub description: String,
    pub size: Option<u64>,
    pub downloaded: u64,
    pub status: Status,
    pub error: Option<String>,
    /// Bayt/sn.
    pub speed: u64,
    /// Kalan saniye.
    pub eta: Option<u64>,
    pub resumable: bool,
    /// `http` ya da `media`.
    pub kind: String,
    pub connections: u8,
    pub max_connections: u8,
    pub speed_limit: Option<u64>,
    pub queue_id: Option<String>,
    pub added_at: i64,
    pub last_try_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub mime: Option<String>,
    pub thumbnail: Option<String>,
}

/// İlerleme penceresindeki bağlantı tablosunun satırı.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentView {
    pub start: u64,
    pub end: Option<u64>,
    pub pos: u64,
    pub active: bool,
    /// Bayt/sn.
    pub speed: u64,
    /// "Veri alınıyor", "Bağlanıyor", "Tamamlandı" gibi kısa durum anahtarı.
    pub info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadDetail {
    #[serde(flatten)]
    pub view: DownloadView,
    pub referer: Option<String>,
    pub final_url: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub segments: Vec<SegmentView>,
    pub after: AfterAction,
    pub checksum: Option<Checksum>,
    pub checksum_ok: Option<bool>,
    pub media: Option<MediaSpec>,
    /// Son hız örnekleri (bayt/sn), grafik için; en yeni sonda.
    pub speed_history: Vec<u64>,
}

/// Saniyede 4 kez yayınlanan ilerleme olayı.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressTick {
    pub id: DownloadId,
    pub status: Status,
    pub size: Option<u64>,
    pub downloaded: u64,
    pub speed: u64,
    pub eta: Option<u64>,
    pub connections: u8,
    pub segments: Vec<SegmentView>,
}

/// `probe_url` sonucu: indirme eklenmeden önce dosya bilgisi.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub url: String,
    pub final_url: String,
    pub file_name: String,
    pub size: Option<u64>,
    pub resumable: bool,
    pub mime: Option<String>,
    pub category: String,
    /// Sunucu dosya yerine HTML sayfası döndürdü (engel ya da giriş sayfası olabilir).
    pub is_html: bool,
    pub status_code: u16,
    /// Adres bilinen bir video sitesine ait; `media_probe` kullanılmalı.
    pub is_media_site: bool,
}

/// Arayüzden ya da eklentiden gelen yeni indirme isteği.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AddRequest {
    pub request: RequestInfo,
    pub file_name: Option<String>,
    pub save_dir: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub size_hint: Option<u64>,
    pub mime_hint: Option<String>,
    pub max_connections: Option<u8>,
    pub media: Option<MediaSpec>,
    /// `true` hemen başlar, `false` "Daha sonra indir".
    pub start: bool,
    pub queue_id: Option<String>,
    pub checksum: Option<Checksum>,
    pub silent: bool,
}

/// `update_download` ile kısmi güncelleme.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadPatch {
    pub url: Option<String>,
    pub file_name: Option<String>,
    pub save_dir: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub max_connections: Option<u8>,
    /// `Some(0)` sınırı kaldırır.
    pub speed_limit: Option<u64>,
    pub queue_id: Option<String>,
    /// Boş metin kuyruktan çıkarır.
    pub clear_queue: bool,
    pub after: Option<AfterAction>,
    pub checksum: Option<Checksum>,
    pub referer: Option<String>,
    pub show_complete_dialog: Option<bool>,
}

/// yt-dlp biçim listesindeki bir seçenek.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MediaFormat {
    /// yt-dlp'ye verilecek seçici.
    pub format_id: String,
    /// "1080p60 MP4", "Yalnız ses M4A 128k" gibi etiket.
    pub label: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub fps: Option<f64>,
    pub ext: String,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    /// Tahmini toplam boyut (görüntü + ses).
    pub filesize: Option<u64>,
    pub tbr: Option<f64>,
    pub audio_only: bool,
    pub hdr: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MediaProbe {
    pub page_url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub uploader: Option<String>,
    pub extractor: String,
    pub formats: Vec<MediaFormat>,
    pub subtitles: Vec<String>,
    pub is_live: bool,
    /// Oynatma listesiyse öğe sayısı.
    pub playlist_count: Option<u32>,
}

/// Site yakalayıcının bulduğu bağlantı.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GrabbedLink {
    pub url: String,
    pub text: String,
    pub file_name: String,
    pub category: String,
    pub depth: u32,
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn new_id() -> DownloadId {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:012x}", rng.gen::<u64>() & 0xffff_ffff_ffff)
}
