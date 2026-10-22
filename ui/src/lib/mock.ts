// Tarayıcıda geliştirme için sahte arka uç. Tauri içinde hiç yüklenmez.
// Gerçekçi indirmeler üretir ve saniyede 4 kez ilerleme olayı yayınlar.
import type {
  AddRequest,
  BrowserInfo,
  DownloadDetail,
  DownloadView,
  GrabbedLink,
  MediaProbe,
  OsIntegration,
  PendingAdd,
  PendingMedia,
  ProgressTick,
  Queue,
  SegmentView,
  Settings,
  Status,
  ToolsStatus,
} from './types';
import { emptyAddRequest, emptyRequest } from './types';

type Cb = (p: unknown) => void;
const listeners = new Map<string, Set<Cb>>();
const channel = typeof BroadcastChannel !== 'undefined' ? new BroadcastChannel('idc-mock') : null;

function emit(event: string, payload: unknown, broadcast = true) {
  listeners.get(event)?.forEach((cb) => cb(payload));
  if (broadcast && channel) channel.postMessage({ event, payload });
}
channel?.addEventListener('message', (e) => {
  const { event, payload } = e.data as { event: string; payload: unknown };
  listeners.get(event)?.forEach((cb) => cb(payload));
});

export function listen(event: string, cb: Cb): () => void {
  if (!listeners.has(event)) listeners.set(event, new Set());
  listeners.get(event)!.add(cb);
  return () => listeners.get(event)?.delete(cb);
}

const HOME = '/home/furkan/İndirilenler';
const MB = 1024 * 1024;

const CAPTURE = '3gp 7z aac ace aif aiff apk appimage arj asf avi bin bz2 cab deb dmg doc docx epub exe flac flv gz gzip img iso jar lzh lz m4a m4v mkv mov mp3 mp4 mpa mpe mpeg mpg msi msix msu odp ods odt ogg ogv opus pdf pps ppt pptx qt r0* r1* ra rar rm rmvb rpm sea sit sitx snap tar tbz2 tgz tif tiff torrent ts txz vob wav webm wma wmv xls xlsx xz z zip zst'.split(' ');

function defaultSettings(): Settings {
  const cat = (id: string, name: string, exts: string, sub: string) => ({
    id,
    name,
    extensions: exts ? exts.split(' ') : [],
    saveDir: sub ? `${HOME}/${sub}` : HOME,
    builtin: true,
  });
  const q: Queue = {
    id: 'main',
    name: 'Main queue',
    maxConcurrent: 3,
    order: ['d3', 'd7'],
    builtin: true,
    schedule: {
      enabled: false,
      startTime: '02:00',
      stopEnabled: false,
      stopTime: '08:00',
      days: [],
      onceDate: null,
      afterAll: 'none',
      retryFailed: 3,
    },
  };
  return {
    version: 1,
    language: 'auto',
    theme: 'light',
    firstRunDone: false,
    defaultSaveDir: HOME,
    perCategoryDirs: true,
    general: {
      launchOnStartup: true,
      closeToTray: true,
      clipboardMonitor: true,
      browserIntegration: true,
      videoPanel: true,
      bypassKey: 'alt',
      forceKey: 'none',
      playSounds: true,
      checkUpdates: true,
    },
    downloads: {
      defaultConnections: 8,
      maxSimultaneous: 0,
      showFileInfoDialog: true,
      startImmediatelyWithDialog: false,
      progressDialog: 'normal',
      showCompleteDialog: true,
      duplicateAction: 'rename',
      retryCount: 20,
      retryDelaySec: 5,
      userAgent: '',
      preallocate: true,
      useServerTime: false,
      antivirusCommand: '',
    },
    connection: {
      speedLimitEnabled: false,
      speedLimitKib: 1024,
      timeoutSec: 30,
      serverLimits: [{ host: '*.sourceforge.net', connections: 4 }],
      http2: false,
      adaptive: true,
    },
    proxy: { mode: 'system', url: '', username: '', password: '', bypass: ['localhost', '127.0.0.1'] },
    fileTypes: {
      captureExtensions: CAPTURE,
      skipExtensions: [],
      skipSites: ['*.update.microsoft.com', 'download.windowsupdate.com'],
      skipAddresses: [],
      minSizeKib: 0,
    },
    categories: [
      cat('general', 'General', '', ''),
      cat('compressed', 'Compressed', 'zip rar 7z tar gz bz2 xz zst tgz iso img', 'Compressed'),
      cat('documents', 'Documents', 'pdf doc docx xls xlsx ppt pptx odt ods odp txt epub', 'Documents'),
      cat('music', 'Music', 'mp3 m4a aac flac wav wma ogg opus', 'Music'),
      cat('programs', 'Programs', 'exe msi apk deb rpm appimage snap dmg jar bin', 'Programs'),
      cat('video', 'Video', 'mp4 mkv webm avi mov wmv flv m4v mpg mpeg 3gp ts', 'Video'),
    ],
    queues: [
      q,
      { ...structuredClone(q), id: 'night', name: 'Gece kuyruğu', builtin: false, order: ['d9'] },
    ],
    siteLogins: [],
    media: {
      defaultQuality: 'best',
      container: 'mp4',
      audioFormat: 'm4a',
      autoUpdateTools: true,
      impersonateFallback: true,
      browserFallback: true,
      downloadSubtitles: false,
      subtitleLangs: ['tr', 'en'],
      embedThumbnail: false,
      sponsorblock: false,
    },
    columns: [],
  };
}

