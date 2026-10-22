<script lang="ts">
  // Ana pencere: menü, araç çubuğu, kategori ağacı, indirme listesi, durum çubuğu.
  import { onMount } from 'svelte';
  import MenuBar from '../lib/components/MenuBar.svelte';
  import Toolbar from '../lib/components/Toolbar.svelte';
  import CategoryTree from '../lib/components/CategoryTree.svelte';
  import DownloadList, { DEFAULT_COLUMNS, type ColId, type Column } from '../lib/components/DownloadList.svelte';
  import StatusBar from '../lib/components/StatusBar.svelte';
  import ContextMenu from '../lib/components/ContextMenu.svelte';
  import ConfirmDialog from '../lib/components/ConfirmDialog.svelte';
  import { api, EV, on, pickFile, readClipboard, writeClipboard } from '../lib/api';
  import { t, lang } from '../lib/i18n/index.svelte';
  import { app, queueName, saveSettings } from '../lib/state.svelte';
  import { SEP, type Filter, type MenuItem, type ToolButton } from '../lib/menu';
  import type { DownloadView, ProgressTick, Settings } from '../lib/types';
  import { isValidUrl as validUrl } from '../lib/format';

  let rows = $state<DownloadView[]>([]);
  let filter = $state<Filter>({ kind: 'all' });
  let selected = $state<string[]>([]);
  let sortBy = $state<ColId>('added');
  let sortDir = $state<1 | -1>(-1);
  let columns = $state<Column[]>(structuredClone(DEFAULT_COLUMNS));
  let ctx = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);
  let confirmBox = $state<{ message: string; check?: string; resolve: (ok: boolean, c: boolean) => void } | null>(null);
  let treeWidth = $state(190);

  // Görünüm tercihleri (yalnız bu bilgisayarda).
  function loadView() {
    try {
      return JSON.parse(localStorage.getItem('idc-view') ?? '{}');
    } catch {
      return {};
    }
  }
  const saved = loadView();
  let showTree = $state<boolean>(saved.showTree ?? true);
  let showToolbar = $state<boolean>(saved.showToolbar ?? true);
  let showStatus = $state<boolean>(saved.showStatus ?? true);
  if (saved.treeWidth) treeWidth = saved.treeWidth;
  if (saved.sortBy) sortBy = saved.sortBy;
  if (saved.sortDir) sortDir = saved.sortDir;
  $effect(() => {
    try {
      localStorage.setItem('idc-view', JSON.stringify({ showTree, showToolbar, showStatus, treeWidth, sortBy, sortDir }));
    } catch {
      /* depolama yok */
    }
  });

  const byId = new Map<string, DownloadView>();

  function upsert(v: DownloadView) {
    const cur = byId.get(v.id);
    if (cur) Object.assign(cur, v);
    else {
      rows.push(v);
      byId.set(v.id, rows[rows.length - 1]);
    }
  }

  function applyTicks(ticks: ProgressTick[]) {
    for (const tk of ticks) {
      const r = byId.get(tk.id);
      if (!r) continue;
      r.status = tk.status;
      r.size = tk.size;
      r.downloaded = tk.downloaded;
      r.speed = tk.speed;
      r.eta = tk.eta;
      r.connections = tk.connections;
    }
  }

  function removeIds(ids: string[]) {
    const set = new Set(ids);
    rows = rows.filter((r) => !set.has(r.id));
    ids.forEach((id) => byId.delete(id));
    selected = selected.filter((s) => !set.has(s));
  }

  async function reload() {
    const list = await api.listDownloads();
    rows = list;
    byId.clear();
    rows.forEach((r) => byId.set(r.id, r));
  }

  onMount(() => {
    const offs: (() => void)[] = [];
    reload();
    on<ProgressTick[]>(EV.progress, applyTicks).then((u) => offs.push(u));
    on<DownloadView>(EV.changed, upsert).then((u) => offs.push(u));
    on<string[]>(EV.removed, removeIds).then((u) => offs.push(u));
    return () => offs.forEach((f) => f());
  });

  $effect(() => {
    const s = app.settings;
    if (!s) return;
    if (s.columns?.length) {
      const map = new Map(s.columns.map((c) => [c.id, c]));
      columns = DEFAULT_COLUMNS.map((d) => {
        const c = map.get(d.id);
        return c ? { id: d.id, width: c.width || d.width, visible: d.id === 'fileName' ? true : c.visible } : { ...d };
      });
    }
  });

  async function persistColumns() {
    if (!app.settings) return;
    const s = $state.snapshot(app.settings) as Settings;
    s.columns = columns.map((c) => ({ id: c.id, width: Math.round(c.width), visible: c.visible }));
    await saveSettings(s);
  }

  // ---------- Süzme ve sıralama ----------

  function matches(r: DownloadView, f: Filter): boolean {
    switch (f.kind) {
      case 'all':
        return true;
      case 'category':
        return r.category === f.id;
      case 'unfinished':
        return r.status !== 'completed';
      case 'finished':
        return r.status === 'completed';
      case 'queue':
        return f.id === '*' ? r.queueId != null : r.queueId === f.id;
    }
  }

  const sortKey: Record<ColId, (r: DownloadView) => string | number> = {
    fileName: (r) => r.fileName.toLowerCase(),
    queue: (r) => r.queueId ?? '',
    size: (r) => r.size ?? -1,
    status: (r) => (r.status === 'completed' ? 200 : r.size ? (r.downloaded / r.size) * 100 : 0),
    timeLeft: (r) => r.eta ?? Number.MAX_SAFE_INTEGER,
    rate: (r) => r.speed,
    lastTry: (r) => r.lastTryAt ?? r.addedAt,
    description: (r) => r.description.toLowerCase(),
    added: (r) => r.addedAt,
    saveTo: (r) => r.saveDir.toLowerCase(),
  };

  const visible = $derived.by(() => {
    const k = sortKey[sortBy];
    const coll = new Intl.Collator(lang() === 'tr' ? 'tr' : 'en', { numeric: true });
    return rows
      .filter((r) => matches(r, filter))
      .sort((a, b) => {
        const x = k(a);
        const y = k(b);
        const c = typeof x === 'string' && typeof y === 'string' ? coll.compare(x, y) : (x as number) - (y as number);
        return c * sortDir;
      });
  });

  const counts = $derived.by(() => {
    const c: Record<string, number> = {};
    for (const r of rows) {
      if (r.status === 'completed') continue;
      c['unfinished:'] = (c['unfinished:'] ?? 0) + 1;
    }
    return c;
  });

  const activeRows = $derived(rows.filter((r) => r.status === 'downloading' || r.status === 'connecting' || r.status === 'processing'));
  const totalSpeed = $derived(activeRows.reduce((a, r) => a + (r.speed || 0), 0));
  const sel = $derived(selected.map((id) => byId.get(id)).filter((r): r is DownloadView => !!r));
  const canResume = $derived(sel.some((r) => r.status === 'paused' || r.status === 'error' || r.status === 'queued'));
  const canStop = $derived(sel.some((r) => r.status === 'downloading' || r.status === 'connecting' || r.status === 'queued' || r.status === 'processing'));
  const currentQueue = $derived(filter.kind === 'queue' && filter.id && filter.id !== '*' ? filter.id : 'main');
  const limitKib = $derived(app.settings?.connection.speedLimitEnabled ? app.settings.connection.speedLimitKib : null);

  // ---------- Eylemler ----------

  function ask(message: string, check?: string): Promise<{ ok: boolean; checked: boolean }> {
    return new Promise((res) => {
      confirmBox = {
        message,
        check,
        resolve: (ok, checked) => {
          confirmBox = null;
          res({ ok, checked });
        },
      };
    });
  }

  const ids = () => sel.map((r) => r.id);

  async function resume() {
    const l = sel.filter((r) => r.status !== 'completed').map((r) => r.id);
    if (l.length) await api.startDownloads(l);
  }
  async function stop() {
    if (sel.length) await api.pauseDownloads(ids());
  }
  async function del(withFiles = false) {
    if (!sel.length) return;
    const r = await ask(t('confirm.delete', { n: sel.length }), t('confirm.deleteFiles'));
    if (!r.ok) return;
    const list = ids();
    await api.removeDownloads(list, withFiles || r.checked);
    removeIds(list);
  }
  async function deleteCompleted() {
    const r = await ask(t('confirm.deleteCompleted'));
    if (r.ok) {
      await api.deleteCompleted();
      removeIds(rows.filter((x) => x.status === 'completed').map((x) => x.id));
    }
  }
  function openItem(id: string) {
    const r = byId.get(id);
    if (!r) return;
    if (r.status === 'completed') api.openWindow('properties', { id });
    else api.openWindow('progress', { id });
  }
  async function toggleLimit() {
    if (!app.settings) return;
    await api.setGlobalSpeedLimit(limitKib != null ? null : app.settings.connection.speedLimitKib || 1024);
  }
  async function moveToQueue(qid: string | null) {
    for (const r of sel) {
      await api.updateDownload(r.id, qid ? { queueId: qid } : { clearQueue: true });
    }
  }
  async function addFromClipboard() {
    const text = await readClipboard();
    const urls = text.split(/\s+/).filter(validUrl);
    if (urls.length === 1) api.openWindow('addurl', { url: urls[0] });
    else if (urls.length > 1) {
      const { emptyAddRequest } = await import('../lib/types');
      await api.addMany(urls.map((u) => ({ ...emptyAddRequest(u), start: false })));
    } else api.openWindow('addurl');
  }
  async function exportList() {
    const p = await pickFile(true, 'idc-liste.json');
    if (p) await api.exportList(p);
  }
  async function importList() {
    const p = await pickFile(false);
    if (p) {
      await api.importList(p);
      reload();
    }
  }
  async function setTheme(theme: string) {
    if (!app.settings) return;
    const s = $state.snapshot(app.settings) as Settings;
    s.theme = theme;
    await saveSettings(s);
  }
  async function setLang(l: string) {
    if (!app.settings) return;
    const s = $state.snapshot(app.settings) as Settings;
    s.language = l;
    await saveSettings(s);
  }

  // ---------- Menüler ----------

  const queueItems = (fn: (id: string) => void): MenuItem[] =>
    (app.settings?.queues ?? []).map((q) => ({ label: queueName(q), icon: 'queue', action: () => fn(q.id) }));

  function selectionMenu(): MenuItem[] {
    const one = sel.length === 1 ? sel[0] : null;
    const done = one?.status === 'completed';
    return [
      { label: t('menu.open'), bold: done, disabled: !done, action: () => one && api.openFile(one.id) },
      { label: t('menu.openWith'), disabled: !done, action: () => one && api.openWith(one.id) },
      { label: t('menu.openFolder'), disabled: !one, icon: 'folder-open', action: () => one && api.openFolder(one.id) },
      SEP,
      { label: t('menu.resume'), icon: 'play', disabled: !canResume, action: resume },
      { label: t('menu.stop'), icon: 'stop', disabled: !canStop, action: stop },
      { label: t('menu.redownload'), icon: 'refresh', disabled: !one, action: () => one && api.redownload(one.id) },
      SEP,
      { label: t('menu.moveToQueue'), disabled: !sel.length, submenu: queueItems((q) => moveToQueue(q)) },
      { label: t('menu.removeFromQueue'), disabled: !sel.some((r) => r.queueId), action: () => moveToQueue(null) },
      SEP,
      { label: t('menu.copyUrl'), icon: 'link', disabled: !sel.length, action: () => writeClipboard(sel.map((r) => r.url).join('\n')) },
      { label: t('menu.delete'), icon: 'x', shortcut: 'Del', disabled: !sel.length, action: () => del() },
      { label: t('menu.deleteWithFile'), disabled: !sel.length, action: () => del(true) },
      SEP,
      { label: t('menu.properties'), disabled: !one, action: () => one && api.openWindow('properties', { id: one.id }) },
    ];
  }

  function emptyMenu(): MenuItem[] {
    return [
      { label: t('menu.addUrl'), icon: 'plus', shortcut: 'Ins', action: () => api.openWindow('addurl') },
      { label: t('menu.addBatch'), action: () => api.openWindow('batch') },
      SEP,
      { label: t('menu.selectAll'), shortcut: 'Ctrl+A', disabled: !visible.length, action: () => (selected = visible.map((r) => r.id)) },
      { label: t('menu.deleteCompleted'), action: deleteCompleted },
    ];
  }

  const menus = $derived([
    {
      label: t('menu.tasks'),
      items: [
        { label: t('menu.addUrl'), icon: 'plus', shortcut: 'Ins', action: () => api.openWindow('addurl') },
        { label: t('menu.addBatch'), action: () => api.openWindow('batch') },
        { label: t('menu.addFromClipboard'), action: addFromClipboard },
        { label: t('menu.mediaUrl'), icon: 'cat-video', action: () => api.openWindow('media') },
        { label: t('menu.grabber'), icon: 'grabber', action: () => api.openWindow('grabber') },
        SEP,
        { label: t('menu.exportList'), action: exportList },
        { label: t('menu.importList'), action: importList },
        SEP,
        { label: t('menu.exit'), shortcut: 'Ctrl+Q', action: () => api.quitApp() },
      ] as MenuItem[],
    },
    { label: t('menu.file'), items: selectionMenu() },
    {
      label: t('menu.downloads'),
      items: [
        { label: t('menu.resumeAll'), icon: 'play', action: () => api.startAll() },
        { label: t('menu.stopAll'), icon: 'stop', action: () => api.stopAll() },
        { label: t('menu.deleteCompleted'), action: deleteCompleted },
        SEP,
        { label: t('menu.scheduler'), icon: 'queues', action: () => api.openWindow('scheduler') },
        { label: t('menu.startQueue'), submenu: queueItems((q) => api.startQueue(q)) },
        { label: t('menu.stopQueue'), submenu: queueItems((q) => api.stopQueue(q)) },
        SEP,
        {
          label: t('menu.speedLimiter'),
          icon: 'speed',
          submenu: [
            { label: t('menu.speedLimiterOn'), radio: true, checked: limitKib != null, action: () => limitKib == null && toggleLimit() },
            { label: t('menu.speedLimiterOff'), radio: true, checked: limitKib == null, action: () => limitKib != null && toggleLimit() },
            SEP,
            { label: t('menu.speedLimiterSettings'), action: () => api.openWindow('options', { tab: 'connection' }) },
          ],
        },
        SEP,
        { label: t('menu.options'), icon: 'gear', action: () => api.openWindow('options') },
      ] as MenuItem[],
    },
    {
      label: t('menu.view'),
      items: [
        { label: t('menu.showCategories'), checked: showTree, action: () => (showTree = !showTree) },
        { label: t('menu.showToolbar'), checked: showToolbar, action: () => (showToolbar = !showToolbar) },
        { label: t('menu.showStatusbar'), checked: showStatus, action: () => (showStatus = !showStatus) },
        SEP,
        {
          label: t('menu.arrangeBy'),
          submenu: (['fileName', 'size', 'status', 'timeLeft', 'rate', 'lastTry', 'added', 'description'] as ColId[]).map((c) => ({
            label: t(`col.${c}`),
            radio: true,
            checked: sortBy === c,
            action: () => (sortBy = c),
          })),
        },
        {
          label: t('menu.theme'),
          submenu: (['light', 'dark', 'system'] as const).map((th) => ({
            label: t(`menu.theme${th[0].toUpperCase()}${th.slice(1)}`),
            radio: true,
            checked: app.settings?.theme === th,
            action: () => setTheme(th),
          })),
        },
        {
          label: t('menu.language'),
          submenu: [
            { label: t('opt.gen.lang.auto'), radio: true, checked: app.settings?.language === 'auto', action: () => setLang('auto') },
            { label: 'Türkçe', radio: true, checked: app.settings?.language === 'tr', action: () => setLang('tr') },
            { label: 'English', radio: true, checked: app.settings?.language === 'en', action: () => setLang('en') },
          ],
        },
      ] as MenuItem[],
    },
    {
      label: t('menu.help'),
      items: [
        { label: t('menu.help.wizard'), action: () => api.openWindow('wizard') },
        { label: t('menu.help.browsers'), icon: 'browser', action: () => api.openWindow('options', { tab: 'general' }) },
        { label: t('menu.help.tools'), action: () => api.openWindow('options', { tab: 'video' }) },
        SEP,
        { label: t('menu.help.about'), icon: 'info', action: () => api.openWindow('about') },
      ] as MenuItem[],
    },
  ]);

  const buttons = $derived<ToolButton[]>([
    { id: 'add', label: t('tb.addUrl'), icon: 'addUrl', action: () => api.openWindow('addurl') },
    { id: 'resume', label: t('tb.resume'), icon: 'resume', action: resume, disabled: !canResume },
    { id: 'stop', label: t('tb.stop'), icon: 'stop', action: stop, disabled: !canStop },
    { id: 'stopAll', label: t('tb.stopAll'), icon: 'stopAll', action: () => api.stopAll(), disabled: !activeRows.length && !rows.some((r) => r.status === 'queued') },
    { id: 'delete', label: t('tb.delete'), icon: 'delete', action: () => del(), disabled: !sel.length, sepBefore: true },
    { id: 'delDone', label: t('tb.deleteCompleted'), icon: 'deleteCompleted', action: deleteCompleted, disabled: !rows.some((r) => r.status === 'completed') },
    { id: 'options', label: t('tb.options'), icon: 'options', action: () => api.openWindow('options'), sepBefore: true },
    { id: 'sched', label: t('tb.scheduler'), icon: 'scheduler', action: () => api.openWindow('scheduler') },
    { id: 'qStart', label: t('tb.startQueue'), icon: 'startQueue', action: () => api.startQueue(currentQueue) },
    { id: 'qStop', label: t('tb.stopQueue'), icon: 'stopQueue', action: () => api.stopQueue(currentQueue) },
    { id: 'grab', label: t('tb.grabber'), icon: 'grabber', action: () => api.openWindow('grabber'), sepBefore: true },
    { id: 'video', label: t('tb.video'), icon: 'video', action: () => api.openWindow('media') },
  ]);

  function onContext(e: MouseEvent) {
    ctx = { x: e.clientX, y: e.clientY, items: sel.length ? selectionMenu() : emptyMenu() };
  }

  function treeContext(e: MouseEvent, f: Filter) {
    if (f.kind === 'queue' && f.id && f.id !== '*') {
      const qid = f.id;
      ctx = {
        x: e.clientX,
        y: e.clientY,
        items: [
          { label: t('menu.startQueue'), icon: 'play', action: () => api.startQueue(qid) },
          { label: t('menu.stopQueue'), icon: 'stop', action: () => api.stopQueue(qid) },
          SEP,
          { label: t('menu.scheduler'), action: () => api.openWindow('scheduler', { queue: qid }) },
        ],
      };
    } else if (f.kind === 'category') {
      ctx = {
        x: e.clientX,
        y: e.clientY,
        items: [{ label: t('opt.tab.saveTo') + '…', icon: 'folder', action: () => api.openWindow('options', { tab: 'saveTo' }) }],
      };
    }
  }

  function globalKey(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    if (target.matches('input, textarea, select')) return;
    if (e.key === 'Insert' || (e.ctrlKey && e.key.toLowerCase() === 'n')) {
      e.preventDefault();
      api.openWindow('addurl');
    } else if (e.ctrlKey && e.key.toLowerCase() === 'q') {
      e.preventDefault();
      api.quitApp();
    } else if (e.ctrlKey && e.key.toLowerCase() === 'v') {
      readClipboard().then((txt) => {
        const u = txt.trim();
        if (validUrl(u)) api.openWindow('addurl', { url: u });
      });
    }
  }

  // Sürükle bırak ile adres ekleme.
  let dragOver = $state(false);
  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const txt = e.dataTransfer?.getData('text/uri-list') || e.dataTransfer?.getData('text/plain') || '';
    const urls = txt.split(/\r?\n/).map((s) => s.trim()).filter((s) => s && !s.startsWith('#') && validUrl(s));
    if (urls.length === 1) api.openWindow('addurl', { url: urls[0] });
    else if (urls.length > 1) {
      import('../lib/types').then(({ emptyAddRequest }) => api.addMany(urls.map((u) => ({ ...emptyAddRequest(u), start: false }))));
    }
  }

  function startTreeResize(e: MouseEvent) {
    e.preventDefault();
    const x0 = e.clientX;
    const w0 = treeWidth;
    const move = (ev: MouseEvent) => (treeWidth = Math.max(130, Math.min(420, w0 + ev.clientX - x0)));
    const up = () => {
      window.removeEventListener('mousemove', move);
      window.removeEventListener('mouseup', up);
    };
    window.addEventListener('mousemove', move);
    window.addEventListener('mouseup', up);
  }

