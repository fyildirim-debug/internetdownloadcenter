// Yakalama kuralları, dosya adı ve medya algılama yardımcıları.

const MIME_EXT = {
  'application/zip': 'zip',
  'application/x-zip-compressed': 'zip',
  'application/x-rar-compressed': 'rar',
  'application/vnd.rar': 'rar',
  'application/x-7z-compressed': '7z',
  'application/gzip': 'gz',
  'application/x-gzip': 'gz',
  'application/x-tar': 'tar',
  'application/x-bzip2': 'bz2',
  'application/x-xz': 'xz',
  'application/zstd': 'zst',
  'application/pdf': 'pdf',
  'application/msword': 'doc',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document': 'docx',
  'application/vnd.ms-excel': 'xls',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet': 'xlsx',
  'application/vnd.ms-powerpoint': 'ppt',
  'application/vnd.openxmlformats-officedocument.presentationml.presentation': 'pptx',
  'application/vnd.oasis.opendocument.text': 'odt',
  'application/epub+zip': 'epub',
  'application/x-msdownload': 'exe',
  'application/x-msdos-program': 'exe',
  'application/vnd.microsoft.portable-executable': 'exe',
  'application/x-msi': 'msi',
  'application/x-ms-installer': 'msi',
  'application/vnd.android.package-archive': 'apk',
  'application/vnd.debian.binary-package': 'deb',
  'application/x-debian-package': 'deb',
  'application/x-rpm': 'rpm',
  'application/x-redhat-package-manager': 'rpm',
  'application/x-iso9660-image': 'iso',
  'application/x-apple-diskimage': 'dmg',
  'application/java-archive': 'jar',
  'application/x-bittorrent': 'torrent',
  'application/vnd.apple.mpegurl': 'm3u8',
  'application/x-mpegurl': 'm3u8',
  'application/dash+xml': 'mpd',
  'video/mp4': 'mp4',
  'video/webm': 'webm',
  'video/x-matroska': 'mkv',
  'video/x-msvideo': 'avi',
  'video/quicktime': 'mov',
  'video/x-flv': 'flv',
  'video/x-ms-wmv': 'wmv',
  'video/mpeg': 'mpg',
  'video/3gpp': '3gp',
  'video/ogg': 'ogv',
  'audio/mpeg': 'mp3',
  'audio/mp4': 'm4a',
  'audio/x-m4a': 'm4a',
  'audio/aac': 'aac',
  'audio/ogg': 'ogg',
  'audio/opus': 'opus',
  'audio/flac': 'flac',
  'audio/x-flac': 'flac',
  'audio/wav': 'wav',
  'audio/x-wav': 'wav',
  'audio/webm': 'webm',
};

const DIRECT_MEDIA_EXT = ['mp4', 'webm', 'flv', 'mkv', 'mov', 'm4v', 'ogv', 'mp3', 'm4a', 'ogg', 'opus', 'wav', 'flac', 'aac', '3gp', 'avi', 'wmv'];

// Video sitesi: doğrudan dosya yerine yt-dlp ile kalite listesi alınır.
const MEDIA_SITE_RE =
  /(^|\.)(youtube\.com|youtu\.be|youtube-nocookie\.com|vimeo\.com|dailymotion\.com|dai\.ly|twitter\.com|x\.com|instagram\.com|facebook\.com|fb\.watch|tiktok\.com|twitch\.tv|reddit\.com|redd\.it|bilibili\.com|soundcloud\.com|streamable\.com|rumble\.com|kick\.com|ok\.ru|vk\.com|vk\.video|bandcamp\.com|mixcloud\.com|pinterest\.com|tumblr\.com|9gag\.com|imgur\.com|bitchute\.com|odysee\.com|nicovideo\.jp|niconico\.jp|ted\.com|coub\.com|izlesene\.com|puhutv\.com|blutv\.com)$/i;

function hostOf(url) {
  try {
    return new URL(url).hostname.toLowerCase();
  } catch (_) {
    return '';
  }
}

function isMediaSite(url) {
  return MEDIA_SITE_RE.test(hostOf(url));
}

