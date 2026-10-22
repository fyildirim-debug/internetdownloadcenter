//! Çok bağlantılı HTTP indirme görevi: IDM tarzı dinamik bölümleme.
//!
//! * İlk bağlantı `bytes=0-` ile açılır; cevap başlıkları gelince boyut ve sürdürme desteği öğrenilir.
//! * Boşta kalan her bağlantı en büyük kalan parçayı ikiye böler ve ikinci yarıyı alır.
//! * Veri tek bir `.idcpart` dosyasına konumlu yazılır; her bağlantının kendi dosya tanıtıcısı vardır.
//! * Diske kaydedilen konumlar ancak `fdatasync`'ten sonra `downloads.json`'a işlenir.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use parking_lot::Mutex;
use reqwest::{Client, StatusCode};
use tokio::task::JoinSet;

use super::{Active, Inner};
use crate::http::{self, HeadInfo};
use crate::model::*;
use crate::settings::Settings;
use crate::util::err_text;

/// Bundan küçük kalan parçalar bölünmez (her yarı en az bu kadar olur).
pub const MIN_SPLIT: u64 = 512 * 1024;
/// Bağlantı başına yazma arabelleği.
const WRITE_BUF: usize = 1 << 20;
/// Ek bağlantılar arasında bekleme (kademeli artış, sunucuyu ürkütmez).
const RAMP_DELAY: Duration = Duration::from_millis(60);

#[derive(Debug, Clone)]
struct Slot {
    start: u64,
    end: Option<u64>,
    /// Diske yazılmış konum.
    pos: u64,
    /// Alınmış (arabellekte olabilir) konum. Bölme bu konuma göre yapılır.
    recv: u64,
    worker: Option<usize>,
    info: &'static str,
    speed: u64,
    recv_mark: u64,
}

impl Slot {
    fn new(start: u64, end: Option<u64>, pos: u64) -> Self {
        Self { start, end, pos, recv: pos, worker: None, info: "", speed: 0, recv_mark: pos }
    }
    fn done(&self) -> bool {
        matches!(self.end, Some(e) if self.pos >= e)
    }
    fn remaining(&self) -> u64 {
        self.end.map(|e| e.saturating_sub(self.recv)).unwrap_or(u64::MAX)
    }
}

struct State {
    slots: Vec<Slot>,
    total: Option<u64>,
    resumable: bool,
    /// İlk cevap başlıkları alındı mı.
    known: bool,
    max_conns: u8,
    failures: u32,
    part_path: PathBuf,
    /// Sunucu aralık isteğini yok saydı; baştan tek bağlantıyla başlanmalı.
    restart_single: bool,
    fatal: Option<String>,
    got_data: bool,
}

enum Exit {
    NoWork,
    Cancelled,
    /// Yeniden denenebilir hata. `rejected`: ek bağlantı sunucu tarafından reddedildi.
    Retry { msg: String, rejected: bool, wait: Option<u64> },
    Fatal(String),
    RangeIgnored,
}

struct Ctx {
    id: DownloadId,
    inner: Arc<Inner>,
    active: Arc<Active>,
    client: Client,
    req: RequestInfo,
    settings: Settings,
    state: Mutex<State>,
    /// İlk bağlantının vardığı adres; ek bağlantılar yönlendirme zincirini yeniden yürümez.
    url: Mutex<String>,
    etag: Option<String>,
    expect_total: Option<u64>,
}

/// Görevin sonucu.
pub enum Outcome {
    Completed,
    Paused,
    Failed(String),
}

