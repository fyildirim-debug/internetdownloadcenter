// IDC uygulamasıyla yerel mesajlaşma. Karşılığı: crates/idc-core/src/ext.rs
//
// Gönderilen mesajlar (type alanı ile):
//   hello          { version, browser }                         → { type:"config", config, appVersion }
//   download       DownloadCapture                               → { type:"accepted", id } | { type:"rejected", reason, message }
//   media_probe    MediaRequest                                  → { type:"media", probe } | { type:"error", message }
//   media_download MediaRequest                                  → accepted | rejected
//   links          { links:[{url,text}], referrer }             → accepted
//   show           {}                                            → accepted
//
// DownloadCapture: url, finalUrl, referrer, pageUrl, fileName, fileSize, mime, headers:[[ad,değer]],
//   cookies:[CookieSpec], userAgent, method, postData, forced, browserDownloadId, browser, tabTitle
// MediaRequest: pageUrl, mediaUrl, formatId, audioOnly, title, headers, cookies, userAgent, referrer, quick
// CookieSpec: name, value, domain, path, secure, httpOnly, hostOnly, expires

const HOST_NAME = 'com.furkanyildirim.idc';

class NativeError extends Error {
  constructor(message, kind) {
    super(message);
    this.kind = kind; // 'missing' | 'timeout' | 'app'
  }
}

/**
 * Uygulamaya tek mesaj gönderir. Host bulunamazsa NativeError('missing') fırlatır.
 * Uygulama kapalıysa host onu başlatıp 10 sn'ye kadar bekleyebilir; zaman aşımı buna göre seçilir.
 */
async function nativeSend(msg, timeoutMs = 15000) {
  let timer;
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new NativeError('timeout', 'timeout')), timeoutMs);
  });
  try {
    const send = new Promise((resolve, reject) => {
      try {
        if (IS_FIREFOX) {
          api.runtime.sendNativeMessage(HOST_NAME, msg).then(resolve, (e) => reject(new NativeError(String((e && e.message) || e), 'missing')));
        } else {
          api.runtime.sendNativeMessage(HOST_NAME, msg, (res) => {
            const err = api.runtime.lastError;
            if (err) reject(new NativeError(err.message || 'native error', 'missing'));
            else resolve(res);
          });
        }
      } catch (e) {
        reject(new NativeError(String((e && e.message) || e), 'missing'));
      }
    });
    const res = await Promise.race([send, timeout]);
    if (!res || typeof res !== 'object') throw new NativeError('empty response', 'app');
    if (res.type === 'error') throw new NativeError(res.message || 'app error', 'app');
    return res;
  } finally {
    clearTimeout(timer);
  }
}

/** Tarayıcı çerezini uygulamanın CookieSpec biçimine çevirir. */
function toCookieSpec(c) {
  return {
    name: c.name,
    value: c.value,
    domain: c.domain || '',
    path: c.path || '/',
    secure: !!c.secure,
    httpOnly: !!c.httpOnly,
    hostOnly: !!c.hostOnly,
    expires: typeof c.expirationDate === 'number' ? c.expirationDate : null,
  };
}
