// Tauri komut ve olay katmanı. Tauri dışında (tarayıcıda `vite dev`) sahte arka uca düşer.
import type {
  AddRequest,
  AppInfo,
  BrowserInfo,
  DownloadDetail,
  DownloadPatch,
  DownloadView,
  GrabbedLink,
  MediaChoice,
  MediaProbe,
  OsIntegration,
  PendingAdd,
  PendingLinks,
  PendingMedia,
  ProbeResult,
  ProgressTick,
  Queue,
  Settings,
  ToolsStatus,
  WindowKind,
} from './types';

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

type Unlisten = () => void;

let mockMod: typeof import('./mock') | null = null;
async function mock() {
  if (!mockMod) mockMod = await import('./mock');
  return mockMod;
}

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  return (await mock()).invoke(cmd, args) as Promise<T>;
}

export async function on<T>(event: string, cb: (payload: T) => void): Promise<Unlisten> {
  if (isTauri) {
    const { listen } = await import('@tauri-apps/api/event');
    return listen<T>(event, (e) => cb(e.payload));
  }
  return (await mock()).listen(event, cb as (p: unknown) => void);
}

/** Olay adları. */
export const EV = {
  progress: 'idc://progress',
  changed: 'idc://changed',
  removed: 'idc://removed',
  settings: 'idc://settings',
  pending: 'idc://pending',
  pendingMedia: 'idc://pending-media',
  tools: 'idc://tools',
} as const;

/** Tip güvenli komut sarmalayıcıları. Argüman adları camelCase (Tauri snake_case'e çevirir). */
export const api = {
  listDownloads: () => call<DownloadView[]>('list_downloads'),
  getDownload: (id: string) => call<DownloadDetail>('get_download', { id }),
  probeUrl: (url: string, headers: [string, string][] | null = null) =>
    call<ProbeResult>('probe_url', { url, headers }),
  addDownload: (req: AddRequest) => call<string>('add_download', { req }),
  addMany: (reqs: AddRequest[]) => call<string[]>('add_many', { reqs }),
  startDownloads: (ids: string[]) => call<void>('start_downloads', { ids }),
  pauseDownloads: (ids: string[]) => call<void>('pause_downloads', { ids }),
  removeDownloads: (ids: string[], deleteFiles: boolean) =>
    call<void>('remove_downloads', { ids, deleteFiles }),
  startAll: () => call<void>('start_all'),
  stopAll: () => call<void>('stop_all'),
  deleteCompleted: () => call<void>('delete_completed'),
  redownload: (id: string) => call<void>('redownload', { id }),
  updateDownload: (id: string, patch: DownloadPatch) => call<void>('update_download', { id, patch }),
  openFile: (id: string) => call<void>('open_file', { id }),
  openWith: (id: string) => call<void>('open_with', { id }),
  openFolder: (id: string) => call<void>('open_folder', { id }),
  getSettings: () => call<Settings>('get_settings'),
  saveSettings: (settings: Settings) => call<Settings>('save_settings', { settings }),
  mediaProbe: (url: string, headers: [string, string][] | null = null) =>
    call<MediaProbe>('media_probe', { url, headers }),
  toolsStatus: () => call<ToolsStatus>('tools_status'),
  toolsUpdate: () => call<ToolsStatus>('tools_update'),
  browsersStatus: () => call<BrowserInfo[]>('browsers_status'),
  browsersInstall: (ids: string[] | null) => call<BrowserInfo[]>('browsers_install', { ids }),
  browserOpenExtensions: (id: string) => call<void>('browser_open_extensions', { id }),
  osIntegrationStatus: () => call<OsIntegration>('os_integration_status'),
  osIntegrationApply: (opts: OsIntegration) => call<OsIntegration>('os_integration_apply', { opts }),
  openWindow: (kind: WindowKind, params: Record<string, string> | null = null) =>
    call<void>('open_window', { kind, params }),
  startQueue: (queueId: string) => call<void>('start_queue', { queueId }),
  stopQueue: (queueId: string) => call<void>('stop_queue', { queueId }),
  saveQueue: (queue: Queue) => call<void>('save_queue', { queue }),
  deleteQueue: (id: string) => call<void>('delete_queue', { id }),
  moveInQueue: (queueId: string, ids: string[], delta: number) =>
    call<void>('move_in_queue', { queueId, ids, delta }),
  batchPreview: (pattern: string, from: string, to: string, step: number, padding: number) =>
    call<string[]>('batch_preview', { pattern, from, to, step, padding }),
  grabLinks: (url: string, depth: number, extensions: string[]) =>
    call<GrabbedLink[]>('grab_links', { url, depth, extensions }),
  appInfo: () => call<AppInfo>('app_info'),
  quitApp: () => call<void>('quit_app'),
  /** Onaylanmamış indirme oluşturur ve dosya bilgisini arka planda yoklar; dönen jetonla `fileinfo` açılır. */
  createPending: (req: AddRequest) => call<string>('create_pending', { req }),
  getPending: (token: string) => call<PendingAdd>('get_pending', { token }),
  confirmPending: (token: string, req: AddRequest) => call<string>('confirm_pending', { token, req }),
  cancelPending: (token: string) => call<void>('cancel_pending', { token }),
  getPendingMedia: (token: string) => call<PendingMedia>('get_pending_media', { token }),
  confirmPendingMedia: (token: string, choice: MediaChoice) =>
    call<string>('confirm_pending_media', { token, choice }),
  getPendingLinks: (token: string) => call<PendingLinks>('get_pending_links', { token }),
  setGlobalSpeedLimit: (kib: number | null) => call<void>('set_global_speed_limit', { kib }),
  exportList: (path: string) => call<void>('export_list', { path }),
  importList: (path: string) => call<number>('import_list', { path }),
};

