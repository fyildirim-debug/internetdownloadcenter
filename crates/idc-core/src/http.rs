//! HTTP istemcisi kurulumu: tarayıcı oturumunun kopyası, vekil sunucu, cevap çözümleme.

use std::sync::Arc;
use std::time::Duration;

use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Response, StatusCode};

use crate::model::{ProbeResult, RequestInfo};
use crate::settings::Settings;

/// Elle eklenen indirmelerde kullanılan güncel bir masaüstü tarayıcı kimliği.
pub fn default_user_agent() -> &'static str {
    if cfg!(windows) {
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36"
    } else {
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36"
    }
}

/// Motorun kendisinin yönettiği, tarayıcıdan aktarılmaması gereken başlıklar.
const SKIP_HEADERS: &[&str] = &[
    "host", "range", "content-length", "cookie", "connection", "keep-alive", "proxy-authorization",
    "proxy-connection", "te", "trailer", "transfer-encoding", "upgrade", "accept-encoding", "if-range",
    "if-match", "if-none-match", "if-modified-since", "if-unmodified-since", "content-type", "expect",
    "priority", "purpose", "sec-purpose", "x-client-data",
];

/// Her isteğe (yönlendirmeler dahil) eklenebilecek, tarayıcının standart başlıkları.
fn is_standard_header(lk: &str) -> bool {
    matches!(lk, "accept" | "accept-language" | "dnt" | "upgrade-insecure-requests" | "cache-control" | "pragma")
        || lk.starts_with("sec-ch-")
        || lk.starts_with("sec-fetch-")
}

/// Sitenin kendisine özel başlıklar (`Authorization`, `X-Api-Key` …): yalnız aynı siteye gönderilir.
pub fn custom_headers(req: &RequestInfo) -> Vec<(HeaderName, HeaderValue)> {
    req.headers
        .iter()
        .filter_map(|(k, v)| {
            let lk = k.to_ascii_lowercase();
            if SKIP_HEADERS.contains(&lk.as_str()) || lk.starts_with(':') || lk == "user-agent" || lk == "referer" || is_standard_header(&lk) {
                return None;
            }
            Some((HeaderName::from_bytes(lk.as_bytes()).ok()?, HeaderValue::from_str(v).ok()?))
        })
        .collect()
}

fn same_origin(a: &str, b: &str) -> bool {
    match (url::Url::parse(a), url::Url::parse(b)) {
        (Ok(x), Ok(y)) => x.host_str() == y.host_str() && x.port_or_known_default() == y.port_or_known_default(),
        _ => false,
    }
}

/// Aralıklı isteği gönderir ve yönlendirmeleri elle izler: çerezler her adımda alan adına göre
/// kavanozdan gelir, siteye özel başlıklar ve kimlik bilgileri başka siteye taşınmaz.
pub async fn send_ranged(client: &Client, url: &str, req: &RequestInfo, settings: &Settings, start: u64, end_excl: Option<u64>) -> reqwest::Result<Response> {
    let mut current = url.to_string();
    let mut method_get = false;
    for _ in 0..15 {
        let trusted = same_origin(&current, &req.url);
        let mut r = req.clone();
        if method_get {
            r.method = None;
            r.post_data = None;
        }
        let mut rb = ranged_get(client, &current, &r, settings, start, end_excl, trusted);
        if trusted {
            for (k, v) in custom_headers(req) {
                rb = rb.header(k, v);
            }
        }
        let resp = rb.send().await?;
        if resp.status().is_redirection() {
            if let Some(loc) = resp.headers().get(reqwest::header::LOCATION).and_then(|v| v.to_str().ok()) {
                if let Ok(next) = resp.url().join(loc) {
                    if matches!(resp.status().as_u16(), 301 | 302 | 303) {
                        method_get = true;
                    }
                    current = next.to_string();
                    continue;
                }
            }
        }
        return Ok(resp);
    }
    // Çok fazla yönlendirme: son adresi bir kez daha dene ve cevabı olduğu gibi döndür.
    ranged_get(client, &current, req, settings, start, end_excl, false).send().await
}