</script>

<svelte:window onkeydown={globalKey} />

<div
  class="main"
  class:drag={dragOver}
  role="application"
  ondragover={(e) => { e.preventDefault(); dragOver = true; }}
  ondragleave={(e) => { if (e.currentTarget === e.target) dragOver = false; }}
  ondrop={onDrop}
>
  <MenuBar {menus} />
  {#if showToolbar}<Toolbar {buttons} />{/if}
  <div class="content">
    {#if showTree}
      <div class="treebox" style:width="{treeWidth}px">
        <CategoryTree bind:filter {counts} onclose={() => (showTree = false)} oncontext={treeContext} />
      </div>
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div class="splitter" role="separator" onmousedown={startTreeResize}></div>
    {/if}
    <div class="listbox">
      <DownloadList
        rows={visible}
        bind:columns
        bind:selected
        bind:sortBy
        bind:sortDir
        onopen={openItem}
        oncontext={onContext}
        ondelete={() => del()}
        oncolumnschange={persistColumns}
      />
    </div>
  </div>
  {#if showStatus}
    <StatusBar items={visible.length} selected={selected.length} active={activeRows.length} speed={totalSpeed} {limitKib} ontogglelimit={toggleLimit} />
  {/if}
</div>

{#if ctx}
  <ContextMenu items={ctx.items} x={ctx.x} y={ctx.y} onclose={() => (ctx = null)} />
{/if}
{#if confirmBox}
  <ConfirmDialog message={confirmBox.message} checkLabel={confirmBox.check} onanswer={confirmBox.resolve} />
{/if}

<style>
  .main {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .main.drag {
    outline: 3px dashed #3c8de0;
    outline-offset: -3px;
  }
  .content {
    flex: 1;
    min-height: 0;
    display: flex;
    padding: 3px;
    gap: 0;
  }
  .treebox {
    flex: none;
    min-width: 0;
  }
  .splitter {
    width: 5px;
    cursor: col-resize;
    flex: none;
  }
  .listbox {
    flex: 1;
    min-width: 0;
  }
</style>
