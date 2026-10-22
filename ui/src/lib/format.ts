// Boyut, hız, süre ve tarih biçimlendirme yardımcıları.
import { lang, t } from './i18n/index.svelte';
import type { Status } from './types';

const UNITS = ['unit.B', 'unit.KB', 'unit.MB', 'unit.GB', 'unit.TB'] as const;

function num(v: number, digits: number): string {
  return v.toLocaleString(lang() === 'tr' ? 'tr-TR' : 'en-US', {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });
}

/** 1536 → "1,50 KB" */
export function formatSize(bytes: number | null | undefined): string {
  if (bytes == null || !isFinite(bytes)) return '';
  if (bytes < 1024) return `${bytes} ${t(UNITS[0])}`;
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  return `${num(v, v >= 100 ? 1 : 2)} ${t(UNITS[i])}`;
}

/** Bayt/sn → "1,25 MB/sn" */
export function formatSpeed(bps: number | null | undefined): string {
  if (!bps || bps <= 0) return '';
  return formatSize(bps) + t('unit.perSec');
}

/** Saniye → "1 sa 05 dk", "3 dk 12 sn", "45 sn" */
export function formatEta(sec: number | null | undefined): string {
  if (sec == null || !isFinite(sec) || sec < 0) return '';
  sec = Math.round(sec);
  const d = Math.floor(sec / 86400);
  const h = Math.floor((sec % 86400) / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = sec % 60;
  const pad = (n: number) => String(n).padStart(2, '0');
  if (d > 0) return `${d} ${t('time.d')} ${h} ${t('time.h')}`;
  if (h > 0) return `${h} ${t('time.h')} ${pad(m)} ${t('time.m')}`;
  if (m > 0) return `${m} ${t('time.m')} ${pad(s)} ${t('time.s')}`;
  return `${s} ${t('time.s')}`;
}

/** Süre (sn) → "1:02:03" ya da "4:05" */
export function formatDuration(sec: number | null | undefined): string {
  if (sec == null || !isFinite(sec)) return '';
  sec = Math.round(sec);
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = sec % 60;
  const pad = (n: number) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

/** Unix ms → yerel tarih/saat. */
export function formatDate(ms: number | null | undefined): string {
  if (!ms) return '';
  const d = new Date(ms);
  return d.toLocaleString(lang() === 'tr' ? 'tr-TR' : 'en-US', {
    year: 'numeric',
    month: 'short',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function percent(downloaded: number, size: number | null | undefined): number | null {
  if (!size || size <= 0) return null;
  return Math.min(100, (downloaded / size) * 100);
}

export function formatPercent(downloaded: number, size: number | null | undefined): string {
  const p = percent(downloaded, size);
  if (p == null) return '';
  return `${num(p, p >= 100 || p === 0 ? 0 : 2)}%`;
}

/** "kod|ayrıntı" biçimindeki hatayı okunur metne çevirir. */
export function formatError(err: string | null | undefined): string {
  if (!err) return '';
  const i = err.indexOf('|');
  const code = i >= 0 ? err.slice(0, i) : err;
  const detail = i >= 0 ? err.slice(i + 1) : '';
  const key = `err.${code}`;
  const text = t(key);
  if (text === key) return err;
  return detail ? `${text}: ${detail}` : text;
}

/** Listede gösterilen durum metni (indirilirken yüzde). */
export function statusText(status: Status, downloaded: number, size: number | null, error?: string | null): string {
  switch (status) {
    case 'downloading':
      return formatPercent(downloaded, size) || t('status.receiving');
    case 'error':
      return error ? formatError(error) : t('status.error');
    case 'paused':
      return size ? `${formatPercent(downloaded, size)} — ${t('status.paused')}` : t('status.paused');
    default:
      return t(`status.${status}`);
  }
}

export function extOf(name: string): string {
  const base = name.split(/[\\/]/).pop() ?? '';
  const i = base.lastIndexOf('.');
  return i > 0 ? base.slice(i + 1).toLowerCase() : '';
}

/** Adresten dosya adı tahmini. */
export function nameFromUrl(url: string): string {
  try {
    const u = new URL(url);
    const last = decodeURIComponent(u.pathname.split('/').filter(Boolean).pop() ?? '');
    return last || u.hostname;
  } catch {
    return url;
  }
}

export function joinPath(dir: string, name: string): string {
  if (!dir) return name;
  const sep = dir.includes('\\') && !dir.includes('/') ? '\\' : '/';
  return dir.endsWith(sep) ? dir + name : dir + sep + name;
}

export function isValidUrl(s: string): boolean {
  try {
    const u = new URL(s.trim());
    return ['http:', 'https:', 'ftp:'].includes(u.protocol);
  } catch {
    return false;
  }
}

export function hostOf(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return '';
  }
}