/// Ham `Cookie` başlığını ad/değer çiftlerine ayırır.
pub fn parse_cookie_header(raw: &str) -> Vec<(String, String)> {
    raw.split(';')
        .filter_map(|p| {
            let (k, v) = p.trim().split_once('=')?;
            let k = k.trim();
            if k.is_empty() { None } else { Some((k.to_string(), v.trim().to_string())) }
        })
        .collect()
}

/// Tarayıcı çerezlerinden alan adına duyarlı bir çerez kavanozu kurar.
pub fn build_jar(req: &RequestInfo) -> Arc<Jar> {
    let jar = Arc::new(Jar::default());
    let mut names = std::collections::HashSet::new();
    for c in &req.cookies {
        if c.name.is_empty() || c.domain.is_empty() {
            continue;
        }
        let host = c.domain.trim_start_matches('.');
        let scheme = if c.secure { "https" } else { "http" };
        let Ok(url) = url::Url::parse(&format!("{}://{}{}", scheme, host, if c.path.is_empty() { "/" } else { &c.path })) else {
            continue;
        };
        let mut s = format!("{}={}; Path={}", c.name, c.value, if c.path.is_empty() { "/" } else { &c.path });
        if !c.host_only {
            s.push_str(&format!("; Domain={}", host));
        }
        if c.secure {
            s.push_str("; Secure");
        }
        jar.add_cookie_str(&s, &url);
        // Çerez https ise http adresinden de eklensin diye ek kayıt gerekmez; Secure http'de gönderilmez.
        names.insert(c.name.clone());
    }
    // Ham başlıktaki çerezler yalnız ilk adresin alanına (host-only) eklenir.
    let raw = req.raw_cookie.clone().or_else(|| {
        req.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("cookie")).map(|(_, v)| v.clone())
    });
    if let (Some(raw), Ok(url)) = (raw, url::Url::parse(&req.url)) {
        for (k, v) in parse_cookie_header(&raw) {
            if names.contains(&k) {
                continue;
            }
            jar.add_cookie_str(&format!("{}={}; Path=/", k, v), &url);
        }
    }
    jar
}

/// İndirme için istemci. Her indirmenin kendi çerez kavanozu ve bağlantı havuzu vardır.
pub fn build_client(req: &RequestInfo, settings: &Settings) -> anyhow::Result<Client> {
    let mut headers = HeaderMap::new();
    for (k, v) in &req.headers {
        let lk = k.to_ascii_lowercase();
        if !is_standard_header(&lk) {
            continue;
        }
        if let (Ok(n), Ok(val)) = (HeaderName::from_bytes(lk.as_bytes()), HeaderValue::from_str(v)) {
            headers.insert(n, val);
        }
    }
    headers.insert(reqwest::header::ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    if !headers.contains_key(reqwest::header::ACCEPT) {
        headers.insert(reqwest::header::ACCEPT, HeaderValue::from_static("*/*"));
    }
    if !headers.contains_key(reqwest::header::ACCEPT_LANGUAGE) {
        headers.insert(reqwest::header::ACCEPT_LANGUAGE, HeaderValue::from_static("tr-TR,tr;q=0.9,en-US;q=0.8,en;q=0.7"));
    }
    if let Some(r) = req.referer.as_deref().filter(|r| !r.is_empty()) {
        if let Ok(v) = HeaderValue::from_str(r) {
            headers.insert(reqwest::header::REFERER, v);
        }
    }
    let ua = req
        .user_agent
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| req.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("user-agent")).map(|(_, v)| v.clone()))
        .or_else(|| Some(settings.downloads.user_agent.clone()).filter(|s| !s.is_empty()))
        .unwrap_or_else(|| default_user_agent().to_string());

    let timeout = Duration::from_secs(settings.connection.timeout_sec.max(5));
    let mut b = Client::builder()
        .user_agent(ua)
        .default_headers(headers)
        .cookie_provider(build_jar(req))
        // Yönlendirmeler `send_ranged` içinde elle izlenir.
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(timeout)
        .pool_max_idle_per_host(crate::MAX_CONNECTIONS as usize)
        .tcp_keepalive(Duration::from_secs(30))
        .tcp_nodelay(true);
    if !settings.connection.http2 {
        b = b.http1_only();
    }
    b = apply_proxy(b, settings)?;
    Ok(b.build()?)
}

