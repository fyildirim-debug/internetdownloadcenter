// Basit, tepkisel çeviri katmanı. `t('anahtar', {n: 3})`.
import tr, { type Dict } from './tr';
import en from './en';

export type Lang = 'tr' | 'en';
export type Key = keyof Dict;

const dicts: Record<Lang, Dict> = { tr, en };

const state = $state<{ lang: Lang }>({ lang: detect('auto') });

/** Ayar değerinden ('auto', 'tr', 'en') gerçek dili çıkarır. */
export function detect(setting: string): Lang {
  if (setting === 'tr' || setting === 'en') return setting;
  const nav = (typeof navigator !== 'undefined' && navigator.language) || 'tr';
  return nav.toLowerCase().startsWith('tr') ? 'tr' : 'en';
}

export function setLanguage(setting: string) {
  state.lang = detect(setting);
  if (typeof document !== 'undefined') document.documentElement.lang = state.lang;
}

export function lang(): Lang {
  return state.lang;
}

export function t(key: Key | string, params?: Record<string, string | number>): string {
  const d = dicts[state.lang] as Record<string, string>;
  let s = d[key] ?? (tr as Record<string, string>)[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) s = s.replaceAll(`{${k}}`, String(v));
  }
  return s;
}

/** Var olan bir anahtar mı (dinamik anahtarlarda yedeğe düşmek için). */
export function has(key: string): boolean {
  return key in (tr as Record<string, string>);
}
