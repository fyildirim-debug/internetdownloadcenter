// Pencereler arası paylaşılan ayar durumu, tema ve yardımcı adlandırmalar.
import { api, EV, on } from './api';
import { setLanguage, t, has } from './i18n/index.svelte';
import type { Category, Queue, Settings } from './types';

export const app = $state<{ settings: Settings | null; ready: boolean }>({ settings: null, ready: false });

let started = false;

/** Ayarları yükler, dili ve temayı uygular, değişiklikleri dinler. */
export async function initApp(): Promise<Settings> {
  if (!started) {
    started = true;
    await on<Settings>(EV.settings, (s) => applySettings(s));
    if (typeof window !== 'undefined' && window.matchMedia) {
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        if (app.settings) applyTheme(app.settings.theme);
      });
    }
  }
  const s = await api.getSettings();
  applySettings(s);
  app.ready = true;
  return s;
}

export function applySettings(s: Settings) {
  app.settings = s;
  setLanguage(s.language);
  applyTheme(s.theme);
}

export function applyTheme(theme: string) {
  const dark =
    theme === 'dark' ||
    (theme === 'system' && typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches);
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
}

/** Ayarları kaydeder ve yerel kopyayı günceller. */
export async function saveSettings(s: Settings): Promise<Settings> {
  const saved = await api.saveSettings($state.snapshot(s) as Settings);
  applySettings(saved);
  return saved;
}

export function categoryName(c: Category | undefined | null): string {
  if (!c) return '';
  if (c.builtin && has(`cat.${c.id}`)) return t(`cat.${c.id}`);
  return c.name;
}

export function categoryNameById(id: string): string {
  const c = app.settings?.categories.find((x) => x.id === id);
  return c ? categoryName(c) : has(`cat.${id}`) ? t(`cat.${id}`) : id;
}

export function queueName(q: Queue | undefined | null): string {
  if (!q) return '';
  if (q.builtin && q.id === 'main') return t('queue.main');
  return q.name;
}

export function queueNameById(id: string | null): string {
  if (!id) return '';
  const q = app.settings?.queues.find((x) => x.id === id);
  return q ? queueName(q) : id;
}

/** Kategori kimliğine göre simge adı. */
export function categoryIcon(id: string): string {
  switch (id) {
    case 'compressed':
    case 'documents':
    case 'music':
    case 'programs':
    case 'video':
      return `cat-${id}`;
    default:
      return 'cat-general';
  }
}

/** Uzantıya göre kategori (ayarlardaki listeye göre). */
export function categoryForName(name: string): string {
  const s = app.settings;
  const i = name.lastIndexOf('.');
  const ext = i > 0 ? name.slice(i + 1).toLowerCase() : '';
  if (!s || !ext) return 'general';
  for (const c of s.categories) {
    if (c.id === 'general') continue;
    if (c.extensions.some((p) => matchExt(p, ext))) return c.id;
  }
  return 'general';
}

function matchExt(pattern: string, ext: string): boolean {
  const p = pattern.toLowerCase().replace(/^\./, '');
  if (p.includes('*')) {
    const re = new RegExp('^' + p.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*') + '$');
    return re.test(ext);
  }
  return p === ext;
}

/** WebAudio ile kısa "tamamlandı" sesi. */
export function playDoneSound() {
  try {
    const ctx = new AudioContext();
    const notes = [880, 1174.66, 1567.98];
    notes.forEach((f, i) => {
      const o = ctx.createOscillator();
      const g = ctx.createGain();
      o.type = 'sine';
      o.frequency.value = f;
      const t0 = ctx.currentTime + i * 0.11;
      g.gain.setValueAtTime(0.0001, t0);
      g.gain.exponentialRampToValueAtTime(0.18, t0 + 0.02);
      g.gain.exponentialRampToValueAtTime(0.0001, t0 + 0.32);
      o.connect(g).connect(ctx.destination);
      o.start(t0);
      o.stop(t0 + 0.35);
    });
    setTimeout(() => ctx.close(), 1200);
  } catch {
    /* ses yok */
  }
}
