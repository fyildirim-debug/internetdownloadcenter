//! İndirme listesini, etkin görevleri, kuyrukları ve zamanlayıcıyı yöneten motor.

mod post;
pub mod task;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::{Mutex, RwLock};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::limiter::RateLimiter;
use crate::model::*;
use crate::settings::{Settings, MAIN_QUEUE};
use crate::store::Store;
use crate::util::err_text;

/// Motordan arayüze giden olaylar.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    Changed(DownloadView),
    Removed(Vec<DownloadId>),
    Progress(Vec<ProgressTick>),
    /// İndirme bitti; uygulama tamamlandı penceresini açar.
    Completed(DownloadId),
    Failed(DownloadId, String),
    /// Bitince yapılacak sistem işlemi (indirme ya da kuyruk).
    After(AfterAction),
    QueueFinished(String),
    Settings(Settings),
}

/// Çalışan bir indirmenin paylaşılan durumu.
pub struct Active {
    pub cancel: CancellationToken,
    /// Toplam alınan bayt (ilerleme için).
    pub recv: AtomicU64,
    pub conns: AtomicU8,
    pub max_conns: AtomicU8,
    pub limiter: RateLimiter,
    pub segs: Mutex<Vec<SegmentView>>,
    pub speed: AtomicU64,
    last_recv: AtomicU64,
    pub history: Mutex<VecDeque<u64>>,
    /// Medya görevlerinin bildirdiği toplam boyut ve hız (yt-dlp).
    pub media_total: AtomicU64,
    pub media_speed: AtomicU64,
    pub media_eta: AtomicU64,
}

impl Active {
    fn new(limit: Option<u64>, max_conns: u8) -> Self {
        Self {
            cancel: CancellationToken::new(),
            recv: AtomicU64::new(0),
            conns: AtomicU8::new(0),
            max_conns: AtomicU8::new(max_conns),
            limiter: RateLimiter::new(limit.unwrap_or(0)),
            segs: Mutex::new(vec![]),
            speed: AtomicU64::new(0),
            last_recv: AtomicU64::new(0),
            history: Mutex::new(VecDeque::with_capacity(120)),
            media_total: AtomicU64::new(0),
            media_speed: AtomicU64::new(0),
            media_eta: AtomicU64::new(u64::MAX),
        }
    }
}

pub struct Inner {
    pub store: Store,
    pub settings: RwLock<Settings>,
    list: Mutex<Vec<Download>>,
    active: Mutex<HashMap<DownloadId, Arc<Active>>>,
    tx: broadcast::Sender<EngineEvent>,
    pub global: Arc<RateLimiter>,
    dirty: AtomicBool,
    running_queues: Mutex<HashSet<String>>,
    last_minute: Mutex<String>,
    pub tools: crate::tools::Tools,
    /// Kuyruktan başlatılanlar (kuyruk bitişini izlemek için).
    started_by_queue: Mutex<HashMap<DownloadId, String>>,
}

impl Inner {
    pub fn emit(&self, e: EngineEvent) {
        let _ = self.tx.send(e);
    }

    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    /// Kaydı değiştirir ve değişikliği yayınlar.
    pub fn update(&self, id: &str, f: impl FnOnce(&mut Download)) {
        let view = {
            let mut list = self.list.lock();
            let Some(d) = list.iter_mut().find(|d| d.id == id) else { return };
            f(d);
            self.view_of(d)
        };
        self.mark_dirty();
        self.emit(EngineEvent::Changed(view));
    }

    pub fn with<T>(&self, id: &str, f: impl FnOnce(&Download) -> T) -> Option<T> {
        self.list.lock().iter().find(|d| d.id == id).map(f)
    }

    fn view_of(&self, d: &Download) -> DownloadView {
        let act = self.active.lock().get(&d.id).cloned();
        make_view(d, act.as_deref())
    }