let settings: Settings = defaultSettings();
try {
  const saved = localStorage.getItem('idc-mock-settings');
  if (saved) settings = { ...settings, ...JSON.parse(saved) };
} catch {
  /* yok */
}

interface MockDl {
  view: DownloadView;
  segs: { start: number; end: number; pos: number; speed: number }[];
  history: number[];
  referer: string | null;
}

const now = Date.now();
function mk(
  id: string,
  fileName: string,
  url: string,
  category: string,
  size: number | null,
  ratio: number,
  status: Status,
  extra: Partial<DownloadView> = {},
): MockDl {
  const downloaded = size ? Math.round(size * ratio) : Math.round(ratio * 40 * MB);
  const conns = status === 'downloading' ? 8 : 0;
  const view: DownloadView = {
    id,
    url,
    fileName,
    saveDir: category === 'general' ? HOME : `${HOME}/${category[0].toUpperCase()}${category.slice(1)}`,
    category,
    description: '',
    size,
    downloaded,
    status,
    error: null,
    speed: 0,
    eta: null,
    resumable: size != null,
    kind: 'http',
    connections: conns,
    maxConnections: 8,
    speedLimit: null,
    queueId: null,
    addedAt: now - Math.random() * 86400000 * 5,
    lastTryAt: now - Math.random() * 3600000 * 10,
    completedAt: status === 'completed' ? now - Math.random() * 3600000 * 30 : null,
    mime: null,
    thumbnail: null,
    ...extra,
  };
  const segs: MockDl['segs'] = [];
  if (size) {
    const n = 8;
    const part = Math.floor(size / n);
    for (let i = 0; i < n; i++) {
      const start = i * part;
      const end = i === n - 1 ? size : (i + 1) * part;
      const pos = start + Math.round((end - start) * Math.min(1, ratio * (0.6 + Math.random() * 0.8)));
      segs.push({ start, end, pos: Math.min(end, pos), speed: 0 });
    }
    view.downloaded = segs.reduce((a, s) => a + (s.pos - s.start), 0);
    if (status === 'completed') {
      segs.forEach((s) => (s.pos = s.end));
      view.downloaded = size;
    }
  }
  return { view, segs, history: [], referer: 'https://example.com/' };
}

