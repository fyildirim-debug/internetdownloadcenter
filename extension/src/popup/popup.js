// Araç çubuğu açılır penceresi.

let overrideMessages = null;

function L(key, subs) {
  if (overrideMessages && overrideMessages[key]) {
    let s = overrideMessages[key].message;
    if (subs && subs.length) s = s.replace(/\$ARG\$/gi, subs[0]);
    return s;
  }
  return t(key, subs);
}

async function loadLanguage(lang) {
  if (!lang || lang === 'auto') return;
  const ui = (api.i18n.getUILanguage() || 'en').slice(0, 2);
  if (ui === lang) return;
  try {
    const res = await fetch(api.runtime.getURL(`_locales/${lang}/messages.json`));
    overrideMessages = await res.json();
  } catch (_) {
    overrideMessages = null;
  }
}

function bg(msg) {
  return new Promise((resolve) => {
    api.runtime.sendMessage(msg, (res) => {
      void api.runtime.lastError;
      resolve(res || null);
    });
  });
}

function applyI18n() {
  for (const el of document.querySelectorAll('[data-i18n]')) el.textContent = L(el.dataset.i18n);
}

function fmtBytes(n) {
  if (!n || n <= 0) return '';
  const u = ['B', 'KB', 'MB', 'GB'];
  let i = 0;
  while (n >= 1024 && i < u.length - 1) {
    n /= 1024;
    i++;
  }
  return (i === 0 ? n.toFixed(0) : n.toFixed(n >= 100 ? 0 : 1)) + ' ' + u[i];
}

let tabId = -1;
let st = null;

async function render() {
  // ?tabId= ile belirli bir sekme için açılabilir (test ve tam sayfa görünümü).
  const forced = Number(new URLSearchParams(location.search).get('tabId'));
  if (forced > 0) tabId = forced;
  else {
    const [tab] = await api.tabs.query({ active: true, currentWindow: true });
    tabId = tab ? tab.id : -1;
  }
  st = await bg({ cmd: 'status', tabId });
  if (!st) return;
  await loadLanguage(st.config && st.config.language);
  applyI18n();

  document.getElementById('ver').textContent = 'v' + st.version + (st.appVersion ? ' · IDC ' + st.appVersion : '');
  const status = document.getElementById('status');
  status.classList.toggle('ok', st.appOk);
  document.getElementById('statusText').textContent = st.appOk ? L('appConnected') : L('appNotFound');
  document.getElementById('missing').hidden = st.appOk;
  document.getElementById('perm').hidden = st.hostPerm !== false;

  const enabled = document.getElementById('enabled');
  enabled.checked = st.localEnabled && (!st.config || st.config.enabled !== false);
  enabled.disabled = !!(st.config && st.config.enabled === false);

  const siteRow = document.getElementById('siteRow');
  siteRow.hidden = !st.host;
  document.getElementById('siteLabel').textContent = L('disableOnSite', [st.host || '']);
  document.getElementById('siteDisabled').checked = st.siteDisabled;

  // Chromium'da "Her dosyanın nereye kaydedileceğini sor" açıksa yakalama çakışabilir.
  document.getElementById('hint').hidden = st.isFirefox;

  const list = document.getElementById('mediaList');
  list.textContent = '';
  const media = st.media || [];
  document.getElementById('mediaCount').textContent = media.length ? String(media.length) : '';
  document.getElementById('mediaEmpty').hidden = media.length > 0 || st.mediaSite;
  for (const m of media) {
    const li = document.createElement('li');
    li.tabIndex = 0;
    const kind = document.createElement('span');
    kind.className = 'kind';
    kind.textContent = m.kind === 'direct' ? m.ext : m.kind;
    const name = document.createElement('span');
    name.className = 'name';
    name.textContent = m.fileName || m.url;
    name.title = m.url;
    const size = document.createElement('span');
    size.className = 'size';
    size.textContent = fmtBytes(m.size);
    li.append(kind, name, size);
    const go = async () => {
      li.style.opacity = '.5';
      const res =
        m.kind === 'direct'
          ? await bg({ cmd: 'downloadUrl', url: m.url, pageUrl: st.pageUrl, fileName: m.fileName, tabId })
          : await bg({ cmd: 'mediaDownload', pageUrl: st.pageUrl, mediaUrl: m.url, quick: false, tabId });
      li.style.opacity = '';
      if (res && res.type === 'accepted') window.close();
      else size.textContent = L('genericError');
    };
    li.addEventListener('click', go);
    li.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') go();
    });
    list.appendChild(li);
  }
  const probeBtn = document.getElementById('probePage');
  probeBtn.hidden = !st.appOk || !/^https?:/i.test(st.pageUrl || '');
  probeBtn.textContent = st.mediaSite ? L('downloadPageVideo') : L('probePage');
}

document.getElementById('enabled').addEventListener('change', async (e) => {
  await bg({ cmd: 'setEnabled', enabled: e.target.checked });
});

document.getElementById('siteDisabled').addEventListener('change', async (e) => {
  await bg({ cmd: 'setSiteDisabled', host: st && st.host, disabled: e.target.checked });
});

document.getElementById('open').addEventListener('click', async () => {
  const res = await bg({ cmd: 'show' });
  if (res && res.type === 'accepted') window.close();
  else render();
});

document.getElementById('retry').addEventListener('click', async (e) => {
  e.target.disabled = true;
  await bg({ cmd: 'refresh' });
  e.target.disabled = false;
  render();
});

document.getElementById('grant').addEventListener('click', async () => {
  try {
    await api.permissions.request({ origins: ['<all_urls>'] });
  } catch (_) {
    /* kullanıcı reddetti */
  }
  render();
});

document.getElementById('probePage').addEventListener('click', async (e) => {
  e.target.disabled = true;
  const res = await bg({ cmd: 'mediaDownload', pageUrl: st.pageUrl, quick: false, tabId });
  e.target.disabled = false;
  if (res && res.type === 'accepted') window.close();
});

render();
bg({ cmd: 'refresh' }).then(render);
