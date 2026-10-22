// IDC içerik betiği: tıklama tuşlarını bildirir, bağlantıları toplar ve videoların üstüne
// IDM tarzı "IDC ile indir" panelini yerleştirir. Bütün çerçevelerde çalışır.

(() => {
  if (window.__idcContentLoaded) return;
  window.__idcContentLoaded = true;

  const ext = typeof browser !== 'undefined' && browser.runtime ? browser : chrome;
  const msg = (key, subs) => {
    try {
      return ext.i18n.getMessage(key, subs) || key;
    } catch (_) {
      return key;
    }
  };

  function sendBg(m) {
    return new Promise((resolve) => {
      try {
        ext.runtime.sendMessage(m, (res) => {
          void ext.runtime.lastError;
          resolve(res || null);
        });
      } catch (_) {
        resolve(null);
      }
    });
  }

  // ------------------------------------------------------------ tıklama tuşları

  function reportKeys(e) {
    const a = e.target && e.target.closest ? e.target.closest('a[href]') : null;
    if (!a && !e.altKey && !e.shiftKey && !e.ctrlKey && !e.metaKey) return;
    sendBg({ cmd: 'keys', alt: e.altKey, shift: e.shiftKey, ctrl: e.ctrlKey, meta: e.metaKey });
  }
  window.addEventListener('mousedown', reportKeys, true);
  window.addEventListener('auxclick', reportKeys, true);

  // ------------------------------------------------------------ bağlantı toplama

  function collectLinks(selectionOnly) {
    const sel = window.getSelection();
    const ranges = [];
    if (selectionOnly && sel) for (let i = 0; i < sel.rangeCount; i++) ranges.push(sel.getRangeAt(i));
    const seen = new Set();
    const out = [];
    const push = (url, text) => {
      if (!url || !/^https?:/i.test(url) || seen.has(url)) return;
      seen.add(url);
      out.push({ url, text: String(text || '').trim().replace(/\s+/g, ' ').slice(0, 200) });
    };
    for (const a of document.querySelectorAll('a[href]')) {
      if (selectionOnly && !ranges.some((r) => r.intersectsNode(a))) continue;
      push(a.href, a.textContent || a.title);
    }
    if (selectionOnly && sel) {
      // Seçili düz metindeki adresler.
      const txt = sel.toString();
      for (const m of txt.matchAll(/https?:\/\/[^\s"'<>]+/g)) push(m[0], '');
    }
    return out;
  }

  ext.runtime.onMessage.addListener((m, _sender, sendResponse) => {
    if (m && m.cmd === 'collectLinks') {
      sendResponse({ links: collectLinks(!!m.selectionOnly) });
      return false;
    }
    return false;
  });

  // ------------------------------------------------------------ video paneli

  const MIN_W = 200;
  const MIN_H = 120;
  let panelCfg = null;
  let panelCfgAt = 0;
  let videos = [];
  let current = null; // panelin bağlı olduğu video
  const dismissed = new WeakSet();
  let host = null;
  let root = null;
  let btn = null;
  let menu = null;
  let hideTimer = null;
  let menuOpen = false;
  let lastUrl = location.href;

  async function loadPanelConfig(force) {
    if (!force && panelCfg && Date.now() - panelCfgAt < 60000) return panelCfg;
    const res = await sendBg({ cmd: 'panelConfig' });
    panelCfg = res || { show: false };
    panelCfgAt = Date.now();
    return panelCfg;
  }

  const SVG_NS = 'http://www.w3.org/2000/svg';

  /** Panel simgesi: mavi daire içinde aşağı ok. */
  function makeIcon() {
    const svg = document.createElementNS(SVG_NS, 'svg');
    svg.setAttribute('viewBox', '0 0 24 24');
    svg.setAttribute('width', '16');
    svg.setAttribute('height', '16');
    svg.setAttribute('aria-hidden', 'true');
    const add = (tag, attrs) => {
      const e = document.createElementNS(SVG_NS, tag);
      for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
      svg.appendChild(e);
    };
    add('circle', { cx: '12', cy: '12', r: '10', fill: '#1e88e5' });
    add('path', { d: 'M12 5v9', stroke: '#fff', 'stroke-width': '2.6', 'stroke-linecap': 'round' });
    add('path', { d: 'M7.5 11.5 12 16l4.5-4.5', fill: 'none', stroke: '#fff', 'stroke-width': '2.6', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' });
    add('path', { d: 'M7 19h10', stroke: '#fff', 'stroke-width': '2', 'stroke-linecap': 'round' });
    return svg;
  }

  const CSS = `
    :host { all: initial; }
    .wrap { position: fixed; z-index: 2147483647; font: 12px/1.3 "Segoe UI", "Noto Sans", Ubuntu, Cantarell, Arial, sans-serif;
      opacity: 0; transform: translateY(-4px); transition: opacity .15s ease, transform .15s ease; pointer-events: none; }
    .wrap.show { opacity: 1; transform: none; pointer-events: auto; }
    .bar { display: flex; align-items: stretch; border-radius: 4px; overflow: hidden; box-shadow: 0 2px 8px rgba(0,0,0,.45);
      border: 1px solid #0d47a1; background: linear-gradient(#f7fbff, #dbe9f9); }
    button { all: unset; cursor: pointer; display: flex; align-items: center; gap: 6px; padding: 5px 9px; color: #0d2a4a; font-weight: 600; white-space: nowrap; }
    button:hover { background: linear-gradient(#ffffff, #c7defa); }
    button:focus-visible { outline: 2px solid #1e88e5; outline-offset: -2px; }
    .sep { width: 1px; background: #9fbbe0; }
    .x { padding: 5px 7px; color: #43546b; font-weight: 400; }
    .menu { position: absolute; right: 0; top: calc(100% + 4px); min-width: 260px; max-width: 380px; max-height: 340px; overflow: auto;
      background: #fff; color: #1b2433; border: 1px solid #8aa8cf; border-radius: 4px; box-shadow: 0 6px 18px rgba(0,0,0,.35); display: none; padding: 3px 0; }
    .menu.open { display: block; }
    .h { padding: 5px 10px 3px; font-size: 11px; color: #5b6b82; text-transform: uppercase; letter-spacing: .04em; }
    .it { display: flex; justify-content: space-between; gap: 12px; padding: 6px 10px; cursor: pointer; }
    .it:hover, .it:focus { background: #e3efff; outline: none; }
    .it .s { color: #5b6b82; white-space: nowrap; }
    .it .n { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .note { padding: 8px 10px; color: #5b6b82; }
    .err { padding: 8px 10px; color: #b3261e; }
    .spin { display: inline-block; width: 12px; height: 12px; border: 2px solid #9fbbe0; border-top-color: #1e88e5; border-radius: 50%;
      animation: r .8s linear infinite; vertical-align: -2px; margin-right: 6px; }
    @keyframes r { to { transform: rotate(360deg); } }
    hr { border: 0; border-top: 1px solid #e1e7ef; margin: 3px 0; }
  `;

  function ensurePanel() {
    if (host) return;
    host = document.createElement('idc-video-panel');
    root = host.attachShadow({ mode: 'closed' });
    const style = document.createElement('style');
    style.textContent = CSS;
    const wrap = el('div', 'wrap');
    const bar = el('div', 'bar');
    btn = el('button', 'dl');
    btn.title = msg('panelTitle');
    btn.append(makeIcon(), el('span', null, msg('panelButton')), el('span', null, '▾'));
    const close = el('button', 'x', '✕');
    close.title = msg('panelClose');
    close.setAttribute('aria-label', msg('panelClose'));
    bar.append(btn, el('div', 'sep'), close);
    menu = el('div', 'menu');
    menu.setAttribute('role', 'menu');
    wrap.append(bar, menu);
    root.append(style, wrap);
    btn.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (menuOpen) closeMenu();
      else openMenu();
    });
    close.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (current) dismissed.add(current);
      hide(true);
    });
    wrap.addEventListener('mouseenter', () => clearTimeout(hideTimer));
    wrap.addEventListener('mouseleave', () => scheduleHide());
    // Sayfanın panel tıklamalarını görmesini engelle (oynat/duraklat tetiklenmesin).
    for (const ev of ['mousedown', 'mouseup', 'click', 'dblclick', 'pointerdown', 'pointerup']) {
      wrap.addEventListener(ev, (e) => e.stopPropagation());
    }
    attachHost();
  }

  function attachHost() {
    const parent = document.fullscreenElement && document.fullscreenElement.tagName !== 'VIDEO' ? document.fullscreenElement : document.documentElement;
    if (host && host.parentNode !== parent && parent) parent.appendChild(host);
  }

  function wrapEl() {
    return root && root.querySelector('.wrap');
  }

  function position() {
    if (!current || !host) return;
    const r = current.getBoundingClientRect();
    const w = wrapEl();
    const bw = w.offsetWidth || 150;
    let top = Math.max(r.top, 0) + 8;
    let left = Math.min(r.right, window.innerWidth) - bw - 8;
    left = Math.max(left, Math.max(r.left, 0) + 4);
    top = Math.min(top, window.innerHeight - 30);
    w.style.top = top + 'px';
    w.style.left = left + 'px';
  }

  function visibleRect(v) {
    const r = v.getBoundingClientRect();
    if (r.width < MIN_W || r.height < MIN_H) return null;
    if (r.bottom < 0 || r.right < 0 || r.top > window.innerHeight || r.left > window.innerWidth) return null;
    const cs = getComputedStyle(v);
    if (cs.visibility === 'hidden' || cs.display === 'none' || Number(cs.opacity) === 0) return null;
    return r;
  }

  function show(v) {
    if (!panelCfg || !panelCfg.show || dismissed.has(v)) return;
    ensurePanel();
    attachHost();
    if (current !== v) {
      if (menuOpen) closeMenu();
      current = v;
    }
    clearTimeout(hideTimer);
    position();
    wrapEl().classList.add('show');
  }

  function hide(force) {
    if (!host) return;
    if (menuOpen && !force) return;
    closeMenu();
    wrapEl().classList.remove('show');
  }

  function scheduleHide() {
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => hide(false), 2200);
  }

  let scanQueued = false;
  function scanVideos() {
    if (scanQueued) return;
    scanQueued = true;
    setTimeout(() => {
      scanQueued = false;
      videos = Array.from(document.querySelectorAll('video'));
      if (location.href !== lastUrl) {
        lastUrl = location.href;
        if (menuOpen) closeMenu();
      }
      if (current && !current.isConnected) {
        current = null;
        hide(true);
      }
    }, 400);
  }

  function videoAt(x, y) {
    let best = null;
    let bestArea = 0;
    for (const v of videos) {
      const r = visibleRect(v);
      if (!r) continue;
      if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) {
        const area = r.width * r.height;
        if (area > bestArea) {
          best = v;
          bestArea = area;
        }
      }
    }
    return best;
  }

  let moveQueued = false;
  async function onMove(e) {
    if (moveQueued || !videos.length) return;
    moveQueued = true;
    const { clientX: x, clientY: y } = e;
    requestAnimationFrame(async () => {
      moveQueued = false;
      const v = videoAt(x, y);
      if (v) {
        await loadPanelConfig(false);
        show(v);
      } else if (current && host && wrapEl().classList.contains('show')) {
        scheduleHide();
      }
    });
  }

  function closeMenu() {
    menuOpen = false;
    if (menu) menu.classList.remove('open');
  }

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  function item(name, size, onClick) {
    const it = el('div', 'it');
    it.tabIndex = 0;
    it.setAttribute('role', 'menuitem');
    it.appendChild(el('span', 'n', name));
    if (size) it.appendChild(el('span', 's', size));
    const go = (e) => {
      e.preventDefault();
      e.stopPropagation();
      onClick();
    };
    it.addEventListener('click', go);
    it.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') go(e);
    });
    return it;
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

  function setNote(text, cls) {
    const n = el('div', cls || 'note', text);
    menu.appendChild(n);
    return n;
  }

  async function done(res) {
    if (res && res.type === 'accepted') {
      menu.textContent = '';
      setNote('✓ ' + msg('sentToIdc'));
      setTimeout(() => {
        closeMenu();
        hide(true);
      }, 1200);
    } else {
      menu.textContent = '';
      setNote(res && res.kind === 'missing' ? msg('appNotFound') : (res && (res.message || res.error)) || msg('genericError'), 'err');
    }
  }

  async function openMenu() {
    if (!current) return;
    menuOpen = true;
    menu.textContent = '';
    menu.classList.add('open');
    const pageUrl = location.href;
    const src = current.currentSrc || current.src || '';
    const blobSrc = /^(blob|mediasource):/i.test(src);
    const info = await sendBg({ cmd: 'panelInfo', pageUrl, blobSrc });
    if (!menuOpen) return;
    if (!info) {
      setNote(msg('genericError'), 'err');
      return;
    }
    if (!info.appOk) {
      setNote(msg('appNotFound'), 'err');
      return;
    }
    let direct = info.direct || [];
    if (/^https?:/i.test(src) && !direct.some((d) => d.url === src)) {
      direct = [{ url: src, fileName: decodeURIComponent(src.split('?')[0].split('/').pop() || 'video'), size: 0 }].concat(direct);
    }
    const title = document.title;
    if (direct.length) {
      menu.appendChild(el('div', 'h', msg('directFiles')));
      for (const d of direct.slice(0, 8)) {
        menu.appendChild(
          item(d.fileName || d.url, fmtBytes(d.size), async () => {
            menu.textContent = '';
            setNote(msg('sending'));
            done(await sendBg({ cmd: 'downloadUrl', url: d.url, pageUrl, fileName: d.fileName }));
          })
        );
      }
    }
    const wantProbe = info.probe || !direct.length;
    if (!wantProbe) return;
    if (direct.length) menu.appendChild(el('hr'));
    const loading = el('div', 'note');
    loading.append(el('span', 'spin'), document.createTextNode(msg('loadingQualities')));
    menu.appendChild(loading);
    const streamUrl = (info.streams && info.streams[0] && info.streams[0].url) || null;
    const res = await sendBg({ cmd: 'probe', pageUrl, mediaUrl: info.probe && !/youtube|youtu\.be/.test(location.hostname) ? streamUrl : null });
    if (!menuOpen) return;
    loading.remove();
    const probe = res && res.probe;
    if (!probe || !probe.formats || !probe.formats.length) {
      setNote(res && res.kind === 'missing' ? msg('appNotFound') : (res && res.error) || msg('noFormats'), res && res.error ? 'err' : 'note');
    } else {
      menu.appendChild(el('div', 'h', probe.title ? msg('qualitiesFor', [probe.title.slice(0, 40)]) : msg('qualities')));
      const vids = probe.formats.filter((f) => !f.audioOnly).slice(0, 12);
      const auds = probe.formats.filter((f) => f.audioOnly).slice(0, 3);
      for (const f of vids.concat(auds)) {
        menu.appendChild(
          item(f.label, fmtBytes(f.filesize), async () => {
            menu.textContent = '';
            setNote(msg('sending'));
            done(await sendBg({ cmd: 'mediaDownload', pageUrl, mediaUrl: streamUrl, formatId: f.formatId, audioOnly: !!f.audioOnly, title: probe.title || title }));
          })
        );
      }
    }
    menu.appendChild(el('hr'));
    menu.appendChild(
      item(msg('quickDownload'), '', async () => {
        menu.textContent = '';
        setNote(msg('sending'));
        done(await sendBg({ cmd: 'mediaDownload', pageUrl, mediaUrl: streamUrl, quick: true, title }));
      })
    );
    menu.appendChild(
      item(msg('chooseQuality'), '', async () => {
        menu.textContent = '';
        setNote(msg('sending'));
        done(await sendBg({ cmd: 'mediaDownload', pageUrl, mediaUrl: streamUrl, quick: false, title }));
      })
    );
  }

  function onScrollOrResize() {
    if (current && host && wrapEl().classList.contains('show')) {
      if (!visibleRect(current)) hide(true);
      else position();
    }
  }

  function initPanel() {
    scanVideos();
    const mo = new MutationObserver((muts) => {
      for (const m of muts) {
        for (const n of m.addedNodes) {
          if (n.nodeType === 1 && (n.tagName === 'VIDEO' || (n.querySelector && n.querySelector('video')))) {
            scanVideos();
            return;
          }
        }
        if (m.removedNodes.length && current && !current.isConnected) {
          scanVideos();
          return;
        }
      }
    });
    mo.observe(document.documentElement, { childList: true, subtree: true });
    document.addEventListener('mousemove', onMove, { capture: true, passive: true });
    window.addEventListener('scroll', onScrollOrResize, { capture: true, passive: true });
    window.addEventListener('resize', onScrollOrResize, { passive: true });
    document.addEventListener('fullscreenchange', () => {
      attachHost();
      setTimeout(position, 50);
    });
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && menuOpen) closeMenu();
    });
    document.addEventListener(
      'click',
      (e) => {
        if (menuOpen && host && !e.composedPath().includes(host)) closeMenu();
      },
      true
    );
    // SPA gezintileri (YouTube ve benzerleri).
    window.addEventListener('yt-navigate-finish', () => {
      closeMenu();
      scanVideos();
    });
    window.addEventListener('popstate', scanVideos);
    // Video öğesinin yeri değişirse panel peşinden gitsin.
    setInterval(() => {
      if (current && host && wrapEl().classList.contains('show')) position();
      if (location.href !== lastUrl) scanVideos();
    }, 1000);
    loadPanelConfig(true);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', initPanel, { once: true });
  else initPanel();
})();