const dls = new Map<string, MockDl>();
[
  mk('d1', 'ubuntu-24.04.1-desktop-amd64.iso', 'https://releases.ubuntu.com/24.04/ubuntu-24.04.1-desktop-amd64.iso', 'compressed', 6114 * MB, 0.37, 'downloading'),
  mk('d2', 'Fedora-Workstation-Live-x86_64-41.iso', 'https://download.fedoraproject.org/pub/fedora/linux/releases/41/Workstation/x86_64/iso/Fedora-Workstation-Live-x86_64-41-1.4.iso', 'compressed', 2245 * MB, 0.71, 'downloading'),
  mk('d3', 'Blender-4.2-tutorial.mp4', 'https://cdn.example.com/videos/blender.mp4', 'video', 842 * MB, 0.12, 'paused', { queueId: 'main' }),
  mk('d4', 'Rapor_2026_Q3.pdf', 'https://example.org/files/rapor.pdf', 'documents', 4.2 * MB, 1, 'completed', { description: 'Çeyrek raporu' }),
  mk('d5', 'VSCodeUserSetup-x64-1.95.exe', 'https://update.code.visualstudio.com/latest/win32-x64-user/stable', 'programs', 98 * MB, 1, 'completed'),
  mk('d6', 'Tarkan - Kuzu Kuzu.m4a', 'https://www.youtube.com/watch?v=xxxx', 'music', 7.8 * MB, 1, 'completed', { kind: 'media' }),
  mk('d7', 'node-v22.11.0-linux-x64.tar.xz', 'https://nodejs.org/dist/v22.11.0/node-v22.11.0-linux-x64.tar.xz', 'compressed', 27 * MB, 0, 'queued', { queueId: 'main' }),
  mk('d8', 'firmware_update.bin', 'https://files.example.net/expired/firmware_update.bin', 'programs', 312 * MB, 0.44, 'error', { error: 'expired|HTTP 403' }),
  mk('d9', 'Big Buck Bunny 1080p.mp4', 'https://www.youtube.com/watch?v=aqz-KE-bpKQ', 'video', 276 * MB, 0.58, 'downloading', { kind: 'media', queueId: 'night', connections: 4, maxConnections: 4 }),
  mk('d10', 'stream_live.ts', 'https://live.example.tv/stream.m3u8', 'video', null, 0.5, 'downloading'),
  mk('d11', 'LibreOffice_24.8_Linux_x86-64_deb.tar.gz', 'https://download.documentfoundation.org/libreoffice/stable/24.8.2/deb/x86_64/LibreOffice_24.8.2_Linux_x86-64_deb.tar.gz', 'compressed', 215 * MB, 1, 'completed'),
  mk('d12', 'kitap.epub', 'https://example.com/kitap.epub', 'documents', 1.3 * MB, 0, 'paused'),
].forEach((d) => dls.set(d.view.id, d));

function segViews(d: MockDl): SegmentView[] {
  return d.segs.map((s) => ({
    start: s.start,
    end: s.end,
    pos: s.pos,
    active: d.view.status === 'downloading' && s.pos < s.end,
    speed: d.view.status === 'downloading' && s.pos < s.end ? s.speed : 0,
    info: s.pos >= s.end ? 'done' : d.view.status === 'downloading' ? 'receiving' : 'idle',
  }));
}