// ---------- Pencere yardımcıları ----------

export async function closeWindow() {
  if (isTauri) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  } else if (window.opener) {
    window.close();
  } else {
    location.hash = '#/main';
  }
}

export async function setTitle(title: string) {
  document.title = title;
  if (isTauri) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().setTitle(title);
  }
}

export async function resizeWindow(width: number, height: number) {
  if (isTauri) {
    const { getCurrentWindow, LogicalSize } = await import('@tauri-apps/api/window');
    await getCurrentWindow().setSize(new LogicalSize(width, height));
  } else if (window.opener) {
    window.resizeTo(width, height + (window.outerHeight - window.innerHeight));
  }
}

/** Pencereyi `.window` içeriğinin doğal yüksekliğine göre boyutlandırır. */
export async function fitToContent(width: number, minHeight = 120, maxHeight = 760) {
  const el = document.querySelector('.window') as HTMLElement | null;
  if (!el) return;
  const prev = el.style.height;
  el.style.height = 'auto';
  const h = Math.ceil(el.getBoundingClientRect().height);
  el.style.height = prev;
  if (h > 0) await resizeWindow(width, Math.min(maxHeight, Math.max(minHeight, h)));
}

/**
 * İçerik değiştikçe pencereyi yeniden boyutlandırır (diyalog pencereleri için).
 * Dönen işlev gözlemi durdurur.
 */
export function autoFit(width: number, minHeight = 120, maxHeight = 760): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let last = '';
  const run = () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      const el = document.querySelector('.window') as HTMLElement | null;
      if (!el) return;
      const prev = el.style.height;
      el.style.height = 'auto';
      const h = Math.ceil(el.getBoundingClientRect().height);
      el.style.height = prev;
      const key = `${width}x${h}`;
      if (h > 0 && key !== last) {
        last = key;
        resizeWindow(width, Math.min(maxHeight, Math.max(minHeight, h)));
      }
    }, 30);
  };
  const mo = new MutationObserver(run);
  mo.observe(document.body, { subtree: true, childList: true, characterData: false, attributes: true, attributeFilter: ['class', 'style', 'hidden', 'open'] });
  run();
  return () => {
    mo.disconnect();
    if (timer) clearTimeout(timer);
  };
}

/** Klasör seçme penceresi. */
export async function pickFolder(defaultPath?: string): Promise<string | null> {
  if (isTauri) {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const r = await open({ directory: true, multiple: false, defaultPath });
    return typeof r === 'string' ? r : null;
  }
  return prompt('Klasör yolu', defaultPath ?? '') || null;
}

export async function pickFile(save: boolean, defaultPath?: string): Promise<string | null> {
  if (isTauri) {
    const d = await import('@tauri-apps/plugin-dialog');
    const filters = [{ name: 'IDC', extensions: ['json'] }];
    const r = save
      ? await d.save({ defaultPath, filters })
      : await d.open({ multiple: false, defaultPath, filters });
    return typeof r === 'string' ? r : null;
  }
  return prompt('Dosya yolu', defaultPath ?? '') || null;
}

export async function readClipboard(): Promise<string> {
  try {
    if (isTauri) {
      const { readText } = await import('@tauri-apps/plugin-clipboard-manager');
      return (await readText()) ?? '';
    }
    return await navigator.clipboard.readText();
  } catch {
    return '';
  }
}

export async function writeClipboard(text: string) {
  try {
    if (isTauri) {
      const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
      await writeText(text);
      return;
    }
    await navigator.clipboard.writeText(text);
  } catch {
    /* pano erişilemiyor */
  }
}

/** Basit evet/hayır sorusu (Tauri'de yerel ileti kutusu). */
export async function ask(message: string, title = 'Internet Download Center'): Promise<boolean> {
  if (isTauri) {
    const { ask } = await import('@tauri-apps/plugin-dialog');
    return ask(message, { title, kind: 'warning' });
  }
  return confirm(message);
}

export async function promptText(message: string, initial = ''): Promise<string | null> {
  // Tauri'de WebView prompt() Linux/Windows'ta çalışır; ayrıca bileşen gerekmez.
  const r = window.prompt(message, initial);
  return r == null ? null : r;
}

export function hashParams(): URLSearchParams {
  const h = location.hash.replace(/^#\/?/, '');
  const q = h.indexOf('?');
  return new URLSearchParams(q >= 0 ? h.slice(q + 1) : '');
}
