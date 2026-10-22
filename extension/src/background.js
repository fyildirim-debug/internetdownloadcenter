// IDC arka plan betiği: indirme yakalama, başlık gözlemi, medya algılama, bağlam menüleri.
// Chromium'da servis çalışanı, Firefox'ta olay sayfası olarak çalışır. Servis çalışanı her an
// uykuya geçebileceği için bütün dinleyiciler en üst düzeyde eşzamanlı kaydedilir ve kalıcı
// olması gereken durum storage.session'da tutulur.

const EXT_VERSION = api.runtime.getManifest().version;
const HELLO_INTERVAL_MIN = 5;
const APP_FRESH_MS = 2 * 60 * 1000; // Firefox engelleme yolu için uygulamanın taze sayılma süresi
const NATIVE_TIMEOUT_MS = 15000;
const PROBE_TIMEOUT_MS = 120000;
const KEY_WINDOW_MS = 3000;
const LOG_TTL_MS = 2 * 60 * 1000;
const LOG_MAX = 300;
const PENDING_MAX_AGE_MS = 30000;

// ---------------------------------------------------------------- durum

const state = {
  config: null, // ExtConfig
  appOk: false,
  appVersion: '',
  lastHelloOk: 0,
  localEnabled: true,
  disabledSites: [],
  browser: IS_FIREFOX ? 'firefox' : 'chromium',
};

// Oturum boyunca kalıcı küçük kümeler (storage.session'a yansıtılır).
const session = {
  handled: [], // işlediğimiz tarayıcı indirme kimlikleri
  passthrough: {}, // url -> bitiş zamanı: tarayıcıya bırakılacak adresler
  pending: {}, // indirme kimliği -> duraklatılma zamanı
  keys: null, // { alt, shift, ctrl, meta, t }
  media: {}, // tabId -> MediaEntry[]
};

const reqLog = new Map(); // url -> { reqHeaders, resHeaders, method, tabId, statusCode, t, postData }
const reqIdUrl = new Map(); // requestId -> ilk url (yönlendirme zinciri için)
const filenameById = new Map(); // Chromium onDeterminingFilename'den gelen adlar

let saveTimer = null;
function persistSession() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    sessionStore.set({ idcSession: session }).catch(() => {});
  }, 150);
}

const ready = (async () => {
  try {
    const loc = await api.storage.local.get(['config', 'appOk', 'appVersion', 'lastHelloOk', 'localEnabled', 'disabledSites']);
    if (loc.config) state.config = loc.config;
    state.appOk = !!loc.appOk;
    state.appVersion = loc.appVersion || '';
    state.lastHelloOk = loc.lastHelloOk || 0;
    state.localEnabled = loc.localEnabled !== false;
    state.disabledSites = Array.isArray(loc.disabledSites) ? loc.disabledSites : [];
  } catch (_) {
    /* ilk çalışma */
  }
  try {
    const s = await sessionStore.get('idcSession');
    if (s && s.idcSession) Object.assign(session, s.idcSession);
  } catch (_) {
    /* boş oturum */
  }
  state.browser = await detectBrowser();
  updateActionIcon();
  // Uyandığımızda yarım kalmış (duraklatılmış ama kararı verilmemiş) indirmeleri sürdür.
  resumeStalePending();
})();

function saveLocal() {
  return api.storage.local
    .set({
      config: state.config,
      appOk: state.appOk,
      appVersion: state.appVersion,
      lastHelloOk: state.lastHelloOk,
      localEnabled: state.localEnabled,
      disabledSites: state.disabledSites,
    })
    .catch(() => {});
}

function captureActive() {
  return !!(state.config && state.config.enabled && state.localEnabled && state.appOk);
}

function localRules() {
  return { enabled: state.localEnabled, disabledSites: state.disabledSites };
}

// ---------------------------------------------------------------- simge ve rozet