pub async fn run(inner: Arc<Inner>, active: Arc<Active>, d: Download) -> Outcome {
    let settings = inner.settings.read().clone();
    let client = match http::build_client(&d.request, &settings) {
        Ok(c) => c,
        Err(e) => return Outcome::Failed(err_text("unknown", e)),
    };
    let max = settings.connections_for_host(
        url::Url::parse(&d.request.url).ok().and_then(|u| u.host_str().map(String::from)).as_deref().unwrap_or(""),
        d.max_connections,
    );
    active.max_conns.store(max, Ordering::Relaxed);

    // Devam: kayıtlı parçalar ve dosya varsa onları kullan.
    let part = d.part_path();
    let resuming = d.resumable && d.size.is_some() && !d.segments.is_empty() && part.exists();
    let slots = if resuming {
        d.segments.iter().map(|s| Slot::new(s.start, s.end, s.pos)).collect()
    } else {
        vec![Slot::new(0, None, 0)]
    };
    let already: u64 = if resuming { d.segments.iter().map(|s| s.pos - s.start).sum() } else { 0 };
    active.recv.store(already, Ordering::Relaxed);

    let ctx = Arc::new(Ctx {
        id: d.id.clone(),
        inner: inner.clone(),
        active: active.clone(),
        client,
        req: d.request.clone(),
        state: Mutex::new(State {
            slots,
            total: if resuming { d.size } else { None },
            resumable: resuming,
            known: resuming,
            max_conns: max,
            failures: 0,
            part_path: part,
            restart_single: false,
            fatal: None,
            got_data: false,
        }),
        url: Mutex::new(d.final_url.clone().filter(|_| resuming).unwrap_or_else(|| d.request.url.clone())),
        etag: if resuming { d.etag.clone() } else { None },
        expect_total: if resuming { d.size } else { None },
        settings,
    });
    if !resuming {
        // Önceki yarım deneme kalmışsa temizle.
        let _ = std::fs::remove_file(d.part_path());
    }
    coordinate(ctx).await
}

async fn coordinate(ctx: Arc<Ctx>) -> Outcome {
    let mut set: JoinSet<Exit> = JoinSet::new();
    let mut next_wid = 0usize;
    let mut running = 0usize;
    let mut next_spawn = Instant::now();
    let mut last_commit = Instant::now();
    let retry_count = ctx.settings.downloads.retry_count.max(1);
    let retry_delay = ctx.settings.downloads.retry_delay_sec.max(1) as u64;
    let cancel = ctx.active.cancel.clone();
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut ticked = false;

    loop {
        // Yeni bağlantı aç.
        if !cancel.is_cancelled() && Instant::now() >= next_spawn {
            let want = {
                let st = ctx.state.lock();
                let max = st.max_conns.min(ctx.active.max_conns.load(Ordering::Relaxed)).max(1) as usize;
                let can_parallel = st.known && st.resumable;
                (running == 0 || (can_parallel && running < max)) && has_work(&st)
            };
            if want {
                let c = ctx.clone();
                let wid = next_wid;
                next_wid += 1;
                running += 1;
                ctx.active.conns.store(running as u8, Ordering::Relaxed);
                set.spawn(async move { worker(c, wid).await });
                next_spawn = Instant::now() + RAMP_DELAY;
            }
        }

        tokio::select! {
            Some(res) = set.join_next(), if running > 0 => {
                running -= 1;
                ctx.active.conns.store(running as u8, Ordering::Relaxed);
                let exit = res.unwrap_or_else(|e| Exit::Fatal(err_text("unknown", e)));
                match exit {
                    Exit::NoWork | Exit::Cancelled => {}
                    Exit::Fatal(m) => {
                        ctx.state.lock().fatal = Some(m);
                        cancel.cancel();
                    }
                    Exit::RangeIgnored => {
                        let mut st = ctx.state.lock();
                        st.restart_single = true;
                    }
                    Exit::Retry { msg, rejected, wait } => {
                        let mut st = ctx.state.lock();
                        if rejected && running > 0 && ctx.settings.connection.adaptive {
                            // Sunucu bu kadar bağlantıya izin vermiyor: sayıyı düşür.
                            st.max_conns = (running as u8).max(1);
                            tracing::info!("{}: bağlantı sayısı {}'e düşürüldü ({msg})", ctx.id, st.max_conns);
                            next_spawn = Instant::now() + Duration::from_secs(wait.unwrap_or(10).min(120));
                        } else {
                            st.failures += 1;
                            if st.failures > retry_count {
                                st.fatal = Some(msg);
                                drop(st);
                                cancel.cancel();
                            } else {
                                let n = st.failures.min(6);
                                let delay = wait.unwrap_or(retry_delay * (1u64 << (n - 1))).min(60);
                                tracing::info!("{}: {msg}; {delay} sn sonra yeniden ({}/{retry_count})", ctx.id, st.failures);
                                for s in st.slots.iter_mut() {
                                    if s.worker.is_none() && !s.done() {
                                        s.info = "retry";
                                    }
                                }
                                next_spawn = Instant::now() + Duration::from_secs(delay);
                            }
                        }
                    }
                }
            }
            _ = tick.tick() => { ticked = true; }
            _ = cancel.cancelled(), if running == 0 => {}
        }

        // Aralık desteği olmadığı ortaya çıktı: tüm bağlantılar bitince baştan tek bağlantı.
        if running == 0 && ctx.state.lock().restart_single && !cancel.is_cancelled() {
            let mut st = ctx.state.lock();
            st.restart_single = false;
            st.resumable = false;
            st.max_conns = 1;
            let total = st.total;
            st.slots = vec![Slot::new(0, total, 0)];
            let path = st.part_path.clone();
            drop(st);
            let _ = std::fs::remove_file(&path);
            ctx.active.recv.store(0, Ordering::Relaxed);
            ctx.inner.update(&ctx.id, |d| d.resumable = false);
        }

        if ticked {
            ticked = false;
            update_views(&ctx);
        }

        if last_commit.elapsed() >= Duration::from_secs(3) {
            last_commit = Instant::now();
            commit(&ctx).await;
        }

        if running == 0 {
            let (fatal, complete) = {
                let st = ctx.state.lock();
                (st.fatal.clone(), st.known && st.total.is_some() && st.slots.iter().all(|s| s.done()))
            };
            if let Some(m) = fatal {
                commit(&ctx).await;
                return Outcome::Failed(m);
            }
            if cancel.is_cancelled() {
                commit(&ctx).await;
                return Outcome::Paused;
            }
            if complete {
                return finalize(&ctx).await;
            }
        }
    }
}