/** `*` ve `?` jokerli, büyük/küçük harf duyarsız eşleşme. */
function wildcardMatch(pattern, text) {
  pattern = String(pattern || '').trim().toLowerCase();
  text = String(text || '').toLowerCase();
  if (!pattern) return false;
  const re = new RegExp('^' + pattern.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.') + '$');
  return re.test(text);
}

function extOf(name) {
  const base = String(name || '').split(/[?#]/)[0].split(/[\\/]/).pop() || '';
  const i = base.lastIndexOf('.');
  if (i <= 0 || i === base.length - 1) return '';
  return base.slice(i + 1).toLowerCase();
}

function extMatches(pattern, ext) {
  pattern = String(pattern || '').trim().toLowerCase().replace(/^\*?\./, '');
  if (!pattern || !ext) return false;
  return pattern.includes('*') || pattern.includes('?') ? wildcardMatch(pattern, ext) : pattern === ext;
}

function baseName(path) {
  return String(path || '').split(/[\\/]/).pop() || '';
}

/** Adresin yol kısmından dosya adı. */
function nameFromUrl(url) {
  try {
    const u = new URL(url);
    const seg = u.pathname.split('/').filter(Boolean).pop() || '';
    return decodeURIComponent(seg);
  } catch (_) {
    return '';
  }
}

/** RFC 6266 Content-Disposition'dan dosya adı (filename* öncelikli). */
function filenameFromDisposition(cd) {
  if (!cd) return '';
  let m = /filename\*\s*=\s*([^']*)'[^']*'([^;]+)/i.exec(cd);
  if (m) {
    try {
      return decodeURIComponent(m[2].trim().replace(/^"|"$/g, ''));
    } catch (_) {
      /* düz ada düş */
    }
  }
  m = /filename\s*=\s*"((?:\\.|[^"])*)"/i.exec(cd) || /filename\s*=\s*([^;]+)/i.exec(cd);
  if (m) {
    let v = m[1].trim().replace(/\\(.)/g, '$1');
    try {
      // Bazı sunucular yüzde kodlu ad gönderir.
      if (/%[0-9a-f]{2}/i.test(v)) v = decodeURIComponent(v);
    } catch (_) {
      /* olduğu gibi kalsın */
    }
    return baseName(v);
  }
  return '';
}

function mimeBase(mime) {
  return String(mime || '').split(';')[0].trim().toLowerCase();
}

function extFromMime(mime) {
  return MIME_EXT[mimeBase(mime)] || '';
}

function headerMap(headers) {
  const h = {};
  for (const x of headers || []) {
    if (!x || !x.name) continue;
    h[x.name.toLowerCase()] = x.value != null ? x.value : '';
  }
  return h;
}

/** Yakalanan isteğin uygulamaya gönderilmeyecek başlıkları. Cookie ham olarak gönderilir. */
const DROP_HEADERS = new Set([
  'range',
  'if-range',
  'host',
  'content-length',
  'connection',
  'keep-alive',
  'proxy-connection',
  'proxy-authorization',
  'transfer-encoding',
  'te',
  'upgrade',
  'accept-encoding',
  'if-none-match',
  'if-modified-since',
  'purpose',
  'sec-purpose',
]);

function filterRequestHeaders(headers) {
  const out = [];
  for (const x of headers || []) {
    if (!x || !x.name) continue;
    const n = x.name.toLowerCase();
    if (DROP_HEADERS.has(n) || n.startsWith('sec-websocket')) continue;
    if (x.value == null) continue;
    out.push([x.name, x.value]);
  }
  return out;
}

/**
 * İndirmenin IDC'ye verilip verilmeyeceği.
 * ctx: { url, fileName, mime, size, forced, bypass, method }
 * cfg: ExtConfig, local: { enabled, disabledSites }
 */
function shouldCapture(ctx, cfg, local) {
  if (!cfg || !cfg.enabled || !local.enabled) return { ok: false, why: 'disabled' };
  const url = ctx.url || '';
  if (!/^https?:/i.test(url)) return { ok: false, why: 'scheme' };
  if (ctx.method && ctx.method.toUpperCase() !== 'GET') return { ok: false, why: 'method' };
  if (ctx.bypass && !ctx.forced) return { ok: false, why: 'bypass' };
  const host = hostOf(url);
  if ((local.disabledSites || []).some((s) => s === host || host.endsWith('.' + s))) return { ok: false, why: 'site-disabled' };
  if (ctx.forced) return { ok: true, why: 'forced' };

  let hostPath = host;
  try {
    const u = new URL(url);
    hostPath = host + u.pathname + u.search;
  } catch (_) {
    /* yok say */
  }
  for (const p of cfg.skipSites || []) {
    const pat = String(p).toLowerCase();
    if (pat.includes('/') ? wildcardMatch(pat, hostPath) : wildcardMatch(pat, host) || wildcardMatch('*.' + pat, host)) {
      return { ok: false, why: 'skip-site' };
    }
  }
  for (const p of cfg.skipAddresses || []) {
    if (wildcardMatch(p, url) || wildcardMatch(p, url.replace(/^https?:\/\//i, ''))) return { ok: false, why: 'skip-address' };
  }

  const ext = extOf(ctx.fileName) || extOf(nameFromUrl(url)) || extFromMime(ctx.mime);
  if (!ext) return { ok: false, why: 'no-ext' };
  if ((cfg.skipExtensions || []).some((p) => extMatches(p, ext))) return { ok: false, why: 'skip-ext' };
  if (!(cfg.captureExtensions || []).some((p) => extMatches(p, ext))) return { ok: false, why: 'ext' };
  const min = Number(cfg.minSizeKib || 0) * 1024;
  if (min > 0 && ctx.size > 0 && ctx.size < min) return { ok: false, why: 'small' };
  return { ok: true, why: 'rules' };
}

/**
 * Ağ cevabının listelenecek bir medya olup olmadığı.
 * Döner: null ya da { kind: 'hls'|'dash'|'direct', ext }
 */
function classifyMedia(url, h, statusCode) {
  if (!/^https?:/i.test(url)) return null;
  const ct = mimeBase(h['content-type']);
  let path = '';
  try {
    const u = new URL(url);
    path = u.pathname.toLowerCase();
    // YouTube ve benzerlerinin parçalı akışları sayfa adresinden yt-dlp ile alınır.
    if (/(^|\.)googlevideo\.com$/.test(u.hostname)) return null;
    if (/[?&](range|bytestart|byteend)=/i.test(u.search) || /\/range\/\d+-\d+/.test(path)) return null;
  } catch (_) {
    return null;
  }
  const ext = extOf(path);
  if (ct.includes('mpegurl') || ext === 'm3u8') return { kind: 'hls', ext: 'm3u8' };
  if (ct === 'application/dash+xml' || ext === 'mpd') return { kind: 'dash', ext: 'mpd' };

  // Parça istekleri listeyi kirletmesin.
  if (ext === 'ts' || ext === 'm4s' || ext === 'cmfv' || ext === 'cmfa') return null;
  if (/(seg|segment|frag|fragment|chunk|part)[-_]?\d+/i.test(path)) return null;
  const cr = h['content-range'];
  if (cr && !/^bytes\s+0-/i.test(cr)) return null;

  const isMediaType = ct.startsWith('video/') || ct.startsWith('audio/');
  if (!isMediaType && !DIRECT_MEDIA_EXT.includes(ext)) return null;
  if (ct && !isMediaType && ct !== 'application/octet-stream' && ct !== 'binary/octet-stream') return null;
  if (statusCode && statusCode >= 300) return null;

  let size = Number(h['content-length'] || 0);
  if (cr) {
    const m = /\/(\d+)\s*$/.exec(cr);
    if (m) size = Number(m[1]);
  }
  if (size > 0 && size < 256 * 1024) return null;
  return { kind: 'direct', ext: ext || extFromMime(ct) || 'mp4', size: size || 0 };
}

/** Bayt değerini okunur yazar. */
function formatBytes(n) {
  if (!n || n <= 0) return '';
  const u = ['B', 'KB', 'MB', 'GB', 'TB'];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i++;
  }
  return (i === 0 ? v.toFixed(0) : v.toFixed(v >= 100 ? 0 : 1)) + ' ' + u[i];
}