function updateActionIcon() {
  const active = state.localEnabled && (!state.config || state.config.enabled);
  const suffix = active ? '' : '-gray';
  const path = {};
  for (const s of [16, 32, 48, 128]) path[s] = `icons/icon${suffix}-${s}.png`;
  try {
    api.action.setIcon({ path }).catch(() => {});
  } catch (_) {
    /* yok say */
  }
  if (!state.appOk) {
    api.action.setBadgeBackgroundColor({ color: '#e8710a' }).catch(() => {});
    api.action.setBadgeText({ text: '!' }).catch(() => {});
    api.action.setTitle({ title: t('extName') + ' — ' + t('appNotFound') }).catch(() => {});
  } else {
    api.action.setBadgeText({ text: '' }).catch(() => {});
    api.action.setTitle({ title: t('extName') }).catch(() => {});
    // Sekme rozetleri medya sayısını gösterir.
    for (const tabId of Object.keys(session.media)) updateTabBadge(Number(tabId));
  }
}

function updateTabBadge(tabId) {
  if (!state.appOk || tabId < 0) return;
  const n = (session.media[tabId] || []).length;
  api.action.setBadgeBackgroundColor({ color: '#1565c0', tabId }).catch(() => {});
  api.action.setBadgeText({ text: n ? String(n) : '', tabId }).catch(() => {});
}

// ---------------------------------------------------------------- uygulama bağlantısı

function setAppOk(ok) {
  const changed = ok !== state.appOk;
  state.appOk = ok;
  if (ok) state.lastHelloOk = Date.now();
  if (changed) updateActionIcon();
  saveLocal();
}

async function send(msg, timeout = NATIVE_TIMEOUT_MS) {
  try {
    const res = await nativeSend(msg, timeout);
    setAppOk(true);
    return res;
  } catch (e) {
    if (e.kind === 'missing') setAppOk(false);
    // Köprü kurulu ama uygulama kapalı: köprü indirme gelince uygulamayı kendisi başlatır,
    // bu yüzden yakalama açık kalır (önbellekteki ayarlarla).
    else if (e.kind === 'app' && /app_unavailable/.test(e.message || '')) setAppOk(!!state.config);
    throw e;
  }
}

async function hello() {
  await ready;
  try {
    const res = await send({ type: 'hello', version: EXT_VERSION, browser: state.browser }, NATIVE_TIMEOUT_MS);
    if (res.type === 'config' && res.config) {
      state.config = res.config;
      state.appVersion = res.appVersion || '';
      state.lastHelloOk = Date.now();
      await saveLocal();
      updateActionIcon();
    }
    return true;
  } catch (_) {
    return false;
  }
}

// ---------------------------------------------------------------- yardımcılar

function prunePassthrough() {
  const now = Date.now();
  for (const [u, until] of Object.entries(session.passthrough)) if (until < now) delete session.passthrough[u];
}

function markPassthrough(url, ms = 60000) {
  if (!url) return;
  prunePassthrough();
  session.passthrough[url] = Date.now() + ms;
  persistSession();
}

function isPassthrough(url) {
  if (!url) return false;
  const until = session.passthrough[url];
  return !!until && until > Date.now();
}

function markHandled(id) {
  if (session.handled.includes(id)) return;
  session.handled.push(id);
  if (session.handled.length > 500) session.handled.splice(0, session.handled.length - 500);
  persistSession();
}

function recentKeys() {
  const k = session.keys;
  const cfg = state.config || {};
  if (!k || Date.now() - k.t > KEY_WINDOW_MS) return { bypass: false, force: false };
  const pressed = (name) => {
    switch (name) {
      case 'alt':
        return !!k.alt;
      case 'shift':
        return !!k.shift;
      case 'ctrl':
        return !!(k.ctrl || k.meta);
      default:
        return false;
    }
  };
  return { bypass: pressed(cfg.bypassKey || 'alt'), force: pressed(cfg.forceKey || 'none') };
}

function logEntry(url) {
  let e = reqLog.get(url);
  if (!e) {
    e = { t: Date.now() };
    reqLog.set(url, e);
    if (reqLog.size > LOG_MAX) pruneLog();
  }
  e.t = Date.now();
  return e;
}

function pruneLog() {
  const now = Date.now();
  for (const [k, v] of reqLog) if (now - v.t > LOG_TTL_MS) reqLog.delete(k);
  while (reqLog.size > LOG_MAX) reqLog.delete(reqLog.keys().next().value);
  if (reqIdUrl.size > LOG_MAX * 2) {
    const drop = reqIdUrl.size - LOG_MAX;
    let i = 0;
    for (const k of reqIdUrl.keys()) {
      if (i++ >= drop) break;
      reqIdUrl.delete(k);
    }
  }
}