fn has_work(st: &State) -> bool {
    if !st.known {
        // İlk cevap gelmeden yalnız ilk bağlantı açılır.
        return st.slots.iter().any(|s| s.worker.is_none() && !s.done());
    }
    st.slots.iter().any(|s| s.worker.is_none() && !s.done()) || (st.resumable && st.slots.iter().any(|s| s.worker.is_some() && s.remaining() >= 2 * MIN_SPLIT))
}

/// Bağlantıya iş verir: sahipsiz parça ya da en büyük parçanın ikinci yarısı.
fn assign(st: &mut State, wid: usize) -> Option<usize> {
    if let Some(i) = st.slots.iter().position(|s| s.worker.is_none() && !s.done()) {
        st.slots[i].worker = Some(wid);
        st.slots[i].info = "connecting";
        return Some(i);
    }
    if !(st.known && st.resumable) {
        return None;
    }
    let (i, rem) = st
        .slots
        .iter()
        .enumerate()
        .filter(|(_, s)| s.worker.is_some() && s.end.is_some())
        .map(|(i, s)| (i, s.remaining()))
        .max_by_key(|(_, r)| *r)?;
    if rem < 2 * MIN_SPLIT {
        return None;
    }
    let s = &mut st.slots[i];
    // Ortadan böl; 64 KiB'a hizala.
    let mid = (s.recv + rem / 2) & !0xFFFF;
    let mid = mid.max(s.recv + MIN_SPLIT);
    let old_end = s.end.unwrap();
    if mid >= old_end {
        return None;
    }
    s.end = Some(mid);
    let mut n = Slot::new(mid, Some(old_end), mid);
    n.worker = Some(wid);
    n.info = "connecting";
    // Başlangıca göre sıralı kalsın (bölüm çubuğu için).
    let at = st.slots.iter().position(|x| x.start > mid).unwrap_or(st.slots.len());
    st.slots.insert(at, n);
    Some(at)
}