    /// Hedef adın diskte ve listedeki diğer indirmelerde çakışmamasını sağlar.
    pub fn ensure_unique_name(&self, id: &str) {
        let mut list = self.list.lock();
        let Some(idx) = list.iter().position(|d| d.id == id) else { return };
        let dir = PathBuf::from(&list[idx].save_dir);
        let name = list[idx].file_name.clone();
        let others: HashSet<String> = list
            .iter()
            .filter(|o| o.id != id && o.save_dir == list[idx].save_dir && o.status != Status::Completed)
            .map(|o| o.file_name.clone())
            .collect();
        // Kendi yarım dosyası varsa ad değişmesin.
        if list[idx].part_path().exists() && !others.contains(&name) {
            return;
        }
        let action = self.settings.read().downloads.duplicate_action.clone();
        let unique = if action == "overwrite" && !others.contains(&name) && !dir.join(format!("{name}.idcpart")).exists() {
            name.clone()
        } else {
            crate::filename::unique_name(&dir, &name, &|n| others.contains(n))
        };
        if unique != name {
            list[idx].file_name = unique;
        }
    }
}

pub fn make_view(d: &Download, act: Option<&Active>) -> DownloadView {
    let (downloaded, speed, conns, size) = match act {
        Some(a) => {
            let media_total = a.media_total.load(Ordering::Relaxed);
            let size = if d.is_media() && media_total > 0 { Some(media_total) } else { d.size };
            let speed = if d.is_media() { a.media_speed.load(Ordering::Relaxed) } else { a.speed.load(Ordering::Relaxed) };
            (a.recv.load(Ordering::Relaxed), speed, a.conns.load(Ordering::Relaxed), size)
        }
        None => (d.downloaded, 0, 0, d.size),
    };
    let eta = match act {
        Some(a) if d.is_media() => Some(a.media_eta.load(Ordering::Relaxed)).filter(|v| *v != u64::MAX),
        _ => match (size, speed) {
            (Some(s), sp) if sp > 0 && s >= downloaded => Some((s - downloaded) / sp),
            _ => None,
        },
    };
    let (kind, thumb) = match &d.kind {
        DownloadKind::Http => ("http", None),
        DownloadKind::Media(m) => ("media", m.thumbnail.clone()),
    };
    DownloadView {
        id: d.id.clone(),
        url: d.request.url.clone(),
        file_name: d.file_name.clone(),
        save_dir: d.save_dir.clone(),
        category: d.category.clone(),
        description: d.description.clone(),
        size,
        downloaded,
        status: d.status,
        error: d.error.clone(),
        speed,
        eta,
        resumable: d.resumable,
        kind: kind.into(),
        connections: conns,
        max_connections: d.max_connections,
        speed_limit: d.speed_limit,
        queue_id: d.queue_id.clone(),
        added_at: d.added_at,
        last_try_at: d.last_try_at,
        completed_at: d.completed_at,
        mime: d.mime.clone(),
        thumbnail: thumb,
    }
}

/// Motorun dış yüzü. Ucuzca kopyalanır.
#[derive(Clone)]
pub struct Engine {
    pub inner: Arc<Inner>,
}