function findLog(...urls) {
  for (const u of urls) if (u && reqLog.has(u)) return reqLog.get(u);
  return null;
}

/** İstek gövdesini (form) uygulamaya gönderilebilir metne çevirir. */
function encodeRequestBody(body) {
  if (!body) return null;
  if (body.formData) {
    const p = new URLSearchParams();
    for (const [k, vals] of Object.entries(body.formData)) for (const v of vals) p.append(k, v);
    return p.toString();
  }
  if (body.raw && body.raw.length && body.raw[0].bytes) {
    try {
      return new TextDecoder().decode(body.raw[0].bytes);
    } catch (_) {
      return null;
    }
  }
  return null;
}

async function getTab(tabId) {
  if (tabId == null || tabId < 0) return null;
  try {
    return await api.tabs.get(tabId);
  } catch (_) {
    return null;
  }
}

function siteOf(url) {
  try {
    const u = new URL(url);
    return u.protocol + '//' + u.hostname;
  } catch (_) {
    return null;
  }
}

/** HttpOnly ve bölümlenmiş (CHIPS) çerezler dahil, adres için tarayıcı oturum çerezleri. */
async function getCookies(url, tab, topUrl) {
  if (!url || !/^https?:/i.test(url)) return [];
  const all = [];
  const add = (list) => {
    for (const c of list || []) all.push(c);
  };
  if (IS_FIREFOX) {
    const opts = { url, firstPartyDomain: null };
    if (tab && tab.cookieStoreId) opts.storeId = tab.cookieStoreId;
    try {
      add(await api.cookies.getAll({ ...opts, partitionKey: {} }));
    } catch (_) {
      try {
        add(await api.cookies.getAll(opts));
      } catch (_) {
        try {
          add(await api.cookies.getAll({ url }));
        } catch (_) {
          /* çerez yok */
        }
      }
    }
  } else {
    try {
      add(await api.cookies.getAll({ url }));
    } catch (_) {
      /* çerez yok */
    }
    const top = siteOf(topUrl || (tab && tab.url) || url);
    if (top) {
      try {
        add(await api.cookies.getAll({ url, partitionKey: { topLevelSite: top } }));
      } catch (_) {
        /* CHIPS desteklenmiyor */
      }
    }
  }
  const seen = new Set();
  const out = [];
  for (const c of all) {
    const k = c.name + '\u0000' + c.domain + '\u0000' + c.path + '\u0000' + JSON.stringify(c.partitionKey || null);
    if (seen.has(k)) continue;
    seen.add(k);
    out.push(toCookieSpec(c));
  }
  return out;
}

/** Yakalanan indirme için DownloadCapture mesajını kurar. */
async function buildCapture(o) {
  const tab = await getTab(o.tabId);
  const log = findLog(o.finalUrl, o.url);
  const headers = log && log.reqHeaders ? filterRequestHeaders(log.reqHeaders) : [];
  const hasHeader = (n) => headers.some(([k]) => k.toLowerCase() === n);
  const referrer = o.referrer || (log && log.referer) || (tab && tab.url) || null;
  if (referrer && !hasHeader('referer') && /^https?:/i.test(referrer)) headers.push(['Referer', referrer]);
  const uaHeader = headers.find(([k]) => k.toLowerCase() === 'user-agent');
  const pageUrl = o.pageUrl || (tab && tab.url) || referrer;
  let cookies = await getCookies(o.url, tab, pageUrl);
  if (o.finalUrl && o.finalUrl !== o.url && hostOf(o.finalUrl) !== hostOf(o.url)) {
    cookies = cookies.concat(await getCookies(o.finalUrl, tab, pageUrl));
  }
  return {
    url: o.url,
    finalUrl: o.finalUrl && o.finalUrl !== o.url ? o.finalUrl : null,
    referrer,
    pageUrl: pageUrl || null,
    fileName: o.fileName || null,
    fileSize: o.size > 0 ? o.size : null,
    mime: o.mime || null,
    headers,
    cookies,
    userAgent: uaHeader ? uaHeader[1] : navigator.userAgent,
    method: (log && log.method) || 'GET',
    postData: (log && log.postData) || null,
    forced: !!o.forced,
    browserDownloadId: o.browserDownloadId != null ? o.browserDownloadId : null,
    browser: state.browser,
    tabTitle: (tab && tab.title) || null,
  };
}