fn slot_index(st: &State, wid: usize) -> Option<usize> {
    st.slots.iter().position(|s| s.worker == Some(wid))
}

async fn worker(ctx: Arc<Ctx>, wid: usize) -> Exit {
    let cancel = ctx.active.cancel.clone();
    let mut file: Option<Arc<File>> = None;
    loop {
        if cancel.is_cancelled() {
            return Exit::Cancelled;
        }
        let (start, end, first) = {
            let mut st = ctx.state.lock();
            let Some(i) = assign(&mut st, wid) else { return Exit::NoWork };
            let s = &st.slots[i];
            (s.recv, s.end, !st.known)
        };
        let res = fetch_slot(&ctx, wid, start, end, first, &mut file).await;
        let mut st = ctx.state.lock();
        if let Some(i) = slot_index(&st, wid) {
            let s = &mut st.slots[i];
            s.worker = None;
            s.speed = 0;
            s.info = if s.done() { "done" } else { "" };
        }
        drop(st);
        match res {
            Ok(()) => continue,
            Err(e) => return e,
        }
    }
}

fn open_part(path: &Path, create: bool, total: Option<u64>, preallocate: bool) -> std::io::Result<File> {
    let f = std::fs::OpenOptions::new().read(true).write(true).create(create).truncate(false).open(path)?;
    if create {
        if let (Some(t), true) = (total, preallocate) {
            if f.metadata()?.len() < t {
                f.set_len(t)?;
            }
        }
    }
    Ok(f)
}

#[cfg(unix)]
fn write_at(f: &File, off: u64, buf: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    f.write_all_at(buf, off)
}

#[cfg(windows)]
fn write_at(f: &File, mut off: u64, mut buf: &[u8]) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        let n = f.seek_write(buf, off)?;
        if n == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::WriteZero, "yazılamadı"));
        }
        buf = &buf[n..];
        off += n as u64;
    }
    Ok(())
}

fn io_err_text(e: &std::io::Error) -> String {
    let code = e.raw_os_error();
    if e.kind() == std::io::ErrorKind::StorageFull || code == Some(28) || code == Some(112) {
        err_text("disk_full", e)
    } else {
        err_text("write_failed", e)
    }
}