fn apply_proxy(mut b: reqwest::ClientBuilder, settings: &Settings) -> anyhow::Result<reqwest::ClientBuilder> {
    match settings.proxy.mode.as_str() {
        "none" => b = b.no_proxy(),
        "manual" if !settings.proxy.url.trim().is_empty() => {
            let mut p = reqwest::Proxy::all(settings.proxy.url.trim())?;
            if !settings.proxy.username.is_empty() {
                p = p.basic_auth(&settings.proxy.username, &settings.proxy.password);
            }
            let bypass = settings.proxy.bypass.join(",");
            if !bypass.is_empty() {
                p = p.no_proxy(reqwest::NoProxy::from_string(&bypass));
            }
            b = b.proxy(p);
        }
        // "system": reqwest ortam değişkenlerini ve Windows kayıt defterindeki vekili kendisi okur.
        _ => {}
    }
    Ok(b)
}

/// Site girişleri listesinde adrese uyan kullanıcı adı/parola.
pub fn site_login<'a>(settings: &'a Settings, url: &str) -> Option<(&'a str, &'a str)> {
    let u = url::Url::parse(url).ok()?;
    let target = format!("{}{}", u.host_str().unwrap_or(""), u.path()).to_lowercase();
    settings.site_logins.iter().find_map(|l| {
        let pat = l.pattern.to_lowercase();
        let pat = pat.trim_start_matches("https://").trim_start_matches("http://");
        let pat = if pat.contains('/') { pat.to_string() } else { format!("{}/*", pat) };
        if crate::util::wildcard_match(&pat, &target) { Some((l.username.as_str(), l.password.as_str())) } else { None }
    })
}

/// Adresteki kullanıcı bilgisini (`https://user:pass@host`) ayırır.
pub fn split_userinfo(url: &str) -> (String, Option<(String, String)>) {
    if let Ok(mut u) = url::Url::parse(url) {
        if !u.username().is_empty() {
            let user = percent_encoding::percent_decode_str(u.username()).decode_utf8_lossy().into_owned();
            let pass = u.password().map(|p| percent_encoding::percent_decode_str(p).decode_utf8_lossy().into_owned()).unwrap_or_default();
            let _ = u.set_username("");
            let _ = u.set_password(None);
            return (u.to_string(), Some((user, pass)));
        }
    }
    (url.to_string(), None)
}

/// Aralıklı GET isteği hazırlar.
pub fn ranged_get(client: &Client, url: &str, req: &RequestInfo, settings: &Settings, start: u64, end_excl: Option<u64>, trusted: bool) -> reqwest::RequestBuilder {
    let (clean, creds) = split_userinfo(url);
    let method = req.method.as_deref().unwrap_or("GET").to_ascii_uppercase();
    let mut rb = if method == "POST" {
        let mut r = client.post(&clean);
        if let Some(body) = &req.post_data {
            r = r.body(body.clone());
            if let Some((_, ct)) = req.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
                r = r.header(reqwest::header::CONTENT_TYPE, ct.clone());
            }
        }
        r
    } else {
        client.get(&clean)
    };
    let range = match end_excl {
        Some(e) if e > start => format!("bytes={}-{}", start, e - 1),
        _ => format!("bytes={}-", start),
    };
    rb = rb.header(reqwest::header::RANGE, range);
    if let (Some((u, p)), true) = (creds, trusted) {
        rb = rb.basic_auth(u, Some(p));
    } else if let Some((u, p)) = site_login(settings, &clean) {
        rb = rb.basic_auth(u, Some(p));
    }
    rb
}

/// Sunucu cevabından çıkarılan dosya bilgileri.
#[derive(Debug, Clone, Default)]
pub struct HeadInfo {
    pub status: u16,
    pub final_url: String,
    /// Toplam dosya boyutu.
    pub total: Option<u64>,
    /// Bu cevabın gövdesinin başladığı konum.
    pub body_start: u64,
    pub ranged: bool,
    pub accept_ranges: bool,
    pub content_disposition: Option<String>,
    pub mime: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub encoded: bool,
    pub retry_after: Option<u64>,
}

impl HeadInfo {
    /// Paralel parçalara bölünebilir mi.
    pub fn resumable(&self) -> bool {
        !self.encoded && self.total.is_some() && (self.ranged || self.accept_ranges)
    }
    pub fn is_html(&self) -> bool {
        self.mime.as_deref().map(|m| m.starts_with("text/html") || m.starts_with("application/xhtml")).unwrap_or(false)
    }
}