async function notify(title, message) {
  try {
    if (!api.notifications) return;
    await api.notifications.create('idc-' + Date.now(), {
      type: 'basic',
      iconUrl: api.runtime.getURL('icons/icon-128.png'),
      title,
      message,
    });
  } catch (_) {
    /* bildirim izni yok */
  }
}

// ---------------------------------------------------------------- başlık gözlemi

const ALL_URLS = { urls: ['http://*/*', 'https://*/*'] };

api.webRequest.onBeforeRequest.addListener(
  (d) => {
    if (d.method && d.method !== 'GET' && (d.type === 'main_frame' || d.type === 'sub_frame')) {
      const e = logEntry(d.url);
      e.method = d.method;
      e.postData = encodeRequestBody(d.requestBody);
      e.tabId = d.tabId;
    }
  },
  { urls: ALL_URLS.urls, types: ['main_frame', 'sub_frame'] },
  ['requestBody']
);

// onSendHeaders, tarayıcının gerçekten gönderdiği başlıkları (Cookie, Referer, sec-ch-ua*) verir.
api.webRequest.onSendHeaders.addListener(
  (d) => {
    const e = logEntry(d.url);
    e.reqHeaders = d.requestHeaders;
    e.method = d.method;
    e.tabId = d.tabId;
    e.type = d.type;
    const ref = (d.requestHeaders || []).find((h) => h.name.toLowerCase() === 'referer');
    if (ref) e.referer = ref.value;
    if (!reqIdUrl.has(d.requestId)) reqIdUrl.set(d.requestId, d.url);
  },
  ALL_URLS,
  IS_FIREFOX ? ['requestHeaders'] : ['requestHeaders', 'extraHeaders']
);

api.webRequest.onBeforeRedirect.addListener(
  (d) => {
    // Yönlendirme hedefi için ilk isteğin başlıkları da kullanılabilsin.
    const first = reqLog.get(reqIdUrl.get(d.requestId) || d.url);
    if (first && d.redirectUrl) {
      const e = logEntry(d.redirectUrl);
      if (!e.reqHeaders) e.reqHeaders = first.reqHeaders;
      if (!e.referer) e.referer = first.referer;
      e.method = first.method;
      e.tabId = d.tabId;
    }
  },
  ALL_URLS
);

api.webRequest.onHeadersReceived.addListener(
  (d) => {
    const h = headerMap(d.responseHeaders);
    const e = logEntry(d.url);
    e.resHeaders = h;
    e.statusCode = d.statusCode;
    e.tabId = d.tabId;
    if (d.tabId >= 0) detectMedia(d, h);
  },
  ALL_URLS,
  IS_FIREFOX ? ['responseHeaders'] : ['responseHeaders', 'extraHeaders']
);

// ---------------------------------------------------------------- medya algılama

function detectMedia(d, h) {
  const m = classifyMedia(d.url, h, d.statusCode);
  if (!m) return;
  const list = session.media[d.tabId] || (session.media[d.tabId] = []);
  const key = d.url.split('?')[0];
  if (list.some((x) => x.key === key)) return;
  const log = reqLog.get(d.url);
  const cdName = filenameFromDisposition(h['content-disposition']);
  list.push({
    key,
    url: d.url,
    kind: m.kind,
    ext: m.ext,
    size: m.size || 0,
    mime: mimeBase(h['content-type']) || null,
    fileName: cdName || nameFromUrl(d.url) || 'media.' + m.ext,
    referer: (log && log.referer) || null,
    frameId: d.frameId,
    t: Date.now(),
  });
  if (list.length > 50) list.splice(0, list.length - 50);
  persistSession();
  updateTabBadge(d.tabId);
}

function clearTabMedia(tabId) {
  if (session.media[tabId]) {
    delete session.media[tabId];
    persistSession();
  }
  updateTabBadge(tabId);
}

api.tabs.onUpdated.addListener((tabId, info) => {
  if (info.url) clearTabMedia(tabId);
});