function tick() {
  const ticks: ProgressTick[] = [];
  const limit = settings.connection.speedLimitEnabled ? settings.connection.speedLimitKib * 1024 : Infinity;
  const active = [...dls.values()].filter((d) => d.view.status === 'downloading' || d.view.status === 'connecting');
  const share = active.length ? limit / active.length : limit;
  for (const d of active) {
    if (d.view.status === 'connecting') {
      d.view.status = 'downloading';
      emit('idc://changed', d.view);
    }
    let total = 0;
    if (d.segs.length) {
      for (const s of d.segs) {
        if (s.pos >= s.end) {
          s.speed = 0;
          continue;
        }
        s.speed = Math.min(share / 8, 300 * 1024 + Math.random() * 900 * 1024);
        const add = Math.round(s.speed / 4);
        s.pos = Math.min(s.end, s.pos + add);
        total += s.speed;
      }
      d.view.downloaded = d.segs.reduce((a, s) => a + (s.pos - s.start), 0);
    } else {
      total = Math.min(share, 600 * 1024 + Math.random() * 300 * 1024);
      d.view.downloaded += Math.round(total / 4);
    }
    d.view.speed = Math.round(total);
    d.history.push(d.view.speed);
    if (d.history.length > 120) d.history.shift();
    d.view.connections = d.segs.filter((s) => s.pos < s.end).length || (d.segs.length ? 0 : 1);
    d.view.eta = d.view.size && total > 0 ? Math.round((d.view.size - d.view.downloaded) / total) : null;
    if (d.view.size && d.view.downloaded >= d.view.size) {
      d.view.status = 'completed';
      d.view.speed = 0;
      d.view.eta = null;
      d.view.connections = 0;
      d.view.completedAt = Date.now();
      emit('idc://changed', d.view);
    }
    ticks.push({
      id: d.view.id,
      status: d.view.status,
      size: d.view.size,
      downloaded: d.view.downloaded,
      speed: d.view.speed,
      eta: d.view.eta,
      connections: d.view.connections,
      segments: segViews(d),
    });
  }
  if (ticks.length) emit('idc://progress', ticks, false);
}
// Her pencere kendi simülasyonunu yürütür; yalnız ana pencere değişiklik yayınlar.
setInterval(tick, 250);

function setStatus(ids: string[], status: Status) {
  for (const id of ids) {
    const d = dls.get(id);
    if (!d || d.view.status === 'completed') continue;
    d.view.status = status;
    if (status !== 'downloading' && status !== 'connecting') {
      d.view.speed = 0;
      d.view.eta = null;
      d.view.connections = 0;
    }
    d.view.lastTryAt = Date.now();
    emit('idc://changed', d.view);
  }
}

const mediaProbe: MediaProbe = {
  pageUrl: 'https://www.youtube.com/watch?v=aqz-KE-bpKQ',
  title: 'Big Buck Bunny 60fps 4K - Official Blender Foundation Short Film',
  thumbnail: null,
  duration: 635,
  uploader: 'Blender',
  extractor: 'youtube',
  isLive: false,
  playlistCount: null,
  subtitles: ['en', 'tr', 'de', 'fr'],
  formats: [
    { formatId: '315+140', label: '2160p60 WEBM', height: 2160, width: 3840, fps: 60, ext: 'webm', vcodec: 'vp9', acodec: 'mp4a', filesize: 1712 * MB, tbr: 22000, audioOnly: false, hdr: false },
    { formatId: '308+140', label: '1440p60 WEBM', height: 1440, width: 2560, fps: 60, ext: 'webm', vcodec: 'vp9', acodec: 'mp4a', filesize: 802 * MB, tbr: 10000, audioOnly: false, hdr: false },
    { formatId: '299+140', label: '1080p60 MP4', height: 1080, width: 1920, fps: 60, ext: 'mp4', vcodec: 'avc1', acodec: 'mp4a', filesize: 412 * MB, tbr: 5000, audioOnly: false, hdr: false },
    { formatId: '298+140', label: '720p60 MP4', height: 720, width: 1280, fps: 60, ext: 'mp4', vcodec: 'avc1', acodec: 'mp4a', filesize: 214 * MB, tbr: 2700, audioOnly: false, hdr: false },
    { formatId: '135+140', label: '480p MP4', height: 480, width: 854, fps: 30, ext: 'mp4', vcodec: 'avc1', acodec: 'mp4a', filesize: 96 * MB, tbr: 1200, audioOnly: false, hdr: false },
    { formatId: '134+140', label: '360p MP4', height: 360, width: 640, fps: 30, ext: 'mp4', vcodec: 'avc1', acodec: 'mp4a', filesize: 52 * MB, tbr: 650, audioOnly: false, hdr: false },
    { formatId: '140', label: 'M4A 128k', height: null, width: null, fps: null, ext: 'm4a', vcodec: null, acodec: 'mp4a', filesize: 10 * MB, tbr: 128, audioOnly: true, hdr: false },
    { formatId: '251', label: 'OPUS 160k', height: null, width: null, fps: null, ext: 'webm', vcodec: null, acodec: 'opus', filesize: 12 * MB, tbr: 160, audioOnly: true, hdr: false },
  ],
};

