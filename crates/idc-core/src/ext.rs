//! Tarayıcı eklentisi ile uygulama arasındaki mesaj tipleri.
//! Eklenti tarafındaki karşılığı `extension/src/protocol.js` içindedir.

use serde::{Deserialize, Serialize};

use crate::model::{CookieSpec, MediaProbe};

/// Eklentiye gönderilen yakalama ayarları.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtConfig {
    pub enabled: bool,
    pub capture_extensions: Vec<String>,
    pub skip_extensions: Vec<String>,
    pub skip_sites: Vec<String>,
    pub skip_addresses: Vec<String>,
    pub min_size_kib: u64,
    pub video_panel: bool,
    /// `alt`, `shift`, `ctrl`, `none`.
    pub bypass_key: String,
    pub force_key: String,
    /// `tr` ya da `en`.
    pub language: String,
    pub browser_fallback: bool,
}

/// Tarayıcıda başlayan ve IDC'ye devredilen indirme.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadCapture {
    pub url: String,
    /// Tarayıcının yönlendirmelerden sonra vardığı adres.
    pub final_url: Option<String>,
    pub referrer: Option<String>,
    pub page_url: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<u64>,
    pub mime: Option<String>,
    /// İsteğin gerçek başlıkları (ad, değer).
    pub headers: Vec<(String, String)>,
    pub cookies: Vec<CookieSpec>,
    pub user_agent: Option<String>,
    pub method: Option<String>,
    pub post_data: Option<String>,
    /// Kullanıcı zorla IDC ile indir dedi (bağlam menüsü, zorlama tuşu): kurallar atlanır.
    pub forced: bool,
    /// Tarayıcı indirmesi duraklatıldı; reddedilirse eklenti sürdürecek.
    pub browser_download_id: Option<i64>,
    /// `chrome`, `edge`, `firefox`, `zen` ...
    pub browser: Option<String>,
    pub tab_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MediaRequest {
    pub page_url: String,
    pub media_url: Option<String>,
    pub format_id: Option<String>,
    pub audio_only: bool,
    pub title: Option<String>,
    pub headers: Vec<(String, String)>,
    pub cookies: Vec<CookieSpec>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    /// Kullanıcı biçim seçmeden "hızlı indir" dedi: ayardaki varsayılan kalite kullanılır.
    pub quick: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LinkItem {
    pub url: String,
    pub text: String,
}

/// Eklentiden gelen mesaj.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtRequest {
    Hello {
        #[serde(default)]
        version: String,
        #[serde(default)]
        browser: String,
    },
    Download(DownloadCapture),
    MediaProbe(MediaRequest),
    MediaDownload(MediaRequest),
    Links {
        links: Vec<LinkItem>,
        #[serde(default)]
        referrer: Option<String>,
    },
    Show,
}

/// Uygulamanın eklentiye cevabı.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtResponse {
    Config {
        config: ExtConfig,
        #[serde(rename = "appVersion")]
        app_version: String,
    },
    Accepted {
        #[serde(default)]
        id: Option<String>,
    },
    Rejected {
        /// `disabled`, `excluded`, `blocked`, `unsupported`, `user`, `error`.
        reason: String,
        #[serde(default)]
        message: Option<String>,
    },
    Media {
        probe: MediaProbe,
    },
    Error {
        message: String,
    },
}
