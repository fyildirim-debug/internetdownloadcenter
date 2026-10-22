//! Motorun uçtan uca testleri: yerel bir HTTP sunucusuna karşı gerçek indirmeler.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::{Request, Response, StatusCode};
use idc_core::model::*;
use idc_core::Engine;

fn data(n: usize) -> Vec<u8> {
    (0..n).map(|i| ((i * 31 + 7) % 251) as u8).collect()
}

#[derive(Default)]
struct Srv {
    active: AtomicUsize,
    requests: AtomicUsize,
    drops: AtomicUsize,
}

type B = BoxBody<Bytes, std::io::Error>;

fn full(b: impl Into<Bytes>) -> B {
    Full::new(b.into()).map_err(|_: Infallible| std::io::Error::other("x")).boxed()
}

struct Guard(Arc<Srv>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Gövdeyi küçük parçalar halinde, isteğe bağlı gecikme ve kopmayla akıtır.
fn stream_body(bytes: Vec<u8>, delay_ms: u64, cut_after: Option<usize>, guard: Guard) -> B {
    let chunks: Vec<Vec<u8>> = bytes.chunks(16 * 1024).map(|c| c.to_vec()).collect();
    let s = futures_util::stream::iter(chunks.into_iter().enumerate()).then(move |(i, c)| {
        let _g = &guard;
        async move {
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            if let Some(cut) = cut_after {
                if i * 16 * 1024 >= cut {
                    return Err(std::io::Error::other("kesildi"));
                }
            }
            Ok(Frame::data(Bytes::from(c)))
        }
    });
    BodyExt::boxed(StreamBody::new(s))
}

fn q(req: &Request<hyper::body::Incoming>, k: &str) -> Option<String> {
    req.uri().query().and_then(|qs| qs.split('&').find_map(|p| p.split_once('=').filter(|(a, _)| *a == k).map(|(_, b)| b.to_string())))
}

async fn handle(srv: Arc<Srv>, req: Request<hyper::body::Incoming>) -> Result<Response<B>, Infallible> {
    srv.requests.fetch_add(1, Ordering::SeqCst);
    let size: usize = q(&req, "size").and_then(|s| s.parse().ok()).unwrap_or(1 << 20);
    let body = data(size);
    let path = req.uri().path().to_string();

    if path == "/leak" {
        // Aynı sitede gizli başlık gerekli; sonra başka bir "siteye" (localhost) yönlendir.
        if req.headers().get("x-secret").is_none() {
            let mut r = Response::new(full("başlık yok"));
            *r.status_mut() = StatusCode::FORBIDDEN;
            return Ok(r);
        }
        let port = req.headers().get("host").and_then(|h| h.to_str().ok()).and_then(|h| h.rsplit(':').next()).unwrap_or("0").to_string();
        let mut r = Response::new(full(""));
        *r.status_mut() = StatusCode::FOUND;
        r.headers_mut().insert("location", format!("http://localhost:{port}/file?size={size}&noleak=1").parse().unwrap());
        return Ok(r);
    }
    if q(&req, "noleak").is_some() && req.headers().get("x-secret").is_some() {
        let mut r = Response::new(full("sızdı"));
        *r.status_mut() = StatusCode::BAD_REQUEST;
        return Ok(r);
    }
    if path == "/redirect" {
        let mut r = Response::new(full(""));
        *r.status_mut() = StatusCode::FOUND;
        r.headers_mut().insert("location", format!("/file?size={size}&cookie=1").parse().unwrap());
        r.headers_mut().insert("set-cookie", "srv=ok; Path=/".parse().unwrap());
        return Ok(r);
    }
    if q(&req, "cookie").is_some() {
        let c = req.headers().get("cookie").and_then(|v| v.to_str().ok()).unwrap_or("");
        if !c.contains("srv=ok") || !c.contains("oturum=42") {
            let mut r = Response::new(full("giriş gerekli"));
            *r.status_mut() = StatusCode::FORBIDDEN;
            return Ok(r);
        }
    }

    let now = srv.active.fetch_add(1, Ordering::SeqCst) + 1;
    let guard = Guard(srv.clone());
    if let Some(max) = q(&req, "maxconn").and_then(|s| s.parse::<usize>().ok()) {
        if now > max {
            let mut r = Response::new(full("çok fazla"));
            *r.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            return Ok(r);
        }
    }
    let delay: u64 = q(&req, "delay").and_then(|s| s.parse().ok()).unwrap_or(0);
    let mode = q(&req, "mode").unwrap_or_default();

    let range = req.headers().get("range").and_then(|v| v.to_str().ok()).and_then(|v| {
        let v = v.strip_prefix("bytes=")?;
        let (a, b) = v.split_once('-')?;
        let a: usize = a.parse().ok()?;
        let b: usize = if b.is_empty() { size.saturating_sub(1) } else { b.parse().ok()? };
        Some((a, b.min(size.saturating_sub(1))))
    });

    // İlk iki istek yarıda kopsun.
    let cut = if mode == "drop" && srv.drops.fetch_add(1, Ordering::SeqCst) < 2 { Some(200 * 1024) } else { None };

    if mode == "chunked" {
        // Boyut bildirmeyen, aralık desteklemeyen akış.
        let r = Response::new(stream_body(body, delay, cut, guard));
        return Ok(r);
    }
    if size == 0 {
        let mut r = Response::new(full(""));
        if range.is_some() {
            *r.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
            r.headers_mut().insert("content-range", "bytes */0".parse().unwrap());
        }
        return Ok(r);
    }
    match (range, mode.as_str()) {
        (Some((a, b)), m) if m != "norange" && a < size => {
            let part = body[a..=b].to_vec();
            let len = part.len();
            let mut r = Response::new(stream_body(part, delay, cut, guard));
            *r.status_mut() = StatusCode::PARTIAL_CONTENT;
            r.headers_mut().insert("content-range", format!("bytes {a}-{b}/{size}").parse().unwrap());
            r.headers_mut().insert("content-length", len.to_string().parse().unwrap());
            r.headers_mut().insert("accept-ranges", "bytes".parse().unwrap());
            r.headers_mut().insert("content-disposition", "attachment; filename*=UTF-8''%C3%B6rnek%20dosya.bin".parse().unwrap());
            r.headers_mut().insert("etag", "\"v1\"".parse().unwrap());
            Ok(r)
        }
        _ => {
            let mut r = Response::new(stream_body(body, delay, cut, guard));
            r.headers_mut().insert("content-length", size.to_string().parse().unwrap());
            if mode != "norange" {
                r.headers_mut().insert("accept-ranges", "bytes".parse().unwrap());
            } else {
                // Yalan söyleyen sunucu: aralık destekliyorum der ama yok sayar.
                r.headers_mut().insert("accept-ranges", "bytes".parse().unwrap());
            }
            Ok(r)
        }
    }
}

async fn server() -> (SocketAddr, Arc<Srv>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let srv = Arc::new(Srv::default());
    let s2 = srv.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else { continue };
            let s3 = s2.clone();
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let svc = hyper::service::service_fn(move |r| handle(s3.clone(), r));
                let _ = hyper::server::conn::http1::Builder::new().serve_connection(io, svc).await;
            });
        }
    });
    (addr, srv)
}