impl Engine {
    /// Motoru kurar, kayıtları yükler ve arka plan döngüsünü başlatır (tokio çalışma ortamında çağrılmalı).
    pub fn new(config_dir: PathBuf) -> Self {
        let store = Store::new(config_dir);
        let settings = store.load_settings();
        let mut list = store.load_downloads();
        // Kapanışta etkin kalanlar duraklatılmış sayılır; kuyruktakiler kuyrukta kalır.
        for d in &mut list {
            if d.status.is_active() {
                d.status = Status::Paused;
            }
        }
        let (tx, _) = broadcast::channel(1024);
        let global = Arc::new(RateLimiter::new(if settings.connection.speed_limit_enabled { settings.connection.speed_limit_kib * 1024 } else { 0 }));
        let inner = Arc::new(Inner {
            store,
            settings: RwLock::new(settings),
            list: Mutex::new(list),
            active: Mutex::new(HashMap::new()),
            tx,
            global,
            dirty: AtomicBool::new(false),
            running_queues: Mutex::new(HashSet::new()),
            last_minute: Mutex::new(String::new()),
            tools: crate::tools::Tools::new(),
            started_by_queue: Mutex::new(HashMap::new()),
        });
        let e = Engine { inner };
        e.spawn_loop();
        e
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EngineEvent> {
        self.inner.tx.subscribe()
    }

    pub fn settings(&self) -> Settings {
        self.inner.settings.read().clone()
    }

    pub fn is_first_run(&self) -> bool {
        !self.inner.settings.read().first_run_done
    }

    pub fn save_settings(&self, mut s: Settings) -> anyhow::Result<Settings> {
        s.sanitize();
        self.inner.store.save_settings(&s)?;
        let rate = if s.connection.speed_limit_enabled { s.connection.speed_limit_kib * 1024 } else { 0 };
        self.inner.global.set_rate(rate);
        *self.inner.settings.write() = s.clone();
        self.inner.emit(EngineEvent::Settings(s.clone()));
        Ok(s)
    }

    pub fn set_global_speed_limit(&self, kib: Option<u64>) -> anyhow::Result<()> {
        let mut s = self.settings();
        match kib {
            Some(k) if k > 0 => {
                s.connection.speed_limit_enabled = true;
                s.connection.speed_limit_kib = k;
            }
            _ => s.connection.speed_limit_enabled = false,
        }
        self.save_settings(s).map(|_| ())
    }

    pub fn list(&self) -> Vec<DownloadView> {
        let list = self.inner.list.lock();
        let act = self.inner.active.lock();
        list.iter().map(|d| make_view(d, act.get(&d.id).map(|a| a.as_ref()))).collect()
    }

    pub fn get(&self, id: &str) -> Option<Download> {
        self.inner.with(id, |d| d.clone())
    }

    pub fn detail(&self, id: &str) -> Option<DownloadDetail> {
        let d = self.get(id)?;
        let act = self.inner.active.lock().get(id).cloned();
        let view = make_view(&d, act.as_deref());
        let segments = match &act {
            Some(a) => a.segs.lock().clone(),
            None => d
                .segments
                .iter()
                .map(|s| SegmentView { start: s.start, end: s.end, pos: s.pos, active: false, speed: 0, info: if s.done() { "done".into() } else { String::new() } })
                .collect(),
        };
        let speed_history = act.map(|a| a.history.lock().iter().copied().collect()).unwrap_or_default();
        Some(DownloadDetail {
            view,
            referer: d.request.referer.clone(),
            final_url: d.final_url.clone(),
            etag: d.etag.clone(),
            last_modified: d.last_modified.clone(),
            segments,
            after: d.after,
            checksum: d.checksum.clone(),
            checksum_ok: d.checksum_ok,
            media: match &d.kind {
                DownloadKind::Media(m) => Some(m.clone()),
                _ => None,
            },
            speed_history,
        })
    }

    /// Yeni indirme ekler. Ad verilmemişse adresten geçici ad türetilir, sunucu cevabıyla düzeltilir.
    pub fn add(&self, req: AddRequest) -> anyhow::Result<DownloadId> {
        let settings = self.settings();
        let url = req.request.url.trim().to_string();
        if url.is_empty() {
            anyhow::bail!("adres boş");
        }
        let given = req.file_name.clone().map(|n| crate::filename::sanitize(&n)).filter(|n| !n.is_empty());
        let media_title = req.media.as_ref().and_then(|m| m.title.clone());
        let name = given.clone().unwrap_or_else(|| {
            if let Some(t) = &media_title {
                crate::filename::sanitize(t)
            } else {
                crate::filename::resolve(None, &url, &url, req.mime_hint.as_deref())
            }
        });
        let category = req.category.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| settings.category_for(&name));
        let save_dir = req.save_dir.clone().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| settings.dir_for_category(&category));
        let _ = std::fs::create_dir_all(&save_dir);
        let mut request = req.request.clone();
        request.url = url;
        let d = Download {
            id: new_id(),
            request,
            file_name: name,
            save_dir,
            category,
            description: req.description.clone().unwrap_or_default(),
            size: req.size_hint.filter(|s| *s > 0),
            mime: req.mime_hint.clone(),
            status: Status::Paused,
            max_connections: req.max_connections.unwrap_or(settings.downloads.default_connections).clamp(1, crate::MAX_CONNECTIONS),
            kind: match req.media.clone() {
                Some(m) => DownloadKind::Media(m),
                None => DownloadKind::Http,
            },
            queue_id: req.queue_id.clone(),
            added_at: now_ms(),
            checksum: req.checksum.clone(),
            silent: req.silent,
            auto_name: given.is_none() && req.media.is_none(),
            ..Default::default()
        };
        let id = d.id.clone();
        {
            let mut list = self.inner.list.lock();
            list.push(d);
        }
        if req.media.is_none() {
            self.inner.ensure_unique_name(&id);
        }
        if let Some(q) = &req.queue_id {
            let mut s = self.inner.settings.write();
            if let Some(queue) = s.queues.iter_mut().find(|x| &x.id == q) {
                queue.order.push(id.clone());
            }
        }
        self.inner.mark_dirty();
        let view = self.inner.with(&id, |d| make_view(d, None)).unwrap();
        self.inner.emit(EngineEvent::Changed(view));
        if req.start {
            self.start(std::slice::from_ref(&id));
        } else if req.queue_id.is_some() {
            self.inner.update(&id, |d| d.status = Status::Queued);
        }
        Ok(id)
    }

    /// Aynı ad ve boyuttaki yarım indirme (adres yenileme için).
    pub fn find_unfinished(&self, file_name: &str, size: Option<u64>) -> Option<DownloadId> {
        let list = self.inner.list.lock();
        list.iter()
            .find(|d| {
                d.status != Status::Completed
                    && !d.status.is_active()
                    && d.file_name == file_name
                    && match (d.size, size) {
                        (Some(a), Some(b)) => a == b,
                        _ => false,
                    }
            })
            .map(|d| d.id.clone())
    }

    pub fn active_count(&self) -> usize {
        self.inner.active.lock().len()
    }

    /// İndirmeleri başlatır. Genel eşzamanlı sınır doluysa sıraya alınır.
    pub fn start(&self, ids: &[DownloadId]) {
        for id in ids {
            let Some(status) = self.inner.with(id, |d| d.status) else { continue };
            if status.is_active() || status == Status::Completed {
                continue;
            }
            let max = self.inner.settings.read().downloads.max_simultaneous as usize;
            if max > 0 && self.active_count() >= max {
                self.inner.update(id, |d| {
                    d.status = Status::Queued;
                    d.error = None;
                });
                continue;
            }
            self.launch(id);
        }
    }

    fn launch(&self, id: &str) {
        let Some(d) = self.get(id) else { return };
        if self.inner.active.lock().contains_key(id) {
            return;
        }
        let act = Arc::new(Active::new(d.speed_limit, d.max_connections));
        self.inner.active.lock().insert(id.to_string(), act.clone());
        self.inner.update(id, |d| {
            d.status = Status::Connecting;
            d.error = None;
            d.last_try_at = Some(now_ms());
        });
        let inner = self.inner.clone();
        let engine = self.clone();
        let id = id.to_string();
        tokio::spawn(async move {
            let outcome = match &d.kind {
                DownloadKind::Http => task::run(inner.clone(), act.clone(), d).await,
                DownloadKind::Media(_) => crate::media::run(inner.clone(), act.clone(), d).await,
            };
            inner.active.lock().remove(&id);
            engine.on_finished(&id, outcome);
        });
    }

    fn on_finished(&self, id: &str, outcome: task::Outcome) {
        match outcome {
            task::Outcome::Completed => {
                let after = self.inner.with(id, |d| d.after).unwrap_or_default();
                self.inner.emit(EngineEvent::Completed(id.to_string()));
                if after != AfterAction::None {
                    self.inner.emit(EngineEvent::After(after));
                }
            }
            task::Outcome::Paused => {
                self.inner.update(id, |d| {
                    if d.status != Status::Queued {
                        d.status = Status::Paused;
                    }
                });
            }
            task::Outcome::Failed(msg) => {
                self.inner.update(id, |d| {
                    d.status = Status::Error;
                    d.error = Some(msg.clone());
                });
                self.inner.emit(EngineEvent::Failed(id.to_string(), msg));
            }
        }
        self.inner.mark_dirty();
        self.pump();
    }

    pub fn pause(&self, ids: &[DownloadId]) {
        for id in ids {
            if let Some(a) = self.inner.active.lock().get(id) {
                a.cancel.cancel();
            }
            self.inner.update(id, |d| {
                if d.status == Status::Queued {
                    d.status = Status::Paused;
                }
            });
            self.inner.started_by_queue.lock().remove(id);
        }
    }

    pub fn start_all(&self) {
        let ids: Vec<_> = self.inner.list.lock().iter().filter(|d| matches!(d.status, Status::Paused | Status::Error | Status::Queued)).map(|d| d.id.clone()).collect();
        self.start(&ids);
    }

    pub fn stop_all(&self) {
        let ids: Vec<_> = self.inner.list.lock().iter().filter(|d| d.status.is_active() || d.status == Status::Queued).map(|d| d.id.clone()).collect();
        self.inner.running_queues.lock().clear();
        self.pause(&ids);
    }

    pub async fn remove(&self, ids: &[DownloadId], delete_files: bool) {
        for id in ids {
            let act = self.inner.active.lock().get(id).cloned();
            if let Some(a) = act {
                a.cancel.cancel();
                // Görevin dosyaları bırakmasını bekle.
                for _ in 0..50 {
                    if !self.inner.active.lock().contains_key(id) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
        let removed: Vec<Download> = {
            let mut list = self.inner.list.lock();
            let (gone, keep): (Vec<_>, Vec<_>) = list.drain(..).partition(|d| ids.contains(&d.id));
            *list = keep;
            gone
        };
        {
            let mut s = self.inner.settings.write();
            for q in &mut s.queues {
                q.order.retain(|x| !ids.contains(x));
            }
        }
        for d in &removed {
            let _ = std::fs::remove_file(d.part_path());
            crate::media::cleanup_parts(d);
            if delete_files && d.status == Status::Completed {
                let _ = std::fs::remove_file(d.target_path());
            }
        }
        self.inner.mark_dirty();
        self.inner.emit(EngineEvent::Removed(ids.to_vec()));
    }

    pub async fn delete_completed(&self) {
        let ids: Vec<_> = self.inner.list.lock().iter().filter(|d| d.status == Status::Completed).map(|d| d.id.clone()).collect();
        self.remove(&ids, false).await;
    }

    /// Baştan indir (dosya silinir).
    pub async fn redownload(&self, id: &str) {
        self.pause(&[id.to_string()]);
        for _ in 0..50 {
            if !self.inner.active.lock().contains_key(id) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if let Some(d) = self.get(id) {
            let _ = std::fs::remove_file(d.part_path());
            if d.status == Status::Completed {
                let _ = std::fs::remove_file(d.target_path());
            }
        }
        self.inner.update(id, |d| {
            d.segments.clear();
            d.downloaded = 0;
            d.status = Status::Paused;
            d.completed_at = None;
            d.error = None;
            d.checksum_ok = None;
        });
        self.start(&[id.to_string()]);
    }

    pub fn update(&self, id: &str, p: DownloadPatch) -> anyhow::Result<()> {
        let active = self.inner.active.lock().get(id).cloned();
        let mut err = None;
        self.inner.update(id, |d| {
            if let Some(u) = p.url.clone().filter(|u| !u.trim().is_empty()) {
                // Adres yenileme: yarım parçalar korunur, sunucu aynı dosyayı vermeli.
                d.request.url = u.trim().to_string();
                d.final_url = None;
            }
            if let Some(r) = p.referer.clone() {
                d.request.referer = Some(r).filter(|s| !s.is_empty());
            }
            let rename = p.file_name.clone().map(|n| crate::filename::sanitize(&n)).filter(|n| !n.is_empty() && *n != d.file_name);
            let move_dir = p.save_dir.clone().filter(|s| !s.trim().is_empty() && *s != d.save_dir);
            if rename.is_some() || move_dir.is_some() {
                if active.is_some() {
                    err = Some("etkin indirmenin adı/klasörü değiştirilemez");
                } else {
                    let old_part = d.part_path();
                    let old_target = d.target_path();
                    if let Some(n) = rename.clone() {
                        d.file_name = n;
                    }
                    if let Some(dir) = move_dir.clone() {
                        let _ = std::fs::create_dir_all(&dir);
                        d.save_dir = dir;
                    }
                    if old_part.exists() {
                        let _ = move_file(&old_part, &d.part_path());
                    }
                    if d.status == Status::Completed && old_target.exists() {
                        let _ = move_file(&old_target, &d.target_path());
                    }
                    d.auto_name = false;
                }
            }
            if let Some(c) = p.category.clone() {
                d.category = c;
            }
            if let Some(desc) = p.description.clone() {
                d.description = desc;
            }
            if let Some(n) = p.max_connections {
                d.max_connections = n.clamp(1, crate::MAX_CONNECTIONS);
                if let Some(a) = &active {
                    a.max_conns.store(d.max_connections, Ordering::Relaxed);
                }
            }
            if let Some(l) = p.speed_limit {
                d.speed_limit = if l == 0 { None } else { Some(l) };
                if let Some(a) = &active {
                    a.limiter.set_rate(l);
                }
            }
            if p.clear_queue {
                d.queue_id = None;
            } else if let Some(q) = p.queue_id.clone() {
                d.queue_id = Some(q);
            }
            if let Some(a) = p.after {
                d.after = a;
            }
            if let Some(c) = p.checksum.clone() {
                d.checksum = if c.expected.trim().is_empty() { None } else { Some(c) };
            }
            if let Some(s) = p.show_complete_dialog {
                d.show_complete_dialog = Some(s);
            }
        });
        // Kuyruk sıra listesini eşitle.
        {
            let qid = self.inner.with(id, |d| d.queue_id.clone()).flatten();
            let mut s = self.inner.settings.write();
            for q in &mut s.queues {
                let inq = Some(&q.id) == qid.as_ref();
                let has = q.order.iter().any(|x| x == id);
                if inq && !has {
                    q.order.push(id.to_string());
                } else if !inq && has {
                    q.order.retain(|x| x != id);
                }
            }
        }
        match err {
            Some(e) => anyhow::bail!(e),
            None => Ok(()),
        }
    }

    pub fn set_silent(&self, id: &str, silent: bool) {
        self.inner.update(id, |d| d.silent = silent);
    }

    /// Tarayıcıdan yeni oturum bilgisiyle gelen aynı dosya: adresi ve çerezleri yenile.
    pub fn refresh_request(&self, id: &str, req: RequestInfo) {
        self.inner.update(id, |d| {
            d.request = req;
            d.final_url = None;
            d.error = None;
        });
    }

    // ---- Kuyruklar ----

    pub fn start_queue(&self, queue_id: &str) {
        let ids: Vec<DownloadId> = self
            .inner
            .list
            .lock()
            .iter()
            .filter(|d| d.queue_id.as_deref() == Some(queue_id) && matches!(d.status, Status::Paused | Status::Error | Status::Queued))
            .map(|d| d.id.clone())
            .collect();
        for id in &ids {
            self.inner.update(id, |d| {
                d.status = Status::Queued;
                d.error = None;
            });
        }
        self.inner.running_queues.lock().insert(queue_id.to_string());
        self.pump();
    }

    pub fn stop_queue(&self, queue_id: &str) {
        self.inner.running_queues.lock().remove(queue_id);
        let ids: Vec<DownloadId> = self
            .inner
            .list
            .lock()
            .iter()
            .filter(|d| d.queue_id.as_deref() == Some(queue_id) && (d.status.is_active() || d.status == Status::Queued))
            .map(|d| d.id.clone())
            .collect();
        self.pause(&ids);
    }

    pub fn running_queues(&self) -> Vec<String> {
        self.inner.running_queues.lock().iter().cloned().collect()
    }

    pub fn move_in_queue(&self, queue_id: &str, ids: &[DownloadId], delta: i32) {
        let mut s = self.inner.settings.write();
        let Some(q) = s.queues.iter_mut().find(|q| q.id == queue_id) else { return };
        let mut order = q.order.clone();
        let positions: Vec<usize> = ids.iter().filter_map(|id| order.iter().position(|x| x == id)).collect();
        let mut sorted = positions.clone();
        sorted.sort();
        if delta < 0 {
            for p in sorted {
                if p > 0 && !ids.contains(&order[p - 1]) {
                    order.swap(p, p - 1);
                }
            }
        } else {
            for p in sorted.into_iter().rev() {
                if p + 1 < order.len() && !ids.contains(&order[p + 1]) {
                    order.swap(p, p + 1);
                }
            }
        }
        q.order = order;
        drop(s);
        let s = self.settings();
        let _ = self.inner.store.save_settings(&s);
        self.inner.emit(EngineEvent::Settings(s));
    }

    /// Bekleyen sıraları işler: genel sıra ve çalışan kuyruklar.
    pub fn pump(&self) {
        let settings = self.settings();
        let running = self.inner.running_queues.lock().clone();
        let list = self.inner.list.lock().clone();
        let active_ids: HashSet<DownloadId> = self.inner.active.lock().keys().cloned().collect();
        let max_global = settings.downloads.max_simultaneous as usize;
        let mut total_active = active_ids.len();
        let mut to_start = vec![];

        for q in &settings.queues {
            if !running.contains(&q.id) {
                continue;
            }
            let in_q: Vec<&Download> = list.iter().filter(|d| d.queue_id.as_deref() == Some(q.id.as_str())).collect();
            let act = in_q.iter().filter(|d| active_ids.contains(&d.id)).count();
            let mut waiting: Vec<&Download> = in_q.iter().copied().filter(|d| d.status == Status::Queued && !active_ids.contains(&d.id)).collect();
            if act == 0 && waiting.is_empty() {
                // Kuyruk bitti.
                self.inner.running_queues.lock().remove(&q.id);
                self.inner.emit(EngineEvent::QueueFinished(q.id.clone()));
                if q.schedule.after_all != AfterAction::None {
                    self.inner.emit(EngineEvent::After(q.schedule.after_all));
                }
                continue;
            }
            waiting.sort_by_key(|d| (q.order.iter().position(|x| x == &d.id).unwrap_or(usize::MAX), d.added_at));
            let free = (q.max_concurrent as usize).saturating_sub(act);
            for d in waiting.into_iter().take(free) {
                if max_global > 0 && total_active >= max_global {
                    break;
                }
                total_active += 1;
                to_start.push((d.id.clone(), Some(q.id.clone())));
            }
        }
        // Kuyruksuz bekleyenler (genel sınır nedeniyle sıraya alınmış).
        if max_global == 0 || total_active < max_global {
            let mut waiting: Vec<&Download> = list.iter().filter(|d| d.status == Status::Queued && d.queue_id.is_none() && !active_ids.contains(&d.id)).collect();
            waiting.sort_by_key(|d| d.added_at);
            for d in waiting {
                if max_global > 0 && total_active >= max_global {
                    break;
                }
                total_active += 1;
                to_start.push((d.id.clone(), None));
            }
        }
        for (id, q) in to_start {
            if let Some(q) = q {
                self.inner.started_by_queue.lock().insert(id.clone(), q);
            }
            self.launch(&id);
        }
    }

    // ---- Arka plan döngüsü ----

    fn spawn_loop(&self) {
        let engine = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(250));
            let mut n: u64 = 0;
            loop {
                tick.tick().await;
                n += 1;
                engine.progress_tick(n.is_multiple_of(4));
                if n.is_multiple_of(8) {
                    engine.save_if_dirty();
                }
                if n.is_multiple_of(4) {
                    engine.check_schedules();
                    engine.pump();
                }
            }
        });
    }

    fn progress_tick(&self, second: bool) {
        let actives: Vec<(DownloadId, Arc<Active>)> = self.inner.active.lock().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        if actives.is_empty() {
            return;
        }
        let mut ticks = Vec::with_capacity(actives.len());
        for (id, a) in actives {
            let recv = a.recv.load(Ordering::Relaxed);
            let last = a.last_recv.swap(recv, Ordering::Relaxed);
            let inst = recv.saturating_sub(last) * 4;
            let prev = a.speed.load(Ordering::Relaxed);
            // Üstel yumuşatma: ani dalgalanmalar ekranı oynatmasın.
            let sp = if prev == 0 { inst } else { ((prev as f64) * 0.7 + (inst as f64) * 0.3) as u64 };
            a.speed.store(sp, Ordering::Relaxed);
            if second {
                let mut h = a.history.lock();
                if h.len() >= 120 {
                    h.pop_front();
                }
                h.push_back(if self.inner.with(&id, |d| d.is_media()).unwrap_or(false) { a.media_speed.load(Ordering::Relaxed) } else { sp });
            }
            let Some(d) = self.get(&id) else { continue };
            // Veri gelmeye başladıysa durum "indiriliyor".
            if d.status == Status::Connecting && recv > d.downloaded {
                self.inner.update(&id, |d| d.status = Status::Downloading);
            }
            let view = make_view(&d, Some(&a));
            ticks.push(ProgressTick {
                id: id.clone(),
                status: if d.status == Status::Connecting && recv > d.downloaded { Status::Downloading } else { d.status },
                size: view.size,
                downloaded: view.downloaded,
                speed: view.speed,
                eta: view.eta,
                connections: view.connections,
                segments: a.segs.lock().clone(),
            });
        }
        // Liste görünümü için indirilen miktarı kayda da yansıt.
        {
            let mut list = self.inner.list.lock();
            for t in &ticks {
                if let Some(d) = list.iter_mut().find(|d| d.id == t.id) {
                    d.downloaded = t.downloaded;
                }
            }
        }
        self.inner.emit(EngineEvent::Progress(ticks));
    }

    pub fn save_if_dirty(&self) {
        if self.inner.dirty.swap(false, Ordering::Relaxed) {
            let list = self.inner.list.lock().clone();
            if let Err(e) = self.inner.store.save_downloads(&list) {
                tracing::error!("liste kaydedilemedi: {e}");
                self.inner.mark_dirty();
            }
            let s = self.settings();
            let _ = self.inner.store.save_settings(&s);
        }
    }

    /// Kapanışta: tüm görevleri durdur, kaydet.
    pub async fn shutdown(&self) {
        let actives: Vec<Arc<Active>> = self.inner.active.lock().values().cloned().collect();
        for a in &actives {
            a.cancel.cancel();
        }
        for _ in 0..40 {
            if self.inner.active.lock().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        self.inner.mark_dirty();
        self.save_if_dirty();
    }

    fn check_schedules(&self) {
        let now = chrono::Local::now();
        let minute = now.format("%Y-%m-%d %H:%M").to_string();
        {
            let mut last = self.inner.last_minute.lock();
            if *last == minute {
                return;
            }
            *last = minute;
        }
        let hm = now.format("%H:%M").to_string();
        let date = now.format("%Y-%m-%d").to_string();
        let weekday = chrono::Datelike::weekday(&now).num_days_from_monday() as u8;
        let settings = self.settings();
        for q in &settings.queues {
            let s = &q.schedule;
            if !s.enabled {
                continue;
            }
            let day_ok = match &s.once_date {
                Some(d) if !d.is_empty() => d == &date,
                _ => s.days.is_empty() || s.days.contains(&weekday),
            };
            if day_ok && s.start_time == hm {
                tracing::info!("zamanlayıcı: {} kuyruğu başlıyor", q.name);
                self.start_queue(&q.id);
            }
            if s.stop_enabled && s.stop_time == hm {
                tracing::info!("zamanlayıcı: {} kuyruğu durduruluyor", q.name);
                self.stop_queue(&q.id);
            }
        }
    }

    // ---- Yardımcı API ----

    pub fn save_queue(&self, q: crate::settings::Queue) -> anyhow::Result<()> {
        let mut s = self.settings();
        if let Some(x) = s.queues.iter_mut().find(|x| x.id == q.id) {
            *x = q;
        } else {
            s.queues.push(q);
        }
        self.save_settings(s).map(|_| ())
    }

    pub fn delete_queue(&self, id: &str) -> anyhow::Result<()> {
        if id == MAIN_QUEUE {
            anyhow::bail!("ana kuyruk silinemez");
        }
        self.stop_queue(id);
        let mut s = self.settings();
        s.queues.retain(|q| q.id != id);
        let ids: Vec<_> = self.inner.list.lock().iter().filter(|d| d.queue_id.as_deref() == Some(id)).map(|d| d.id.clone()).collect();
        for d in ids {
            self.inner.update(&d, |d| {
                d.queue_id = None;
                if d.status == Status::Queued {
                    d.status = Status::Paused;
                }
            });
        }
        self.save_settings(s).map(|_| ())
    }

    pub async fn probe(&self, req: &RequestInfo) -> anyhow::Result<ProbeResult> {
        let s = self.settings();
        crate::http::probe(req, &s).await
    }

    pub fn tools(&self) -> &crate::tools::Tools {
        &self.inner.tools
    }

    /// Hata kaydını kullanıcıya gösterilecek biçime getirir.
    pub fn error_of(&self, id: &str) -> Option<String> {
        self.inner.with(id, |d| d.error.clone()).flatten()
    }

    pub fn fail(&self, id: &str, code: &str, detail: &str) {
        self.inner.update(id, |d| {
            d.status = Status::Error;
            d.error = Some(err_text(code, detail));
        });
    }
}

pub fn mark_origin_pub(path: &Path, url: &str, referer: Option<&str>) {
    post::mark_origin(path, url, referer);
}

/// Aynı dosya sisteminde yeniden adlandırır, değilse kopyalayıp siler.
pub fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        // Farklı disk/bölüm: kopyala ve sil.
        Err(e) if e.raw_os_error() == Some(18) || e.raw_os_error() == Some(17) || e.kind() == std::io::ErrorKind::CrossesDevices => {
            std::fs::copy(from, to)?;
            std::fs::remove_file(from)
        }
        // Windows'ta antivirüs kısa süre kilitleyebilir.
        Err(_) => crate::store::replace_file(from, to),
    }
}