const browsers: BrowserInfo[] = [
  { id: 'chrome', name: 'Google Chrome', family: 'chromium', installed: true, hostRegistered: true, extensionPath: '/usr/share/internet-download-center/extension/chromium', extensionsPage: 'chrome://extensions', executable: '/usr/bin/google-chrome', sandboxed: false },
  { id: 'edge', name: 'Microsoft Edge', family: 'chromium', installed: true, hostRegistered: false, extensionPath: '/usr/share/internet-download-center/extension/chromium', extensionsPage: 'edge://extensions', executable: '/usr/bin/microsoft-edge', sandboxed: false },
  { id: 'brave', name: 'Brave', family: 'chromium', installed: false, hostRegistered: false, extensionPath: '/usr/share/internet-download-center/extension/chromium', extensionsPage: 'brave://extensions', executable: null, sandboxed: false },
  { id: 'firefox', name: 'Mozilla Firefox', family: 'firefox', installed: true, hostRegistered: true, extensionPath: '/usr/share/internet-download-center/extension/idc-firefox.xpi', extensionsPage: 'about:addons', executable: '/usr/bin/firefox', sandboxed: false },
  { id: 'zen', name: 'Zen Browser', family: 'firefox', installed: true, hostRegistered: false, extensionPath: '/usr/share/internet-download-center/extension/idc-firefox.xpi', extensionsPage: 'about:addons', executable: '/opt/zen/zen', sandboxed: false },
];

let tools: ToolsStatus = {
  ytdlp: { installed: true, version: '2026.10.10', path: '~/.local/share/internet-download-center/tools/yt-dlp', source: 'managed' },
  ffmpeg: { installed: true, version: '7.1', path: '/usr/bin/ffmpeg', source: 'system' },
  deno: { installed: false, version: null, path: null, source: 'none' },
  busy: false,
  message: null,
};

let osInt: OsIntegration = {
  autostart: true,
  urlHandler: true,
  kdeDownloadManager: false,
  contextMenu: false,
  desktop: 'kde',
  session: 'wayland',
  traySupported: true,
};

const pendingAdd = (token: string): PendingAdd => {
  const req = emptyAddRequest('https://github.com/neovim/neovim/releases/download/v0.10.2/nvim-linux64.tar.gz');
  req.request.referer = 'https://github.com/neovim/neovim/releases';
  return {
    token,
    request: req,
    probe: {
      url: req.request.url,
      finalUrl: 'https://objects.githubusercontent.com/github-production-release-asset/nvim-linux64.tar.gz',
      fileName: 'nvim-linux64.tar.gz',
      size: 11.4 * MB,
      resumable: true,
      mime: 'application/gzip',
      category: 'compressed',
      isHtml: false,
      statusCode: 206,
      isMediaSite: false,
    },
    probing: false,
    error: null,
    suggestedName: 'nvim-linux64.tar.gz',
    suggestedDir: `${HOME}/Compressed`,
    category: 'compressed',
    duplicateOf: null,
  };
};

function detail(id: string): DownloadDetail {
  const d = dls.get(id);
  if (!d) throw new Error('not_found|' + id);
  return {
    ...d.view,
    referer: d.referer,
    finalUrl: null,
    etag: '"5f1a-62e9"',
    lastModified: 'Tue, 15 Oct 2026 10:12:00 GMT',
    segments: segViews(d),
    after: 'none',
    checksum: null,
    checksumOk: null,
    media:
      d.view.kind === 'media'
        ? {
            pageUrl: d.view.url,
            mediaUrl: null,
            formatId: '299+140',
            audioOnly: false,
            audioFormat: null,
            container: 'mp4',
            title: d.view.fileName,
            thumbnail: null,
            impersonate: false,
            subtitles: [],
          }
        : null,
    speedHistory: d.history.slice(),
  };
}

