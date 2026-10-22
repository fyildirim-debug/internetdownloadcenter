//! Uygulama durumu: motor ve onay bekleyen işlemler.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;

use idc_core::ext::MediaRequest;
use idc_core::model::{AddRequest, GrabbedLink, MediaProbe, ProbeResult};
use idc_core::Engine;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Kullanıcı onayı bekleyen indirme ("İndirme Dosya Bilgisi" penceresi).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingAdd {
    pub token: String,
    pub request: AddRequest,
    pub probe: Option<ProbeResult>,
    pub probing: bool,
    pub error: Option<String>,
    pub suggested_name: String,
    pub suggested_dir: String,
    pub category: String,
    pub duplicate_of: Option<String>,
    /// "Pencere açıkken indirmeye başla" ayarıyla arka planda başlatılmış indirme.
    #[serde(skip)]
    pub download_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingMedia {
    pub token: String,
    pub request: MediaRequest,
    pub probe: Option<MediaProbe>,
    pub probing: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingLinks {
    pub token: String,
    pub links: Vec<GrabbedLink>,
    pub referrer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaChoice {
    pub format_id: String,
    pub audio_only: bool,
    pub file_name: String,
    pub save_dir: String,
    #[serde(default)]
    pub subtitles: Vec<String>,
}

pub struct AppState {
    pub engine: Engine,
    pub pending: Mutex<HashMap<String, PendingAdd>>,
    pub pending_media: Mutex<HashMap<String, PendingMedia>>,
    pub pending_links: Mutex<HashMap<String, PendingLinks>>,
    pub quitting: AtomicBool,
    /// Tepsi simgesi oluşturulabildi mi (GNOME'da uzantı yoksa görünmeyebilir).
    pub tray_ok: AtomicBool,
}

impl AppState {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            pending: Mutex::new(HashMap::new()),
            pending_media: Mutex::new(HashMap::new()),
            pending_links: Mutex::new(HashMap::new()),
            quitting: AtomicBool::new(false),
            tray_ok: AtomicBool::new(false),
        }
    }
}