api.tabs.onRemoved.addListener((tabId) => {
  if (session.media[tabId]) {
    delete session.media[tabId];
    persistSession();
  }
});

// ---------------------------------------------------------------- indirme yakalama (downloads API)

if (!IS_FIREFOX && api.downloads.onDeterminingFilename) {
  // Chrome bütün dinleyicilerin suggest() çağırmasını bekler; hemen varsayılanla devam ediyoruz.
  api.downloads.onDeterminingFilename.addListener((item, suggest) => {
    if (item && item.filename) filenameById.set(item.id, item.filename);
    if (filenameById.size > 200) filenameById.delete(filenameById.keys().next().value);
    suggest();
  });
}

api.downloads.onCreated.addListener((item) => {
  handleCreated(item).catch((e) => console.warn('IDC onCreated', e));
});

async function waitForFilename(id) {
  for (let i = 0; i < 8; i++) {
    if (filenameById.has(id)) return baseName(filenameById.get(id));
    await new Promise((r) => setTimeout(r, 50));
  }
  return '';
}

async function currentItem(id) {
  try {
    const [it] = await api.downloads.search({ id });
    return it || null;
  } catch (_) {
    return null;
  }
}

async function handleCreated(item) {
  await ready;
  if (!item || session.handled.includes(item.id)) return;
  if (item.byExtensionId && item.byExtensionId === api.runtime.id) return;
  if (item.state && item.state !== 'in_progress') return;
  const url = item.url || '';
  const finalUrl = item.finalUrl || url;
  if (!/^https?:/i.test(url) || !/^https?:/i.test(finalUrl)) return; // blob:, data:, filesystem:, file:
  if (isPassthrough(url) || isPassthrough(finalUrl)) return;
  if (!captureActive()) return;

  const keys = recentKeys();
  const log = findLog(finalUrl, url);
  const resH = (log && log.resHeaders) || {};
  let fileName = baseName(item.filename);
  if (!fileName) fileName = filenameFromDisposition(resH['content-disposition']);
  const mime = item.mime || mimeBase(resH['content-type']);
  const size = item.totalBytes > 0 ? item.totalBytes : Number(resH['content-length'] || 0);
  if (!fileName && !IS_FIREFOX) fileName = await waitForFilename(item.id);
  if (!fileName) fileName = nameFromUrl(finalUrl) || nameFromUrl(url);
  if (fileName && !extOf(fileName)) {
    const ext = extFromMime(mime);
    if (ext) fileName += '.' + ext;
  }
  const method = (log && log.method) || 'GET';
  const decision = shouldCapture({ url: finalUrl, fileName, mime, size, forced: keys.force, bypass: keys.bypass, method }, state.config, localRules());
  if (!decision.ok) {
    if (decision.why === 'bypass') markHandled(item.id);
    return;
  }

  markHandled(item.id);
  try {
    await api.downloads.pause(item.id);
  } catch (_) {
    /* duraklatılamadıysa da devam: iptal edebiliriz */
  }
  // Duraklatma sırasında tamamlanmış olabilir (çok küçük dosya); o zaman dokunma.
  const now = await currentItem(item.id);
  if (now && now.state === 'complete') return;
  session.pending[item.id] = Date.now();
  persistSession();

  let res = null;
  try {
    const capture = await buildCapture({
      url,
      finalUrl,
      referrer: item.referrer,
      fileName,
      size,
      mime,
      tabId: (log && log.tabId) != null ? log.tabId : -1,
      forced: keys.force,
      browserDownloadId: item.id,
    });
    res = await send({ type: 'download', ...capture }, NATIVE_TIMEOUT_MS);
  } catch (_) {
    res = null;
  }

  delete session.pending[item.id];
  persistSession();
  if (res && res.type === 'accepted') {
    try {
      await api.downloads.cancel(item.id);
    } catch (_) {
      /* zaten bitmiş olabilir */
    }
    try {
      await api.downloads.erase({ id: item.id });
    } catch (_) {
      /* yok say */
    }
  } else {
    try {
      await api.downloads.resume(item.id);
    } catch (_) {
      /* yok say */
    }
    if (res && res.type === 'rejected' && res.reason === 'blocked') {
      notify(t('extName'), t('blockedFallback'));
    }
  }
}

