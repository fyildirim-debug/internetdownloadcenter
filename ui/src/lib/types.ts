// crates/idc-core/src/{model,settings,ext,system}.rs dosyalarının TypeScript aynası.
// Alan adları camelCase; Rust'taki Option<T> burada `T | null`.

export type DownloadId = string;

export type Status =
  | 'queued'
  | 'connecting'
  | 'downloading'
  | 'paused'
  | 'processing'
  | 'completed'
  | 'error';

export const ACTIVE_STATUSES: Status[] = ['connecting', 'downloading', 'processing'];

export interface CookieSpec {
  name: string;
  value: string;
  domain: string;
  path: string;
  secure: boolean;
  httpOnly: boolean;
  hostOnly: boolean;
  expires: number | null;
}

export interface RequestInfo {
  url: string;
  referer: string | null;
  headers: [string, string][];
  cookies: CookieSpec[];
  rawCookie: string | null;
  userAgent: string | null;
  method: string | null;
  postData: string | null;
}

export interface Segment {
  start: number;
  end: number | null;
  pos: number;
}

export interface MediaSpec {
  pageUrl: string;
  mediaUrl: string | null;
  formatId: string;
  audioOnly: boolean;
  audioFormat: string | null;
  container: string | null;
  title: string | null;
  thumbnail: string | null;
  impersonate: boolean;
  subtitles: string[];
}

export type DownloadKind = { type: 'http' } | ({ type: 'media' } & MediaSpec);

export type AfterAction = 'none' | 'exit' | 'shutdown' | 'sleep' | 'hibernate';

export interface Checksum {
  algo: string;
  expected: string;
}

export interface Download {
  id: DownloadId;
  request: RequestInfo;
  finalUrl: string | null;
  fileName: string;
  saveDir: string;
  category: string;
  description: string;
  size: number | null;
  downloaded: number;
  resumable: boolean;
  mime: string | null;
  etag: string | null;
  lastModified: string | null;
  status: Status;
  error: string | null;
  maxConnections: number;
  speedLimit: number | null;
  segments: Segment[];
  kind: DownloadKind;
  queueId: string | null;
  addedAt: number;
  lastTryAt: number | null;
  completedAt: number | null;
  after: AfterAction;
  checksum: Checksum | null;
  checksumOk: boolean | null;
  showCompleteDialog: boolean | null;
  silent: boolean;
  autoName: boolean;
  fromBrowser: boolean;
}

export interface DownloadView {
  id: DownloadId;
  url: string;
  fileName: string;
  saveDir: string;
  category: string;
  description: string;
  size: number | null;
  downloaded: number;
  status: Status;
  error: string | null;
  speed: number;
  eta: number | null;
  resumable: boolean;
  kind: 'http' | 'media' | string;
  connections: number;
  maxConnections: number;
  speedLimit: number | null;
  queueId: string | null;
  addedAt: number;
  lastTryAt: number | null;
  completedAt: number | null;
  mime: string | null;
  thumbnail: string | null;
}

export interface SegmentView {
  start: number;
  end: number | null;
  pos: number;
  active: boolean;
  speed: number;
  info: string;
}

export interface DownloadDetail extends DownloadView {
  referer: string | null;
  finalUrl: string | null;
  etag: string | null;
  lastModified: string | null;
  segments: SegmentView[];
  after: AfterAction;
  checksum: Checksum | null;
  checksumOk: boolean | null;
  media: MediaSpec | null;
  speedHistory: number[];
}

export interface ProgressTick {
  id: DownloadId;
  status: Status;
  size: number | null;
  downloaded: number;
  speed: number;
  eta: number | null;
  connections: number;
  segments: SegmentView[];
}

export interface ProbeResult {
  url: string;
  finalUrl: string;
  fileName: string;
  size: number | null;
  resumable: boolean;
  mime: string | null;
  category: string;
  isHtml: boolean;
  statusCode: number;
  isMediaSite: boolean;
}

