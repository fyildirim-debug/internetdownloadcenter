// Chromium ve Firefox arasındaki ince uyum katmanı.
// Chromium MV3'te chrome.* API'leri geri çağırma verilmezse Promise döner; Firefox'ta browser.* zaten Promise tabanlıdır.

/* eslint-disable no-unused-vars */
const IS_FIREFOX =
  typeof globalThis.browser !== 'undefined' &&
  !!globalThis.browser.runtime &&
  typeof globalThis.browser.runtime.getBrowserInfo === 'function';

const api = IS_FIREFOX ? globalThis.browser : globalThis.chrome;

// Bağlam menüsü API'si Firefox'ta `menus` adıyla da bulunur.
const menusApi = api.contextMenus || api.menus;

// Oturum deposu (Chromium 102+, Firefox 115+); yoksa yerel depoya düşer.
const sessionStore = (api.storage && api.storage.session) || api.storage.local;

/** Tarayıcı adını kısa kimlik olarak döner: chrome, edge, brave, opera, vivaldi, chromium, firefox, zen ... */
async function detectBrowser() {
  try {
    if (IS_FIREFOX) {
      const info = await api.runtime.getBrowserInfo();
      const n = String(info.name || 'firefox').toLowerCase();
      if (n.includes('zen')) return 'zen';
      if (n.includes('librewolf')) return 'librewolf';
      if (n.includes('floorp')) return 'floorp';
      if (n.includes('waterfox')) return 'waterfox';
      return 'firefox';
    }
    if (navigator.brave && typeof navigator.brave.isBrave === 'function') return 'brave';
    const brands = (navigator.userAgentData && navigator.userAgentData.brands) || [];
    const names = brands.map((b) => String(b.brand).toLowerCase());
    if (names.some((b) => b.includes('edge'))) return 'edge';
    if (names.some((b) => b.includes('opera'))) return 'opera';
    if (names.some((b) => b.includes('vivaldi')) || /Vivaldi/i.test(navigator.userAgent)) return 'vivaldi';
    if (names.some((b) => b.includes('google chrome'))) return 'chrome';
    if (/OPR\//.test(navigator.userAgent)) return 'opera';
    if (/Edg\//.test(navigator.userAgent)) return 'edge';
    return 'chromium';
  } catch (_) {
    return IS_FIREFOX ? 'firefox' : 'chromium';
  }
}

function t(key, subs) {
  try {
    return api.i18n.getMessage(key, subs) || key;
  } catch (_) {
    return key;
  }
}