async function resumeStalePending() {
  const now = Date.now();
  let changed = false;
  for (const [id, since] of Object.entries(session.pending)) {
    if (now - since < PENDING_MAX_AGE_MS) continue;
    delete session.pending[id];
    changed = true;
    try {
      const it = await currentItem(Number(id));
      if (it && it.state === 'in_progress' && it.paused) await api.downloads.resume(Number(id));
    } catch (_) {
      /* yok say */
    }
  }
  if (changed) persistSession();
}

// ---------------------------------------------------------------- Firefox: indirme hiç başlamadan yakalama

if (IS_FIREFOX) {
  api.webRequest.onHeadersReceived.addListener(
    firefoxIntercept,
    { urls: ALL_URLS.urls, types: ['main_frame', 'sub_frame'] },
    ['blocking', 'responseHeaders']
  );
}

function firefoxIntercept(d) {
  // Karar önbellekteki ayarlardan beklemeden verilir; uygulama yakın zamanda cevap vermediyse
  // bu yol kullanılmaz, downloads.onCreated yolu devreye girer.
  if (!state.config || !captureActive()) return {};
  if (Date.now() - state.lastHelloOk > APP_FRESH_MS) return {};
  if (d.statusCode < 200 || d.statusCode >= 300) return {};
  if (d.method && d.method !== 'GET') return {};
  if (isPassthrough(d.url)) return {};
  const h = headerMap(d.responseHeaders);
  const cd = h['content-disposition'] || '';
  const ct = mimeBase(h['content-type']);
  const attachment = /^\s*attachment/i.test(cd);
  if (!attachment) {
    // Tarayıcının kendisinin göstereceği içerikleri (sayfa, düz metin, resim) yakalama.
    if (!ct || ct.startsWith('text/') || ct.includes('html') || ct.includes('xml') || ct.includes('json') || ct.startsWith('image/')) return {};
  }
  const keys = recentKeys();
  let fileName = filenameFromDisposition(cd) || nameFromUrl(d.url);
  if (fileName && !extOf(fileName)) {
    const ext = extFromMime(ct);
    if (ext) fileName += '.' + ext;
  }
  const size = Number(h['content-length'] || 0);
  const decision = shouldCapture({ url: d.url, fileName, mime: ct, size, forced: keys.force, bypass: keys.bypass, method: d.method }, state.config, localRules());
  if (!decision.ok) {
    if (decision.why === 'bypass') markPassthrough(d.url, 30000);
    return {};
  }
  // İstek iptal edilir; uygulama reddederse dosya tarayıcıyla yeniden indirilir.
  markPassthrough(d.url, 60000);
  handOffAfterCancel(d, fileName, ct, size, keys.force);
  return { cancel: true };
}

async function handOffAfterCancel(d, fileName, mime, size, forced) {
  let res = null;
  try {
    const log = reqLog.get(d.url);
    const capture = await buildCapture({
      url: d.url,
      referrer: (log && log.referer) || d.originUrl || null,
      pageUrl: d.type === 'sub_frame' ? d.documentUrl || d.originUrl : d.originUrl,
      fileName,
      size,
      mime,
      tabId: d.tabId,
      forced,
    });
    res = await send({ type: 'download', ...capture }, NATIVE_TIMEOUT_MS);
  } catch (_) {
    res = null;
  }
  if (d.type === 'main_frame') restoreTabAfterCancel(d);
  if (!res || res.type !== 'accepted') {
    // Tarayıcıya geri ver: oturum çerezleriyle kendisi indirsin.
    try {
      await api.downloads.download({ url: d.url, filename: fileName || undefined, conflictAction: 'uniquify' });
    } catch (_) {
      try {
        await api.downloads.download({ url: d.url });
      } catch (_) {
        /* yok say */
      }
    }
    if (res && res.type === 'rejected' && res.reason === 'blocked') notify(t('extName'), t('blockedFallback'));
  }
}

/** İptal edilen ana çerçeve isteği sekmede "engellendi" sayfası bıraktıysa geri al. */
function restoreTabAfterCancel(d) {
  setTimeout(async () => {
    const tab = await getTab(d.tabId);
    if (!tab || tab.url !== d.url) return;
    try {
      await api.tabs.goBack(d.tabId);
    } catch (_) {
      if (tab.openerTabId != null) api.tabs.remove(d.tabId).catch(() => {});
    }
  }, 600);
}