export interface AddRequest {
  request: RequestInfo;
  fileName: string | null;
  saveDir: string | null;
  category: string | null;
  description: string | null;
  sizeHint: number | null;
  mimeHint: string | null;
  maxConnections: number | null;
  media: MediaSpec | null;
  start: boolean;
  queueId: string | null;
  checksum: Checksum | null;
  silent: boolean;
}

export interface DownloadPatch {
  url?: string | null;
  fileName?: string | null;
  saveDir?: string | null;
  category?: string | null;
  description?: string | null;
  maxConnections?: number | null;
  speedLimit?: number | null;
  queueId?: string | null;
  clearQueue?: boolean;
  after?: AfterAction | null;
  checksum?: Checksum | null;
  referer?: string | null;
  showCompleteDialog?: boolean | null;
}

export interface MediaFormat {
  formatId: string;
  label: string;
  height: number | null;
  width: number | null;
  fps: number | null;
  ext: string;
  vcodec: string | null;
  acodec: string | null;
  filesize: number | null;
  tbr: number | null;
  audioOnly: boolean;
  hdr: boolean;
}

export interface MediaProbe {
  pageUrl: string;
  title: string;
  thumbnail: string | null;
  duration: number | null;
  uploader: string | null;
  extractor: string;
  formats: MediaFormat[];
  subtitles: string[];
  isLive: boolean;
  playlistCount: number | null;
}

export interface GrabbedLink {
  url: string;
  text: string;
  fileName: string;
  category: string;
  depth: number;
}

// ---------- settings.rs ----------

export interface Category {
  id: string;
  name: string;
  extensions: string[];
  saveDir: string;
  builtin: boolean;
}

export interface Schedule {
  enabled: boolean;
  startTime: string;
  stopEnabled: boolean;
  stopTime: string;
  days: number[];
  onceDate: string | null;
  afterAll: AfterAction;
  retryFailed: number;
}

export interface Queue {
  id: string;
  name: string;
  maxConcurrent: number;
  schedule: Schedule;
  order: string[];
  builtin: boolean;
}

export interface SiteLogin {
  pattern: string;
  username: string;
  password: string;
}

export interface ServerLimit {
  host: string;
  connections: number;
}

export interface GeneralSettings {
  launchOnStartup: boolean;
  closeToTray: boolean;
  clipboardMonitor: boolean;
  browserIntegration: boolean;
  videoPanel: boolean;
  bypassKey: string;
  forceKey: string;
  playSounds: boolean;
  checkUpdates: boolean;
}

export interface DownloadSettings {
  defaultConnections: number;
  maxSimultaneous: number;
  showFileInfoDialog: boolean;
  startImmediatelyWithDialog: boolean;
  progressDialog: 'normal' | 'minimized' | 'none' | string;
  showCompleteDialog: boolean;
  duplicateAction: 'ask' | 'rename' | 'overwrite' | string;
  retryCount: number;
  retryDelaySec: number;
  userAgent: string;
  preallocate: boolean;
  useServerTime: boolean;
  antivirusCommand: string;
}

export interface ConnectionSettings {
  speedLimitEnabled: boolean;
  speedLimitKib: number;
  timeoutSec: number;
  serverLimits: ServerLimit[];
  http2: boolean;
  adaptive: boolean;
}

export interface ProxySettings {
  mode: 'system' | 'none' | 'manual' | string;
  url: string;
  username: string;
  password: string;
  bypass: string[];
}

export interface FileTypeSettings {
  captureExtensions: string[];
  skipExtensions: string[];
  skipSites: string[];
  skipAddresses: string[];
  minSizeKib: number;
}

export interface MediaSettings {
  defaultQuality: string;
  container: string;
  audioFormat: string;
  autoUpdateTools: boolean;
  impersonateFallback: boolean;
  browserFallback: boolean;
  downloadSubtitles: boolean;
  subtitleLangs: string[];
  embedThumbnail: boolean;
  sponsorblock: boolean;
}

export interface ColumnSetting {
  id: string;
  width: number;
  visible: boolean;
}