/// Bir parçayı indirir. Başarıda parça bitmiştir.
async fn fetch_slot(ctx: &Arc<Ctx>, wid: usize, start: u64, end: Option<u64>, first: bool, file: &mut Option<Arc<File>>) -> Result<(), Exit> {
    let cancel = ctx.active.cancel.clone();
    let url = ctx.url.lock().clone();
    let rb = http::ranged_get(&ctx.client, &url, &ctx.req, &ctx.settings, start, end);
    let resp = tokio::select! {
        _ = cancel.cancelled() => return Err(Exit::Cancelled),
        r = rb.send() => r,
    };
    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            // Ek bağlantılarda kaydedilmiş son adres reddedilirse orijinale dön.
            let msg = if e.is_timeout() { err_text("timeout", e) } else { err_text("network", short_err(&e)) };
            return Err(Exit::Retry { msg, rejected: false, wait: None });
        }
    };
    let info = http::head_info(&resp);
    let status = resp.status();

    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        let mut st = ctx.state.lock();
        // Boş dosya ya da zaten tamamlanmış parça.
        if info.total == Some(0) || (st.known && st.total.is_some_and(|t| start >= t)) {
            if !st.known {
                drop(st);
                on_headers(ctx, &HeadInfo { total: Some(0), ..info.clone() }).map_err(Exit::Fatal)?;
                st = ctx.state.lock();
            }
            if let Some(i) = slot_index(&st, wid) {
                let s = &mut st.slots[i];
                s.end = Some(s.recv);
                s.pos = s.recv;
            }
            return Ok(());
        }
        return Err(Exit::Retry { msg: err_text("http_status", 416), rejected: false, wait: None });
    }
    if !status.is_success() {
        let code = status.as_u16();
        let others = ctx.active.conns.load(Ordering::Relaxed) > 1;
        return Err(match code {
            404 | 410 if !others => Exit::Fatal(err_text(if code == 410 { "expired" } else { "not_found" }, code)),
            401 | 403 | 429 | 503 | 509 | 420 | 421 if others => Exit::Retry { msg: err_text("http_status", code), rejected: true, wait: info.retry_after },
            403 | 401 => {
                // Ek bağlantının imzalı son adresi reddedilmiş olabilir: orijinal adresle dene.
                *ctx.url.lock() = ctx.req.url.clone();
                Exit::Retry { msg: err_text("expired", code), rejected: false, wait: info.retry_after }
            }
            _ => Exit::Retry { msg: err_text("http_status", code), rejected: false, wait: info.retry_after },
        });
    }

    if first {
        on_headers(ctx, &info).map_err(Exit::Fatal)?;
    } else {
        // Aralık doğrulaması: sunucu istenen yerden başlamalı.
        if start > 0 && (!info.ranged || info.body_start != start) {
            return Err(Exit::RangeIgnored);
        }
        let st = ctx.state.lock();
        if let (Some(t), Some(exp)) = (info.total, st.total) {
            if info.ranged && t != exp {
                return Err(Exit::Fatal(err_text("changed", format!("{exp} → {t}"))));
            }
        }
        drop(st);
        if let (Some(old), Some(new)) = (ctx.etag.as_deref(), info.etag.as_deref()) {
            if !old.starts_with("W/") && !new.starts_with("W/") && old != new {
                return Err(Exit::Fatal(err_text("changed", "ETag")));
            }
        }
    }

    // İlk bağlantı yönlendirmelerden sonraki adresi paylaşır.
    if first && info.final_url != ctx.req.url {
        *ctx.url.lock() = info.final_url.clone();
    }

    if file.is_none() {
        let (path, total, create) = {
            let st = ctx.state.lock();
            (st.part_path.clone(), st.total, true)
        };
        let pre = ctx.settings.downloads.preallocate;
        let f = tokio::task::spawn_blocking(move || open_part(&path, create, total, pre))
            .await
            .map_err(|e| Exit::Fatal(err_text("write_failed", e)))?
            .map_err(|e| Exit::Fatal(io_err_text(&e)))?;
        *file = Some(Arc::new(f));
    }
    let f = file.clone().unwrap();

    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(WRITE_BUF);
    let mut buf_off = start;
    let mut last_flush = Instant::now();
    {
        let mut st = ctx.state.lock();
        if let Some(i) = slot_index(&st, wid) {
            st.slots[i].info = "receiving";
        }
    }

    let result: Result<bool, Exit> = loop {
        let next = tokio::select! {
            _ = cancel.cancelled() => break Err(Exit::Cancelled),
            n = stream.next() => n,
        };
        let chunk = match next {
            None => break Ok(false),
            Some(Err(e)) => {
                let msg = if e.is_timeout() { err_text("timeout", short_err(&e)) } else { err_text("network", short_err(&e)) };
                break Err(Exit::Retry { msg, rejected: false, wait: None });
            }
            Some(Ok(c)) => c,
        };
        ctx.active.limiter.acquire(chunk.len() as u64).await;
        ctx.inner.global.acquire(chunk.len() as u64).await;

        // Parçanın (bölünmüş olabilecek) sonuna göre kes.
        let (take, reached_end) = {
            let mut st = ctx.state.lock();
            st.got_data = true;
            st.failures = 0;
            let Some(i) = slot_index(&st, wid) else { break Err(Exit::Cancelled) };
            let s = &mut st.slots[i];
            let allowed = s.end.map(|e| e.saturating_sub(s.recv)).unwrap_or(u64::MAX);
            let take = (chunk.len() as u64).min(allowed);
            s.recv += take;
            (take as usize, s.end.is_some_and(|e| s.recv >= e))
        };
        ctx.active.recv.fetch_add(take as u64, Ordering::Relaxed);
        buf.extend_from_slice(&chunk[..take]);

        if buf.len() >= WRITE_BUF || reached_end || last_flush.elapsed() > Duration::from_secs(2) {
            if let Err(e) = flush(ctx, wid, &f, &mut buf, &mut buf_off).await {
                break Err(e);
            }
            last_flush = Instant::now();
        }
        if reached_end {
            break Ok(true);
        }
    };

    // Çıkışta arabellekte kalan veriyi her durumda diske yaz; alınan veri boşa gitmesin.
    if let Err(e) = flush(ctx, wid, &f, &mut buf, &mut buf_off).await {
        return Err(e);
    }

    match result {
        Ok(true) => Ok(()),
        Ok(false) => {
            // Akış bitti: boyut bilinmiyorsa dosya burada biter.
            let mut st = ctx.state.lock();
            let Some(i) = slot_index(&st, wid) else { return Ok(()) };
            let s = &mut st.slots[i];
            match s.end {
                None => {
                    s.end = Some(s.pos);
                    let t = s.pos;
                    st.total = Some(t);
                    drop(st);
                    ctx.inner.update(&ctx.id, |d| d.size = Some(t));
                    Ok(())
                }
                Some(e) if s.pos >= e => Ok(()),
                Some(_) => Err(Exit::Retry { msg: err_text("network", "bağlantı erken kapandı"), rejected: false, wait: None }),
            }
        }
        Err(e) => Err(e),
    }
}