// ---------------------------------------------------------------- bağlam menüleri

function createMenus() {
  if (!menusApi) return;
  menusApi.removeAll(() => {
    void api.runtime.lastError;
    const mk = (id, title, contexts) => {
      try {
        menusApi.create({ id, title, contexts }, () => void api.runtime.lastError);
      } catch (_) {
        /* yok say */
      }
    };
    mk('idc-link', t('menuDownloadLink'), ['link']);
    mk('idc-selection', t('menuDownloadSelection'), ['selection']);
    mk('idc-all', t('menuDownloadAll'), ['page', 'frame']);
    mk('idc-video', t('menuDownloadVideo'), ['video', 'audio']);
    mk('idc-image', t('menuDownloadImage'), ['image']);
  });
}

api.runtime.onInstalled.addListener(() => {
  createMenus();
  hello();
  api.alarms.create('idc-hello', { periodInMinutes: HELLO_INTERVAL_MIN });
});

api.runtime.onStartup.addListener(() => {
  createMenus();
  hello();
  api.alarms.create('idc-hello', { periodInMinutes: HELLO_INTERVAL_MIN });
});

api.alarms.onAlarm.addListener((a) => {
  if (a.name === 'idc-hello') {
    hello();
    resumeStalePending();
  }
});

if (menusApi) {
  menusApi.onClicked.addListener((info, tab) => {
    onMenu(info, tab).catch((e) => console.warn('IDC menu', e));
  });
}

async function forcedDownload(url, referrer, tabId, fileName) {
  if (!/^https?:/i.test(url || '')) {
    notify(t('extName'), t('unsupportedUrl'));
    return { type: 'rejected', reason: 'unsupported' };
  }
  const capture = await buildCapture({ url, referrer, tabId, fileName, forced: true });
  return send({ type: 'download', ...capture });
}

async function mediaRequest(pageUrl, extra, tabId) {
  const tab = await getTab(tabId);
  const log = findLog(extra.mediaUrl, pageUrl);
  const headers = log && log.reqHeaders ? filterRequestHeaders(log.reqHeaders) : [];
  const ua = headers.find(([k]) => k.toLowerCase() === 'user-agent');
  let cookies = await getCookies(pageUrl, tab, pageUrl);
  if (extra.mediaUrl && hostOf(extra.mediaUrl) !== hostOf(pageUrl)) cookies = cookies.concat(await getCookies(extra.mediaUrl, tab, pageUrl));
  return {
    pageUrl,
    mediaUrl: extra.mediaUrl || null,
    formatId: extra.formatId || null,
    audioOnly: !!extra.audioOnly,
    title: extra.title || (tab && tab.title) || null,
    headers,
    cookies,
    userAgent: ua ? ua[1] : navigator.userAgent,
    referrer: extra.referrer || pageUrl,
    quick: !!extra.quick,
  };
}

async function onMenu(info, tab) {
  await ready;
  const tabId = tab ? tab.id : -1;
  const pageUrl = info.frameUrl || info.pageUrl || (tab && tab.url) || '';
  try {
    switch (info.menuItemId) {
      case 'idc-link':
        await forcedDownload(info.linkUrl, pageUrl, tabId);
        break;
      case 'idc-image':
        await forcedDownload(info.srcUrl, pageUrl, tabId);
        break;
      case 'idc-video': {
        const src = info.srcUrl || '';
        if (/^https?:/i.test(src) && !isMediaSite(pageUrl) && !/\.(m3u8|mpd)(\?|$)/i.test(src)) {
          await forcedDownload(src, pageUrl, tabId);
        } else {
          const req = await mediaRequest(pageUrl, { mediaUrl: /^https?:/i.test(src) ? src : null, quick: false }, tabId);
          await send({ type: 'media_download', ...req });
        }
        break;
      }
      case 'idc-all':
      case 'idc-selection': {
        const links = await collectLinks(tabId, info.frameId || 0, info.menuItemId === 'idc-selection');
        if (!links.length) {
          notify(t('extName'), t('noLinks'));
          break;
        }
        await send({ type: 'links', links, referrer: pageUrl });
        break;
      }
      default:
        break;
    }
  } catch (e) {
    if (e && e.kind === 'missing') notify(t('extName'), t('appNotFound'));
  }
}