export interface Settings {
  version: number;
  language: 'auto' | 'tr' | 'en' | string;
  theme: 'light' | 'dark' | 'system' | string;
  firstRunDone: boolean;
  defaultSaveDir: string;
  perCategoryDirs: boolean;
  general: GeneralSettings;
  downloads: DownloadSettings;
  connection: ConnectionSettings;
  proxy: ProxySettings;
  fileTypes: FileTypeSettings;
  categories: Category[];
  queues: Queue[];
  siteLogins: SiteLogin[];
  media: MediaSettings;
  columns: ColumnSetting[];
}

export const MAIN_QUEUE = 'main';
export const MAX_CONNECTIONS = 16;
export const DEFAULT_CONNECTIONS = 8;

// ---------- ext.rs ----------

export interface ExtConfig {
  enabled: boolean;
  captureExtensions: string[];
  skipExtensions: string[];
  skipSites: string[];
  skipAddresses: string[];
  minSizeKib: number;
  videoPanel: boolean;
  bypassKey: string;
  forceKey: string;
  language: string;
  browserFallback: boolean;
}

export interface MediaRequest {
  pageUrl: string;
  mediaUrl: string | null;
  formatId: string | null;
  audioOnly: boolean;
  title: string | null;
  headers: [string, string][];
  cookies: CookieSpec[];
  userAgent: string | null;
  referrer: string | null;
  quick: boolean;
}

// ---------- system.rs ----------

export interface BrowserInfo {
  id: string;
  name: string;
  family: 'chromium' | 'firefox' | string;
  installed: boolean;
  hostRegistered: boolean;
  extensionPath: string;
  extensionsPage: string;
  executable: string | null;
  sandboxed: boolean;
}

export interface ToolInfo {
  installed: boolean;
  version: string | null;
  path: string | null;
  source: 'managed' | 'system' | 'none' | string;
}

export interface ToolsStatus {
  ytdlp: ToolInfo;
  ffmpeg: ToolInfo;
  deno: ToolInfo;
  busy: boolean;
  message: string | null;
}

export interface OsIntegration {
  autostart: boolean;
  urlHandler: boolean;
  kdeDownloadManager: boolean | null;
  contextMenu: boolean;
  desktop: string;
  session: string;
  traySupported: boolean;
}

// ---------- Uygulama düzeyi (src-tauri) ----------

export interface AppInfo {
  version: string;
  os: string;
  dataDir: string;
  firstRun: boolean;
}

/** Tarayıcıdan yakalanan ya da URL ile eklenen, onay bekleyen indirme. */
export interface PendingAdd {
  token: string;
  request: AddRequest;
  probe: ProbeResult | null;
  probing: boolean;
  error: string | null;
  suggestedName: string;
  suggestedDir: string;
  category: string;
  /** Aynı dosya zaten listede varsa onun kimliği. */
  duplicateOf: string | null;
}

export interface PendingMedia {
  token: string;
  request: MediaRequest;
  probe: MediaProbe | null;
  probing: boolean;
  error: string | null;
}

export interface MediaChoice {
  formatId: string;
  audioOnly: boolean;
  fileName: string;
  saveDir: string;
  subtitles: string[];
}

export interface PendingLinks {
  token: string;
  links: GrabbedLink[];
  referrer: string | null;
}

export type WindowKind =
  | 'main'
  | 'progress'
  | 'fileinfo'
  | 'complete'
  | 'addurl'
  | 'options'
  | 'scheduler'
  | 'batch'
  | 'grabber'
  | 'media'
  | 'about'
  | 'wizard'
  | 'properties'
  | 'links';

/** Boş bir istek nesnesi. */
export function emptyRequest(url = ''): RequestInfo {
  return {
    url,
    referer: null,
    headers: [],
    cookies: [],
    rawCookie: null,
    userAgent: null,
    method: null,
    postData: null,
  };
}

export function emptyAddRequest(url = ''): AddRequest {
  return {
    request: emptyRequest(url),
    fileName: null,
    saveDir: null,
    category: null,
    description: null,
    sizeHint: null,
    mimeHint: null,
    maxConnections: null,
    media: null,
    start: true,
    queueId: null,
    checksum: null,
    silent: false,
  };
}