fn header_str(r: &Response, name: reqwest::header::HeaderName) -> Option<String> {
    r.headers().get(name).and_then(|v| v.to_str().ok()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// `bytes 100-199/1000` → (başlangıç, bitiş, toplam).
pub fn parse_content_range(v: &str) -> Option<(Option<(u64, u64)>, Option<u64>)> {
    let v = v.trim();
    let rest = v.strip_prefix("bytes")?.trim_start_matches([' ', '=']);
    let (range, total) = rest.split_once('/')?;
    let total = total.trim().parse::<u64>().ok();
    let range = if range.trim() == "*" {
        None
    } else {
        let (a, b) = range.split_once('-')?;
        Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
    };
    Some((range, total))
}

pub fn head_info(r: &Response) -> HeadInfo {
    let status = r.status();
    let mut info = HeadInfo {
        status: status.as_u16(),
        final_url: r.url().to_string(),
        content_disposition: header_str(r, reqwest::header::CONTENT_DISPOSITION),
        mime: header_str(r, reqwest::header::CONTENT_TYPE),
        etag: header_str(r, reqwest::header::ETAG),
        last_modified: header_str(r, reqwest::header::LAST_MODIFIED),
        accept_ranges: header_str(r, reqwest::header::ACCEPT_RANGES).map(|v| v.eq_ignore_ascii_case("bytes")).unwrap_or(false),
        encoded: header_str(r, reqwest::header::CONTENT_ENCODING).map(|v| !v.eq_ignore_ascii_case("identity")).unwrap_or(false),
        retry_after: header_str(r, reqwest::header::RETRY_AFTER).and_then(|v| v.parse().ok()),
        ..Default::default()
    };
    let len = r.content_length();
    if status == StatusCode::PARTIAL_CONTENT {
        info.ranged = true;
        if let Some((range, total)) = header_str(r, reqwest::header::CONTENT_RANGE).as_deref().and_then(parse_content_range) {
            info.total = total;
            if let Some((a, _)) = range {
                info.body_start = a;
            }
        }
    } else if status == StatusCode::RANGE_NOT_SATISFIABLE {
        if let Some((_, total)) = header_str(r, reqwest::header::CONTENT_RANGE).as_deref().and_then(parse_content_range) {
            info.total = total;
        }
    } else if status.is_success() {
        info.total = len;
    }
    info
}

/// Arayüzdeki "URL ekle" için dosya bilgisini sorar (gövde okunmaz).
pub async fn probe(req: &RequestInfo, settings: &Settings) -> anyhow::Result<ProbeResult> {
    let client = build_client(req, settings)?;
    let resp = send_ranged(&client, &req.url, req, settings, 0, None).await?;
    let info = head_info(&resp);
    drop(resp);
    let name = crate::filename::resolve(info.content_disposition.as_deref(), &info.final_url, &req.url, info.mime.as_deref());
    let is_html = info.is_html() && !matches!(crate::util::extension_of(&name).as_str(), "html" | "htm");
    Ok(ProbeResult {
        url: req.url.clone(),
        final_url: info.final_url.clone(),
        category: settings.category_for(&name),
        file_name: name,
        size: info.total,
        resumable: info.resumable(),
        mime: info.mime.clone(),
        is_html,
        status_code: info.status,
        is_media_site: crate::media::is_media_site(&req.url),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_range() {
        assert_eq!(parse_content_range("bytes 0-99/1000"), Some((Some((0, 99)), Some(1000))));
        assert_eq!(parse_content_range("bytes */0"), Some((None, Some(0))));
        assert_eq!(parse_content_range("bytes 5-9/*"), Some((Some((5, 9)), None)));
    }

    #[test]
    fn cookies_header() {
        assert_eq!(parse_cookie_header("a=1; b=x=y"), vec![("a".into(), "1".into()), ("b".into(), "x=y".into())]);
    }

    #[test]
    fn userinfo() {
        let (u, c) = split_userinfo("https://ali:gizli%21@ex.com/a.zip");
        assert_eq!(u, "https://ex.com/a.zip");
        assert_eq!(c, Some(("ali".into(), "gizli!".into())));
    }
}