async fn flush(ctx: &Arc<Ctx>, wid: usize, f: &Arc<File>, buf: &mut Vec<u8>, off: &mut u64) -> Result<(), Exit> {
    if buf.is_empty() {
        return Ok(());
    }
    let data = std::mem::replace(buf, Vec::with_capacity(WRITE_BUF));
    let len = data.len() as u64;
    let at = *off;
    let fc = f.clone();
    let (res, data) = tokio::task::spawn_blocking(move || {
        let r = write_at(&fc, at, &data);
        (r, data)
    })
    .await
    .map_err(|e| Exit::Fatal(err_text("write_failed", e)))?;
    if let Err(e) = res {
        return Err(Exit::Fatal(io_err_text(&e)));
    }
    *off += len;
    let mut st = ctx.state.lock();
    if let Some(i) = slot_index(&st, wid) {
        let s = &mut st.slots[i];
        s.pos = s.pos.max(*off);
    }
    drop(st);
    // Arabelleği yeniden kullan.
    let mut data = data;
    data.clear();
    *buf = data;
    Ok(())
}

fn short_err(e: &reqwest::Error) -> String {
    let mut s = e.to_string();
    let mut src = std::error::Error::source(e);
    while let Some(x) = src {
        s = x.to_string();
        src = x.source();
    }
    s
}

/// İlk cevap başlıkları: boyut, ad, sürdürme desteği.
fn on_headers(ctx: &Arc<Ctx>, info: &HeadInfo) -> Result<(), String> {
    if info.ranged && info.body_start != 0 {
        return Err(err_text("http_status", "Content-Range"));
    }
    let resumable = info.resumable() && ctx.req.method.as_deref().map(|m| m.eq_ignore_ascii_case("GET")).unwrap_or(true);
    let total = info.total;
    if let (Some(exp), Some(t)) = (ctx.expect_total, total) {
        if exp != t {
            return Err(err_text("changed", format!("{exp} → {t}")));
        }
    }

    // Dosya adı ve boş alan.
    let settings = &ctx.settings;
    let inner = &ctx.inner;
    let mut new_part = None;
    let mut space_err = None;
    inner.update(&ctx.id, |d| {
        d.final_url = Some(info.final_url.clone());
        d.size = total;
        d.resumable = resumable;
        d.mime = info.mime.clone();
        d.etag = info.etag.clone();
        d.last_modified = info.last_modified.clone();
        if d.auto_name {
            let name = crate::filename::resolve(info.content_disposition.as_deref(), &info.final_url, &d.request.url, info.mime.as_deref());
            if name != d.file_name {
                let cat = settings.category_for(&name);
                if d.category == "general" && cat != "general" {
                    d.category = cat;
                }
                d.file_name = name;
            }
            d.auto_name = false;
        }
        new_part = Some(d.part_path());
        if let Some(t) = total {
            if let Ok(free) = fs4::available_space(Path::new(&d.save_dir)) {
                if free < t {
                    space_err = Some(err_text("disk_full", format!("{} / {}", crate::util::human_size(free), crate::util::human_size(t))));
                }
            }
        }
    });
    if let Some(e) = space_err {
        return Err(e);
    }
    // Aynı adlı başka indirme/dosya ile çakışma: adı benzersiz yap.
    inner.ensure_unique_name(&ctx.id);
    let part = inner.with(&ctx.id, |d| d.part_path()).or(new_part).unwrap();

    let mut st = ctx.state.lock();
    st.known = true;
    st.total = total;
    st.resumable = resumable;
    st.part_path = part;
    if !resumable {
        st.max_conns = 1;
    }
    if let Some(s) = st.slots.first_mut() {
        s.end = total;
    }
    Ok(())
}