function openWin(kind: string, params: Record<string, string> | null) {
  const qs = params ? '?' + new URLSearchParams(params).toString() : '';
  const sizes: Record<string, [number, number]> = {
    progress: [470, 360],
    fileinfo: [560, 330],
    complete: [460, 270],
    addurl: [520, 210],
    options: [680, 560],
    scheduler: [720, 520],
    batch: [560, 470],
    grabber: [760, 560],
    media: [640, 560],
    about: [440, 330],
    wizard: [680, 500],
    properties: [520, 520],
    links: [700, 520],
    main: [1100, 680],
  };
  const [w, h] = sizes[kind] ?? [600, 400];
  window.open(`${location.pathname}#/${kind}${qs}`, `idc-${kind}-${params?.id ?? params?.token ?? ''}`, `width=${w},height=${h}`);
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function invoke(cmd: string, args: Record<string, any> = {}): Promise<unknown> {
  await sleep(20);
  switch (cmd) {
    case 'list_downloads':
      return [...dls.values()].map((d) => ({ ...d.view }));
    case 'get_download':
      return detail(args.id);
    case 'probe_url': {
      await sleep(400);
      const name = decodeURIComponent(new URL(args.url).pathname.split('/').pop() || 'index.html');
      return {
        url: args.url,
        finalUrl: args.url,
        fileName: name,
        size: 48 * MB,
        resumable: true,
        mime: null,
        category: 'general',
        isHtml: false,
        statusCode: 206,
        isMediaSite: /youtube|vimeo/.test(args.url),
      };
    }
    case 'add_download':
    case 'confirm_pending': {
      const req: AddRequest = args.req;
      const id = 'n' + Math.random().toString(16).slice(2, 8);
      const d = mk(id, req.fileName || req.request.url.split('/').pop() || 'file', req.request.url, req.category || 'general', 64 * MB, 0, req.start ? 'connecting' : 'paused', {
        description: req.description ?? '',
        maxConnections: req.maxConnections ?? 8,
        queueId: req.queueId,
      });
      dls.set(id, d);
      emit('idc://changed', d.view);
      if (req.start && settings.downloads.progressDialog !== 'none') openWin('progress', { id });
      return id;
    }
    case 'add_many':
      return Promise.all((args.reqs as AddRequest[]).map((r) => invoke('add_download', { req: { ...r, start: false } })));
    case 'start_downloads':
      setStatus(args.ids, 'connecting');
      return;
    case 'pause_downloads':
      setStatus(args.ids, 'paused');
      return;
    case 'start_all':
      setStatus([...dls.keys()].filter((k) => dls.get(k)!.view.status !== 'completed'), 'connecting');
      return;
    case 'stop_all':
      setStatus([...dls.keys()].filter((k) => ['downloading', 'connecting', 'queued'].includes(dls.get(k)!.view.status)), 'paused');
      return;
    case 'remove_downloads':
      for (const id of args.ids) dls.delete(id);
      emit('idc://removed', args.ids);
      return;
    case 'delete_completed': {
      const ids = [...dls.values()].filter((d) => d.view.status === 'completed').map((d) => d.view.id);
      ids.forEach((id) => dls.delete(id));
      emit('idc://removed', ids);
      return;
    }
    case 'redownload': {
      const d = dls.get(args.id);
      if (d) {
        d.segs.forEach((s) => (s.pos = s.start));
        d.view.downloaded = 0;
        d.view.status = 'connecting';
        emit('idc://changed', d.view);
      }
      return;
    }
    case 'update_download': {
      const d = dls.get(args.id);
      if (d) {
        const p = args.patch;
        if (p.fileName) d.view.fileName = p.fileName;
        if (p.saveDir) d.view.saveDir = p.saveDir;
        if (p.description != null) d.view.description = p.description;
        if (p.url) d.view.url = p.url;
        if (p.maxConnections) d.view.maxConnections = p.maxConnections;
        if (p.speedLimit != null) d.view.speedLimit = p.speedLimit === 0 ? null : p.speedLimit;
        if (p.queueId) d.view.queueId = p.queueId;
        if (p.clearQueue) d.view.queueId = null;
        if (p.category) d.view.category = p.category;
        emit('idc://changed', d.view);
      }
      return;
    }
    case 'open_file':
    case 'open_with':
    case 'open_folder':
      console.info('[mock]', cmd, args.id);
      return;
    case 'get_settings':
      return structuredClone(settings);
    case 'save_settings':
      settings = structuredClone(args.settings);
      try {
        localStorage.setItem('idc-mock-settings', JSON.stringify(settings));
      } catch {
        /* yok */
      }
      emit('idc://settings', settings);
      return structuredClone(settings);
    case 'media_probe':
    case 'get_pending_media': {
      await sleep(cmd === 'media_probe' ? 900 : 0);
      if (cmd === 'media_probe') return { ...mediaProbe, pageUrl: args.url };
      const p: PendingMedia = {
        token: args.token,
        request: {
          pageUrl: mediaProbe.pageUrl,
          mediaUrl: null,
          formatId: null,
          audioOnly: false,
          title: mediaProbe.title,
          headers: [],
          cookies: [],
          userAgent: null,
          referrer: null,
          quick: false,
        },
        probe: null,
        probing: true,
        error: null,
      };
      setTimeout(() => emit('idc://pending-media', { ...p, probe: mediaProbe, probing: false }), 1200);
      return p;
    }
    case 'confirm_pending_media': {
      const c = args.choice;
      const id = 'm' + Math.random().toString(16).slice(2, 8);
      const d = mk(id, c.fileName, mediaProbe.pageUrl, c.audioOnly ? 'music' : 'video', 200 * MB, 0, 'connecting', { kind: 'media' });
      dls.set(id, d);
      emit('idc://changed', d.view);
      return id;
    }
    case 'tools_status':
      return tools;
    case 'tools_update':
      tools = { ...tools, busy: true, message: 'deno indiriliyor… %42' };
      emit('idc://tools', tools);
      await sleep(1500);
      tools = {
        ...tools,
        busy: false,
        message: 'Araçlar güncel.',
        deno: { installed: true, version: '2.5.4', path: '~/.local/share/internet-download-center/tools/deno', source: 'managed' },
      };
      emit('idc://tools', tools);
      return tools;
    case 'browsers_status':
      return browsers;
    case 'browsers_install':
      browsers.forEach((b) => {
        if (b.installed && (!args.ids || args.ids.includes(b.id))) b.hostRegistered = true;
      });
      return browsers;
    case 'browser_open_extensions':
      return;
    case 'os_integration_status':
      return osInt;
    case 'os_integration_apply':
      osInt = { ...osInt, ...args.opts };
      return osInt;
    case 'open_window':
      openWin(args.kind, args.params);
      return;
    case 'start_queue':
    case 'stop_queue': {
      const q = settings.queues.find((x) => x.id === args.queueId);
      if (q) setStatus(q.order, cmd === 'start_queue' ? 'connecting' : 'paused');
      return;
    }
    case 'save_queue': {
      const i = settings.queues.findIndex((x) => x.id === args.queue.id);
      if (i >= 0) settings.queues[i] = args.queue;
      else settings.queues.push(args.queue);
      emit('idc://settings', settings);
      return;
    }
    case 'delete_queue':
      settings.queues = settings.queues.filter((q) => q.id !== args.id);
      emit('idc://settings', settings);
      return;
    case 'move_in_queue': {
      const q = settings.queues.find((x) => x.id === args.queueId);
      if (q) {
        for (const id of args.ids as string[]) {
          const i = q.order.indexOf(id);
          const j = i + args.delta;
          if (i >= 0 && j >= 0 && j < q.order.length) [q.order[i], q.order[j]] = [q.order[j], q.order[i]];
        }
        emit('idc://settings', settings);
      }
      return;
    }
    case 'batch_preview': {
      const { pattern, from, to, step, padding } = args;
      const out: string[] = [];
      if (/^\d+$/.test(from) && /^\d+$/.test(to)) {
        for (let i = +from; i <= +to && out.length < 10000; i += Math.max(1, step)) {
          out.push(pattern.replace('*', String(i).padStart(padding, '0')));
        }
      } else {
        for (let c = from.charCodeAt(0); c <= to.charCodeAt(0); c += Math.max(1, step)) {
          out.push(pattern.replace('*', String.fromCharCode(c)));
        }
      }
      return out;
    }
    case 'grab_links': {
      await sleep(1200);
      const base = args.url.replace(/\/$/, '');
      const list: GrabbedLink[] = [
        ['photos/manzara-01.jpg', 'Manzara 1', 'general'],
        ['photos/manzara-02.jpg', 'Manzara 2', 'general'],
        ['files/setup.exe', 'Kurulum', 'programs'],
        ['files/belgeler.zip', 'Belgeler arşivi', 'compressed'],
        ['media/tanitim.mp4', 'Tanıtım videosu', 'video'],
        ['media/podcast-12.mp3', 'Podcast #12', 'music'],
        ['docs/kilavuz.pdf', 'Kullanım kılavuzu', 'documents'],
      ].map(([p, text, cat], i) => ({ url: `${base}/${p}`, text, fileName: p.split('/').pop()!, category: cat, depth: i < 4 ? 1 : 2 }));
      return list;
    }
    case 'app_info':
      return { version: '1.0.0', os: 'linux', dataDir: '~/.config/internet-download-center', firstRun: !settings.firstRunDone };
    case 'quit_app':
      console.info('[mock] quit');
      return;
    case 'create_pending': {
      const token = 'p' + Math.random().toString(16).slice(2, 8);
      try {
        sessionStorage.setItem('idc-pending-' + token, JSON.stringify(args.req));
        localStorage.setItem('idc-pending-' + token, JSON.stringify(args.req));
      } catch {
        /* yok */
      }
      return token;
    }
    case 'get_pending': {
      const base = pendingAdd(args.token);
      let raw: string | null = null;
      try {
        raw = localStorage.getItem('idc-pending-' + args.token);
      } catch {
        /* yok */
      }
      if (!raw) return base;
      const req: AddRequest = JSON.parse(raw);
      const name = decodeURIComponent(new URL(req.request.url).pathname.split('/').pop() || 'index.html');
      const p: PendingAdd = { ...base, request: req, probe: null, probing: true, suggestedName: name, suggestedDir: HOME, category: 'general' };
      setTimeout(() => emit('idc://pending', { ...p, probing: false, probe: { ...base.probe!, url: req.request.url, finalUrl: req.request.url, fileName: name, size: 23.7 * MB, isMediaSite: /youtube|vimeo/.test(req.request.url) } }, false), 700);
      return p;
    }
    case 'cancel_pending':
      return;
    case 'get_pending_links':
      return {
        token: args.token,
        referrer: 'https://example.com/downloads',
        links: [
          'https://example.com/a/rapor.pdf',
          'https://example.com/a/sunum.pptx',
          'https://example.com/b/arsiv-2025.zip',
          'https://example.com/b/arsiv-2026.zip',
          'https://example.com/c/klip.mp4',
          'https://example.com/c/muzik.mp3',
          'https://example.com/index.html',
        ].map((url) => ({ url, text: url.split('/').pop()!, fileName: url.split('/').pop()!, category: 'general', depth: 0 })),
      };
    case 'set_global_speed_limit':
      settings.connection.speedLimitEnabled = args.kib != null;
      if (args.kib != null) settings.connection.speedLimitKib = args.kib;
      emit('idc://settings', settings);
      return;
    case 'export_list':
    case 'import_list':
      return 0;
    default:
      console.warn('[mock] bilinmeyen komut', cmd, args);
      return null;
  }
}

// Kullanılmayan içe aktarma uyarısını önler.
void emptyRequest;