async function collectLinks(tabId, frameId, selectionOnly) {
  try {
    const res = await api.tabs.sendMessage(tabId, { cmd: 'collectLinks', selectionOnly }, { frameId });
    return (res && res.links) || [];
  } catch (_) {
    return [];
  }
}

// ---------------------------------------------------------------- içerik betiği ve açılır pencere mesajları

api.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (!msg || typeof msg.cmd !== 'string') return false;
  handleMessage(msg, sender)
    .then((r) => sendResponse(r))
    .catch((e) => sendResponse({ error: String((e && e.message) || e), kind: e && e.kind }));
  return true;
});

async function handleMessage(msg, sender) {
  await ready;
  const tabId = msg.tabId != null ? msg.tabId : sender.tab ? sender.tab.id : -1;
  switch (msg.cmd) {
    case 'keys':
      session.keys = { alt: !!msg.alt, shift: !!msg.shift, ctrl: !!msg.ctrl, meta: !!msg.meta, t: Date.now() };
      persistSession();
      return { ok: true };

    case 'panelConfig':
      return {
        show: !!(state.config && state.config.videoPanel && state.localEnabled && state.config.enabled !== false),
        appOk: state.appOk,
        language: (state.config && state.config.language) || '',
      };

    case 'panelInfo': {
      const media = (session.media[tabId] || []).slice().reverse();
      const pageUrl = msg.pageUrl || '';
      const hasStream = media.some((m) => m.kind !== 'direct');
      return {
        appOk: state.appOk,
        direct: media.filter((m) => m.kind === 'direct'),
        streams: media.filter((m) => m.kind !== 'direct'),
        probe: isMediaSite(pageUrl) || hasStream || !!msg.blobSrc,
      };
    }

    case 'probe': {
      const req = await mediaRequest(msg.pageUrl, { mediaUrl: msg.mediaUrl || null }, tabId);
      const res = await send({ type: 'media_probe', ...req }, PROBE_TIMEOUT_MS);
      if (res.type === 'media') return { probe: res.probe };
      return { error: res.message || res.reason || 'error' };
    }

    case 'mediaDownload': {
      const req = await mediaRequest(msg.pageUrl, msg, tabId);
      return send({ type: 'media_download', ...req });
    }

    case 'downloadUrl':
      return forcedDownload(msg.url, msg.referrer || msg.pageUrl, tabId, msg.fileName);

    case 'status': {
      const tab = await getTab(tabId);
      const host = tab && /^https?:/i.test(tab.url || '') ? hostOf(tab.url) : '';
      let hostPerm = true;
      try {
        hostPerm = await api.permissions.contains({ origins: ['<all_urls>'] });
      } catch (_) {
        /* eski sürüm */
      }
      return {
        appOk: state.appOk,
        appVersion: state.appVersion,
        config: state.config,
        localEnabled: state.localEnabled,
        host,
        siteDisabled: !!host && state.disabledSites.includes(host),
        media: (session.media[tabId] || []).slice().reverse(),
        pageUrl: tab ? tab.url : '',
        mediaSite: tab ? isMediaSite(tab.url) : false,
        version: EXT_VERSION,
        browser: state.browser,
        isFirefox: IS_FIREFOX,
        hostPerm,
      };
    }

    case 'refresh': {
      const ok = await hello();
      return { ok };
    }

    case 'setEnabled':
      state.localEnabled = !!msg.enabled;
      await saveLocal();
      updateActionIcon();
      return { ok: true };

    case 'setSiteDisabled': {
      const host = String(msg.host || '').toLowerCase();
      if (!host) return { ok: false };
      state.disabledSites = state.disabledSites.filter((h) => h !== host);
      if (msg.disabled) state.disabledSites.push(host);
      await saveLocal();
      return { ok: true };
    }

    case 'show':
      return send({ type: 'show' });

    default:
      return { error: 'unknown command' };
  }
}

// Olay sayfası/servis çalışanı her uyandığında ayarlar eskiyse tazele.
ready.then(() => {
  if (Date.now() - state.lastHelloOk > HELLO_INTERVAL_MIN * 60 * 1000) hello();
});