struct Env {
    engine: Engine,
    dir: tempfile::TempDir,
}

fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::new(dir.path().join("cfg"));
    let mut s = engine.settings();
    s.default_save_dir = dir.path().join("dl").to_string_lossy().into();
    for c in &mut s.categories {
        c.save_dir = s.default_save_dir.clone();
    }
    s.downloads.retry_delay_sec = 1;
    s.downloads.retry_count = 8;
    engine.save_settings(s).unwrap();
    Env { engine, dir }
}

async fn wait_done(e: &Engine, id: &str, secs: u64) -> Download {
    let mut rx = e.subscribe();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    loop {
        if let Some(d) = e.get(id) {
            if matches!(d.status, Status::Completed | Status::Error) && !e.inner_active(id) {
                return d;
            }
        }
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        if left.is_zero() {
            panic!("zaman aşımı: {:?}", e.get(id).map(|d| (d.status, d.error, d.downloaded)));
        }
        let _ = tokio::time::timeout(left.min(Duration::from_millis(200)), rx.recv()).await;
    }
}

trait ActiveExt {
    fn inner_active(&self, id: &str) -> bool;
}
impl ActiveExt for Engine {
    fn inner_active(&self, id: &str) -> bool {
        self.list().iter().any(|v| v.id == id && v.status.is_active())
    }
}

fn add(e: &Engine, url: String, conns: u8) -> String {
    e.add(AddRequest { request: RequestInfo { url, ..Default::default() }, max_connections: Some(conns), start: true, ..Default::default() }).unwrap()
}