/// Arayüz için parça görünümleri ve hızlar.
fn update_views(ctx: &Arc<Ctx>) {
    let mut st = ctx.state.lock();
    let views: Vec<SegmentView> = st
        .slots
        .iter_mut()
        .map(|s| {
            let delta = s.recv.saturating_sub(s.recv_mark);
            s.recv_mark = s.recv;
            // 100 ms'lik örneklerden yumuşatılmış hız.
            s.speed = ((s.speed as f64) * 0.8 + (delta as f64 * 10.0) * 0.2) as u64;
            SegmentView {
                start: s.start,
                end: s.end,
                pos: s.recv,
                active: s.worker.is_some(),
                speed: if s.worker.is_some() { s.speed } else { 0 },
                info: if s.done() { "done".into() } else if s.worker.is_some() { s.info.to_string() } else { s.info.to_string() },
            }
        })
        .collect();
    drop(st);
    *ctx.active.segs.lock() = views;
}

/// Diske yazılmış konumları kalıcı kayda işler (önce fdatasync).
async fn commit(ctx: &Arc<Ctx>) {
    let (segs, path, total) = {
        let st = ctx.state.lock();
        (
            st.slots.iter().map(|s| Segment { start: s.start, end: s.end, pos: s.pos }).collect::<Vec<_>>(),
            st.part_path.clone(),
            st.total,
        )
    };
    let synced = tokio::task::spawn_blocking(move || match std::fs::OpenOptions::new().write(true).open(&path) {
        Ok(f) => f.sync_data().is_ok(),
        Err(_) => false,
    })
    .await
    .unwrap_or(false);
    let downloaded: u64 = segs.iter().map(|s| s.pos - s.start).sum();
    ctx.inner.update(&ctx.id, |d| {
        if synced {
            d.segments = segs.clone();
        }
        d.downloaded = downloaded;
        if total.is_some() {
            d.size = total;
        }
    });
    ctx.inner.mark_dirty();
}

async fn finalize(ctx: &Arc<Ctx>) -> Outcome {
    let (path, total) = {
        let st = ctx.state.lock();
        (st.part_path.clone(), st.total.unwrap_or(0))
    };
    ctx.active.recv.store(total, Ordering::Relaxed);
    let p = path.clone();
    let r = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        let f = std::fs::OpenOptions::new().write(true).create(true).truncate(false).open(&p)?;
        f.set_len(total)?;
        f.sync_all()?;
        Ok(())
    })
    .await;
    match r {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Outcome::Failed(io_err_text(&e)),
        Err(e) => return Outcome::Failed(err_text("write_failed", e)),
    }
    ctx.inner.update(&ctx.id, |d| {
        d.size = Some(total);
        d.downloaded = total;
        d.segments.clear();
    });
    super::post::finish_file(&ctx.inner, &ctx.id, &path).await
}