fn check(d: &Download, size: usize) {
    assert_eq!(d.status, Status::Completed, "hata: {:?}", d.error);
    let got = std::fs::read(d.target_path()).unwrap();
    assert_eq!(got.len(), size);
    assert!(got == data(size), "içerik bozuk");
    assert!(!d.part_path().exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn segmented_download() {
    let (addr, srv) = server().await;
    let env = env();
    let size = 9 * 1024 * 1024 + 123;
    let id = add(&env.engine, format!("http://{addr}/file?size={size}&delay=10"), 8);
    let d = wait_done(&env.engine, &id, 60).await;
    check(&d, size);
    assert_eq!(d.file_name, "örnek dosya.bin");
    assert!(d.resumable);
    assert!(srv.requests.load(Ordering::SeqCst) >= 8, "paralel bağlantı açılmadı: {}", srv.requests.load(Ordering::SeqCst));
    drop(env.dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn server_ignores_range() {
    let (addr, _srv) = server().await;
    let env = env();
    let size = 3 * 1024 * 1024;
    let id = add(&env.engine, format!("http://{addr}/file?size={size}&mode=norange&delay=3"), 8);
    let d = wait_done(&env.engine, &id, 60).await;
    check(&d, size);
    assert!(!d.resumable);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn adaptive_connection_limit() {
    let (addr, _srv) = server().await;
    let env = env();
    let size = 6 * 1024 * 1024;
    let id = add(&env.engine, format!("http://{addr}/file?size={size}&maxconn=2&delay=3"), 16);
    let d = wait_done(&env.engine, &id, 90).await;
    check(&d, size);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dropped_connections_retry() {
    let (addr, _srv) = server().await;
    let env = env();
    let size = 2 * 1024 * 1024;
    let id = add(&env.engine, format!("http://{addr}/file?size={size}&mode=drop"), 4);
    let d = wait_done(&env.engine, &id, 60).await;
    check(&d, size);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unknown_size_stream() {
    let (addr, _srv) = server().await;
    let env = env();
    let size = 1024 * 1024 + 7;
    let id = add(&env.engine, format!("http://{addr}/stream.dat?size={size}&mode=chunked"), 8);
    let d = wait_done(&env.engine, &id, 60).await;
    check(&d, size);
    assert_eq!(d.size, Some(size as u64));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn zero_byte_file() {
    let (addr, _srv) = server().await;
    let env = env();
    let id = add(&env.engine, format!("http://{addr}/empty.txt?size=0"), 8);
    let d = wait_done(&env.engine, &id, 30).await;
    check(&d, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cookies_across_redirect() {
    let (addr, _srv) = server().await;
    let env = env();
    let size = 2 * 1024 * 1024;
    let req = RequestInfo {
        url: format!("http://{addr}/redirect?size={size}"),
        cookies: vec![CookieSpec { name: "oturum".into(), value: "42".into(), domain: "127.0.0.1".into(), path: "/".into(), host_only: true, ..Default::default() }],
        ..Default::default()
    };
    let id = env.engine.add(AddRequest { request: req, start: true, ..Default::default() }).unwrap();
    let d = wait_done(&env.engine, &id, 60).await;
    check(&d, size);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pause_and_resume() {
    let (addr, _srv) = server().await;
    let env = env();
    let size = 16 * 1024 * 1024;
    // Yavaş sunucu: duraklatma anında indirme kesinlikle sürüyor olsun.
    let id = add(&env.engine, format!("http://{addr}/file?size={size}&delay=30"), 8);
    // Bir süre insin, sonra duraklat.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    env.engine.pause(&[id.clone()]);
    for _ in 0..50 {
        if env.engine.get(&id).unwrap().status == Status::Paused {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let d = env.engine.get(&id).unwrap();
    assert_eq!(d.status, Status::Paused);
    assert!(!d.segments.is_empty(), "parçalar kaydedilmedi");
    let partial: u64 = d.segments.iter().map(|s| s.pos - s.start).sum();
    assert!(partial > 0 && partial < size as u64, "kısmi: {partial}");

    // Motoru yeniden kur (uygulama kapanıp açılmış gibi).
    env.engine.save_if_dirty();
    let cfg = env.dir.path().join("cfg");
    let e2 = Engine::new(cfg);
    e2.start(&[id.clone()]);
    let d = wait_done(&e2, &id, 90).await;
    check(&d, size);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn duplicate_names_are_unique() {
    let (addr, _srv) = server().await;
    let env = env();
    let size = 300 * 1024;
    let a = add(&env.engine, format!("http://{addr}/same.bin?size={size}&mode=norange"), 2);
    let b = add(&env.engine, format!("http://{addr}/same.bin?size={size}&mode=norange"), 2);
    let da = wait_done(&env.engine, &a, 30).await;
    let db = wait_done(&env.engine, &b, 30).await;
    check(&da, size);
    check(&db, size);
    assert_ne!(da.file_name, db.file_name);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn custom_headers_stay_on_origin() {
    let (addr, _srv) = server().await;
    let env = env();
    let size = 1024 * 1024;
    let req = RequestInfo {
        url: format!("http://{addr}/leak?size={size}"),
        headers: vec![("X-Secret".into(), "gizli".into())],
        ..Default::default()
    };
    let id = env.engine.add(AddRequest { request: req, start: true, ..Default::default() }).unwrap();
    let d = wait_done(&env.engine, &id, 60).await;
    check(&d, size);
}
